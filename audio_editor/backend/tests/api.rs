//! Integration tests for the P0-T1 API surface.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use silence_cutter::api;
use silence_cutter::config::Config;

fn test_router() -> axum::Router {
    api::router(Config::parse(None, None, None, None))
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("response body is JSON")
}

#[tokio::test]
async fn health_returns_ok_and_version() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn unknown_route_returns_json_404() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_json(response).await;
    assert_eq!(body["code"], "E_NOT_FOUND");
    assert!(body["message"].as_str().unwrap().contains("/api/nope"));
    assert!(!body["hint"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn cors_allows_configured_origin() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/health")
                .header(header::ORIGIN, "http://localhost:5173")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:5173")
    );
}

#[tokio::test]
async fn cors_omits_header_for_unknown_origin() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .header(header::ORIGIN, "http://evil.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}

#[tokio::test]
async fn cors_empty_allowlist_falls_back_to_default_origins() {
    // SILENCE_CUTTER_ORIGINS=" , " (or all-invalid) must NOT open the API to
    // any origin (coding-guide §7 forbids `*`); it falls back to the Vite
    // dev/preview origins instead.
    let config = Config::parse(None, Some(" , "), None, None);
    assert!(config.allowed_origins.is_empty());

    let response = api::router(config)
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/health")
                .header(header::ORIGIN, "http://localhost:5173")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:5173")
    );

    // …and an unknown origin is still denied.
    let response = api::router(Config::parse(None, Some(" , "), None, None))
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .header(header::ORIGIN, "http://evil.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}
