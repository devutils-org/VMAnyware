use crate::{error, proxmox::Proxmox, ApiError};
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub lifecycle_enabled: bool,
    pub provisioning_enabled: bool,
    pub provider: Option<Proxmox>,
    pub store: crate::store::Store,
    pub catalog: Option<crate::provisioning::Catalog>,
}

#[derive(Serialize)]
pub struct Inventory {
    provider: &'static str,
    version: String,
    nodes: Vec<Node>,
    templates: Vec<Template>,
}

#[derive(Serialize)]
struct Node {
    name: String,
    status: String,
}

#[derive(Serialize)]
struct Template {
    node: String,
    vmid: u32,
    name: Option<String>,
}

pub(crate) async fn get(
    State(state): State<AppState>,
) -> Result<Json<Inventory>, (StatusCode, Json<ApiError>)> {
    let provider = state.provider.ok_or_else(|| {
        error(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_not_configured",
            "Configure Proxmox on the control plane before reading inventory.",
        )
    })?;
    let inventory = refresh(&provider)
        .await
        .map_err(|err| {
            use crate::proxmox::Error;
            let (status, code) = match err {
                Error::Unauthorized => (StatusCode::BAD_GATEWAY, "provider_unauthorized"),
                Error::Forbidden => (StatusCode::BAD_GATEWAY, "provider_forbidden"),
                Error::Timeout => (StatusCode::GATEWAY_TIMEOUT, "provider_timeout"),
                _ => (StatusCode::BAD_GATEWAY, "provider_unavailable"),
            };
            let message = match err {
                Error::Unauthorized => "Proxmox rejected the configured API token.",
                Error::Forbidden => "The Proxmox token lacks inventory permissions.",
                Error::Timeout => "Proxmox inventory timed out.",
                _ => "Cannot read Proxmox inventory; check the control-plane configuration and TLS trust.",
            };
            error(status, code, message)
        })?;
    Ok(Json(inventory))
}

pub(crate) async fn refresh(provider: &Proxmox) -> Result<Inventory, crate::proxmox::Error> {
    tokio::time::timeout(Duration::from_secs(20), collect(provider))
        .await
        .map_err(|_| crate::proxmox::Error::Timeout)?
}

async fn collect(provider: &Proxmox) -> Result<Inventory, crate::proxmox::Error> {
    let version = provider.version().await?;
    let nodes = provider.nodes().await?;
    if nodes.len() > 100 {
        return Err(crate::proxmox::Error::ResponseTooLarge);
    }
    let mut templates = Vec::new();
    for node in &nodes {
        if node.status != "online" {
            continue;
        }
        for vm in provider.virtual_machines(&node.node).await? {
            if vm.template == 1 {
                templates.push(Template {
                    node: node.node.clone(),
                    vmid: vm.vmid,
                    name: vm.name,
                });
            }
        }
    }
    templates.sort_by(|a, b| (&a.node, a.vmid).cmp(&(&b.node, b.vmid)));
    let mut nodes: Vec<Node> = nodes
        .into_iter()
        .map(|n| Node {
            name: n.node,
            status: n.status,
        })
        .collect();
    nodes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Inventory {
        provider: "proxmox",
        version: version.version,
        nodes,
        templates,
    })
}
