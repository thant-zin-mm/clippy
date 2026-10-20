//! API error type: every failure maps to `{code, message, hint}` (FR-21).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),
}

impl ApiError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "E_NOT_FOUND",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Self::NotFound(_) => {
                "Check the URL path and HTTP method against the API contract (docs/architecture.md §6)."
            }
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
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
}
