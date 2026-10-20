//! Source lifecycle endpoints (arch §6): upload, prepare, delete, peaks.
//!
//! - `POST /api/sources` — streamed multipart → temp file (never fully
//!   buffered), size-guarded, returns `{source_id}`.
//! - `POST /api/sources/{id}/prepare` — queue a prepare job → `{job_id}`.
//! - `DELETE /api/sources/{id}` — drop source + derived caches + temp file.
//! - `GET /api/sources/{id}/peaks` — windowed min/max pairs as binary i16 LE.

use std::io::Write;
use std::time::Duration;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::api::Shared;
use crate::audio::peaks::PeakPyramid;
use crate::error::ApiError;
use crate::state::{new_source_id, new_temp_path, sanitize_filename};

/// Multipart field name the frontend uses for the audio file.
pub const FILE_FIELD: &str = "file";

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub source_id: String,
}

#[derive(Debug, Serialize)]
pub struct PrepareResponse {
    pub job_id: String,
}

/// Router subtree for the source routes.
pub fn routes() -> axum::Router<Shared> {
    axum::Router::new()
        .route("/", axum::routing::post(upload))
        .route("/{id}", axum::routing::delete(delete))
        .route("/{id}/prepare", axum::routing::post(prepare))
        .route("/{id}/peaks", axum::routing::get(peaks))
}

/// `POST /api/sources` — stream a multipart upload to a temp file.
pub async fn upload(
    State((state, config, _jobs)): State<Shared>,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    // Idle safety net: drop sources whose tabs vanished without DELETE.
    state.sweep_idle(Duration::from_secs(config.tmp_ttl_secs));

    let mut uploaded: Option<String> = None;
    let mut multipart = multipart;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::InvalidParameter("malformed multipart body".into()))?
    {
        if field.name() != Some(FILE_FIELD) {
            continue;
        }

        let raw_name = field.file_name().unwrap_or("").to_owned();
        let filename = sanitize_filename(&raw_name)?;

        let source_id = new_source_id();
        let temp_path = new_temp_path(&source_id);
        let mut file = std::fs::File::create(&temp_path)
            .map_err(|e| ApiError::PrepareFailed(format!("could not create temp file: {e}")))?;

        let mut written: u64 = 0;
        loop {
            let chunk = field
                .chunk()
                .await
                .map_err(|_| ApiError::InvalidParameter("malformed multipart chunk".into()))?;
            let Some(bytes) = chunk else { break };
            written += bytes.len() as u64;
            if written > config.max_upload_bytes {
                drop(file);
                let _ = std::fs::remove_file(&temp_path);
                return Err(ApiError::UploadTooLarge {
                    limit_bytes: config.max_upload_bytes,
                });
            }
            file.write_all(&bytes)
                .map_err(|e| ApiError::PrepareFailed(format!("disk write failed: {e}")))?;
        }

        file.flush()
            .map_err(|e| ApiError::PrepareFailed(format!("disk flush failed: {e}")))?;
        drop(file);

        if written == 0 {
            let _ = std::fs::remove_file(&temp_path);
            return Err(ApiError::InvalidParameter("uploaded file is empty".into()));
        }

        state.insert(source_id.clone(), filename, temp_path);
        uploaded = Some(source_id);
        break;
    }

    let source_id =
        uploaded.ok_or_else(|| ApiError::InvalidParameter("no file field in upload".into()))?;
    Ok(Json(UploadResponse { source_id }))
}

/// `POST /api/sources/{id}/prepare` — queue a prepare job.
pub async fn prepare(
    State((state, _config, jobs)): State<Shared>,
    Path(id): Path<String>,
) -> Result<Json<PrepareResponse>, ApiError> {
    {
        let reg = state.lock();
        let rec = reg
            .get(&id)
            .ok_or_else(|| ApiError::SourceNotFound(id.clone()))?;
        if rec.prepared.is_some() {
            return Err(ApiError::AlreadyPrepared);
        }
    }

    let job_id = jobs.enqueue_prepare(&id)?;
    Ok(Json(PrepareResponse { job_id }))
}

/// `DELETE /api/sources/{id}` — drop source + caches + temp file.
pub async fn delete(
    State((state, _config, _jobs)): State<Shared>,
    Path(id): Path<String>,
) -> Response {
    match state.remove(&id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => ApiError::SourceNotFound(id).into_response(),
    }
}

/// Query parameters for the peaks endpoint.
#[derive(Debug, Deserialize)]
pub struct PeaksParams {
    pub start_sample: Option<usize>,
    pub end_sample: Option<usize>,
    pub columns: Option<usize>,
}

/// `GET /api/sources/{id}/peaks` — windowed min/max pairs as binary i16 LE.
pub async fn peaks(
    State((state, _config, _jobs)): State<Shared>,
    Path(id): Path<String>,
    Query(params): Query<PeaksParams>,
) -> Result<Response, ApiError> {
    let columns = params.columns.unwrap_or(256);
    if columns == 0 || columns > crate::audio::peaks::MAX_COLUMNS {
        return Err(ApiError::InvalidParameter(format!(
            "columns must be in 1..={}",
            crate::audio::peaks::MAX_COLUMNS
        )));
    }

    let (pyramid, sample_rate, duration_ms) = {
        let reg = state.lock();
        let rec = reg
            .get(&id)
            .ok_or_else(|| ApiError::SourceNotFound(id.clone()))?;
        let prepared = rec.prepared.as_ref().ok_or(ApiError::NotPrepared)?;
        (
            prepared.peak_pyramid.clone(),
            prepared.meta.sample_rate,
            prepared.meta.duration_ms,
        )
    };

    let start_sample = params.start_sample.unwrap_or(0);
    let end_sample = params
        .end_sample
        .unwrap_or(duration_ms as usize * sample_rate as usize / 1000);

    let pairs = pyramid.window(start_sample, end_sample, columns)?;
    let bytes = PeakPyramid::encode_pairs(&pairs);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(axum::body::Body::from(bytes))
        .expect("response builder");
    Ok(response)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_field_name_is_stable() {
        assert_eq!(FILE_FIELD, "file");
    }

    #[test]
    fn peaks_params_deserialize_with_defaults() {
        let params: PeaksParams = serde_urlencoded::from_str("").unwrap();
        assert!(params.start_sample.is_none());
        assert!(params.columns.is_none());
    }
}
