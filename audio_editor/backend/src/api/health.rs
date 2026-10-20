//! `GET /api/health` — liveness + version (FR-20, arch §6).

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::api::Shared;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub version: String,
}

pub async fn handler(State(_state): State<Shared>) -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}
