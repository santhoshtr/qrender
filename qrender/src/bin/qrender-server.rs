//! Factoid web service.
//!
//! - GET /{lang}/{qid}       factoid HTML page
//! - GET /api/{lang}/{qid}   card IR as JSON (".json" suffix accepted)
//! - GET /healthz            liveness probe
//!
//! Environment: PORT (default 4243), REDIS_URL (optional cache), read
//! from the environment or a .env file, matching the Go qjson tool.

use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use std::sync::Arc;

use qrender::cards::synthesize;
use qrender::factoid::render_page;
use qrender::grouping::{GroupingConfig, load_grouping_config};

const CACHE_CONTROL: &str = "public, max-age=3600";

#[derive(Clone)]
struct AppState {
    qjson: Arc<qjson::Client>,
    grouping: Arc<GroupingConfig>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let state = AppState {
        qjson: Arc::new(qjson::Client::new()),
        grouping: Arc::new(load_grouping_config().expect("embedded groups.toml must parse")),
    };

    let app = Router::new()
        .route("/", get(usage))
        .route("/healthz", get(async || "ok"))
        .route("/{lang}/{qid}", get(factoid_page))
        .route("/api/{lang}/{qid}", get(factoid_json))
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "4243".to_string())
        .parse()
        .expect("PORT must be a number");
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind");
    println!("qrender-server listening on port {port}");
    axum::serve(listener, app).await.expect("server error");
}

async fn factoid_page(
    State(state): State<AppState>,
    Path((language, qid)): Path<(String, String)>,
) -> Response {
    let item = match state.qjson.get_item(&qid, &language).await {
        Ok(item) => item,
        Err(error) => return error_response(&error),
    };
    let page = synthesize(&item, &language, &state.grouping, true);
    match render_page(&page) {
        Ok(html) => ([(header::CACHE_CONTROL, CACHE_CONTROL)], Html(html)).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn factoid_json(
    State(state): State<AppState>,
    Path((language, qid)): Path<(String, String)>,
) -> Response {
    let qid = qid.trim_end_matches(".json");
    let item = match state.qjson.get_item(qid, &language).await {
        Ok(item) => item,
        Err(error) => return error_response(&error),
    };
    let page = synthesize(&item, &language, &state.grouping, true);
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, CACHE_CONTROL),
        ],
        serde_json::to_string(&page).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}")),
    )
        .into_response()
}

fn error_response(error: &qjson::QjsonError) -> Response {
    let status = match error {
        qjson::QjsonError::InvalidQid(_) | qjson::QjsonError::InvalidLanguage(_) => {
            StatusCode::BAD_REQUEST
        }
        _ => StatusCode::BAD_GATEWAY,
    };
    (status, error.to_string()).into_response()
}

async fn usage() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>QRender</title><meta name="color-scheme" content="dark light"></head>
<body>
<h1>QRender — Wikidata factoids</h1>
<p>Render a Wikidata item as a card page:</p>
<ul>
  <li><a href="/en/Q3870">/en/Q3870</a> — HTML factoid page</li>
  <li><a href="/api/en/Q3870">/api/en/Q3870</a> — card data as JSON</li>
</ul>
</body>
</html>"#,
    )
}
