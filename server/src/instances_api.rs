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

/// Read only the registered provider scope/target; caller query parameters cannot retarget it.
pub async fn snapshots(
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
            "Configure Proxmox before listing snapshots.",
        )
        .into_response();
    };
    let target = match instance.summary.target(&provider) {
        Ok(target) => target,
        Err(error) => return observation_error(error),
    };
    match provider.snapshots(&target).await {
        Ok(snapshots) => Json(serde_json::json!({
            "instanceId": instance.summary.id,
            "providerScopeId": provider.scope_id(),
            "node": target.node(), "vmid": target.vmid(), "snapshots": snapshots,
        }))
        .into_response(),
        Err(problem) => {
            use crate::proxmox::Error;
            let (status, code, message) = match problem {
                Error::Unauthorized => (
                    StatusCode::BAD_GATEWAY,
                    "provider_unauthorized",
                    "Proxmox rejected the configured API token.",
                ),
                Error::Forbidden => (
                    StatusCode::BAD_GATEWAY,
                    "provider_forbidden",
                    "The Proxmox token lacks snapshot read permissions.",
                ),
                Error::Timeout => (
                    StatusCode::GATEWAY_TIMEOUT,
                    "provider_timeout",
                    "Proxmox snapshot listing timed out.",
                ),
                _ => (
                    StatusCode::BAD_GATEWAY,
                    "provider_unavailable",
                    "Cannot read Proxmox snapshots; check target, permissions and TLS trust.",
                ),
            };
            error(status, code, message).into_response()
        }
    }
}

pub async fn snapshot_preflight(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<Json<crate::snapshot::CreateRequest>, JsonRejection>,
) -> Response {
    let Some(id) = instance_id(path) else {
        return not_found();
    };
    let request = match body {
        Ok(Json(request)) if request.is_valid() => request,
        _ => return error(StatusCode::BAD_REQUEST, "invalid_snapshot_request", "Use a 2–40 character snapshot name starting with a letter, containing letters/digits/underscore/hyphen, excluding current; description is limited to 4 KiB.").into_response(),
    };
    let instance = match tokio::task::spawn_blocking(move || state.store.instance(&id)).await {
        Ok(Ok(Some(instance))) => instance,
        Ok(Ok(None)) => return not_found(),
        Ok(Err(problem)) => return storage_error(problem).into_response(),
        Err(_) => return storage_error(RegistrationError::Unavailable).into_response(),
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before checking snapshot prerequisites.",
        )
        .into_response();
    };
    match instance
        .summary
        .snapshot_preflight(&provider, request)
        .await
    {
        Ok(mut report) => {
            report.execution_enabled = state.snapshots_enabled;
            Json(report).into_response()
        }
        Err(crate::instance::ObservationError::Provider(problem)) => {
            let status = if problem == crate::proxmox::Error::Timeout {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };
            error(status, "snapshot_preflight_unavailable", "Cannot verify snapshot prerequisites; check provider permissions, target and TLS trust.").into_response()
        }
        Err(problem) => observation_error(problem),
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

pub async fn snapshot_restore_preflight(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<Json<crate::snapshot_restore::Request>, JsonRejection>,
) -> Response {
    let Some(id) = instance_id(path) else {
        return not_found();
    };
    let request=match body {
        Ok(Json(request)) if request.is_valid()=>request,
        _=>return error(StatusCode::BAD_REQUEST,"invalid_restore_request","Select a valid named snapshot; current and additional restore options are not accepted.").into_response(),
    };
    let instance = match tokio::task::spawn_blocking(move || state.store.instance(&id)).await {
        Ok(Ok(Some(instance))) => instance,
        Ok(Ok(None)) => return not_found(),
        Ok(Err(problem)) => return storage_error(problem).into_response(),
        Err(_) => return storage_error(RegistrationError::Unavailable).into_response(),
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before checking restore prerequisites.",
        )
        .into_response();
    };
    match instance.summary.snapshot_restore_preflight(&provider,request).await {
        Ok(mut report)=>{report.execution_enabled=state.restores_enabled;Json(report).into_response()},
        Err(crate::instance::ObservationError::Provider(problem))=>error(if problem==crate::proxmox::Error::Timeout {StatusCode::GATEWAY_TIMEOUT}else{StatusCode::BAD_GATEWAY},"restore_preflight_unavailable","Cannot verify restore prerequisites; check provider permissions, selected snapshot and TLS trust.").into_response(),
        Err(problem)=>observation_error(problem),
    }
}

pub async fn deletion_preflight(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Response {
    let Some(id) = instance_id(path) else {
        return not_found();
    };
    let instance = match tokio::task::spawn_blocking(move || state.store.instance(&id)).await {
        Ok(Ok(Some(instance))) => instance,
        Ok(Ok(None)) => return not_found(),
        _ => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "storage_unavailable",
                "Cannot read the saved instance.",
            )
            .into_response()
        }
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before reviewing deletion prerequisites.",
        )
        .into_response();
    };
    match instance.summary.deletion_preflight(&provider).await {
        Ok(mut report)=>{report.execution_enabled=state.deletions_enabled;Json(report).into_response()},
        Err(crate::instance::ObservationError::Provider(problem))=>error(if problem==crate::proxmox::Error::Timeout{StatusCode::GATEWAY_TIMEOUT}else{StatusCode::BAD_GATEWAY},"delete_preflight_unavailable","Cannot verify deletion scope; check VM, cluster and storage read permissions and TLS trust.").into_response(),
        Err(problem)=>observation_error(problem),
    }
}
