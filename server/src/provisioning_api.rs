use crate::{definition::Definition, error, inventory::AppState, provisioning, ApiError};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PlanRequest {
    yaml: String,
    target_vmid: u32,
}

pub async fn images(State(state): State<AppState>) -> Response {
    match state.catalog {
        Some(catalog) => Json(catalog).into_response(),
        None => missing_catalog().into_response(),
    }
}

pub async fn preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<PlanRequest>, JsonRejection>,
) -> Response {
    respond(state, headers, request, false).await
}

pub async fn preflight(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<PlanRequest>, JsonRejection>,
) -> Response {
    respond(state, headers, request, true).await
}

async fn respond(
    state: AppState,
    headers: HeaderMap,
    request: Result<Json<PlanRequest>, JsonRejection>,
    live: bool,
) -> Response {
    if headers.contains_key(header::ORIGIN) {
        return error(
            StatusCode::FORBIDDEN,
            "origin_forbidden",
            "Use the native client to preview provisioning.",
        )
        .into_response();
    }
    let request = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            return error(
                rejection.status(),
                "invalid_plan_request",
                "Send JSON containing yaml and targetVmid.",
            )
            .into_response()
        }
    };
    let definition = match Definition::parse(&request.yaml) {
        Ok(definition) => definition,
        Err(issue) => return (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error":{"code":"invalid_definition","message":issue.message,"field":issue.field}}))).into_response(),
    };
    let Some(catalog) = state.catalog else {
        return missing_catalog().into_response();
    };
    match catalog.plan(&definition, request.target_vmid) {
        Ok(mut plan) if live => {
            plan.provider_scope_id = state.provider.as_ref().map(|p| p.scope_id());
            plan.creation_enabled = state.provisioning_enabled
                && plan.provider_scope_id.is_some()
                && plan.minimum_create_issue().is_none();
            match state.provider {
                Some(provider) => {
                    Json(crate::preflight::run(&provider, plan).await).into_response()
                }
                None => error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "provider_not_configured",
                    "Configure the Proxmox provider before checking a plan.",
                )
                .into_response(),
            }
        }
        Ok(mut plan) => {
            plan.provider_scope_id = state.provider.as_ref().map(|p| p.scope_id());
            plan.creation_enabled = state.provisioning_enabled
                && plan.provider_scope_id.is_some()
                && plan.minimum_create_issue().is_none();
            Json(plan).into_response()
        }
        Err(issue) => {
            let (code, message) = match issue {
                provisioning::Error::ImageNotMapped => ("image_not_mapped", "No template is mapped to this image on the control plane."),
                provisioning::Error::InvalidVmid => ("invalid_target_vmid", "Choose a target VMID between 100 and 999999999, different from the source template."),
                provisioning::Error::DiskShrink => ("disk_shrink_unsupported", "Requested disk is smaller than the source template disk."),
                _ => ("invalid_plan", "Cannot build a plan from this definition and catalog."),
            };
            error(StatusCode::UNPROCESSABLE_ENTITY, code, message).into_response()
        }
    }
}

fn missing_catalog() -> (StatusCode, Json<ApiError>) {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "template_catalog_not_configured",
        "Configure VMANYWARE_TEMPLATES on the control plane before previewing provisioning.",
    )
}

/// Queue only; no provider request is performed by this handler.
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<crate::provisioning_intent::Approval>, JsonRejection>,
) -> Response {
    let key = match crate::operations_api::request_key(&headers) {
        Ok(key) => key,
        Err(issue) => return issue.into_response(),
    };
    if headers.get_all("idempotency-key").iter().count() != 1 {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_request_key",
            "Supply exactly one Idempotency-Key.",
        )
        .into_response();
    }
    if !state.provisioning_enabled {
        return error(
            StatusCode::FORBIDDEN,
            "provisioning_disabled",
            "Provisioning execution is disabled on this control plane.",
        )
        .into_response();
    }
    let approval =
        match body {
            Ok(Json(approval)) if approval.valid() => approval,
            Ok(_) => return create_problem(crate::store::ProvisioningError::InvalidApproval),
            Err(rejection) => return error(
                rejection.status(),
                "invalid_provisioning_approval",
                "Send a bounded explicit confirmation of the complete definition, plan and target.",
            )
            .into_response(),
        };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before requesting creation.",
        )
        .into_response();
    };
    if provider.scope_id() != approval.provider_scope_id {
        return error(
            StatusCode::CONFLICT,
            "provider_scope_changed",
            "The confirmed provider scope differs from this control plane.",
        )
        .into_response();
    }
    let Some(catalog) = state.catalog else {
        return missing_catalog().into_response();
    };
    let result = tokio::task::spawn_blocking(move || {
        let record = state
            .store
            .get(&approval.definition_name)
            .map_err(|_| crate::store::ProvisioningError::Unavailable)?
            .ok_or(crate::store::ProvisioningError::DefinitionNotFound)?;
        let definition = Definition::parse(&record.yaml)
            .map_err(|_| crate::store::ProvisioningError::Unavailable)?;
        let plan = catalog
            .plan(&definition, approval.target_vmid)
            .map_err(|_| crate::store::ProvisioningError::TargetChanged)?;
        if !approval.matches(&plan) {
            return Err(CreateIssue::Storage(
                crate::store::ProvisioningError::TargetChanged,
            ));
        }
        if let Some((code, message)) = plan.minimum_create_issue() {
            return Err(CreateIssue::Unsupported(code, message));
        }
        state
            .store
            .enqueue_provisioning(&key, &approval, &catalog)
            .map_err(CreateIssue::Storage)
    })
    .await;
    match result {
        Ok(Ok(operation)) => (
            StatusCode::ACCEPTED,
            [(
                header::LOCATION,
                format!("/api/v1/operations/{}", operation.id),
            )],
            Json(operation),
        )
            .into_response(),
        Ok(Err(CreateIssue::Unsupported(code, message))) => {
            error(StatusCode::UNPROCESSABLE_ENTITY, code, message).into_response()
        }
        Ok(Err(CreateIssue::Storage(issue))) => create_problem(issue),
        Err(_) => create_problem(crate::store::ProvisioningError::Unavailable),
    }
}
enum CreateIssue {
    Storage(crate::store::ProvisioningError),
    Unsupported(&'static str, &'static str),
}
impl From<crate::store::ProvisioningError> for CreateIssue {
    fn from(issue: crate::store::ProvisioningError) -> Self {
        Self::Storage(issue)
    }
}
fn create_problem(issue: crate::store::ProvisioningError) -> Response {
    use crate::store::ProvisioningError as E;
    let (status, code, message) = match issue {
        E::InvalidApproval => (
            StatusCode::BAD_REQUEST,
            "invalid_provisioning_approval",
            "Confirm the exact saved definition, plan, scope and target.",
        ),
        E::DefinitionNotFound => (
            StatusCode::NOT_FOUND,
            "definition_not_found",
            "Save the definition before requesting creation.",
        ),
        E::TargetChanged => (
            StatusCode::CONFLICT,
            "provisioning_target_changed",
            "The confirmed definition, catalog or target changed; review a fresh plan.",
        ),
        E::Conflict => (
            StatusCode::CONFLICT,
            "operation_conflict",
            "This request conflicts with retained work or a previously used request key.",
        ),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "Provisioning operation storage is unavailable.",
        ),
    };
    error(status, code, message).into_response()
}
