use reqwest::{redirect::Policy, Client};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct Node {
    name: String,
    status: String,
}

#[derive(Deserialize, Serialize)]
pub struct Template {
    node: String,
    vmid: u32,
    name: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Inventory {
    provider: String,
    version: String,
    nodes: Vec<Node>,
    templates: Vec<Template>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionRecord {
    name: String,
    yaml: String,
    hash: String,
    revision: u32,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    id: String,
    kind: String,
    status: OperationStatus,
    result: Option<Inventory>,
    failure: Option<String>,
    created_at: String,
    updated_at: String,
}

#[tauri::command]
pub async fn begin_inventory_refresh(port: u16, request_key: String) -> Result<Operation, String> {
    if request_key.is_empty()
        || request_key.len() > 64
        || !request_key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err("Invalid inventory request key.".into());
    }
    request(
        port,
        "/api/v1/provider/inventory/refresh",
        Some(Payload::RequestKey(request_key)),
    )
    .await
}

#[tauri::command]
pub async fn read_operation(port: u16, id: String) -> Result<Operation, String> {
    if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid operation identifier.".into());
    }
    request(port, &format!("/api/v1/operations/{id}"), None).await
}

/// Initial local-only spike; arbitrary remote URLs are intentionally not exposed.
#[tauri::command]
pub async fn inspect_control_plane(port: u16) -> Result<Inventory, String> {
    request(port, "/api/v1/provider/inventory", None).await
}

#[tauri::command]
pub async fn list_definitions(port: u16) -> Result<Vec<DefinitionRecord>, String> {
    request(port, "/api/v1/definitions", None).await
}

#[tauri::command]
pub async fn save_definition(port: u16, yaml: String) -> Result<DefinitionRecord, String> {
    if yaml.len() > 64 * 1024 {
        return Err("Definition exceeds 64 KiB.".into());
    }
    request(port, "/api/v1/definitions", Some(Payload::Yaml(yaml))).await
}

enum Payload {
    Yaml(String),
    RequestKey(String),
    Json(serde_json::Value),
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSource {
    image: String,
    node: String,
    template_vmid: u32,
    template_version: String,
    storage: String,
    bridge: String,
    disk_device: String,
    #[serde(rename = "sourceDiskGiB")]
    source_disk_gib: u32,
    guest_user: String,
}

#[derive(Deserialize, Serialize)]
pub struct PlanResources {
    cpu: u16,
    #[serde(rename = "memoryMiB")]
    memory_mib: u32,
    #[serde(rename = "diskGiB")]
    disk_gib: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanTarget {
    node: String,
    vmid: u32,
    name: String,
    storage: String,
    bridge: String,
    resources: PlanResources,
    guest_user: String,
    ipv4: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningPlan {
    definition_name: String,
    definition_hash: String,
    plan_hash: String,
    provider: String,
    source: PlanSource,
    target: PlanTarget,
    ssh_key_count: usize,
    packages: Vec<String>,
    snapshot_after_provisioning: bool,
    steps: Vec<String>,
    live_verified: bool,
    vmid_reserved: bool,
    creation_enabled: bool,
}

#[tauri::command]
pub async fn preview_provisioning(
    port: u16,
    yaml: String,
    target_vmid: u32,
) -> Result<ProvisioningPlan, String> {
    if yaml.len() > 64 * 1024 {
        return Err("Definition exceeds 64 KiB.".into());
    }
    if !(100..=999_999_999).contains(&target_vmid) {
        return Err("Choose a target VMID between 100 and 999999999.".into());
    }
    request(
        port,
        "/api/v1/provisioning/plan",
        Some(Payload::Json(
            serde_json::json!({"yaml":yaml,"targetVmid":target_vmid}),
        )),
    )
    .await
}

async fn request<T: DeserializeOwned>(
    port: u16,
    path: &str,
    payload: Option<Payload>,
) -> Result<T, String> {
    if port == 0 {
        return Err("Enter a port between 1 and 65535.".into());
    }
    let client = Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|_| "Cannot initialize the control-plane connection.")?;
    let url = format!("http://127.0.0.1:{port}{path}");
    let builder = match payload {
        Some(Payload::RequestKey(key)) => client.post(url).header("idempotency-key", key),
        Some(Payload::Yaml(yaml)) => client
            .post(url)
            .header("content-type", "application/yaml")
            .body(yaml),
        Some(Payload::Json(value)) => client.post(url).json(&value),
        None => client.get(url),
    };
    let mut response = builder.send().await.map_err(|_| {
        "Cannot reach the local control plane. Start it with make server and check the port."
    })?;
    let status = response.status();
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Control-plane response interrupted.")?
    {
        if body.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err("Control-plane response is too large.".into());
        }
        body.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        if path == "/api/v1/provisioning/plan" {
            let problem: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
            let message = match problem["error"]["code"].as_str() {
                Some("image_not_mapped") => Some("No template is mapped to this image on the control plane."),
                Some("invalid_target_vmid") => Some("Choose a target VMID between 100 and 999999999, different from the source template."),
                Some("disk_shrink_unsupported") => Some("Requested disk is smaller than the source template disk. Increase diskGiB in the definition."),
                _ => None,
            };
            if let Some(message) = message {
                return Err(message.into());
            }
        }
        return Err(match status.as_u16() {
            409 if path == "/api/v1/definitions" => "A definition with this name already exists. Choose a new name; saved definitions are never overwritten.",
            409 => "Another inventory refresh is active. Wait briefly, then retry.",
            404 => "This feature is not available on the selected control plane.",
            413 => "Definition exceeds 64 KiB.",
            422 if path == "/api/v1/provisioning/plan" => "Cannot preview this configuration. Check the YAML, image mapping, target VMID and template disk size.",
            503 if path == "/api/v1/provisioning/plan" => "Configure POCKETLAB_TEMPLATES on the control plane before previewing provisioning.",
            422 => "Definition is invalid. Check its schema, resource limits and inline public SSH key.",
            503 if path == "/api/v1/provider/inventory" => "The control plane is reachable. Configure Proxmox on the server, then retry.",
            503 => "Control-plane storage is unavailable. Check the control plane and retry.",
            502 => "The control plane cannot read Proxmox. Check its token permissions, endpoint and TLS trust.",
            504 => "Proxmox inventory timed out. Check the server connection and retry.",
            _ => "The control plane rejected the request. Check that you are connecting to PocketLab.",
        }.into());
    }
    serde_json::from_slice(&body)
        .map_err(|_| "The control plane returned an invalid response.".into())
}

#[cfg(test)]
mod tests;
