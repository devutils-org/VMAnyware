//! Confirmed idempotent snapshot enqueue; provider writes remain in the worker.
use crate::{error, inventory::AppState, snapshot::Approval, store::SnapshotError};
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub async fn create(
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
    if !state.snapshots_enabled {
        return error(
            StatusCode::FORBIDDEN,
            "snapshots_disabled",
            "Snapshot creation is disabled on this control plane.",
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
                "invalid_snapshot_approval",
                "Send a bounded explicit confirmation of the complete target and snapshot request.",
            )
            .into_response()
        }
        _ => return problem(SnapshotError::InvalidApproval),
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before requesting snapshot creation.",
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
    match tokio::task::spawn_blocking(move || state.store.enqueue_snapshot(&key, &approval)).await {
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
        Err(_) => problem(SnapshotError::Unavailable),
    }
}
fn problem(issue: SnapshotError) -> Response {
    let (status, code, message) = match issue {
        SnapshotError::InvalidApproval => (
            StatusCode::BAD_REQUEST,
            "invalid_snapshot_approval",
            "Confirm the exact saved target, revision and snapshot request.",
        ),
        SnapshotError::InstanceNotFound => (
            StatusCode::NOT_FOUND,
            "instance_not_found",
            "Registered instance not found.",
        ),
        SnapshotError::TargetMismatch => (
            StatusCode::CONFLICT,
            "snapshot_target_changed",
            "The confirmed target no longer matches the saved instance.",
        ),
        SnapshotError::Conflict => (
            StatusCode::CONFLICT,
            "operation_conflict",
            "This request conflicts with active work or a previously used request key.",
        ),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "Snapshot operation storage is unavailable.",
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
        "snapshot_operation_not_found",
        "Snapshot operation not found for this instance.",
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
        state.store.recover_snapshot(&instance_id, &operation_id)
    })
    .await
    {
        Ok(Ok(Some(recovered))) => Json(recovered).into_response(),
        Ok(Ok(None)) => missing_operation(),
        Ok(Err(issue)) => problem(issue),
        Err(_) => problem(SnapshotError::Unavailable),
    }
}
pub async fn history(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let instance_id = match path {
        Ok(Path(id)) if valid_id(&id) => id,
        _ => return problem(SnapshotError::InstanceNotFound),
    };
    match tokio::task::spawn_blocking(move || state.store.snapshot_history(&instance_id)).await {
        Ok(Ok(history)) => Json(history).into_response(),
        Ok(Err(issue)) => problem(issue),
        Err(_) => problem(SnapshotError::Unavailable),
    }
}
