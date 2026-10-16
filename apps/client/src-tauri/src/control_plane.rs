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
        None,
        Some(&request_key),
    )
    .await
}

#[tauri::command]
pub async fn read_operation(port: u16, id: String) -> Result<Operation, String> {
    if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid operation identifier.".into());
    }
    request(port, &format!("/api/v1/operations/{id}"), None, None).await
}

/// Initial local-only spike; arbitrary remote URLs are intentionally not exposed.
#[tauri::command]
pub async fn inspect_control_plane(port: u16) -> Result<Inventory, String> {
    request(port, "/api/v1/provider/inventory", None, None).await
}

#[tauri::command]
pub async fn list_definitions(port: u16) -> Result<Vec<DefinitionRecord>, String> {
    request(port, "/api/v1/definitions", None, None).await
}

#[tauri::command]
pub async fn save_definition(port: u16, yaml: String) -> Result<DefinitionRecord, String> {
    if yaml.len() > 64 * 1024 {
        return Err("Definition exceeds 64 KiB.".into());
    }
    request(port, "/api/v1/definitions", Some(yaml), None).await
}

async fn request<T: DeserializeOwned>(
    port: u16,
    path: &str,
    yaml: Option<String>,
    request_key: Option<&str>,
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
    let builder = match (yaml, request_key) {
        (_, Some(key)) => client.post(url).header("idempotency-key", key),
        (Some(yaml), None) => client
            .post(url)
            .header("content-type", "application/yaml")
            .body(yaml),
        (None, None) => client.get(url),
    };
    let mut response = builder.send().await.map_err(|_| {
        "Cannot reach the local control plane. Start it with make server and check the port."
    })?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            409 if path == "/api/v1/definitions" => "A definition with this name already exists. Choose a new name; saved definitions are never overwritten.",
            409 => "Another inventory refresh is active. Wait briefly, then retry.",
            404 => "This operation is not available on the selected control plane.",
            413 => "Definition exceeds 64 KiB.",
            422 => "Definition is invalid. Check its schema, resource limits and inline public SSH key.",
            503 if path == "/api/v1/provider/inventory" => "The control plane is reachable. Configure Proxmox on the server, then retry.",
            503 => "Definition storage is unavailable. Check the control plane and retry.",
            502 => "The control plane cannot read Proxmox. Check its token permissions, endpoint and TLS trust.",
            504 => "Proxmox inventory timed out. Check the server connection and retry.",
            _ => "The control plane rejected the request. Check that you are connecting to PocketLab.",
        }.into());
    }
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
    serde_json::from_slice(&body)
        .map_err(|_| "The control plane returned an invalid response.".into())
}

#[cfg(test)]
mod tests;
