use crate::{definition::Definition, error, inventory::AppState, store, ApiError};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Validated {
    name: String,
    hash: String,
    yaml: String,
}

type Problem = (StatusCode, Json<serde_json::Value>);

fn problem(status: StatusCode, code: &str, message: &str) -> Problem {
    (
        status,
        Json(serde_json::json!({"error":{"code":code,"message":message}})),
    )
}

fn parse(
    headers: &HeaderMap,
    body: Result<String, axum::extract::rejection::StringRejection>,
) -> Result<Definition, Problem> {
    if headers.contains_key(header::ORIGIN) {
        return Err(problem(
            StatusCode::FORBIDDEN,
            "origin_forbidden",
            "Browser origins cannot modify local definitions.",
        ));
    }
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .map(str::trim)
        != Some("application/yaml")
    {
        return Err(problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "Send YAML using Content-Type: application/yaml.",
        ));
    }
    let yaml = body.map_err(|e| {
        problem(
            e.status(),
            "invalid_body",
            "YAML body is invalid or exceeds 64 KiB.",
        )
    })?;
    Definition::parse(&yaml).map_err(|issue| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": {"code":"invalid_definition", "message":issue.message, "field":issue.field}
            })),
        )
    })
}

pub async fn validate(
    headers: HeaderMap,
    yaml: Result<String, axum::extract::rejection::StringRejection>,
) -> Response {
    let definition = match parse(&headers, yaml) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match (definition.hash(), definition.canonical_yaml()) {
        (Ok(hash), Ok(yaml)) => Json(Validated {
            name: definition.metadata.name,
            hash,
            yaml,
        })
        .into_response(),
        _ => storage_error(store::Error::InvalidDefinition).into_response(),
    }
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    yaml: Result<String, axum::extract::rejection::StringRejection>,
) -> Response {
    let definition = match parse(&headers, yaml) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    match tokio::task::spawn_blocking(move || state.store.create(&definition)).await {
        Ok(Ok(record)) => (StatusCode::CREATED, Json(record)).into_response(),
        Ok(Err(e)) => storage_error(e).into_response(),
        Err(_) => storage_error(store::Error::Unavailable).into_response(),
    }
}

pub async fn list(State(state): State<AppState>) -> Response {
    match tokio::task::spawn_blocking(move || state.store.list()).await {
        Ok(Ok(records)) => Json(records).into_response(),
        Ok(Err(e)) => storage_error(e).into_response(),
        Err(_) => storage_error(store::Error::Unavailable).into_response(),
    }
}

// The static validation route takes precedence over /definitions/{name}.
pub async fn get_validate(state: State<AppState>) -> Response {
    get(state, Path("validate".to_owned())).await
}

pub async fn get(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    match tokio::task::spawn_blocking(move || state.store.get(&name)).await {
        Ok(Ok(Some(record))) => Json(record).into_response(),
        Ok(Ok(None)) => error(
            StatusCode::NOT_FOUND,
            "definition_not_found",
            "Definition not found.",
        )
        .into_response(),
        Ok(Err(e)) => storage_error(e).into_response(),
        Err(_) => storage_error(store::Error::Unavailable).into_response(),
    }
}

fn storage_error(e: store::Error) -> (StatusCode, Json<ApiError>) {
    let (status, code, message) = match e {
        store::Error::Conflict => (
            StatusCode::CONFLICT,
            "definition_exists",
            "A definition with this name already exists.",
        ),
        store::Error::InvalidDefinition => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_definition",
            "Definition is invalid.",
        ),
        store::Error::InvalidOperation => (
            StatusCode::BAD_REQUEST,
            "invalid_operation",
            "Invalid operation.",
        ),
        store::Error::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "Definition storage is unavailable.",
        ),
    };
    error(status, code, message)
}
