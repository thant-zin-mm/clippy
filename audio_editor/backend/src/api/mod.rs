//! HTTP API surface (arch §6). Route modules grow per plan task.

mod health;

use axum::http::{header, Method, Request};
use axum::routing::get;
use axum::Router;
use http::HeaderValue;
use tower_http::cors::CorsLayer;
use tracing::warn;

use crate::config::{Config, DEFAULT_ORIGINS};
use crate::error::ApiError;
use crate::state::AppState;

/// Build the application router (stateful, CORS-configured).
pub fn router(config: Config) -> Router {
    let cors = build_cors(&config);
    Router::new()
        .route("/api/health", get(health::handler))
        .fallback(fallback_404)
        .layer(cors)
        .with_state(AppState)
}

/// Explicit-origin CORS layer. Never `*`: the coding guide forbids an open
/// allowlist. If the configured list is empty or wholly invalid, fall back to
/// the documented default origins (Vite dev/preview) rather than allowing any.
pub fn build_cors(config: &Config) -> CorsLayer {
    let parse_origins = |list: &[String]| -> Vec<HeaderValue> {
        list.iter()
            .filter_map(|origin| match origin.parse() {
                Ok(value) => Some(value),
                Err(err) => {
                    warn!(origin, error = %err, "skipping unparseable origin");
                    None
                }
            })
            .collect()
    };

    let origins = parse_origins(&config.allowed_origins);
    let origins = if origins.is_empty() {
        warn!(
            "CORS allowlist empty or all entries invalid; \
             falling back to the default origins (never an open allowlist)"
        );
        let defaults: Vec<String> = DEFAULT_ORIGINS
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect();
        parse_origins(&defaults)
    } else {
        origins
    };

    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .allow_origin(origins)
}

async fn fallback_404(request: Request<axum::body::Body>) -> ApiError {
    ApiError::NotFound(request.uri().path().to_owned())
}
