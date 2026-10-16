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
        Ok(plan) => Json(plan).into_response(),
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
        "Configure POCKETLAB_TEMPLATES on the control plane before previewing provisioning.",
    )
}
