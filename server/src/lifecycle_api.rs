//! Explicit confirmed enqueue only; provider writes happen in the durable worker.
use crate::{
    error,
    inventory::AppState,
    lifecycle::{Action, Approval},
    store::LifecycleError,
};
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub async fn start(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<Approval>, JsonRejection>,
) -> Response {
    enqueue(state, path, headers, body, Action::Start).await
}
pub async fn shutdown(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<Approval>, JsonRejection>,
) -> Response {
    enqueue(state, path, headers, body, Action::Shutdown).await
}
async fn enqueue(
    state: AppState,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<Approval>, JsonRejection>,
    action: Action,
) -> Response {
    let key = match crate::operations_api::request_key(&headers) {
        Ok(key) => key,
        Err(error) => return error.into_response(),
    };
    if !state.lifecycle_enabled {
        return error(
            StatusCode::FORBIDDEN,
            "lifecycle_disabled",
            "Lifecycle execution is disabled on this control plane.",
        )
        .into_response();
    }
    let approval = match (path, body) {
        (Ok(Path(id)), Ok(Json(approval)))
            if approval.is_valid() && approval.instance_id == id && approval.action == action =>
        {
            approval
        }
        (_, Err(rejection)) => {
            return error(
                rejection.status(),
                "invalid_lifecycle_approval",
                "Send a bounded explicit confirmation of the complete target and action.",
            )
            .into_response()
        }
        _ => return problem(LifecycleError::InvalidApproval),
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before requesting lifecycle execution.",
        )
        .into_response();
    };
    if provider.scope_id() != approval.provider_scope_id {
        return error(
            StatusCode::CONFLICT,
            "provider_scope_changed",
            "The confirmed provider scope does not match this control plane.",
        )
        .into_response();
    }
    match tokio::task::spawn_blocking(move || state.store.enqueue_lifecycle(&key, &approval)).await
    {
        Ok(Ok(operation)) => (
            StatusCode::ACCEPTED,
            [(
                header::LOCATION,
                format!("/api/v1/operations/{}", operation.id),
            )],
            Json(operation),
        )
            .into_response(),
        Ok(Err(error)) => problem(error),
        Err(_) => problem(LifecycleError::Unavailable),
    }
}
fn problem(problem: LifecycleError) -> Response {
    let (status, code, message) = match problem {
        LifecycleError::InvalidApproval => (
            StatusCode::BAD_REQUEST,
            "invalid_lifecycle_approval",
            "Confirm the exact saved target, revision and action.",
        ),
        LifecycleError::InstanceNotFound => (
            StatusCode::NOT_FOUND,
            "instance_not_found",
            "Registered instance not found.",
        ),
        LifecycleError::TargetMismatch => (
            StatusCode::CONFLICT,
            "lifecycle_target_changed",
            "The confirmed target no longer matches the saved instance.",
        ),
        LifecycleError::Conflict => (
            StatusCode::CONFLICT,
            "operation_conflict",
            "This request conflicts with active work or a previously used request key.",
        ),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "Lifecycle operation storage is unavailable.",
        ),
    };
    error(status, code, message).into_response()
}
