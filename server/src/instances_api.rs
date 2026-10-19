use crate::{
    error, instance::Registration, inventory::AppState, store::RegistrationError, ApiError,
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

fn instance_id(path: Result<Path<String>, PathRejection>) -> Option<String> {
    match path {
        Ok(Path(id)) if id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()) => Some(id),
        _ => None,
    }
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<Registration>, JsonRejection>,
) -> Response {
    if headers.contains_key(header::ORIGIN) {
        return error(
            StatusCode::FORBIDDEN,
            "origin_forbidden",
            "Browser origins cannot register local environments.",
        )
        .into_response();
    }
    let request = match body {
        Ok(Json(request)) if request.is_valid() => request,
        Ok(_) => return storage_error(RegistrationError::InvalidRequest).into_response(),
        Err(rejection) => {
            return error(
                rejection.status(),
                "invalid_instance_request",
                "Send a bounded JSON definition name/hash, node and VMID.",
            )
            .into_response()
        }
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before registering a local environment reference.",
        )
        .into_response();
    };
    let scope = provider.scope_id();
    match tokio::task::spawn_blocking(move || state.store.register_instance(&request, &scope)).await
    {
        Ok(Ok(instance)) => {
            let location = format!("/api/v1/environments/{}", instance.summary.id);
            (
                StatusCode::CREATED,
                [(header::LOCATION, location)],
                Json(instance),
            )
                .into_response()
        }
        Ok(Err(error)) => storage_error(error).into_response(),
        Err(_) => storage_error(RegistrationError::Unavailable).into_response(),
    }
}

pub async fn list(State(state): State<AppState>) -> Response {
    match tokio::task::spawn_blocking(move || state.store.recent_instances()).await {
        Ok(Ok(instances)) => Json(instances).into_response(),
        Ok(Err(error)) => storage_error(error).into_response(),
        Err(_) => storage_error(RegistrationError::Unavailable).into_response(),
    }
}

pub async fn get(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let Some(id) = instance_id(path) else {
        return not_found();
    };
    match tokio::task::spawn_blocking(move || state.store.instance(&id)).await {
        Ok(Ok(Some(instance))) => Json(instance).into_response(),
        Ok(Ok(None)) => not_found(),
        Ok(Err(error)) => storage_error(error).into_response(),
        Err(_) => storage_error(RegistrationError::Unavailable).into_response(),
    }
}

pub async fn status(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let Some(id) = instance_id(path) else {
        return not_found();
    };
    let instance = match tokio::task::spawn_blocking(move || state.store.instance(&id)).await {
        Ok(Ok(Some(instance))) => instance,
        Ok(Ok(None)) => return not_found(),
        Ok(Err(error)) => return storage_error(error).into_response(),
        Err(_) => return storage_error(RegistrationError::Unavailable).into_response(),
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before observing an environment instance.",
        )
        .into_response();
    };
    match instance.summary.observe(&provider).await {
        Ok(observation) => Json(observation).into_response(),
        Err(error) => observation_error(error),
    }
}

pub async fn lifecycle_preflight(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let Some(id) = instance_id(path) else {
        return not_found();
    };
    let instance = match tokio::task::spawn_blocking(move || state.store.instance(&id)).await {
        Ok(Ok(Some(instance))) => instance,
        Ok(Ok(None)) => return not_found(),
        Ok(Err(error)) => return storage_error(error).into_response(),
        Err(_) => return storage_error(RegistrationError::Unavailable).into_response(),
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before checking lifecycle preconditions.",
        )
        .into_response();
    };
    match instance.summary.lifecycle_preflight(&provider).await {
        Ok(mut preflight) => {
            preflight.execution_enabled = state.lifecycle_enabled;
            Json(preflight).into_response()
        }
        Err(error) => observation_error(error),
    }
}

fn observation_error(problem: crate::instance::ObservationError) -> Response {
    match problem {
        crate::instance::ObservationError::ScopeChanged => error(
            StatusCode::CONFLICT,
            "provider_scope_changed",
            "The configured Proxmox endpoint differs from this instance reference.",
        )
        .into_response(),
        crate::instance::ObservationError::InvalidReference => {
            storage_error(RegistrationError::Unavailable).into_response()
        }
        crate::instance::ObservationError::Provider(error) => crate::vm_api::provider_error(error),
    }
}

pub async fn refresh(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
) -> Response {
    let key = match crate::operations_api::request_key(&headers) {
        Ok(key) => key,
        Err(response) => return response.into_response(),
    };
    let Some(id) = instance_id(path) else {
        return not_found();
    };
    match tokio::task::spawn_blocking(move || state.store.enqueue_instance_observation(&key, &id))
        .await
    {
        Ok(Ok(operation)) => {
            let location = format!("/api/v1/operations/{}", operation.id);
            (
                StatusCode::ACCEPTED,
                [(header::LOCATION, location)],
                Json(operation),
            )
                .into_response()
        }
        Ok(Err(RegistrationError::Conflict)) => error(
            StatusCode::CONFLICT,
            "operation_conflict",
            "This request conflicts with active work or a previously used request key.",
        )
        .into_response(),
        Ok(Err(error)) => storage_error(error).into_response(),
        Err(_) => storage_error(RegistrationError::Unavailable).into_response(),
    }
}

fn not_found() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "instance_not_found",
        "Environment instance not found.",
    )
    .into_response()
}

fn storage_error(error_kind: RegistrationError) -> (StatusCode, Json<ApiError>) {
    let (status, code, message) = match error_kind {
        RegistrationError::InvalidRequest => (
            StatusCode::BAD_REQUEST,
            "invalid_instance_request",
            "Supply a valid definition name, lowercase SHA-256 hash, node and VMID.",
        ),
        RegistrationError::InstanceNotFound => (
            StatusCode::NOT_FOUND,
            "instance_not_found",
            "Environment instance not found.",
        ),
        RegistrationError::DefinitionNotFound => (
            StatusCode::NOT_FOUND,
            "definition_not_found",
            "Save the environment definition before registering an instance.",
        ),
        RegistrationError::DefinitionChanged => (
            StatusCode::CONFLICT,
            "definition_changed",
            "The definition hash differs from the saved definition. Reload it before registering.",
        ),
        RegistrationError::Conflict => (
            StatusCode::CONFLICT,
            "instance_exists",
            "This provider VMID already has a registered instance.",
        ),
        RegistrationError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "Environment instance storage is unavailable.",
        ),
    };
    error(status, code, message)
}
