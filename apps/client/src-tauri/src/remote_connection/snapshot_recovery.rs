//! Fixed GET-only saved-operation reads; never restore a mutation intent.
use super::{
    hex, identifier,
    snapshot_create::{valid_name, Failure, Operation, Request, Status},
    RemoteConnection,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    instance_id: String,
    operations: Vec<Entry>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    name: String,
    status: Status,
    failure: Option<Failure>,
    provider_task_known: bool,
    created_at: String,
}
impl History {
    fn valid(&self, instance_id: &str) -> bool {
        let mut ids = HashSet::new();
        self.instance_id == instance_id
            && self.operations.len() <= 100
            && self.operations.iter().all(|entry| {
                hex(&entry.id, 32)
                    && ids.insert(&entry.id)
                    && valid_name(&entry.name)
                    && !entry.created_at.is_empty()
                    && entry.created_at.len() <= 64
                    && !entry.created_at.chars().any(char::is_control)
                    && match entry.status {
                        Status::Succeeded => entry.provider_task_known && entry.failure.is_none(),
                        Status::Queued => !entry.provider_task_known && entry.failure.is_none(),
                        Status::Running => entry.failure.is_none(),
                        Status::Failed | Status::Interrupted | Status::Uncertain => {
                            entry.failure.is_some()
                        }
                    }
            })
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovered {
    operation: Operation,
    target: Target,
    request: Request,
    submission_started: bool,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    instance_id: String,
    provider_scope_id: String,
    definition_hash: String,
    node: String,
    vmid: u32,
}
impl Recovered {
    fn valid(&self, instance_id: &str, operation_id: &str) -> bool {
        let target = &self.target;
        target.instance_id == instance_id
            && hex(&target.provider_scope_id, 64)
            && hex(&target.definition_hash, 64)
            && identifier(&target.node, 63)
            && (100..=999999999).contains(&target.vmid)
            && self.request.valid()
            && self.operation.valid_context(
                instance_id,
                &target.provider_scope_id,
                &target.node,
                target.vmid,
                &self.request,
                Some(operation_id),
            )
            && self
                .operation
                .consistent_submission(self.submission_started)
    }
}
fn read_failure(failure: String) -> String {
    match failure.as_str() {
        "The server rejected the access token." | "The server does not confirm access-token authentication." => failure,
        _ => "Cannot read saved snapshot operations. Check server access, availability and saved IDs.".into(),
    }
}

impl RemoteConnection {
    async fn snapshot_history(&self, id: u64, instance_id: &str) -> Result<History, String> {
        if !hex(instance_id, 32) {
            return Err("Choose a valid registered instance.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<History, String> = profile
            .snapshot_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{instance_id}/snapshot-operations"
                        ))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await;
        self.current(id)?;
        let history = result.map_err(read_failure)?;
        if !history.valid(instance_id) {
            return Err("The server returned an invalid snapshot operation history.".into());
        }
        Ok(history)
    }
    async fn recover_snapshot(
        &self,
        id: u64,
        instance_id: &str,
        operation_id: &str,
    ) -> Result<Recovered, String> {
        if !hex(instance_id, 32) || !hex(operation_id, 32) {
            return Err("Supply valid instance and snapshot operation IDs.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<Recovered, String> = profile
            .snapshot_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{instance_id}/snapshot-operations/{operation_id}"
                        ))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await;
        self.current(id)?;
        let recovered = result.map_err(read_failure)?;
        if !recovered.valid(instance_id, operation_id) {
            return Err("The server returned a mismatched saved snapshot operation.".into());
        }
        Ok(recovered)
    }
}
#[tauri::command]
pub async fn read_remote_snapshot_history(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
) -> Result<History, String> {
    state.snapshot_history(connection_id, &instance_id).await
}
#[tauri::command]
pub async fn recover_remote_snapshot(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
    operation_id: String,
) -> Result<Recovered, String> {
    state
        .recover_snapshot(connection_id, &instance_id, &operation_id)
        .await
}

#[cfg(test)]
#[path = "snapshot_recovery_tests.rs"]
mod tests;
