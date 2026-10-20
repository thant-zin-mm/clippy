//! API error type: every failure maps to `{code, message, hint}` (FR-21).
//!
//! Codes are stable strings; they are the contract with the frontend, which
//! surfaces `hint` to the user (FR-21, ui-guide §error-copy). Never leak
//! internal paths or stack traces here (arch §7).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("source not found: {0}")]
    SourceNotFound(String),

    #[error("job not found: {0}")]
    JobNotFound(String),

    #[error("unsupported or unrecognized audio format")]
    UnsupportedFormat,

    #[error("the audio file appears to be corrupt or truncated")]
    CorruptFile,

    #[error("upload exceeds the size limit of {limit_bytes} bytes")]
    UploadTooLarge { limit_bytes: u64 },

    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("source is not prepared yet")]
    NotPrepared,

    #[error("source has already been prepared")]
    AlreadyPrepared,

    #[error("prepare failed: {0}")]
    PrepareFailed(String),
}

impl ApiError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "E_NOT_FOUND",
            Self::SourceNotFound(_) => "E_SOURCE_NOT_FOUND",
            Self::JobNotFound(_) => "E_JOB_NOT_FOUND",
            Self::UnsupportedFormat => "E_UNSUPPORTED_FORMAT",
            Self::CorruptFile => "E_CORRUPT_FILE",
            Self::UploadTooLarge { .. } => "E_UPLOAD_TOO_LARGE",
            Self::InvalidParameter(_) => "E_INVALID_PARAMETER",
            Self::NotPrepared => "E_NOT_PREPARED",
            Self::AlreadyPrepared => "E_ALREADY_PREPARED",
            Self::PrepareFailed(_) => "E_PREPARE_FAILED",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::NotFound(_) => {
                "Check the URL path and HTTP method against the API contract (docs/architecture.md §6)."
            }
            Self::SourceNotFound(_) => {
                "That source no longer exists. Import the file again to start a new session."
            }
            Self::JobNotFound(_) => {
                "The job id is unknown or expired. Re-run the action that created it."
            }
            Self::UnsupportedFormat => {
                "Use an MP3 or WAV file. Other formats are not supported in v1 (PRD D2)."
            }
            Self::CorruptFile => {
                "The file may be truncated or damaged. Re-export it from the source app and retry."
            }
            Self::UploadTooLarge { .. } => {
                "Split the recording into smaller parts, or raise SILENCE_CUTTER_MAX_UPLOAD_BYTES."
            }
            Self::InvalidParameter(_) => {
                "A request parameter was out of range. Reload the page and try again."
            }
            Self::NotPrepared => "Prepare the source before requesting its peaks.",
            Self::AlreadyPrepared => {
                "This source is already prepared. Import a new file to work on another."
            }
            Self::PrepareFailed(_) => "Preparing the audio failed. Retry, or try a different file.",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) | Self::SourceNotFound(_) | Self::JobNotFound(_) => {
                StatusCode::NOT_FOUND
            }
            Self::UnsupportedFormat | Self::CorruptFile => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::UploadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            Self::InvalidParameter(_) => StatusCode::BAD_REQUEST,
            Self::NotPrepared | Self::AlreadyPrepared => StatusCode::CONFLICT,
            Self::PrepareFailed(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = json!({
            "code": self.code(),
            "message": self.to_string(),
            "hint": self.hint(),
        });
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_metadata_is_stable() {
        let error = ApiError::NotFound("/nope".into());
        assert_eq!(error.code(), "E_NOT_FOUND");
        assert_eq!(error.status(), StatusCode::NOT_FOUND);
        assert!(!error.hint().is_empty());
    }

    #[test]
    fn upload_too_large_carries_limit_and_maps_status() {
        let error = ApiError::UploadTooLarge {
            limit_bytes: 4294967296,
        };
        assert_eq!(error.code(), "E_UPLOAD_TOO_LARGE");
        assert_eq!(error.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert!(error.hint().contains("SILENCE_CUTTER_MAX_UPLOAD_BYTES"));
    }

    #[test]
    fn format_and_corrupt_map_to_415() {
        assert_eq!(
            ApiError::UnsupportedFormat.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            ApiError::CorruptFile.status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(ApiError::UnsupportedFormat.code(), "E_UNSUPPORTED_FORMAT");
        assert_eq!(ApiError::CorruptFile.code(), "E_CORRUPT_FILE");
    }

    #[test]
    fn conflict_codes_map_to_409() {
        assert_eq!(ApiError::NotPrepared.status(), StatusCode::CONFLICT);
        assert_eq!(ApiError::AlreadyPrepared.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn every_variant_has_nonempty_hint_and_unique_code() {
        let all = [
            ApiError::NotFound("x".into()),
            ApiError::SourceNotFound("x".into()),
            ApiError::JobNotFound("x".into()),
            ApiError::UnsupportedFormat,
            ApiError::CorruptFile,
            ApiError::UploadTooLarge { limit_bytes: 1 },
            ApiError::InvalidParameter("x".into()),
            ApiError::NotPrepared,
            ApiError::AlreadyPrepared,
            ApiError::PrepareFailed("x".into()),
        ];
        let mut codes: Vec<&str> = all.iter().map(ApiError::code).collect();
        let before = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), before, "error codes must be unique");
        for error in all {
            assert!(!error.hint().is_empty(), "hint empty for {}", error.code());
            assert!(!error.to_string().is_empty());
        }
    }
}
