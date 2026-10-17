use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::router;

#[tokio::test]
async fn inventory_requires_explicit_provider_configuration() {
    let response = router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/provider/inventory")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"]["code"], "provider_not_configured");
}

#[tokio::test]
async fn health_returns_only_liveness_without_configuration() {
    let response = router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value, json!({ "status": "ok" }));
}

#[tokio::test]
async fn unsupported_routes_and_methods_return_stable_safe_errors() {
    let cases = [
        (
            Method::GET,
            "/api/v1/environments?token=do-not-echo",
            StatusCode::NOT_FOUND,
            "not_found",
            "Route not found.",
        ),
        (
            Method::POST,
            "/api/v1/health",
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "Method not allowed.",
        ),
    ];

    for (method, uri, status, code, message) in cases {
        let response = router()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value,
            json!({ "error": { "code": code, "message": message } })
        );
    }
}
