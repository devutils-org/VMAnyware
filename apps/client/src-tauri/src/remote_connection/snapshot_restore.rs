//! Retained destructive restore review; target, revisions and approval stay native.
use super::{hex, identifier, snapshots::Snapshot, RemoteConnection, RemoteProvider};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub(super) struct RestoreState {
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
    snapshot_name: String,
}
impl Request {
    pub(super) fn valid(&self) -> bool {
        super::snapshot_create::valid_name(&self.snapshot_name)
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
const CHECKS: [&str; 13] = [
    "instance_marker",
    "not_template",
    "unlocked",
    "no_pending_changes",
    "configuration_unchanged",
    "restore_vm_stopped",
    "restore_snapshot_exists",
    "restore_snapshot_complete",
    "restore_snapshot_disk_only",
    "restore_snapshot_identity",
    "restore_snapshot_stable",
    "restore_snapshot_storage",
    "restore_target_unchanged",
];
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum PowerState {
    Running,
    Stopped,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    node: String,
    vmid: u32,
    status: PowerState,
    qmp_status: Option<String>,
}
impl Observation {
    fn stopped_target(&self, node: &str, vmid: u32) -> bool {
        self.node == node
            && self.vmid == vmid
            && matches!(self.status, PowerState::Stopped)
            && self
                .qmp_status
                .as_deref()
                .is_none_or(|qmp| qmp == "stopped")
    }
}
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
    observation: Observation,
    selected_snapshot: Option<Snapshot>,
    snapshot_config_hash: Option<String>,
    destructive: bool,
    starts_vm: bool,
    checks: Vec<Check>,
    restore_preconditions: CheckStatus,
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
            && self.destructive
            && !self.starts_vm
            && self
                .snapshot_config_hash
                .as_ref()
                .is_none_or(|hash| hex(hash, 64))
            && self.observation.node == self.node
            && self.observation.vmid == self.vmid
            && self
                .observation
                .qmp_status
                .as_ref()
                .is_none_or(|qmp| qmp.len() <= 64 && !qmp.chars().any(char::is_control))
            && self.selected_snapshot.as_ref().is_none_or(|snapshot| {
                snapshot.name == self.request.snapshot_name && snapshot.description.len() <= 65536
            })
            && (self.restore_preconditions != CheckStatus::Passed
                || (self.snapshot_config_hash.is_some()
                    && self.observation.stopped_target(&self.node, self.vmid)
                    && self.selected_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot.includes_ram == Some(false) && snapshot.provider_state.is_none()
                    })))
            && self.checks.len() == CHECKS.len()
            && CHECKS.iter().all(|code| {
                self.checks
                    .iter()
                    .filter(|check| check.code == *code)
                    .count()
                    == 1
            })
            && (self.restore_preconditions != CheckStatus::Passed
                || self
                    .checks
                    .iter()
                    .all(|check| check.status == CheckStatus::Passed))
    }
    fn eligible(&self) -> bool {
        self.execution_enabled && self.restore_preconditions == CheckStatus::Passed
    }
    fn approval(&self) -> serde_json::Value {
        serde_json::json!({"instanceId":self.instance_id,"providerScopeId":self.provider_scope_id,"definitionHash":self.definition_hash,"node":self.node,"vmid":self.vmid,"configDigest":self.config_digest,"request":self.request,"snapshotConfigHash":self.snapshot_config_hash,"confirmed":true,"acknowledgeDataLoss":true})
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
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Failure {
    RestoreDisabled,
    RestoreTargetChanged,
    RestorePreconditionsFailed,
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
    RestoreStateUnconfirmed,
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
    snapshot_name: String,
    snapshot_config_hash: String,
    observation: Observation,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Succeeded,
}
// Borrow only recorded validation data; this context carries no submission consent.
pub(super) struct RestoreContext<'a> {
    pub instance_id: &'a str,
    pub provider_scope_id: &'a str,
    pub node: &'a str,
    pub vmid: u32,
    pub request: &'a Request,
    pub snapshot_config_hash: Option<&'a str>,
}
impl Operation {
    fn valid(&self, review: &Review, expected: Option<&str>) -> bool {
        self.valid_context(
            RestoreContext {
                instance_id: &review.instance_id,
                provider_scope_id: &review.provider_scope_id,
                node: &review.node,
                vmid: review.vmid,
                request: &review.request,
                snapshot_config_hash: review.snapshot_config_hash.as_deref(),
            },
            expected,
        )
    }
    pub(super) fn valid_context(
        &self,
        context: RestoreContext<'_>,
        expected: Option<&str>,
    ) -> bool {
        let RestoreContext {
            instance_id,
            provider_scope_id,
            node,
            vmid,
            request,
            snapshot_config_hash,
        } = context;
        if !hex(&self.id, 32)
            || self.kind != "vm_snapshot_restore"
            || (self.status == Status::Queued && self.provider_task_known)
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
                            && result.snapshot_name == request.snapshot_name
                            && Some(result.snapshot_config_hash.as_str()) == snapshot_config_hash
                            && result.observation.stopped_target(node, vmid)
                    })
            }
            Status::Failed => {
                self.result.is_none()
                    && self.failure.as_ref().is_some_and(|failure| {
                        if self.provider_task_known {
                            *failure == Failure::ProviderTaskFailed
                        } else {
                            matches!(
                                failure,
                                Failure::RestoreDisabled
                                    | Failure::RestoreTargetChanged
                                    | Failure::RestorePreconditionsFailed
                                    | Failure::ApprovalExpired
                                    | Failure::ProviderNotConfigured
                                    | Failure::ProviderScopeChanged
                                    | Failure::InstanceReferenceInvalid
                                    | Failure::ProviderUnauthorized
                                    | Failure::ProviderForbidden
                                    | Failure::ProviderTimeout
                                    | Failure::ProviderUnavailable
                            )
                        }
                    })
            }
            Status::Interrupted => {
                self.result.is_none()
                    && !self.provider_task_known
                    && self.failure == Some(Failure::ProcessInterrupted)
            }
            Status::Uncertain => {
                self.result.is_none()
                    && self.failure.as_ref().is_some_and(|failure| {
                        if !self.provider_task_known {
                            *failure == Failure::ProviderSubmissionUnconfirmed
                        } else {
                            matches!(
                                failure,
                                Failure::ProviderTaskUnconfirmed
                                    | Failure::RestoreStateUnconfirmed
                                    | Failure::ProcessInterrupted
                                    | Failure::ProviderScopeChanged
                                    | Failure::ProviderNotConfigured
                            )
                        }
                    })
            }
            Status::Queued | Status::Running => self.result.is_none() && self.failure.is_none(),
        }
    }
    /// Recorded dispatch state is historical context, never permission to dispatch again.
    pub(super) fn consistent_submission(&self, started: bool) -> bool {
        (!self.provider_task_known || started)
            && match self.status {
                Status::Queued | Status::Interrupted => !started,
                Status::Succeeded | Status::Uncertain => started,
                Status::Running => true,
                Status::Failed => {
                    !started
                        || self.provider_task_known
                        || matches!(
                            self.failure,
                            Some(Failure::ProviderUnauthorized | Failure::ProviderForbidden)
                        )
                }
            }
    }
    fn terminal(&self) -> bool {
        matches!(
            self.status,
            Status::Succeeded | Status::Failed | Status::Interrupted
        )
    }
}
impl RemoteConnection {
    fn restore_revision(&self, id: u64, revision: u64) -> Result<(), String> {
        self.current(id)?;
        if self
            .0
            .lock()
            .map_err(|_| "Connection state unavailable.")?
            .snapshot_restore
            .revision
            != revision
        {
            return Err("Discarded an outdated snapshot preparation.".into());
        }
        Ok(())
    }
    async fn prepare_restore(
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
            let snapshot = &mut state.snapshot_restore;
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
        self.restore_revision(id, revision)?;
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
                                "api/v1/environments/{instance_id}/snapshot-restore-preflight"
                            ))
                            .map_err(|_| "Invalid server address.")?,
                    )
                    .json(&request),
                false,
            )
            .await?;
        self.restore_revision(id, revision)?;
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
        if state.generation != id || state.snapshot_restore.revision != revision {
            return Err("Discarded an outdated snapshot preparation.".into());
        }
        state.snapshot_restore.prepared = Some(Prepared {
            key,
            review: review.clone(),
            submitted: false,
            inflight: false,
            operation: None,
            terminal: false,
        });
        Ok(review)
    }
    async fn confirm_restore(
        &self,
        id: u64,
        key: &str,
        confirmed: bool,
        acknowledge_data_loss: bool,
    ) -> Result<Operation, String> {
        if !confirmed || !acknowledge_data_loss {
            return Err("Explicitly confirm the displayed VM/snapshot and acknowledge loss of current data.".into());
        }
        let profile = self.snapshot(id)?;
        let review = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let intent = state
                .snapshot_restore
                .prepared
                .as_mut()
                .filter(|intent| intent.key == key)
                .ok_or("Prepare this snapshot first.")?;
            if !intent.review.eligible() {
                return Err(
                    "Restore prerequisites or server capability do not permit rollback.".into(),
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
                                "api/v1/environments/{}/snapshots/{}/restore",
                                review.instance_id, review.request.snapshot_name
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
            .snapshot_restore
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
    async fn restore_operation(&self, id: u64, key: &str) -> Result<Operation, String> {
        let profile = self.snapshot(id)?;
        let (review, operation_id) = {
            let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let intent = state
                .snapshot_restore
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
            .snapshot_restore
            .prepared
            .as_mut()
            .filter(|intent| {
                intent.key == key && intent.operation.as_deref() == Some(&operation_id)
            })
            .ok_or("Snapshot state changed.")?;
        intent.terminal = result.terminal();
        Ok(result)
    }
    fn clear_restore(&self, id: u64, key: &str) -> Result<(), String> {
        self.current(id)?;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed.".into());
        }
        let snapshot = &mut state.snapshot_restore;
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
pub async fn prepare_remote_snapshot_restore(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    instance_id: String,
    snapshot_name: String,
) -> Result<Review, String> {
    state
        .prepare_restore(
            connection_id,
            request_key,
            instance_id,
            Request { snapshot_name },
        )
        .await
}
#[tauri::command]
pub async fn confirm_remote_snapshot_restore(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    confirmed: bool,
    acknowledge_data_loss: bool,
) -> Result<Operation, String> {
    state
        .confirm_restore(
            connection_id,
            &request_key,
            confirmed,
            acknowledge_data_loss,
        )
        .await
}
#[tauri::command]
pub async fn read_remote_snapshot_restore_operation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<Operation, String> {
    state.restore_operation(connection_id, &request_key).await
}
#[tauri::command]
pub fn clear_remote_snapshot_restore(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<(), String> {
    state.clear_restore(connection_id, &request_key)
}

#[cfg(test)]
#[path = "snapshot_restore_tests.rs"]
mod tests;

#[cfg(test)]
impl RemoteConnection {
    pub(super) fn snapshot_restore_is_empty(&self) -> bool {
        self.0.lock().unwrap().snapshot_restore.prepared.is_none()
    }
}
