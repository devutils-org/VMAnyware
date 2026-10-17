//! Deliberately small diagnostic events: no request-controlled text is logged.
use axum::{
    extract::{MatchedPath, Request},
    http::{HeaderValue, Method},
    middleware::Next,
    response::Response,
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) async fn request(request: Request, next: Next) -> Response {
    let request_id = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed).to_string();
    let method = match *request.method() {
        Method::GET => "GET",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::PATCH => "PATCH",
        Method::DELETE => "DELETE",
        Method::HEAD => "HEAD",
        Method::OPTIONS => "OPTIONS",
        _ => "OTHER",
    };
    let route = match request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
    {
        Some("/api/v1/health") => "/api/v1/health",
        Some("/api/v1/images") => "/api/v1/images",
        Some("/api/v1/provisioning/plan") => "/api/v1/provisioning/plan",
        Some("/api/v1/provisioning/preflight") => "/api/v1/provisioning/preflight",
        Some("/api/v1/provider/inventory") => "/api/v1/provider/inventory",
        Some("/api/v1/provider/inventory/refresh") => "/api/v1/provider/inventory/refresh",
        Some("/api/v1/operations/{id}") => "/api/v1/operations/{id}",
        Some("/api/v1/definitions") => "/api/v1/definitions",
        Some("/api/v1/definitions/validate") => "/api/v1/definitions/validate",
        Some("/api/v1/definitions/{name}") => "/api/v1/definitions/{name}",
        _ => "unmatched",
    };
    let started = Instant::now();
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).expect("numeric request ID"),
    );
    eprintln!(
        "{}",
        serde_json::json!({
            "event": "request_complete", "requestId": request_id,
            "method": method, "route": route, "status": response.status().as_u16(),
            "durationMs": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        })
    );
    response
}

pub fn listening(port: u16) {
    eprintln!(
        "{}",
        serde_json::json!({"event": "server_listening", "port": port})
    );
}

pub fn stopped() {
    eprintln!("{}", serde_json::json!({"event": "server_stopped"}));
}

pub fn drain_timed_out() {
    eprintln!("{}", serde_json::json!({"event": "server_drain_timed_out"}));
}

/// Reasons are static strings chosen by the service, never external errors.
pub fn failed(reason: &'static str) {
    eprintln!(
        "{}",
        serde_json::json!({"event": "server_failed", "reason": reason})
    );
}
