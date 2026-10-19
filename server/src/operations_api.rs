use crate::{error, inventory::AppState, store, ApiError};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub(crate) fn request_key(headers: &HeaderMap) -> Result<String, (StatusCode, Json<ApiError>)> {
    if headers.contains_key(header::ORIGIN) {
        return Err(error(
            StatusCode::FORBIDDEN,
            "origin_forbidden",
            "Browser origins cannot start local operations.",
        ));
    }
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok());
    match key {
        Some(key) if crate::operation::valid_request_key(key) => Ok(key.to_owned()),
        _ => Err(error(
            StatusCode::BAD_REQUEST,
            "invalid_request_key",
            "Supply an Idempotency-Key of 1–64 letters, digits, dots, hyphens or underscores.",
        )),
    }
}

pub async fn enqueue(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let key = match request_key(&headers) {
        Ok(key) => key,
        Err(response) => return response.into_response(),
    };
    match tokio::task::spawn_blocking(move || state.store.enqueue_inventory(&key)).await {
        Ok(Ok(operation)) => {
            let location = format!("/api/v1/operations/{}", operation.id);
            (
                StatusCode::ACCEPTED,
                [(header::LOCATION, location)],
                Json(operation),
            )
                .into_response()
        }
        Ok(Err(e)) => storage_error(e).into_response(),
        Err(_) => storage_error(store::Error::Unavailable).into_response(),
    }
}

pub async fn get(
    State(state): State<AppState>,
    path: Result<Path<String>, axum::extract::rejection::PathRejection>,
) -> Response {
    let id = path
        .ok()
        .map(|Path(id)| id)
        .filter(|id| id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()));
    let Some(id) = id else {
        return error(
            StatusCode::NOT_FOUND,
            "operation_not_found",
            "Operation not found.",
        )
        .into_response();
    };
    match tokio::task::spawn_blocking(move || state.store.operation(&id)).await {
        Ok(Ok(Some(operation))) => Json(operation).into_response(),
        Ok(Ok(None)) => error(
            StatusCode::NOT_FOUND,
            "operation_not_found",
            "Operation not found.",
        )
        .into_response(),
        Ok(Err(e)) => storage_error(e).into_response(),
        Err(_) => storage_error(store::Error::Unavailable).into_response(),
    }
}

pub async fn list(State(state): State<AppState>) -> Response {
    match tokio::task::spawn_blocking(move || state.store.recent_operations()).await {
        Ok(Ok(operations)) => Json(operations).into_response(),
        Ok(Err(e)) => storage_error(e).into_response(),
        Err(_) => storage_error(store::Error::Unavailable).into_response(),
    }
}

fn storage_error(e: store::Error) -> (StatusCode, Json<ApiError>) {
    match e {
        store::Error::Conflict => error(
            StatusCode::CONFLICT,
            "operation_conflict",
            "This request conflicts with active work or a previously used request key.",
        ),
        store::Error::InvalidOperation => error(
            StatusCode::BAD_REQUEST,
            "invalid_request_key",
            "Supply an Idempotency-Key of 1–64 letters, digits, dots, hyphens or underscores.",
        ),
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "Operation storage is unavailable.",
        ),
    }
}
