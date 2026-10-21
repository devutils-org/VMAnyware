//! Confirmed idempotent deletion enqueue; provider writes remain in the worker.
use crate::{deletion::Approval, error, inventory::AppState, store::DeletionError};
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub async fn delete(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<Approval>, JsonRejection>,
) -> Response {
    let key = match crate::operations_api::request_key(&headers) {
        Ok(key) => key,
        Err(problem) => return problem.into_response(),
    };
    if headers.get_all("idempotency-key").iter().count() != 1 {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_request_key",
            "Supply exactly one Idempotency-Key.",
        )
        .into_response();
    }
    if !state.deletions_enabled {
        return error(
            StatusCode::FORBIDDEN,
            "deletions_disabled",
            "VM deletion is disabled on this control plane.",
        )
        .into_response();
    }
    let approval = match (path, body) {
        (Ok(Path(id)), Ok(Json(approval))) if approval.is_valid() && approval.instance_id == id => {
            approval
        }
        (_, Err(rejection)) => {
            return error(
                rejection.status(),
                "invalid_deletion_approval",
                "Send a bounded explicit confirmation of the complete target, reviewed cleanup plan and data loss.",
            )
            .into_response()
        }
        _ => return problem(DeletionError::InvalidApproval),
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before requesting VM deletion.",
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
    match tokio::task::spawn_blocking(move || state.store.enqueue_deletion(&key, &approval)).await {
        Ok(Ok(operation)) => (
            StatusCode::ACCEPTED,
            [(
                header::LOCATION,
                format!("/api/v1/operations/{}", operation.id),
            )],
            Json(operation),
        )
            .into_response(),
        Ok(Err(issue)) => problem(issue),
        Err(_) => problem(DeletionError::Unavailable),
    }
}
fn problem(issue: DeletionError) -> Response {
    let (status, code, message) = match issue {
        DeletionError::InvalidApproval => (
            StatusCode::BAD_REQUEST,
            "invalid_deletion_approval",
            "Confirm the exact saved target, digest, complete cleanup plan and loss of VM data.",
        ),
        DeletionError::InstanceNotFound => (
            StatusCode::NOT_FOUND,
            "instance_not_found",
            "Registered instance not found.",
        ),
        DeletionError::TargetMismatch => (
            StatusCode::CONFLICT,
            "deletion_target_changed",
            "The confirmed target no longer matches the saved instance.",
        ),
        DeletionError::Conflict => (
            StatusCode::CONFLICT,
            "operation_conflict",
            "This request conflicts with active work or a previously used request key.",
        ),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "Deletion operation storage is unavailable.",
        ),
    };
    error(status, code, message).into_response()
}

fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn missing_operation() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "deletion_operation_not_found",
        "Deletion operation not found for this instance.",
    )
    .into_response()
}
pub async fn recover(
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Response {
    let (instance_id, operation_id) = match path {
        Ok(Path((instance_id, operation_id)))
            if valid_id(&instance_id) && valid_id(&operation_id) =>
        {
            (instance_id, operation_id)
        }
        _ => return missing_operation(),
    };
    match tokio::task::spawn_blocking(move || {
        state.store.recover_deletion(&instance_id, &operation_id)
    })
    .await
    {
        Ok(Ok(Some(recovered))) => Json(recovered).into_response(),
        Ok(Ok(None)) => missing_operation(),
        Ok(Err(issue)) => problem(issue),
        Err(_) => problem(DeletionError::Unavailable),
    }
}
pub async fn history(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let instance_id = match path {
        Ok(Path(id)) if valid_id(&id) => id,
        _ => return problem(DeletionError::InstanceNotFound),
    };
    match tokio::task::spawn_blocking(move || state.store.deletion_history(&instance_id)).await {
        Ok(Ok(history)) => Json(history).into_response(),
        Ok(Err(issue)) => problem(issue),
        Err(_) => problem(DeletionError::Unavailable),
    }
}
