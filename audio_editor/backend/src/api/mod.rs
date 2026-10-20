//! HTTP API surface (arch §6). Route modules grow per plan task.

mod health;
mod jobs;
mod sources;

use std::sync::Arc;
use std::time::Duration;

use axum::http::{header, Method, Request};
use axum::routing::get;
use axum::Router;
use http::HeaderValue;
use tower_http::cors::CorsLayer;
use tracing::warn;

use crate::config::{Config, DEFAULT_ORIGINS};
use crate::error::ApiError;
use crate::jobs::JobManager;
use crate::state::AppState;

/// Shared axum state carried by every handler: registry + config + job manager.
pub type Shared = (AppState, Config, JobManager);

/// Upper bound on the upload request body (arch §6, PRD upload size guard).
/// The per-process cap is additionally enforced while streaming the multipart
/// body so a large upload never gets fully buffered before we can reject it.
pub const UPLOAD_BODY_LIMIT_BYTES: usize = 4 * 1024 * 1024 * 1024;

/// Build the application router (stateful, CORS-configured, job worker spawned).
pub fn router(config: Config) -> Router {
    let state = AppState::new();
    let jobs = JobManager::new(state.clone());

    // Global serial worker: prepares run off the async runtime.
    Arc::new(jobs.clone()).spawn_worker();

    // Periodic idle TTL sweep so sources whose tabs vanished without DELETE
    // do not leak temp files (arch §3.1).
    spawn_idle_sweeper(state.clone(), Duration::from_secs(config.tmp_ttl_secs));

    build_router_with_state(state, config, jobs)
}

/// Build the router from an existing shared state (used by tests).
pub fn build_router_with_state(state: AppState, config: Config, jobs: JobManager) -> Router {
    let cors = build_cors(&config);
    let shared: Shared = (state, config, jobs);

    Router::new()
        .route("/api/health", get(health::handler))
        .nest("/api/sources", sources::routes())
        .nest("/api/jobs", jobs::routes())
        .fallback(fallback_404)
        .layer(axum::extract::DefaultBodyLimit::max(
            UPLOAD_BODY_LIMIT_BYTES,
        ))
        .layer(cors)
        .with_state::<()>(shared)
}

fn spawn_idle_sweeper(state: AppState, ttl: Duration) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        let removed = state.sweep_idle(ttl);
        if !removed.is_empty() {
            tracing::info!(count = removed.len(), "idle sweep removed stale sources");
        }
    });
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
