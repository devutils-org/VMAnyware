//! Stateless fixed saved-lifecycle GETs. Never installs consent, key or review.
use super::{hex, identifier, ReadPath, RemoteConnection};
use crate::control_plane::{
    checked_lifecycle_evidence, valid_lifecycle_failure, LifecycleAction, LifecycleOperation,
    OperationStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
const INVALID: &str = "The server returned unrelated or inconsistent saved lifecycle evidence.";
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    instance_id: String,
    operations: Vec<Entry>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    action: LifecycleAction,
    status: OperationStatus,
    failure: Option<String>,
    provider_task_known: bool,
    created_at: String,
}
impl History {
    fn valid(&self, id: &str) -> bool {
        let mut ids = HashSet::new();
        self.instance_id == id
            && self.operations.len() <= 100
            && self.operations.iter().all(|op| {
                hex(&op.id, 32)
                    && ids.insert(&op.id)
                    && !op.created_at.is_empty()
                    && op.created_at.len() <= 64
                    && !op.created_at.chars().any(char::is_control)
                    && match op.status {
                        OperationStatus::Queued => !op.provider_task_known && op.failure.is_none(),
                        OperationStatus::Running => op.failure.is_none(),
                        OperationStatus::Succeeded => {
                            op.provider_task_known && op.failure.is_none()
                        }
                        OperationStatus::Failed
                        | OperationStatus::Interrupted
                        | OperationStatus::Uncertain => {
                            op.failure.as_deref().is_some_and(valid_lifecycle_failure)
                        }
                    }
            })
            && self.operations.windows(2).all(|pair| {
                (&pair[0].created_at, &pair[0].id) >= (&pair[1].created_at, &pair[1].id)
            })
    }
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    instance_id: String,
    provider_scope_id: String,
    definition_hash: String,
    node: String,
    vmid: u32,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    operation: LifecycleOperation,
    action: LifecycleAction,
    target: Target,
    submission_started: bool,
}
impl RemoteConnection {
    async fn lifecycle_history(&self, id: u64, instance: &str) -> Result<History, String> {
        if !hex(instance, 32) {
            return Err("Choose a canonical saved instance ID.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<History, String> =
            profile.get(ReadPath::LifecycleHistory(instance)).await;
        self.current(id)?;
        let history = result?;
        if !history.valid(instance) {
            return Err(INVALID.into());
        }
        Ok(history)
    }
    async fn recover_lifecycle(
        &self,
        id: u64,
        instance: &str,
        operation: &str,
    ) -> Result<Saved, String> {
        if !hex(instance, 32) || !hex(operation, 32) {
            return Err("Choose canonical saved instance and operation IDs.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<Saved, String> = profile
            .get(ReadPath::LifecycleOperation(instance, operation))
            .await;
        self.current(id)?;
        let mut saved = result?;
        let target = &saved.target;
        if target.instance_id != instance
            || !hex(&target.provider_scope_id, 64)
            || !hex(&target.definition_hash, 64)
            || !identifier(&target.node, 63)
            || !target.node.as_bytes()[0].is_ascii_alphanumeric()
            || !(100..=999999999).contains(&target.vmid)
            || saved.operation.id != operation
            || (saved.operation.provider_task_known && !saved.submission_started)
            || (saved.operation.status == OperationStatus::Queued && saved.submission_started)
        {
            return Err(INVALID.into());
        }
        saved.operation = checked_lifecycle_evidence(
            saved.operation,
            saved.action,
            instance,
            &target.provider_scope_id,
            &target.node,
            target.vmid,
        )?;
        Ok(saved)
    }
}
#[tauri::command]
pub async fn read_remote_lifecycle_history(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
) -> Result<History, String> {
    state.lifecycle_history(connection_id, &instance_id).await
}
#[tauri::command]
pub async fn recover_remote_lifecycle(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
    operation_id: String,
) -> Result<Saved, String> {
    state
        .recover_lifecycle(connection_id, &instance_id, &operation_id)
        .await
}
#[cfg(test)]
#[path = "lifecycle_recovery_tests.rs"]
mod tests;
