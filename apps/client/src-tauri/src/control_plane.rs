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

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
    Uncertain,
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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationSummary {
    id: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instance_id: Option<String>,
    status: OperationStatus,
    failure: Option<String>,
    #[serde(default)]
    provider_task_known: bool,
    created_at: String,
    updated_at: String,
}

#[tauri::command]
pub async fn list_operations(port: u16) -> Result<Vec<OperationSummary>, String> {
    request(port, "/api/v1/operations", None).await
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

#[derive(Deserialize)]
struct ServiceHealth {
    status: String,
}

/// Checks the local service independently of provider configuration.
#[tauri::command]
pub async fn check_control_plane(port: u16) -> Result<(), String> {
    let health: ServiceHealth = request(port, "/api/v1/health", None).await?;
    if health.status != "ok" {
        return Err("The local service did not report a healthy status.".into());
    }
    Ok(())
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

#[derive(Deserialize, Serialize)]
pub struct ValidatedDefinition {
    name: String,
    hash: String,
    yaml: String,
}

#[tauri::command]
pub async fn validate_definition(port: u16, yaml: String) -> Result<ValidatedDefinition, String> {
    if yaml.len() > 64 * 1024 {
        return Err("Definition exceeds 64 KiB.".into());
    }
    request(
        port,
        "/api/v1/definitions/validate",
        Some(Payload::Yaml(yaml)),
    )
    .await
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
    ConfirmedJson(String, serde_json::Value),
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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStatus {
    Passed,
    Failed,
    Unknown,
}

#[derive(Deserialize, Serialize)]
pub struct PreflightCheck {
    code: String,
    status: PreflightStatus,
    message: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningPreflight {
    plan_hash: String,
    checks: Vec<PreflightCheck>,
    creation_enabled: bool,
    vmid_reserved: bool,
}

#[tauri::command]
pub async fn preflight_provisioning(
    port: u16,
    yaml: String,
    target_vmid: u32,
) -> Result<ProvisioningPreflight, String> {
    if yaml.len() > 64 * 1024 {
        return Err("Definition exceeds 64 KiB.".into());
    }
    if !(100..=999_999_999).contains(&target_vmid) {
        return Err("Choose a target VMID between 100 and 999999999.".into());
    }
    request(
        port,
        "/api/v1/provisioning/preflight",
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
        .retry(reqwest::retry::never())
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
        Some(Payload::ConfirmedJson(key, value)) => {
            client.post(url).header("idempotency-key", key).json(&value)
        }
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
        if status.as_u16() == 422
            && matches!(path, "/api/v1/definitions" | "/api/v1/definitions/validate")
        {
            let problem: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
            if problem["error"]["code"] == "invalid_definition" {
                // Only known field identifiers select application-owned messages.
                let message = match problem["error"]["field"].as_str() {
                    Some("metadata.name") => "metadata.name: use 1–63 lowercase letters, digits or internal hyphens.",
                    Some("metadata.description") => "metadata.description: keep the description within 1024 bytes.",
                    Some("spec.image") => "spec.image: use a 1–64 character lowercase image identifier.",
                    Some("spec.resources.cpu") => "spec.resources.cpu: choose 1–64 CPUs.",
                    Some("spec.resources.memoryMiB") => "spec.resources.memoryMiB: choose 256–262144 MiB.",
                    Some("spec.resources.diskGiB") => "spec.resources.diskGiB: choose 4–4096 GiB.",
                    Some("spec.provisioning.cloudInit") => "spec.provisioning.cloudInit: Cloud-Init must be true.",
                    Some("spec.provisioning.sshAuthorizedKeys") => "spec.provisioning.sshAuthorizedKeys: provide 1–16 unique, inline OpenSSH public keys, not file paths.",
                    Some("spec.provisioning.packages") => "spec.provisioning.packages: provide at most 128 unique Debian package declarations; shell syntax is forbidden.",
                    _ => "Definition is invalid. Check the v1 schema, required fields and public SSH keys.",
                };
                return Err(message.into());
            }
        }
        if path.starts_with("/api/v1/environments") {
            let problem: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
            let message = match problem["error"]["code"].as_str() {
                Some("provider_scope_changed") => Some("The configured Proxmox endpoint differs from this instance. Check the control-plane configuration."),
                Some("instance_not_found") => Some("This instance is no longer available. Reload the instance list."),
                Some("operation_conflict") => Some("This request conflicts with active work or a previously used key. Review operation history before another action."),
                Some("lifecycle_disabled") => Some("Lifecycle execution is disabled on the control plane."),
                Some("invalid_lifecycle_approval") => Some("Confirm the complete target and action using a fresh lifecycle check."),
                Some("lifecycle_target_changed") => Some("The confirmed instance has changed. Reload it and check lifecycle again."),
                Some("provider_not_configured") => Some("Configure Proxmox on the control plane before using this feature."),
                _ => None,
            };
            if let Some(message) = message {
                return Err(message.into());
            }
        }
        if path.starts_with("/api/v1/provisioning/") {
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
            409 => "This request conflicts with active work or a previously used request key.",
            404 => "This feature is not available on the selected control plane.",
            413 => "Definition exceeds 64 KiB.",
            422 if path.starts_with("/api/v1/provisioning/") => "Cannot preview this configuration. Check the YAML, image mapping, target VMID and template disk size.",
            503 if path == "/api/v1/provisioning/preflight" => "Configure the Proxmox provider and VMANYWARE_TEMPLATES on the control plane before checking provisioning.",
            503 if path == "/api/v1/provisioning/plan" => "Configure VMANYWARE_TEMPLATES on the control plane before previewing provisioning.",
            422 => "Definition is invalid. Check its schema, resource limits and inline public SSH key.",
            503 if path == "/api/v1/provider/inventory" => "The control plane is reachable. Configure Proxmox on the server, then retry.",
            503 => "Control-plane storage is unavailable. Check the control plane and retry.",
            502 => "The control plane cannot read Proxmox. Check its token permissions, endpoint and TLS trust.",
            504 => "Proxmox checks timed out. Check the server connection and retry.",
            _ => "The control plane rejected the request. Check that you are connecting to VMAnyware.",
        }.into());
    }
    serde_json::from_slice(&body)
        .map_err(|_| "The control plane returned an invalid response.".into())
}

#[cfg(test)]
#[path = "control_plane/tests.rs"]
mod tests;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceProvider {
    provider: String,
    scope_id: String,
    node: String,
    vmid: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    id: String,
    definition_name: String,
    definition_hash: String,
    definition_revision: u32,
    provider_reference: InstanceProvider,
    status: InstanceStatus,
    created_at: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Registered,
}

#[derive(PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VmPowerState {
    Running,
    Stopped,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VmQmpState {
    Running,
    Stopped,
    Paused,
    Prelaunch,
    Shutdown,
    Suspended,
    GuestPanicked,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmObservation {
    node: String,
    vmid: u32,
    status: VmPowerState,
    qmp_status: Option<VmQmpState>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceObservation {
    instance_id: String,
    provider_scope_id: String,
    observation: VmObservation,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceOperation {
    id: String,
    kind: String,
    instance_id: String,
    status: OperationStatus,
    result: Option<InstanceObservation>,
    failure: Option<String>,
    created_at: String,
    updated_at: String,
}

fn valid_instance_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn checked_instance_operation(
    operation: InstanceOperation,
    instance_id: &str,
) -> Result<InstanceOperation, String> {
    if operation.kind != "instance_observe"
        || operation.instance_id != instance_id
        || !valid_instance_id(&operation.id)
        || operation
            .result
            .as_ref()
            .is_some_and(|result| result.instance_id != instance_id)
    {
        return Err("The control plane returned an unrelated instance operation.".into());
    }
    Ok(operation)
}

#[tauri::command]
pub async fn list_instances(port: u16) -> Result<Vec<InstanceSummary>, String> {
    request(port, "/api/v1/environments", None).await
}

#[tauri::command]
pub async fn begin_instance_refresh(
    port: u16,
    instance_id: String,
    request_key: String,
) -> Result<InstanceOperation, String> {
    if !valid_instance_id(&instance_id) {
        return Err("Invalid instance identifier.".into());
    }
    if request_key.is_empty()
        || request_key.len() > 64
        || !request_key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err("Invalid observation request key.".into());
    }
    let operation = request(
        port,
        &format!("/api/v1/environments/{instance_id}/refresh"),
        Some(Payload::RequestKey(request_key)),
    )
    .await?;
    checked_instance_operation(operation, &instance_id)
}

#[tauri::command]
pub async fn read_instance_operation(
    port: u16,
    id: String,
    instance_id: String,
) -> Result<InstanceOperation, String> {
    if !valid_instance_id(&id) || !valid_instance_id(&instance_id) {
        return Err("Invalid operation or instance identifier.".into());
    }
    let operation: InstanceOperation =
        request(port, &format!("/api/v1/operations/{id}"), None).await?;
    if operation.id != id {
        return Err("The control plane returned an unrelated instance operation.".into());
    }
    checked_instance_operation(operation, &instance_id)
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleReport {
    instance_id: String,
    provider_scope_id: String,
    definition_hash: String,
    expected_marker: String,
    config_digest: String,
    observation: VmObservation,
    checks: Vec<PreflightCheck>,
    start_preconditions: PreflightStatus,
    shutdown_preconditions: PreflightStatus,
    execution_enabled: bool,
}

#[tauri::command]
pub async fn check_instance_lifecycle(
    port: u16,
    instance_id: String,
) -> Result<LifecycleReport, String> {
    if !valid_instance_id(&instance_id) {
        return Err("Invalid instance identifier.".into());
    }
    let report: LifecycleReport = request(
        port,
        &format!("/api/v1/environments/{instance_id}/lifecycle-preflight"),
        None,
    )
    .await?;
    if report.instance_id != instance_id
        || report.expected_marker != format!("vmanyware-{instance_id}")
    {
        return Err(
            "The control plane returned an unrelated or invalid lifecycle preflight.".into(),
        );
    }
    Ok(report)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    Start,
    Shutdown,
}
impl LifecycleAction {
    fn suffix(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Shutdown => "shutdown",
        }
    }
    fn kind(self) -> &'static str {
        match self {
            Self::Start => "vm_start",
            Self::Shutdown => "vm_shutdown",
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LifecycleApproval {
    action: LifecycleAction,
    instance_id: String,
    provider_scope_id: String,
    node: String,
    vmid: u32,
    definition_hash: String,
    config_digest: String,
    confirmed: bool,
}
fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl LifecycleApproval {
    fn valid(&self) -> bool {
        self.confirmed
            && lower_hex(&self.instance_id, 32)
            && lower_hex(&self.provider_scope_id, 64)
            && lower_hex(&self.definition_hash, 64)
            && lower_hex(&self.config_digest, 40)
            && (100..=999_999_999).contains(&self.vmid)
            && !self.node.is_empty()
            && self.node.len() <= 63
            && self.node.as_bytes()[0].is_ascii_alphanumeric()
            && self
                .node
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleResult {
    action: LifecycleAction,
    instance_id: String,
    provider_scope_id: String,
    task_status: String,
    observation: VmObservation,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleOperation {
    id: String,
    kind: String,
    instance_id: String,
    status: OperationStatus,
    result: Option<LifecycleResult>,
    failure: Option<String>,
    #[serde(default)]
    provider_task_known: bool,
    created_at: String,
    updated_at: String,
}
fn checked_lifecycle_operation(
    operation: LifecycleOperation,
    approval: &LifecycleApproval,
) -> Result<LifecycleOperation, String> {
    let success = operation.status == OperationStatus::Succeeded;
    let failed = matches!(
        operation.status,
        OperationStatus::Failed | OperationStatus::Interrupted | OperationStatus::Uncertain
    );
    if !lower_hex(&operation.id, 32)
        || operation.kind != approval.action.kind()
        || operation.instance_id != approval.instance_id
        || success != operation.result.is_some()
        || failed != operation.failure.is_some()
        || (success && !operation.provider_task_known)
        || operation.failure.as_ref().is_some_and(|code| {
            !matches!(
                code.as_str(),
                "provider_not_configured"
                    | "provider_unauthorized"
                    | "provider_forbidden"
                    | "provider_timeout"
                    | "provider_unavailable"
                    | "process_interrupted"
                    | "provider_scope_changed"
                    | "instance_reference_invalid"
                    | "lifecycle_disabled"
                    | "approval_expired"
                    | "lifecycle_target_changed"
                    | "lifecycle_preconditions_failed"
                    | "provider_submission_unconfirmed"
                    | "provider_task_failed"
                    | "provider_task_unconfirmed"
                    | "power_state_unconfirmed"
            )
        })
        || operation.result.as_ref().is_some_and(|result| {
            result.action != approval.action
                || result.instance_id != approval.instance_id
                || result.provider_scope_id != approval.provider_scope_id
                || result.task_status != "succeeded"
                || result.observation.node != approval.node
                || result.observation.vmid != approval.vmid
                || result.observation.status
                    != match approval.action {
                        LifecycleAction::Start => VmPowerState::Running,
                        LifecycleAction::Shutdown => VmPowerState::Stopped,
                    }
        })
    {
        return Err(
            "The control plane returned an unrelated or invalid lifecycle operation.".into(),
        );
    }
    Ok(operation)
}

#[tauri::command]
pub async fn begin_lifecycle_action(
    port: u16,
    approval: LifecycleApproval,
    request_key: String,
) -> Result<LifecycleOperation, String> {
    if !approval.valid()
        || request_key.is_empty()
        || request_key.len() > 64
        || !request_key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err("Invalid lifecycle confirmation or request key.".into());
    }
    let body = serde_json::to_value(&approval).map_err(|_| "Invalid lifecycle confirmation.")?;
    let operation = request(
        port,
        &format!(
            "/api/v1/environments/{}/{}",
            approval.instance_id,
            approval.action.suffix()
        ),
        Some(Payload::ConfirmedJson(request_key, body)),
    )
    .await?;
    checked_lifecycle_operation(operation, &approval)
}

/// GET-only resumption does not require current write capability or a new confirmation.
#[tauri::command]
pub async fn read_lifecycle_operation(
    port: u16,
    id: String,
    approval: LifecycleApproval,
) -> Result<LifecycleOperation, String> {
    if !approval.valid() || !lower_hex(&id, 32) {
        return Err("Invalid lifecycle operation or target.".into());
    }
    let operation: LifecycleOperation =
        request(port, &format!("/api/v1/operations/{id}"), None).await?;
    if operation.id != id {
        return Err("The control plane returned an unrelated lifecycle operation.".into());
    }
    checked_lifecycle_operation(operation, &approval)
}
