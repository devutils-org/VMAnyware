//! Retained-definition recreation planning, confirmed enqueue and saved reads.
use crate::{definition::Definition, error, instance::Status, inventory::AppState};
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Request {
    target_vmid: u32,
}

pub async fn preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request: Result<Json<Request>, JsonRejection>,
) -> Response {
    respond(state, id, headers, request, false).await
}

pub async fn preflight(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    request: Result<Json<Request>, JsonRejection>,
) -> Response {
    respond(state, id, headers, request, true).await
}

async fn respond(
    state: AppState,
    id: String,
    headers: HeaderMap,
    request: Result<Json<Request>, JsonRejection>,
    live: bool,
) -> Response {
    if headers.contains_key(header::ORIGIN) {
        return error(
            StatusCode::FORBIDDEN,
            "origin_forbidden",
            "Use the native client to review recreation.",
        )
        .into_response();
    }
    if id.len() != 32
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return missing();
    }
    let request = match request {
        Ok(Json(value)) => value,
        Err(rejection) => {
            return error(
                rejection.status(),
                "invalid_recreation_request",
                "Send JSON containing only targetVmid.",
            )
            .into_response()
        }
    };
    let instance = match tokio::task::spawn_blocking(move || state.store.instance(&id)).await {
        Ok(Ok(Some(instance))) => instance,
        Ok(Ok(None)) => return missing(),
        _ => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "storage_unavailable",
                "Cannot read the retained instance.",
            )
            .into_response()
        }
    };
    if instance.summary.status != Status::Deleted {
        return error(
            StatusCode::CONFLICT,
            "instance_not_deleted",
            "Recreation requires a retired instance. Delete it explicitly first.",
        )
        .into_response();
    }
    let definition = match Definition::parse(&instance.definition_yaml) {
        Ok(definition)
            if definition.hash().ok().as_ref() == Some(&instance.summary.definition_hash) =>
        {
            definition
        }
        _ => {
            return error(
                StatusCode::CONFLICT,
                "retained_definition_invalid",
                "The retained definition does not match its recorded hash.",
            )
            .into_response()
        }
    };
    let Some(catalog) = state.catalog else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "template_catalog_not_configured",
            "Configure the template catalog before reviewing recreation.",
        )
        .into_response();
    };
    let mut plan = match catalog.plan(&definition, request.target_vmid) {
        Ok(plan) => plan,
        Err(issue) => {
            let code = match issue {
                crate::provisioning::Error::InvalidVmid => "invalid_target_vmid",
                crate::provisioning::Error::ImageNotMapped => "image_not_mapped",
                crate::provisioning::Error::DiskShrink => "disk_shrink_unsupported",
                _ => "invalid_plan",
            };
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                code,
                "Cannot plan recreation from the retained definition and current catalog.",
            )
            .into_response();
        }
    };
    plan.provider_scope_id = state.provider.as_ref().map(|provider| provider.scope_id());
    let enabled = live
        && state.provisioning_enabled
        && plan.provider_scope_id.is_some()
        && plan.minimum_create_issue(state.snapshots_enabled).is_none();
    plan.creation_enabled = enabled;
    let mut response = serde_json::json!({
        "sourceInstanceId": instance.summary.id,
        "sourceDefinitionHash": instance.summary.definition_hash,
        "sourceDefinitionRevision": instance.summary.definition_revision,
        "definitionYaml": instance.definition_yaml,
        "plan": plan,
        "restoresPreviousDiskContents": false,
        "requiresNewInstanceIdentity": true,
        "recreationEnabled": enabled,
    });
    if live {
        let Some(provider) = state.provider else {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "provider_not_configured",
                "Configure Proxmox before checking recreation.",
            )
            .into_response();
        };
        response["preflight"] = serde_json::to_value(crate::preflight::run(&provider, plan).await)
            .expect("Preflight report is JSON serializable");
    }
    Json(response).into_response()
}

fn missing() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "instance_not_found",
        "Retained instance not found.",
    )
    .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Confirmation {
    source_instance_id: String,
    approval: crate::provisioning_intent::Approval,
}
fn canonical_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Queue only. The existing provisioning worker revalidates before dispatch.
pub async fn create(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Confirmation>, JsonRejection>,
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
    if !canonical_id(&id) {
        return missing();
    }
    if !state.provisioning_enabled {
        return error(
            StatusCode::FORBIDDEN,
            "provisioning_disabled",
            "Provisioning execution is disabled on this control plane.",
        )
        .into_response();
    }
    let confirmation = match body {
        Ok(Json(value)) if value.source_instance_id == id && value.approval.valid() => value,
        Ok(_) => {
            return crate::provisioning_api::create_problem(
                crate::store::ProvisioningError::InvalidApproval,
            )
        }
        Err(rejection) => {
            return error(
                rejection.status(),
                "invalid_recreation_approval",
                "Confirm the source instance and complete fresh provisioning approval.",
            )
            .into_response()
        }
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox before requesting recreation.",
        )
        .into_response();
    };
    if provider.scope_id() != confirmation.approval.provider_scope_id {
        return error(
            StatusCode::CONFLICT,
            "provider_scope_changed",
            "The confirmed provider scope differs from this control plane.",
        )
        .into_response();
    }
    let Some(catalog) = state.catalog else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "template_catalog_not_configured",
            "Configure the template catalog before requesting recreation.",
        )
        .into_response();
    };
    let result = tokio::task::spawn_blocking(move || {
        let source = match state.store.instance(&id) {
            Ok(Some(source)) => source,
            Ok(None) => return missing(),
            Err(_) => {
                return crate::provisioning_api::create_problem(
                    crate::store::ProvisioningError::Unavailable,
                )
            }
        };
        if source.summary.status != Status::Deleted {
            return error(
                StatusCode::CONFLICT,
                "instance_not_deleted",
                "Recreation requires a retired instance.",
            )
            .into_response();
        }
        let approval = confirmation.approval;
        let definition = match Definition::parse(&source.definition_yaml) {
            Ok(value) if value.hash().ok().as_ref() == Some(&source.summary.definition_hash) => {
                value
            }
            _ => {
                return crate::provisioning_api::create_problem(
                    crate::store::ProvisioningError::Unavailable,
                )
            }
        };
        let plan = match catalog.plan(&definition, approval.target_vmid) {
            Ok(plan) if approval.matches(&plan) => plan,
            _ => {
                return crate::provisioning_api::create_problem(
                    crate::store::ProvisioningError::TargetChanged,
                )
            }
        };
        if let Some((code, message)) = plan.minimum_create_issue(state.snapshots_enabled) {
            return error(
                if code == "snapshot_disabled" {
                    StatusCode::FORBIDDEN
                } else {
                    StatusCode::UNPROCESSABLE_ENTITY
                },
                code,
                message,
            )
            .into_response();
        }
        match state
            .store
            .enqueue_recreation(&key, &id, &approval, &catalog)
        {
            Ok(operation) => (
                StatusCode::ACCEPTED,
                [(
                    header::LOCATION,
                    format!(
                        "/api/v1/environments/{id}/recreation-operations/{}",
                        operation.id
                    ),
                )],
                Json(operation),
            )
                .into_response(),
            Err(issue) => crate::provisioning_api::create_problem(issue),
        }
    })
    .await;
    result.unwrap_or_else(|_| {
        crate::provisioning_api::create_problem(crate::store::ProvisioningError::Unavailable)
    })
}

/// Historical saved evidence only; no provider access or mutation capability.
pub async fn recover(
    State(state): State<AppState>,
    Path((source_id, operation_id)): Path<(String, String)>,
) -> Response {
    if !canonical_id(&source_id) || !canonical_id(&operation_id) {
        return missing();
    }
    let result = tokio::task::spawn_blocking(move || {
        let operation = state.store.operation(&operation_id).map_err(|_| ())?;
        if operation
            .as_ref()
            .is_none_or(|op| op.kind != "vm_provision")
        {
            return Ok(None);
        }
        let job = state
            .store
            .provisioning_job(&operation_id)
            .map_err(|_| ())?;
        let Some(mut job) =
            job.filter(|job| job.recreated_from.as_deref() == Some(source_id.as_str()))
        else {
            return Ok(None);
        };
        // The retained approval binds the historical endpoint scope; rebuilding
        // a catalog plan alone has no scope. Do not substitute current config.
        job.plan.provider_scope_id = Some(job.approval.provider_scope_id.clone());
        Ok::<_, ()>(Some(serde_json::json!({
            "sourceInstanceId": source_id,
            "operation": job.operation,
            "plan": job.plan,
            "restoresPreviousDiskContents": false,
            "requiresNewInstanceIdentity": true,
        })))
    })
    .await;
    match result {
        Ok(Ok(Some(value))) => Json(value).into_response(),
        Ok(Ok(None)) => missing(),
        _ => crate::provisioning_api::create_problem(crate::store::ProvisioningError::Unavailable),
    }
}

/// Discover recorded recreation IDs for one retired source, without live reads.
pub async fn history(State(state): State<AppState>, Path(source_id): Path<String>) -> Response {
    match tokio::task::spawn_blocking(move || state.store.recreation_history(&source_id)).await {
        Ok(Ok(Some(history))) => Json(history).into_response(),
        Ok(Ok(None)) => missing(),
        _ => crate::provisioning_api::create_problem(crate::store::ProvisioningError::Unavailable),
    }
}
