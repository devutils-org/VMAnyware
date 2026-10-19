pub mod auth;
pub mod config;
pub mod definition;
mod definitions_api;
pub mod diagnostics;
pub mod instance;
mod instances_api;
mod inventory;
pub mod lifecycle;
mod lifecycle_api;
pub mod lifecycle_executor;
pub mod operation;
mod operations_api;
mod preflight;
pub mod provisioning;
mod provisioning_api;
pub mod provisioning_executor;
pub mod provisioning_intent;
pub mod proxmox;
pub mod service_lock;
pub mod store;
mod vm_api;
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
    router_with_catalog(provider, store, None)
}

pub fn router_with_catalog(
    provider: Option<proxmox::Proxmox>,
    store: store::Store,
    catalog: Option<provisioning::Catalog>,
) -> Router {
    router_with_lifecycle(provider, store, catalog, false)
}

pub fn router_with_lifecycle(
    provider: Option<proxmox::Proxmox>,
    store: store::Store,
    catalog: Option<provisioning::Catalog>,
    lifecycle_enabled: bool,
) -> Router {
    router_with_execution(provider, store, catalog, lifecycle_enabled, false)
}

pub fn router_with_execution(
    provider: Option<proxmox::Proxmox>,
    store: store::Store,
    catalog: Option<provisioning::Catalog>,
    lifecycle_enabled: bool,
    provisioning_enabled: bool,
) -> Router {
    Router::new()
        .route(
            "/api/v1/provisioning/create",
            post(provisioning_api::create).layer(DefaultBodyLimit::max(4096)),
        )
        .route("/api/v1/health", get(health))
        .route("/api/v1/images", get(provisioning_api::images))
        .route(
            "/api/v1/provisioning/plan",
            post(provisioning_api::preview)
                .layer(DefaultBodyLimit::max(6 * definition::MAX_YAML_BYTES + 1024)),
        )
        .route(
            "/api/v1/provisioning/preflight",
            post(provisioning_api::preflight)
                .layer(DefaultBodyLimit::max(6 * definition::MAX_YAML_BYTES + 1024)),
        )
        .route("/api/v1/provider/inventory", get(inventory::get))
        .route(
            "/api/v1/provider/nodes/{node}/vms/{vmid}/status",
            get(vm_api::get),
        )
        .route(
            "/api/v1/provider/inventory/refresh",
            post(operations_api::enqueue),
        )
        .route("/api/v1/operations", get(operations_api::list))
        .route(
            "/api/v1/environments",
            get(instances_api::list).post(instances_api::create),
        )
        .route("/api/v1/environments/{id}", get(instances_api::get))
        .route(
            "/api/v1/environments/{id}/start",
            post(lifecycle_api::start).layer(DefaultBodyLimit::max(4096)),
        )
        .route(
            "/api/v1/environments/{id}/shutdown",
            post(lifecycle_api::shutdown).layer(DefaultBodyLimit::max(4096)),
        )
        .route(
            "/api/v1/environments/{id}/lifecycle-preflight",
            get(instances_api::lifecycle_preflight),
        )
        .route(
            "/api/v1/environments/{id}/refresh",
            post(instances_api::refresh),
        )
        .route(
            "/api/v1/environments/{id}/status",
            get(instances_api::status),
        )
        .route("/api/v1/operations/{id}", get(operations_api::get))
        .route(
            "/api/v1/definitions",
            get(definitions_api::list).post(definitions_api::create),
        )
        .route(
            "/api/v1/definitions/validate",
            get(definitions_api::get_validate).post(definitions_api::validate),
        )
        .route("/api/v1/definitions/{name}", get(definitions_api::get))
        .layer(DefaultBodyLimit::max(definition::MAX_YAML_BYTES))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .with_state(inventory::AppState {
            provider,
            store,
            catalog,
            lifecycle_enabled,
            provisioning_enabled,
        })
        .layer(axum::middleware::from_fn(diagnostics::request))
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
