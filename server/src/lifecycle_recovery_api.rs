//! Scoped historical GETs; no provider, write capability or consent preparation.
use crate::{error, inventory::AppState, store::LifecycleError};
use axum::{
    extract::{rejection::PathRejection, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn missing() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "lifecycle_operation_not_found",
        "Saved lifecycle instance or operation not found.",
    )
    .into_response()
}
fn unavailable() -> Response {
    error(StatusCode::SERVICE_UNAVAILABLE,"lifecycle_recovery_unavailable","Cannot read saved lifecycle evidence. Check the control-plane journal before another action.").into_response()
}
pub async fn recover(
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Response {
    let (instance, operation) = match path {
        Ok(Path((instance, operation))) if valid_id(&instance) && valid_id(&operation) => {
            (instance, operation)
        }
        _ => return missing(),
    };
    match tokio::task::spawn_blocking(move || state.store.recover_lifecycle(&instance, &operation))
        .await
    {
        Ok(Ok(Some(saved))) => Json(saved).into_response(),
        Ok(Ok(None)) => missing(),
        _ => unavailable(),
    }
}
pub async fn history(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let instance = match path {
        Ok(Path(id)) if valid_id(&id) => id,
        _ => return missing(),
    };
    match tokio::task::spawn_blocking(move || state.store.lifecycle_history(&instance)).await {
        Ok(Ok(history)) => Json(history).into_response(),
        Ok(Err(LifecycleError::InstanceNotFound)) => missing(),
        _ => unavailable(),
    }
}
