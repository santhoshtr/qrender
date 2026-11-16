//! The qrender web service.
//!
//! - GET /{lang}/{qid}               the page (HTML); "/{lang}/{qid}.md": Markdown.
//!   A page needs a cookie that a script sets (see `challenge`)
//! - GET /api/content/{qid}          content JSON
//! - GET /api/composition/{qid}      composition JSON (the same in every language)
//! - GET /api/view/{lang}/{qid}      view JSON
//!   The API routes are off for now, while scrapers crawl the service.
//! - GET /design/{version}/page.css  the design system; also echarts.js,
//!   leaflet.js and leaflet.css, which the page scripts load. The version
//!   has a hash of the files, so a changed file has a new address.
//! - GET /healthz
//! - GET /favicon.svg                the icon of the page
//! - GET /robots.txt                 no crawling: an item page links to the
//!   pages of other items, and each new page is fetched live
//!
//! Environment (or a .env file): PORT (default 4243), TOOL_REDIS_URI
//! (optional cache), REDIS_PREFIX (for every cache key), BASE_PATH (when
//! served under a subpath, e.g. "/qrender"), USER_AGENT.

use std::sync::{Arc, LazyLock};
use std::time::Instant;

use app::Service;
use axum::Router;
use axum::extract::{MatchedPath, Path, Request, State};
use axum::http::{HeaderMap, HeaderName, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use content::fetch::{FetchError, Mode};
use content::{Lang, Qid};
use qrender::render::design::{DESIGN, ECHARTS, FAVICON, LEAFLET_CSS, LEAFLET_JS, STYLESHEET};
use qrender::render::html::{self, Styles};
use qrender::render::markdown;
use tokio::sync::Semaphore;
use tower_http::compression::CompressionLayer;

const PAGE_CACHE: &str = "public, max-age=3600";
const ASSET_CACHE: &str = "public, max-age=604800, immutable";
/// The icon has no version in its address: a day.
const ICON_CACHE: &str = "public, max-age=86400";
/// Item requests in progress at the same time. The fetcher queues its
/// upstream queries (two at a time to the query service); beyond this,
/// a request is refused at once instead of waiting in that queue.
const ITEMS_PARALLEL: usize = 16;
/// The wait a refused client is asked for, in seconds.
const RETRY_AFTER: &str = "30";
const SEC_FETCH_MODE: HeaderName = HeaderName::from_static("sec-fetch-mode");
/// The cookie a browser sets by running the script of the challenge.
const PASS: &str = "qrender-pass=1";
/// How long a browser keeps the pass: thirty days, in seconds.
const PASS_AGE: u32 = 30 * 24 * 3600;

#[derive(Clone)]
struct AppState {
    service: Arc<Service>,
    /// The mount point behind a reverse proxy, without a trailing slash.
    base_path: Arc<str>,
    /// A slot for each item request in progress.
    slots: Arc<Semaphore>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let service = Service::from_env(Mode::Live).expect("an HTTP client");
    let state = AppState {
        service: Arc::new(service),
        base_path: std::env::var("BASE_PATH")
            .unwrap_or_default()
            .trim_end_matches('/')
            .into(),
        slots: Arc::new(Semaphore::new(ITEMS_PARALLEL)),
    };
    // The routes that fetch: limited, and shed when busy.
    let items = Router::new()
        // Off for now, while scrapers crawl the service.
        // .route("/api/content/{qid}", get(content_json))
        // .route("/api/composition/{qid}", get(composition_json))
        // .route("/api/view/{lang}/{qid}", get(view_json))
        .route("/{lang}/{qid}", get(page))
        .route_layer(middleware::from_fn_with_state(state.clone(), shed))
        .route_layer(middleware::from_fn_with_state(state.clone(), challenge));
    let app = Router::new()
        .route("/", get(usage))
        .route("/healthz", get(async || "ok"))
        .route("/favicon.svg", get(favicon))
        .route("/robots.txt", get(async || "User-agent: *\nDisallow: /\n"))
        .route("/design/{version}/{file}", get(design))
        .merge(items)
        .layer(middleware::from_fn(log_request))
        .layer(CompressionLayer::new())
        .with_state(state);
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "4243".to_string())
        .parse()
        .expect("PORT is a number");
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("the port is free");
    println!("qrender-server on port {port}");
    axum::serve(listener, app).await.expect("the server runs");
}

/// The version of the design system: "design/0.2" -> "0.2".
fn design_system() -> &'static str {
    DESIGN.rsplit('/').next().unwrap_or(DESIGN)
}

/// The version of the design files in their URL: the version of the
/// design system and a hash of the files ("0.2-1a2b3c4d"). The files are
/// cached for a week as immutable, so a changed file needs a new address.
/// FNV-1a, 64 bit: the same in every build of the same files.
fn design_version() -> &'static str {
    static VERSION: LazyLock<String> = LazyLock::new(|| {
        let hash = [STYLESHEET.as_str(), ECHARTS, LEAFLET_JS, LEAFLET_CSS]
            .iter()
            .flat_map(|file| file.bytes().chain([0]))
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            });
        format!("{}-{:08x}", design_system(), hash as u32)
    });
    &VERSION
}

/// Each request as it arrives: who asks for what. Logged on arrival, so a
/// request that never completes is still seen. Sec-Fetch-Mode: a browser
/// sends it ("navigate" for a page); a client that only claims to be a
/// browser often does not.
async fn log_request(request: Request, next: Next) -> Response {
    {
        let field = |name| {
            request
                .headers()
                .get(name)
                .map_or("-", |value| value.to_str().unwrap_or("?"))
        };
        eprintln!(
            "request {} {:?} {}",
            request.uri(),
            field(header::USER_AGENT),
            field(SEC_FETCH_MODE)
        );
    }
    next.run(request).await
}

/// An item request runs only in a free slot; without one, 503 at once.
async fn shed(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Ok(_slot) = state.slots.try_acquire() else {
        eprintln!("shed {}", request.uri());
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::RETRY_AFTER, RETRY_AFTER)],
            "busy, try again later",
        )
            .into_response();
    };
    next.run(request).await
}

/// A page (HTML or Markdown) needs the pass cookie. Without it, a small
/// page whose script sets the cookie and loads the page again: a browser
/// passes in one round trip; a scraper that runs no script gets only this
/// page. 200, not an error, so that the scraper does not retry.
async fn challenge(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let is_page = request
        .extensions()
        .get::<MatchedPath>()
        .is_some_and(|path| path.as_str() == "/{lang}/{qid}");
    if !is_page || has_pass(request.headers()) {
        return next.run(request).await;
    }
    eprintln!("challenge {}", request.uri());
    let path = format!("{}/", state.base_path);
    (
        [(header::CACHE_CONTROL, "no-store")],
        Html(format!(
            r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>QRender</title><meta name="color-scheme" content="dark light"></head>
<body>
<p id="message">Loading the page…</p>
<noscript><p>This page needs JavaScript on the first visit.</p></noscript>
<script>
document.cookie = "{PASS}; Path={path}; Max-Age={PASS_AGE}; SameSite=Lax; Secure";
if ( document.cookie.includes( "{PASS}" ) ) {{
	location.reload();
}} else {{
	document.getElementById( "message" ).textContent = "This page needs cookies on the first visit.";
}}
</script>
</body></html>"#
        )),
    )
        .into_response()
}

/// Whether a Cookie header has the pass.
fn has_pass(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .any(|cookie| cookie.trim() == PASS)
}

async fn favicon() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, ICON_CACHE),
        ],
        FAVICON,
    )
        .into_response()
}

/// The files of the design system. A page that a browser cached with the
/// address of older files gets the files of this build.
async fn design(Path((version, file)): Path<(String, String)>) -> Response {
    if version.split('-').next() != Some(design_system()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (body, content_type) = match file.as_str() {
        "page.css" => (STYLESHEET.as_str(), "text/css; charset=utf-8"),
        "echarts.js" => (ECHARTS, "text/javascript; charset=utf-8"),
        "leaflet.js" => (LEAFLET_JS, "text/javascript; charset=utf-8"),
        "leaflet.css" => (LEAFLET_CSS, "text/css; charset=utf-8"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, ASSET_CACHE),
        ],
        body,
    )
        .into_response()
}

async fn page(
    State(state): State<AppState>,
    Path((lang, qid)): Path<(String, String)>,
) -> Response {
    let (qid, markdown) = match qid.strip_suffix(".md") {
        Some(q) => (q.to_string(), true),
        None => (qid, false),
    };
    let (qid, lang) = match parse(&qid, &lang) {
        Ok(ids) => ids,
        Err(response) => return response,
    };
    let view = match state.service.view(qid, &lang).await {
        Ok(view) => view,
        Err(e) => return error(&e),
    };
    let start = Instant::now();
    if markdown {
        let page = markdown::page(&view);
        eprintln!("render {qid}.{lang}.md: {:.1?}", start.elapsed());
        return (
            [
                (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
                (header::CACHE_CONTROL, PAGE_CACHE),
            ],
            page,
        )
            .into_response();
    }
    let base = format!("{}/design/{}", state.base_path, design_version());
    let page = html::page(&view, Styles::Linked(&base), Some(&state.base_path));
    eprintln!("render {qid}.{lang}: {:.1?}", start.elapsed());
    ([(header::CACHE_CONTROL, PAGE_CACHE)], Html(page)).into_response()
}

#[expect(dead_code, reason = "the API routes are off")]
async fn content_json(State(state): State<AppState>, Path(qid): Path<String>) -> Response {
    let Ok(qid) = parse_qid(&qid) else {
        return bad_request("not an item id");
    };
    json(state.service.content(qid).await)
}

#[expect(dead_code, reason = "the API routes are off")]
async fn composition_json(State(state): State<AppState>, Path(qid): Path<String>) -> Response {
    let Ok(qid) = parse_qid(&qid) else {
        return bad_request("not an item id");
    };
    json(state.service.composition(qid).await)
}

#[expect(dead_code, reason = "the API routes are off")]
async fn view_json(
    State(state): State<AppState>,
    Path((lang, qid)): Path<(String, String)>,
) -> Response {
    let (qid, lang) = match parse(&qid, &lang) {
        Ok(ids) => ids,
        Err(response) => return response,
    };
    json(state.service.view(qid, &lang).await)
}

#[expect(dead_code, reason = "the API routes are off")]
fn json<T: serde::Serialize>(result: Result<T, FetchError>) -> Response {
    match result {
        Ok(value) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, PAGE_CACHE),
            ],
            serde_json::to_string(&value).unwrap_or_default(),
        )
            .into_response(),
        Err(e) => error(&e),
    }
}

/// A QID in a path: "Q42", also "q42" and "Q42.json".
fn parse_qid(qid: &str) -> Result<Qid, ()> {
    let qid = qid.strip_suffix(".json").unwrap_or(qid);
    qid.to_uppercase().parse().map_err(|_| ())
}

#[allow(clippy::result_large_err)]
fn parse(qid: &str, lang: &str) -> Result<(Qid, Lang), Response> {
    let qid = parse_qid(qid).map_err(|()| bad_request("not an item id"))?;
    let lang = lang
        .parse()
        .map_err(|_| bad_request("not a language code"))?;
    Ok((qid, lang))
}

fn bad_request(why: &'static str) -> Response {
    (StatusCode::BAD_REQUEST, why).into_response()
}

/// No such item: 404. Any other failure is upstream: 502.
fn error(e: &FetchError) -> Response {
    let status = match e {
        FetchError::NoItem(_) => StatusCode::NOT_FOUND,
        _ => StatusCode::BAD_GATEWAY,
    };
    eprintln!("qrender-server: {e}");
    (status, e.to_string()).into_response()
}

async fn usage(State(state): State<AppState>) -> Html<String> {
    let b = &state.base_path;
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>QRender</title><meta name="color-scheme" content="dark light"></head>
<body>
<h1>QRender</h1>
<p>A page for a Wikidata item, composed by rule, in any language.</p>
<ul>
<li><a href="{b}/en/Q3870">{b}/en/Q3870</a>: the page</li>
<li><a href="{b}/en/Q3870.md">{b}/en/Q3870.md</a>: the page as Markdown</li>
</ul>
</body></html>"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_design_route_serves_its_files() {
        let get = |file: &str| design(Path((design_version().to_string(), file.to_string())));
        for (file, kind) in [
            ("page.css", "text/css"),
            ("echarts.js", "text/javascript"),
            ("leaflet.js", "text/javascript"),
            ("leaflet.css", "text/css"),
        ] {
            let response = get(file).await;
            assert_eq!(response.status(), StatusCode::OK, "{file}");
            let content_type = response.headers()[header::CONTENT_TYPE].to_str().unwrap();
            assert!(content_type.starts_with(kind), "{file}: {content_type}");
        }
        assert_eq!(get("other.js").await.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn the_pass_is_found_among_the_cookies() {
        let with = |cookies: &[&str]| {
            let mut headers = HeaderMap::new();
            for cookie in cookies {
                headers.append(header::COOKIE, cookie.parse().unwrap());
            }
            has_pass(&headers)
        };
        assert!(with(&[PASS]));
        assert!(with(&["a=b; qrender-pass=1; c=d"]));
        assert!(with(&["a=b", "qrender-pass=1"]));
        assert!(!with(&[]));
        assert!(!with(&["qrender-pass=0", "xqrender-pass=1"]));
    }

    #[tokio::test]
    async fn the_design_version_follows_the_files() {
        let version = design_version();
        assert!(
            version.starts_with(&format!("{}-", design_system())),
            "{version}"
        );
        // The address of older files of the same design system: the files
        // of this build. Another design system: none.
        let older = format!("{}-00000000", design_system());
        let response = design(Path((older, "page.css".to_string()))).await;
        assert_eq!(response.status(), StatusCode::OK);
        let other = design(Path(("9.9".to_string(), "page.css".to_string()))).await;
        assert_eq!(other.status(), StatusCode::NOT_FOUND);
    }
}
