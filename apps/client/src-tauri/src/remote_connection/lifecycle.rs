//! Retained remote lifecycle review, confirmed submission and fixed status reads.
use super::{hex, identifier, ReadPath, RemoteConnection};
use crate::control_plane::{
    checked_lifecycle_operation, LifecycleAction, LifecycleApproval, LifecycleOperation,
    OperationStatus,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
const CHECKS: [&str; 5] = [
    "instance_marker",
    "not_template",
    "unlocked",
    "no_pending_changes",
    "configuration_unchanged",
];
const INVALID: &str = "The server returned an unrelated or inconsistent lifecycle review.";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Start,
    Shutdown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Passed,
    Failed,
    Unknown,
}
#[derive(Clone, Deserialize, Serialize)]
struct Check {
    code: String,
    status: Status,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    node: String,
    vmid: u32,
    status: String,
    qmp_status: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub(super) instance_id: String,
    pub(super) provider_scope_id: String,
    pub(super) node: String,
    pub(super) vmid: u32,
}
impl Target {
    pub(super) fn valid(&self) -> bool {
        hex(&self.instance_id, 32)
            && hex(&self.provider_scope_id, 64)
            && identifier(&self.node, 63)
            && (100..=999999999).contains(&self.vmid)
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Instance {
    id: String,
    definition_hash: String,
    provider_reference: Reference,
    status: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reference {
    provider: String,
    scope_id: String,
    node: String,
    vmid: u32,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Preflight {
    instance_id: String,
    provider_scope_id: String,
    definition_hash: String,
    expected_marker: String,
    config_digest: String,
    observation: Observation,
    checks: Vec<Check>,
    start_preconditions: Status,
    shutdown_preconditions: Status,
    execution_enabled: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    target: Target,
    action: Action,
    preflight: Preflight,
}
impl Preflight {
    fn valid(&self, target: &Target, definition_hash: &str) -> bool {
        let mut codes = HashSet::new();
        if self.instance_id != target.instance_id
            || self.provider_scope_id != target.provider_scope_id
            || self.definition_hash != definition_hash
            || !hex(&self.definition_hash, 64)
            || self.expected_marker != format!("vmanyware-{}", target.instance_id)
            || self.config_digest.len() != 40
            || !self.config_digest.bytes().all(|b| b.is_ascii_hexdigit())
            || self.observation.node != target.node
            || self.observation.vmid != target.vmid
            || self.checks.len() != CHECKS.len()
            || !self
                .checks
                .iter()
                .all(|c| CHECKS.contains(&c.code.as_str()) && codes.insert(&c.code))
        {
            return false;
        }
        let qmp = self.observation.qmp_status.as_deref();
        if qmp.is_some_and(|q| {
            ![
                "running",
                "stopped",
                "paused",
                "prelaunch",
                "shutdown",
                "suspended",
                "guest-panicked",
                "unknown",
            ]
            .contains(&q)
        }) {
            return false;
        }
        let start = match (self.observation.status.as_str(), qmp) {
            ("stopped", None | Some("stopped")) => Status::Passed,
            ("stopped", _) => Status::Unknown,
            ("running", _) => Status::Failed,
            _ => return false,
        };
        let shutdown = match (self.observation.status.as_str(), qmp) {
            ("running", Some("running")) => Status::Passed,
            ("running", None | Some("unknown")) => Status::Unknown,
            _ => Status::Failed,
        };
        let combined = |power| {
            if power == Status::Failed || self.checks.iter().any(|c| c.status == Status::Failed) {
                Status::Failed
            } else if power == Status::Unknown
                || self.checks.iter().any(|c| c.status == Status::Unknown)
            {
                Status::Unknown
            } else {
                Status::Passed
            }
        };
        self.start_preconditions == combined(start)
            && self.shutdown_preconditions == combined(shutdown)
    }
}
#[derive(Default)]
pub(super) struct LifecycleState {
    epoch: u64,
    key: Option<String>,
    reading: bool,
    review: Option<Review>,
    prepared_at: Option<Instant>,
    submitted: bool,
    operation: Option<String>,
    terminal: bool,
}
impl RemoteConnection {
    async fn prepare_lifecycle(
        &self,
        id: u64,
        key: String,
        target: Target,
        action: Action,
    ) -> Result<Review, String> {
        if !target.valid() || !identifier(&key, 64) {
            return Err("Choose a valid instance, target and request key.".into());
        }
        let profile = self.snapshot(id)?;
        let epoch = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id || state.lifecycle.reading || state.lifecycle.submitted {
                return Err("The lifecycle review is busy or the connection changed.".into());
            }
            let epoch = state
                .lifecycle
                .epoch
                .checked_add(1)
                .ok_or("Lifecycle state unavailable.")?;
            state.lifecycle = LifecycleState {
                epoch,
                key: Some(key.clone()),
                reading: true,
                ..Default::default()
            };
            epoch
        };
        let result = async {
            let instance: Instance = profile
                .get(ReadPath::Environment(&target.instance_id))
                .await?;
            {
                let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
                if state.generation != id
                    || state.profile.is_none()
                    || state.lifecycle.epoch != epoch
                    || !state.lifecycle.reading
                    || state.lifecycle.key.as_deref() != Some(&key)
                {
                    return Err(
                        "The lifecycle review changed. Discarded an outdated response.".into(),
                    );
                }
            }
            if instance.id != target.instance_id
                || instance.status != "registered"
                || instance.provider_reference.provider != "proxmox"
                || instance.provider_reference.scope_id != target.provider_scope_id
                || instance.provider_reference.node != target.node
                || instance.provider_reference.vmid != target.vmid
                || !hex(&instance.definition_hash, 64)
            {
                return Err(INVALID.into());
            }
            let preflight: Preflight = profile
                .get(ReadPath::Lifecycle(&target.instance_id))
                .await?;
            if !preflight.valid(&target, &instance.definition_hash) {
                return Err(INVALID.into());
            }
            Ok(Review {
                target,
                action,
                preflight,
            })
        }
        .await;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id
            || state.profile.is_none()
            || state.lifecycle.epoch != epoch
            || state.lifecycle.key.as_deref() != Some(&key)
        {
            return Err("The lifecycle review changed. Discarded an outdated response.".into());
        }
        state.lifecycle.reading = false;
        if let Ok(review) = &result {
            state.lifecycle.review = Some(review.clone());
            state.lifecycle.prepared_at = Some(Instant::now());
        }
        result
    }
    fn clear_lifecycle(&self, id: u64, key: &str) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id
            || state.profile.is_none()
            || state.lifecycle.key.as_deref() != Some(key)
        {
            return Err("The lifecycle review changed.".into());
        }
        if state.lifecycle.submitted && (state.lifecycle.reading || !state.lifecycle.terminal) {
            return Err("Retain the submitted lifecycle request and read its operation. Disconnecting does not cancel server work.".into());
        }
        let epoch = state
            .lifecycle
            .epoch
            .checked_add(1)
            .ok_or("Lifecycle state unavailable.")?;
        state.lifecycle = LifecycleState {
            epoch,
            ..Default::default()
        };
        Ok(())
    }
}
impl Review {
    fn approval(&self) -> LifecycleApproval {
        LifecycleApproval {
            action: match self.action {
                Action::Start => LifecycleAction::Start,
                Action::Shutdown => LifecycleAction::Shutdown,
            },
            instance_id: self.target.instance_id.clone(),
            provider_scope_id: self.target.provider_scope_id.clone(),
            node: self.target.node.clone(),
            vmid: self.target.vmid,
            definition_hash: self.preflight.definition_hash.clone(),
            config_digest: self.preflight.config_digest.clone(),
            confirmed: true,
        }
    }
    fn allowed(&self) -> bool {
        self.preflight.execution_enabled
            && match self.action {
                Action::Start => self.preflight.start_preconditions == Status::Passed,
                Action::Shutdown => self.preflight.shutdown_preconditions == Status::Passed,
            }
    }
}
fn checked(
    operation: LifecycleOperation,
    review: &Review,
    known: Option<&str>,
) -> Result<LifecycleOperation, String> {
    if known.is_some_and(|id| id != operation.id)
        || (operation.status == OperationStatus::Queued && operation.provider_task_known)
    {
        return Err("The server returned an unrelated or inconsistent lifecycle operation.".into());
    }
    checked_lifecycle_operation(operation, &review.approval())
}
fn terminal(operation: &LifecycleOperation) -> bool {
    matches!(
        operation.status,
        OperationStatus::Succeeded | OperationStatus::Failed
    ) || (operation.status == OperationStatus::Interrupted && !operation.provider_task_known)
}
impl RemoteConnection {
    async fn confirm_lifecycle(
        &self,
        id: u64,
        key: &str,
        confirmed: bool,
    ) -> Result<LifecycleOperation, String> {
        if !confirmed {
            return Err("Explicitly confirm the reviewed lifecycle target and action.".into());
        }
        let profile = self.snapshot(id)?;
        let (review, known) = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id
                || state.lifecycle.key.as_deref() != Some(key)
                || state.lifecycle.reading
            {
                return Err("The lifecycle request changed or is busy.".into());
            }
            let retained = &mut state.lifecycle;
            let review = retained
                .review
                .clone()
                .ok_or("Prepare this lifecycle request first.")?;
            if !review.approval().valid() || !review.allowed() {
                return Err(
                    "Lifecycle execution is disabled or its preconditions have not passed.".into(),
                );
            }
            if !retained.submitted
                && retained
                    .prepared_at
                    .is_none_or(|time| time.elapsed() > Duration::from_secs(120))
            {
                return Err("The lifecycle review expired. Prepare and review again.".into());
            }
            retained.submitted = true;
            retained.reading = true;
            (review, retained.operation.clone())
        };
        let result = self
            .lifecycle_request(&profile, &review, key, known.as_deref())
            .await;
        self.finish_lifecycle(id, key, &review, known.as_deref(), result)
    }
    async fn lifecycle_request(
        &self,
        profile: &super::Profile,
        review: &Review,
        key: &str,
        known: Option<&str>,
    ) -> Result<LifecycleOperation, String> {
        let request = if let Some(operation) = known {
            profile.client.get(
                profile
                    .endpoint
                    .join(&format!("api/v1/operations/{operation}"))
                    .map_err(|_| "Invalid server address.")?,
            )
        } else {
            let action = match review.action {
                Action::Start => "start",
                Action::Shutdown => "shutdown",
            };
            profile
                .client
                .post(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{}/{action}",
                            review.target.instance_id
                        ))
                        .map_err(|_| "Invalid server address.")?,
                )
                .header("idempotency-key", key)
                .json(&review.approval())
        };
        let (status, body) = profile.create_response(request).await?;
        if status.as_u16() != if known.is_some() { 200 } else { 202 } {
            return Err(match status.as_u16(){403=>"Lifecycle execution is disabled or access was denied.",409=>"Lifecycle inputs conflict with saved work. Retain the same request key and inspect the operation.",_=>"Cannot complete the lifecycle request. Retain its key; retry only that request or read its known operation."}.into());
        }
        serde_json::from_slice(&body)
            .map_err(|_| "The server returned an invalid lifecycle response.".into())
    }
    fn finish_lifecycle(
        &self,
        id: u64,
        key: &str,
        review: &Review,
        known: Option<&str>,
        result: Result<LifecycleOperation, String>,
    ) -> Result<LifecycleOperation, String> {
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id
            || state.profile.is_none()
            || state.lifecycle.key.as_deref() != Some(key)
        {
            return Err(
                "The connection changed. Submitted lifecycle work may continue on the server."
                    .into(),
            );
        }
        let retained = &mut state.lifecycle;
        retained.reading = false;
        let operation = checked(result?, review, known)?;
        retained.operation = Some(operation.id.clone());
        retained.terminal = terminal(&operation);
        Ok(operation)
    }
    async fn lifecycle_operation(&self, id: u64, key: &str) -> Result<LifecycleOperation, String> {
        let profile = self.snapshot(id)?;
        let (review, known) = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id
                || state.lifecycle.key.as_deref() != Some(key)
                || state.lifecycle.reading
            {
                return Err("The lifecycle request changed or is busy.".into());
            }
            let retained = &mut state.lifecycle;
            let known = retained
                .operation
                .clone()
                .ok_or("No operation ID was received. Retry only the same confirmed request.")?;
            let review = retained
                .review
                .clone()
                .ok_or("Lifecycle review unavailable.")?;
            retained.reading = true;
            (review, known)
        };
        let result = self
            .lifecycle_request(&profile, &review, key, Some(&known))
            .await;
        self.finish_lifecycle(id, key, &review, Some(&known), result)
    }
}
#[tauri::command]
pub async fn confirm_remote_lifecycle(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    confirmed: bool,
) -> Result<LifecycleOperation, String> {
    state
        .confirm_lifecycle(connection_id, &request_key, confirmed)
        .await
}
#[tauri::command]
pub async fn read_remote_lifecycle_operation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<LifecycleOperation, String> {
    state.lifecycle_operation(connection_id, &request_key).await
}
#[tauri::command]
pub async fn prepare_remote_lifecycle(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    target: Target,
    action: Action,
) -> Result<Review, String> {
    state
        .prepare_lifecycle(connection_id, request_key, target, action)
        .await
}
#[tauri::command]
pub fn clear_remote_lifecycle(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<(), String> {
    state.clear_lifecycle(connection_id, &request_key)
}
#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
