use crate::{
    error,
    inventory::AppState,
    proxmox::{Error, VmReference},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

pub async fn get(
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, axum::extract::rejection::PathRejection>,
) -> Response {
    let target = path.ok().and_then(|Path((node, vmid))| {
        // Canonical decimal form avoids aliases and rejects signs or whitespace.
        let number: u32 = vmid.parse().ok()?;
        if number.to_string() != vmid {
            return None;
        }
        VmReference::new(&node, number).ok()
    });
    let Some(target) = target else {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_vm_target",
            "Supply a valid node and VMID between 100 and 999999999.",
        )
        .into_response();
    };
    let Some(provider) = state.provider else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox on the control plane before reading VM status.",
        )
        .into_response();
    };
    match provider.vm_status(&target).await {
        Ok(observation) => Json(observation).into_response(),
        Err(err) => provider_error(err),
    }
}

pub(crate) fn provider_error(err: Error) -> Response {
    let (status, code, message) = match err {
        Error::Unauthorized => (
            StatusCode::BAD_GATEWAY,
            "provider_unauthorized",
            "Proxmox rejected the configured API token.",
        ),
        Error::Forbidden => (
            StatusCode::BAD_GATEWAY,
            "provider_forbidden",
            "The Proxmox token lacks VM status permissions.",
        ),
        Error::Timeout => (
            StatusCode::GATEWAY_TIMEOUT,
            "provider_timeout",
            "Proxmox VM status timed out.",
        ),
        _ => (
            StatusCode::BAD_GATEWAY,
            "provider_unavailable",
            "Cannot read Proxmox VM status; check target, permissions and TLS trust.",
        ),
    };
    error(status, code, message).into_response()
}
