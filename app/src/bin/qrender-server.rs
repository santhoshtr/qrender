//! The qrender web service.
//!
//! - GET /{lang}/{qid}               the page (HTML); "/{lang}/{qid}.md": Markdown
//! - GET /api/content/{qid}          content JSON
//! - GET /api/composition/{qid}      composition JSON (the same in every language)
//! - GET /api/view/{lang}/{qid}      view JSON
//! - GET /design/{version}/page.css  the design system
//! - GET /healthz
//!
//! Environment (or a .env file): PORT (default 4243), TOOL_REDIS_URI
//! (optional cache), REDIS_PREFIX (for every cache key), BASE_PATH (when
//! served under a subpath, e.g. "/qrender"), USER_AGENT.

use std::sync::Arc;

use app::Service;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use content::fetch::{FetchError, Mode};
use content::{Lang, Qid};
use qrender::render::design::{DESIGN, ECHARTS, STYLESHEET};
use qrender::render::html::{self, Styles};
use qrender::render::markdown;
use tower_http::compression::CompressionLayer;

const PAGE_CACHE: &str = "public, max-age=3600";
const ASSET_CACHE: &str = "public, max-age=604800, immutable";

#[derive(Clone)]
struct AppState {
    service: Arc<Service>,
    /// The mount point behind a reverse proxy, without a trailing slash.
    base_path: Arc<str>,
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
    };
    let app = Router::new()
        .route("/", get(usage))
        .route("/healthz", get(async || "ok"))
        .route("/design/{version}/{file}", get(design))
        .route("/api/content/{qid}", get(content_json))
        .route("/api/composition/{qid}", get(composition_json))
        .route("/api/view/{lang}/{qid}", get(view_json))
        .route("/{lang}/{qid}", get(page))
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

/// The version of the design system in its URL: "design/0.2" -> "0.2".
fn design_version() -> &'static str {
    DESIGN.rsplit('/').next().unwrap_or(DESIGN)
}

async fn design(Path((version, file)): Path<(String, String)>) -> Response {
    if version != design_version() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (body, content_type) = match file.as_str() {
        "page.css" => (STYLESHEET.as_str(), "text/css; charset=utf-8"),
        "echarts.js" => (ECHARTS, "text/javascript; charset=utf-8"),
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
    if markdown {
        return (
            [
                (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
                (header::CACHE_CONTROL, PAGE_CACHE),
            ],
            markdown::page(&view),
        )
            .into_response();
    }
    let base = format!("{}/design/{}", state.base_path, design_version());
    (
        [(header::CACHE_CONTROL, PAGE_CACHE)],
        Html(html::page(&view, Styles::Linked(&base))),
    )
        .into_response()
}

async fn content_json(State(state): State<AppState>, Path(qid): Path<String>) -> Response {
    let Ok(qid) = parse_qid(&qid) else {
        return bad_request("not an item id");
    };
    json(state.service.content(qid).await)
}

async fn composition_json(State(state): State<AppState>, Path(qid): Path<String>) -> Response {
    let Ok(qid) = parse_qid(&qid) else {
        return bad_request("not an item id");
    };
    json(state.service.composition(qid).await)
}

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
<li><a href="{b}/api/composition/Q3870">{b}/api/composition/Q3870</a>: the composition (JSON)</li>
<li><a href="{b}/api/view/en/Q3870">{b}/api/view/en/Q3870</a>: the view (JSON)</li>
<li><a href="{b}/api/content/Q3870">{b}/api/content/Q3870</a>: the content (JSON)</li>
</ul>
</body></html>"#
    ))
}
