//! Fixed saved-deletion GETs; no prepared intent or consent is installed.
use super::{
    deletion::{Approval, Failure, Operation, Plan},
    hex,
    snapshot_restore::Status,
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
    status: Status,
    failure: Option<Failure>,
    provider_task_known: bool,
    approval_hash: String,
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
                    && hex(&entry.approval_hash, 64)
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
                        Status::Failed => {
                            !entry.provider_task_known
                                && entry.failure.as_ref().is_some_and(|failure| {
                                    matches!(
                                        failure,
                                        Failure::DeletionDisabled
                                            | Failure::DeletionTargetChanged
                                            | Failure::DeletionPreconditionsFailed
                                            | Failure::ApprovalExpired
                                            | Failure::ProviderNotConfigured
                                            | Failure::ProviderScopeChanged
                                            | Failure::InstanceReferenceInvalid
                                            | Failure::ProviderUnauthorized
                                            | Failure::ProviderForbidden
                                            | Failure::ProviderTimeout
                                            | Failure::ProviderUnavailable
                                    )
                                })
                        }
                        Status::Uncertain => entry.failure.as_ref().is_some_and(|failure| {
                            if entry.provider_task_known {
                                matches!(
                                    failure,
                                    Failure::ProviderTaskUnconfirmed
                                        | Failure::DeletionStateUnconfirmed
                                        | Failure::ProcessInterrupted
                                        | Failure::ProviderScopeChanged
                                        | Failure::ProviderNotConfigured
                                        | Failure::ProviderTaskFailed
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
    config_digest: String,
    plan: Plan,
    approval_hash: String,
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
        // Reconstruct serialization solely to verify saved evidence, never to prepare consent.
        let saved = Approval {
            instance_id: target.instance_id.clone(),
            provider_scope_id: target.provider_scope_id.clone(),
            definition_hash: target.definition_hash.clone(),
            node: target.node.clone(),
            vmid: target.vmid,
            config_digest: self.config_digest.clone(),
            plan: self.plan.clone(),
            confirmed: true,
            acknowledge_data_loss: true,
        };
        target.instance_id == instance_id
            && saved.valid()
            && hex(&self.approval_hash, 64)
            && saved.hash().as_deref() == Some(self.approval_hash.as_str())
            && self.operation.valid_context(&saved, Some(operation_id))
            && self
                .operation
                .consistent_submission(self.submission_started)
    }
}
fn read_failure(failure: String) -> String {
    match failure.as_str(){"The server rejected the access token."|"The server does not confirm access-token authentication."=>failure,_=>"Cannot read saved deletion operations. Check server access, availability and saved IDs.".into()}
}
impl RemoteConnection {
    async fn deletion_history(&self, id: u64, instance_id: &str) -> Result<History, String> {
        if !hex(instance_id, 32) {
            return Err("Choose a valid retained instance.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<History, String> = profile
            .deletion_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{instance_id}/deletion-operations"
                        ))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await;
        self.current(id)?;
        let history = result.map_err(read_failure)?;
        if !history.valid(instance_id) {
            return Err("The server returned an invalid deletion operation history.".into());
        }
        self.current(id)?;
        Ok(history)
    }
    async fn recover_deletion(
        &self,
        id: u64,
        instance_id: &str,
        operation_id: &str,
    ) -> Result<Recovered, String> {
        if !hex(instance_id, 32) || !hex(operation_id, 32) {
            return Err("Supply valid instance and deletion operation IDs.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<Recovered, String> = profile
            .deletion_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{instance_id}/deletion-operations/{operation_id}"
                        ))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await;
        self.current(id)?;
        let recovered = result.map_err(read_failure)?;
        if !recovered.valid(instance_id, operation_id) {
            return Err("The server returned a mismatched saved deletion operation.".into());
        }
        self.current(id)?;
        Ok(recovered)
    }
}
#[tauri::command]
pub async fn read_remote_deletion_history(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
) -> Result<History, String> {
    state.deletion_history(connection_id, &instance_id).await
}
#[tauri::command]
pub async fn recover_remote_deletion(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
    operation_id: String,
) -> Result<Recovered, String> {
    state
        .recover_deletion(connection_id, &instance_id, &operation_id)
        .await
}
#[cfg(test)]
#[path = "deletion_recovery_tests.rs"]
mod tests;
