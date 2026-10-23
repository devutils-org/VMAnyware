//! Fixed GET-only saved-operation reads; never restore a mutation intent.
use super::{
    hex, identifier,
    snapshot_create::valid_name,
    snapshot_restore::{Failure, Operation, Request, RestoreContext, Status},
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
    snapshot_name: String,
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
                    && valid_name(&entry.snapshot_name)
                    && !entry.created_at.is_empty()
                    && entry.created_at.len() <= 64
                    && !entry.created_at.chars().any(char::is_control)
                    && match entry.status {
                        Status::Succeeded => entry.provider_task_known && entry.failure.is_none(),
                        Status::Queued => !entry.provider_task_known && entry.failure.is_none(),
                        Status::Running => entry.failure.is_none(),
                        Status::Interrupted => {
                            !entry.provider_task_known
                                && entry.failure == Some(Failure::ProcessInterrupted)
                        }
                        Status::Failed => entry.failure.as_ref().is_some_and(|failure| {
                            if entry.provider_task_known {
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
                        }),
                        Status::Uncertain => entry.failure.as_ref().is_some_and(|failure| {
                            if entry.provider_task_known {
                                matches!(
                                    failure,
                                    Failure::ProviderTaskUnconfirmed
                                        | Failure::RestoreStateUnconfirmed
                                        | Failure::ProcessInterrupted
                                        | Failure::ProviderScopeChanged
                                        | Failure::ProviderNotConfigured
                                )
                            } else {
                                *failure == Failure::ProviderSubmissionUnconfirmed
                            }
                        }),
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
    snapshot_config_hash: String,
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
            && hex(&self.snapshot_config_hash, 64)
            && self.operation.valid_context(
                RestoreContext {
                    instance_id,
                    provider_scope_id: &target.provider_scope_id,
                    node: &target.node,
                    vmid: target.vmid,
                    request: &self.request,
                    snapshot_config_hash: Some(&self.snapshot_config_hash),
                },
                Some(operation_id),
            )
            && self
                .operation
                .consistent_submission(self.submission_started)
    }
}
fn read_failure(failure: String) -> String {
    match failure.as_str() {
        "The server rejected the access token."
        | "The server does not confirm access-token authentication." => failure,
        _ => {
            "Cannot read saved restore operations. Check server access, availability and saved IDs."
                .into()
        }
    }
}

impl RemoteConnection {
    async fn restore_history(&self, id: u64, instance_id: &str) -> Result<History, String> {
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
                            "api/v1/environments/{instance_id}/restore-operations"
                        ))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await;
        self.current(id)?;
        let history = result.map_err(read_failure)?;
        if !history.valid(instance_id) {
            return Err("The server returned an invalid restore operation history.".into());
        }
        Ok(history)
    }
    async fn recover_restore(
        &self,
        id: u64,
        instance_id: &str,
        operation_id: &str,
    ) -> Result<Recovered, String> {
        if !hex(instance_id, 32) || !hex(operation_id, 32) {
            return Err("Supply valid instance and restore operation IDs.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<Recovered, String> = profile
            .snapshot_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{instance_id}/restore-operations/{operation_id}"
                        ))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await;
        self.current(id)?;
        let recovered = result.map_err(read_failure)?;
        if !recovered.valid(instance_id, operation_id) {
            return Err("The server returned a mismatched saved restore operation.".into());
        }
        Ok(recovered)
    }
}
#[tauri::command]
pub async fn read_remote_restore_history(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
) -> Result<History, String> {
    state.restore_history(connection_id, &instance_id).await
}
#[tauri::command]
pub async fn recover_remote_restore(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
    operation_id: String,
) -> Result<Recovered, String> {
    state
        .recover_restore(connection_id, &instance_id, &operation_id)
        .await
}

#[cfg(test)]
#[path = "restore_recovery_tests.rs"]
mod tests;
