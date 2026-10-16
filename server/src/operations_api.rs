use crate::{error, inventory::AppState, store, ApiError};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub async fn enqueue(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if headers.contains_key(header::ORIGIN) {
        return error(
            StatusCode::FORBIDDEN,
            "origin_forbidden",
            "Browser origins cannot start local operations.",
        )
        .into_response();
    }
    let Some(key) = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
    else {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_request_key",
            "Supply an Idempotency-Key of 1–64 letters, digits, dots, hyphens or underscores.",
        )
        .into_response();
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

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return error(
            StatusCode::NOT_FOUND,
            "operation_not_found",
            "Operation not found.",
        )
        .into_response();
    }
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

fn storage_error(e: store::Error) -> (StatusCode, Json<ApiError>) {
    match e {
        store::Error::Conflict => error(
            StatusCode::CONFLICT,
            "operation_conflict",
            "Another inventory refresh is active. Wait for it to finish before retrying.",
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
