//! Read-only saved deletion recovery; no provider, claims, expiry renewal or dispatch.
use super::{deletion::read_job, DeletionError, Store};
use crate::{
    deletion::{History, HistoryEntry, Recovered, ResultEvidence},
    instance::Status as InstanceStatus,
    operation::{Failure, Status},
    proxmox::TaskStatus,
    snapshot::RecoveryTarget,
};
use rusqlite::{params, Connection, OptionalExtension};
fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn checked(connection: &Connection, id: &str) -> Result<Recovered, DeletionError> {
    // Check persisted sizes before loading/parsing potentially damaged JSON.
    let bounded:bool=connection.query_row("SELECT length(CAST(approval_json AS BLOB))<=65536 AND (result_json IS NULL OR length(CAST(result_json AS BLOB))<=4096) AND length(created_at)<=64 AND length(updated_at)<=64 FROM operations WHERE id=?1",[id],|row|row.get(0)).map_err(|_|DeletionError::Unavailable)?;
    if !bounded || !valid_id(id) {
        return Err(DeletionError::Unavailable);
    }
    let job = read_job(connection, id)?.ok_or(DeletionError::Unavailable)?;
    let mut operation = job.operation;
    let approval = job.approval;
    let instance = super::instances::read(connection, &approval.instance_id)
        .map_err(|_| DeletionError::Unavailable)?
        .ok_or(DeletionError::Unavailable)?;
    if !approval.matches_instance(&instance.summary) || job.confirmed_at < 0 {
        return Err(DeletionError::Unavailable);
    }
    let known = job.provider_task.is_some();
    let active = matches!(
        operation.status,
        Status::Queued | Status::Running | Status::Uncertain
    );
    if active && instance.summary.status != InstanceStatus::Registered {
        return Err(DeletionError::Unavailable);
    }
    let consistent = (!known || job.dispatch_started)
        && match operation.status {
            Status::Queued => {
                !job.dispatch_started
                    && !known
                    && operation.result.is_none()
                    && operation.failure.is_none()
            }
            Status::Running => operation.result.is_none() && operation.failure.is_none(),
            Status::Succeeded => {
                job.dispatch_started && known && instance.summary.status == InstanceStatus::Deleted
            }
            Status::Interrupted => {
                !job.dispatch_started
                    && !known
                    && operation.result.is_none()
                    && operation.failure == Some(Failure::ProcessInterrupted)
            }
            Status::Failed => {
                !known
                    && operation.result.is_none()
                    && operation.failure.is_some_and(|failure| {
                        if job.dispatch_started {
                            matches!(
                                failure,
                                Failure::ProviderUnauthorized | Failure::ProviderForbidden
                            )
                        } else {
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
                        }
                    })
            }
            Status::Uncertain => {
                job.dispatch_started
                    && operation.result.is_none()
                    && operation.failure.is_some_and(|failure| {
                        if known {
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
                            failure == Failure::ProviderSubmissionUnconfirmed
                        }
                    })
            }
        };
    if !consistent
        || operation.provider_task_known != known
        || [&operation.created_at, &operation.updated_at]
            .iter()
            .any(|stamp| stamp.chars().any(char::is_control))
    {
        return Err(DeletionError::Unavailable);
    }
    let approval_hash = approval
        .input_hash()
        .map_err(|_| DeletionError::Unavailable)?;
    if operation.status == Status::Succeeded {
        let evidence: ResultEvidence =
            serde_json::from_value(operation.result.clone().ok_or(DeletionError::Unavailable)?)
                .map_err(|_| DeletionError::Unavailable)?;
        if !operation.provider_task_known
            || operation.failure.is_some()
            || evidence.task_status != TaskStatus::Succeeded
            || evidence.instance_id != approval.instance_id
            || evidence.provider_scope_id != approval.provider_scope_id
            || evidence.node != approval.node
            || evidence.vmid != approval.vmid
            || evidence.approval_hash != approval_hash
            || !evidence.vm_absence_verified
            || !evidence.volumes_absence_verified
            || !evidence.definition_retained
            || !evidence.association_retired
        {
            return Err(DeletionError::Unavailable);
        }
        operation.result =
            Some(serde_json::to_value(evidence).map_err(|_| DeletionError::Unavailable)?);
    }
    Ok(Recovered {
        operation,
        target: RecoveryTarget {
            instance_id: approval.instance_id,
            provider_scope_id: approval.provider_scope_id,
            definition_hash: approval.definition_hash,
            node: approval.node,
            vmid: approval.vmid,
        },
        config_digest: approval.config_digest,
        plan: approval.plan,
        approval_hash,
        submission_started: job.dispatch_started,
    })
}
impl Store {
    pub fn recover_deletion(
        &self,
        instance_id: &str,
        operation_id: &str,
    ) -> Result<Option<Recovered>, DeletionError> {
        let mut connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| DeletionError::Unavailable)?;
        // Resolve kind and instance before decoding: unrelated damaged records stay private.
        let matching = transaction
            .query_row(
                "SELECT 1 FROM operations WHERE id=?1 AND instance_id=?2 AND kind='vm_delete'",
                params![operation_id, instance_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| DeletionError::Unavailable)?
            .is_some();
        if !matching {
            return Ok(None);
        }
        let recovered = checked(&transaction, operation_id)?;
        transaction
            .commit()
            .map_err(|_| DeletionError::Unavailable)?;
        Ok(Some(recovered))
    }
    pub fn deletion_history(&self, instance_id: &str) -> Result<History, DeletionError> {
        let mut connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| DeletionError::Unavailable)?;
        let exists = transaction
            .query_row("SELECT 1 FROM instances WHERE id=?1", [instance_id], |_| {
                Ok(())
            })
            .optional()
            .map_err(|_| DeletionError::Unavailable)?
            .is_some();
        if !exists {
            return Err(DeletionError::InstanceNotFound);
        }
        let ids = {
            let mut statement=transaction.prepare("SELECT id FROM operations WHERE instance_id=?1 AND kind='vm_delete' ORDER BY created_at DESC,id DESC LIMIT 100").map_err(|_|DeletionError::Unavailable)?;
            let rows = statement
                .query_map([instance_id], |row| row.get::<_, String>(0))
                .map_err(|_| DeletionError::Unavailable)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| DeletionError::Unavailable)?
        };
        let mut operations = Vec::with_capacity(ids.len());
        for id in ids {
            let recovered = checked(&transaction, &id)?;
            operations.push(HistoryEntry {
                id: recovered.operation.id,
                status: recovered.operation.status,
                failure: recovered.operation.failure,
                provider_task_known: recovered.operation.provider_task_known,
                approval_hash: recovered.approval_hash,
                created_at: recovered.operation.created_at,
            });
        }
        transaction
            .commit()
            .map_err(|_| DeletionError::Unavailable)?;
        Ok(History {
            instance_id: instance_id.into(),
            operations,
        })
    }
}
