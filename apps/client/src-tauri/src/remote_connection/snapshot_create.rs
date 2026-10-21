//! Retained disk-only snapshot review/confirmation; no renderer-controlled URL or approval.
use super::{hex, identifier, snapshots::Snapshot, Profile, RemoteConnection, RemoteProvider};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub(super) struct CreationState {
    revision: u64,
    prepared: Option<Prepared>,
}
struct Prepared {
    key: String,
    review: Review,
    submitted: bool,
    inflight: bool,
    operation: Option<String>,
    terminal: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Request {
    name: String,
    description: String,
}
pub(super) fn valid_name(name: &str) -> bool {
    (2..=40).contains(&name.len())
        && name != "current"
        && name.as_bytes()[0].is_ascii_alphabetic()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
impl Request {
    pub(super) fn valid(&self) -> bool {
        valid_name(&self.name)
            && self.description.len() <= 4096
            && !self
                .description
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t')
    }
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Passed,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Check {
    code: String,
    status: CheckStatus,
}
const CHECKS: [&str; 9] = [
    "instance_marker",
    "not_template",
    "unlocked",
    "no_pending_changes",
    "configuration_unchanged",
    "snapshot_supported",
    "snapshot_name_available",
    "snapshot_target_unchanged",
    "snapshot_tree_complete",
];
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    instance_id: String,
    provider_scope_id: String,
    definition_hash: String,
    node: String,
    vmid: u32,
    config_digest: String,
    request: Request,
    includes_ram: bool,
    checks: Vec<Check>,
    creation_preconditions: CheckStatus,
    execution_enabled: bool,
}
impl Review {
    fn valid(&self) -> bool {
        hex(&self.instance_id, 32)
            && hex(&self.provider_scope_id, 64)
            && hex(&self.definition_hash, 64)
            && identifier(&self.node, 63)
            && (100..=999999999).contains(&self.vmid)
            && hex(&self.config_digest, 40)
            && self.request.valid()
            && !self.includes_ram
            && self.checks.len() == CHECKS.len()
            && CHECKS.iter().all(|code| {
                self.checks
                    .iter()
                    .filter(|check| check.code == *code)
                    .count()
                    == 1
            })
            && (self.creation_preconditions != CheckStatus::Passed
                || self
                    .checks
                    .iter()
                    .all(|check| check.status == CheckStatus::Passed))
    }
    fn eligible(&self) -> bool {
        self.execution_enabled && self.creation_preconditions == CheckStatus::Passed
    }
    fn approval(&self) -> serde_json::Value {
        serde_json::json!({"instanceId":self.instance_id,"providerScopeId":self.provider_scope_id,"definitionHash":self.definition_hash,"node":self.node,"vmid":self.vmid,"configDigest":self.config_digest,"request":self.request,"confirmed":true})
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    id: String,
    definition_hash: String,
    provider_reference: RemoteProvider,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Status {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
    Uncertain,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Failure {
    SnapshotDisabled,
    SnapshotTargetChanged,
    SnapshotPreconditionsFailed,
    ApprovalExpired,
    ProviderNotConfigured,
    ProviderUnauthorized,
    ProviderForbidden,
    ProviderTimeout,
    ProviderUnavailable,
    ProviderScopeChanged,
    InstanceReferenceInvalid,
    ProviderSubmissionUnconfirmed,
    ProviderTaskFailed,
    ProviderTaskUnconfirmed,
    SnapshotStateUnconfirmed,
    ProcessInterrupted,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    id: String,
    kind: String,
    instance_id: String,
    status: Status,
    #[serde(default)]
    provider_task_known: bool,
    result: Option<Evidence>,
    failure: Option<Failure>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Evidence {
    instance_id: String,
    provider_scope_id: String,
    node: String,
    vmid: u32,
    task_status: TaskStatus,
    snapshot: Snapshot,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Succeeded,
}
impl Operation {
    fn valid(&self, review: &Review, expected: Option<&str>) -> bool {
        self.valid_context(
            &review.instance_id,
            &review.provider_scope_id,
            &review.node,
            review.vmid,
            &review.request,
            expected,
        )
    }
    pub(super) fn valid_context(
        &self,
        instance_id: &str,
        provider_scope_id: &str,
        node: &str,
        vmid: u32,
        request: &Request,
        expected: Option<&str>,
    ) -> bool {
        if !hex(&self.id, 32)
            || self.kind != "vm_snapshot_create"
            || self.instance_id != instance_id
            || expected.is_some_and(|id| id != self.id)
        {
            return false;
        }
        match self.status {
            Status::Succeeded => {
                self.provider_task_known
                    && self.failure.is_none()
                    && self.result.as_ref().is_some_and(|result| {
                        result.instance_id == instance_id
                            && result.provider_scope_id == provider_scope_id
                            && result.node == node
                            && result.vmid == vmid
                            && result.snapshot.name == request.name
                            && result.snapshot.description == request.description
                            && result.snapshot.includes_ram == Some(false)
                            && result.snapshot.provider_state.is_none()
                    })
            }
            Status::Failed | Status::Interrupted | Status::Uncertain => {
                self.result.is_none() && self.failure.is_some()
            }
            Status::Queued | Status::Running => self.result.is_none() && self.failure.is_none(),
        }
    }
    pub(super) fn consistent_submission(&self, started: bool) -> bool {
        (!self.provider_task_known || started)
            && match self.status {
                Status::Queued => !started,
                Status::Succeeded | Status::Uncertain => started,
                _ => true,
            }
    }
    fn terminal(&self) -> bool {
        matches!(
            self.status,
            Status::Succeeded | Status::Failed | Status::Interrupted
        )
    }
}
impl Profile {
    pub(super) async fn snapshot_json<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        accepted: bool,
    ) -> Result<T, String> {
        let mut response=request.header(reqwest::header::AUTHORIZATION,self.authorization.clone()).send().await.map_err(|_|"Snapshot request interrupted. If confirmed, retain the same request key and retry only that request.")?;
        if response.status().as_u16() == 429 {
            return Err(super::RATE_LIMITED.into());
        }
        if response.status().as_u16() == 401 {
            return Err("The server rejected the access token.".into());
        }
        if response
            .headers()
            .get("x-vmanyware-authenticated")
            .and_then(|value| value.to_str().ok())
            != Some("bearer")
        {
            return Err("The server does not confirm access-token authentication.".into());
        }
        if response.status().as_u16() != if accepted { 202 } else { 200 } {
            return Err(match response.status().as_u16() {403=>"Snapshot action is disabled or access was denied.",409=>"Snapshot inputs conflict with the server. Retain the original request key and inspect its operation.",_=>"Cannot complete the snapshot request. If confirmed, retain the same request key."}.into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Snapshot response interrupted; retain the request key.")?
        {
            if bytes.len() + chunk.len() > 256 * 1024 {
                return Err("The server returned an invalid snapshot response.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "The server returned an invalid snapshot response.".into())
    }
}
impl RemoteConnection {
    fn snapshot_revision(&self, id: u64, revision: u64) -> Result<(), String> {
        self.current(id)?;
        if self
            .0
            .lock()
            .map_err(|_| "Connection state unavailable.")?
            .snapshot_creation
            .revision
            != revision
        {
            return Err("Discarded an outdated snapshot preparation.".into());
        }
        Ok(())
    }
    async fn prepare_snapshot(
        &self,
        id: u64,
        key: String,
        instance_id: String,
        request: Request,
    ) -> Result<Review, String> {
        if key.is_empty()
            || key.len() > 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || !hex(&instance_id, 32)
            || !request.valid()
        {
            return Err("Supply a valid snapshot request, instance and request key.".into());
        }
        let profile = self.snapshot(id)?;
        let revision = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let snapshot = &mut state.snapshot_creation;
            if snapshot
                .prepared
                .as_ref()
                .is_some_and(|intent| intent.submitted)
            {
                return Err("Read or retry the submitted snapshot before preparing another. Disconnect does not cancel it.".into());
            }
            snapshot.revision = snapshot
                .revision
                .checked_add(1)
                .ok_or("Snapshot state unavailable.")?;
            snapshot.prepared = None;
            snapshot.revision
        };
        let target: Target = profile
            .snapshot_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/environments/{instance_id}"))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await?;
        self.snapshot_revision(id, revision)?;
        let reference = &target.provider_reference;
        if target.id != instance_id
            || !hex(&target.definition_hash, 64)
            || reference.provider != "proxmox"
            || !hex(&reference.scope_id, 64)
            || !identifier(&reference.node, 63)
            || !(100..=999999999).contains(&reference.vmid)
        {
            return Err("The server returned an invalid snapshot target.".into());
        }
        let review: Review = profile
            .snapshot_json(
                profile
                    .client
                    .post(
                        profile
                            .endpoint
                            .join(&format!(
                                "api/v1/environments/{instance_id}/snapshot-preflight"
                            ))
                            .map_err(|_| "Invalid server address.")?,
                    )
                    .json(&request),
                false,
            )
            .await?;
        self.snapshot_revision(id, revision)?;
        if !review.valid()
            || review.instance_id != instance_id
            || review.provider_scope_id != reference.scope_id
            || review.node != reference.node
            || review.vmid != reference.vmid
            || review.definition_hash != target.definition_hash
            || review.request != request
        {
            return Err("The server returned a mismatched snapshot review.".into());
        }
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id || state.snapshot_creation.revision != revision {
            return Err("Discarded an outdated snapshot preparation.".into());
        }
        state.snapshot_creation.prepared = Some(Prepared {
            key,
            review: review.clone(),
            submitted: false,
            inflight: false,
            operation: None,
            terminal: false,
        });
        Ok(review)
    }
    async fn confirm_snapshot(
        &self,
        id: u64,
        key: &str,
        confirmed: bool,
    ) -> Result<Operation, String> {
        if !confirmed {
            return Err("Explicitly confirm the displayed snapshot target and request.".into());
        }
        let profile = self.snapshot(id)?;
        let review = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let intent = state
                .snapshot_creation
                .prepared
                .as_mut()
                .filter(|intent| intent.key == key)
                .ok_or("Prepare this snapshot first.")?;
            if !intent.review.eligible() {
                return Err(
                    "Snapshot prerequisites or server capability do not permit creation.".into(),
                );
            }
            if intent.inflight {
                return Err("Snapshot submission is already in progress.".into());
            }
            if intent.operation.is_some() {
                return Err(
                    "Read the retained snapshot operation instead of submitting again.".into(),
                );
            }
            intent.submitted = true;
            intent.inflight = true;
            intent.review.clone()
        };
        let result: Result<Operation, String> = profile
            .snapshot_json(
                profile
                    .client
                    .post(
                        profile
                            .endpoint
                            .join(&format!(
                                "api/v1/environments/{}/snapshots",
                                review.instance_id
                            ))
                            .map_err(|_| "Invalid server address.")?,
                    )
                    .header("idempotency-key", key)
                    .json(&review.approval()),
                true,
            )
            .await;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err(
                "Connection changed; the submitted snapshot may continue on the server.".into(),
            );
        }
        let intent = state
            .snapshot_creation
            .prepared
            .as_mut()
            .filter(|intent| intent.key == key)
            .ok_or("Snapshot state unavailable.")?;
        intent.inflight = false;
        let operation = result?;
        if !operation.valid(&intent.review, None) {
            return Err(
                "The server returned a mismatched snapshot operation. Retain the request key."
                    .into(),
            );
        }
        intent.operation = Some(operation.id.clone());
        intent.terminal = operation.terminal();
        Ok(operation)
    }
    async fn snapshot_operation(&self, id: u64, key: &str) -> Result<Operation, String> {
        let profile = self.snapshot(id)?;
        let (review, operation_id) = {
            let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let intent = state
                .snapshot_creation
                .prepared
                .as_ref()
                .filter(|intent| intent.key == key)
                .ok_or("Prepare this snapshot first.")?;
            (
                intent.review.clone(),
                intent.operation.clone().ok_or(
                    "No operation ID was received. Retry only the same confirmed request.",
                )?,
            )
        };
        let result: Operation = profile
            .snapshot_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/operations/{operation_id}"))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await?;
        self.current(id)?;
        if !result.valid(&review, Some(&operation_id)) {
            return Err("The server returned a mismatched snapshot operation.".into());
        }
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed.".into());
        }
        let intent = state
            .snapshot_creation
            .prepared
            .as_mut()
            .filter(|intent| {
                intent.key == key && intent.operation.as_deref() == Some(&operation_id)
            })
            .ok_or("Snapshot state changed.")?;
        intent.terminal = result.terminal();
        Ok(result)
    }
    fn clear_snapshot(&self, id: u64, key: &str) -> Result<(), String> {
        self.current(id)?;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed.".into());
        }
        let snapshot = &mut state.snapshot_creation;
        let intent = snapshot
            .prepared
            .as_ref()
            .filter(|intent| intent.key == key)
            .ok_or("Snapshot state changed.")?;
        if intent.inflight || (intent.submitted && !intent.terminal) {
            return Err("Read or retry the retained snapshot before clearing it. Disconnect does not cancel it.".into());
        }
        snapshot.revision = snapshot
            .revision
            .checked_add(1)
            .ok_or("Snapshot state unavailable.")?;
        snapshot.prepared = None;
        Ok(())
    }
}
#[tauri::command]
pub async fn prepare_remote_snapshot(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    instance_id: String,
    name: String,
    description: String,
) -> Result<Review, String> {
    state
        .prepare_snapshot(
            connection_id,
            request_key,
            instance_id,
            Request { name, description },
        )
        .await
}
#[tauri::command]
pub async fn confirm_remote_snapshot(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    confirmed: bool,
) -> Result<Operation, String> {
    state
        .confirm_snapshot(connection_id, &request_key, confirmed)
        .await
}
#[tauri::command]
pub async fn read_remote_snapshot_operation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<Operation, String> {
    state.snapshot_operation(connection_id, &request_key).await
}
#[tauri::command]
pub fn clear_remote_snapshot(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<(), String> {
    state.clear_snapshot(connection_id, &request_key)
}

#[cfg(test)]
#[path = "snapshot_create_tests.rs"]
mod tests;

#[cfg(test)]
impl RemoteConnection {
    pub(super) fn snapshot_creation_is_empty(&self) -> bool {
        self.0.lock().unwrap().snapshot_creation.prepared.is_none()
    }
}
