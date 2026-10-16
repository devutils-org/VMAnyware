pub mod config;
pub mod definition;
mod definitions_api;
mod inventory;
pub mod operation;
mod operations_api;
pub mod proxmox;
pub mod service_lock;
pub mod store;
pub mod worker;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

/// Build the local API without binding a socket or contacting a provider.
pub fn router() -> Router {
    router_with_provider(None)
}

pub fn router_with_provider(provider: Option<proxmox::Proxmox>) -> Router {
    router_with_services(
        provider,
        store::Store::memory().expect("in-memory database initialization failed"),
    )
}

pub fn router_with_services(provider: Option<proxmox::Proxmox>, store: store::Store) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/provider/inventory", get(inventory::get))
        .route(
            "/api/v1/provider/inventory/refresh",
            post(operations_api::enqueue),
        )
        .route("/api/v1/operations/{id}", get(operations_api::get))
        .route(
            "/api/v1/definitions",
            get(definitions_api::list).post(definitions_api::create),
        )
        .route(
            "/api/v1/definitions/validate",
            post(definitions_api::validate),
        )
        .route("/api/v1/definitions/{name}", get(definitions_api::get))
        .layer(DefaultBodyLimit::max(definition::MAX_YAML_BYTES))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .with_state(inventory::AppState { provider, store })
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Serialize)]
struct ApiError {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}

fn error(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: ErrorDetail { code, message },
        }),
    )
}

async fn not_found() -> (StatusCode, Json<ApiError>) {
    error(StatusCode::NOT_FOUND, "not_found", "Route not found.")
}

async fn method_not_allowed() -> (StatusCode, Json<ApiError>) {
    error(
        StatusCode::METHOD_NOT_ALLOWED,
        "method_not_allowed",
        "Method not allowed.",
    )
}
