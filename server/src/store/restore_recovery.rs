//! Read-only, instance-scoped journal recovery. Never claims or re-enqueues work.
use super::{snapshot_restore::read_job, RestoreError, Store};
use crate::snapshot::RecoveryTarget;
use crate::snapshot_restore::{History, HistoryEntry, Job, Recovered, ResultEvidence};
use crate::{operation::Status, proxmox::TaskStatus};
use rusqlite::{params, OptionalExtension};

fn checked_recovery(job: Job) -> Result<Recovered, RestoreError> {
    let mut operation = job.operation;
    let approval = job.approval;
    use crate::operation::Failure;
    let known = job.provider_task.is_some();
    let consistent = (!known || job.dispatch_started)
        && match operation.status {
            Status::Queued => {
                !job.dispatch_started && operation.result.is_none() && operation.failure.is_none()
            }
            Status::Running => operation.result.is_none() && operation.failure.is_none(),
            Status::Succeeded => job.dispatch_started && known,
            Status::Interrupted => {
                !job.dispatch_started
                    && !known
                    && operation.result.is_none()
                    && operation.failure == Some(Failure::ProcessInterrupted)
            }
            Status::Failed => {
                operation.result.is_none()
                    && operation.failure.is_some_and(|failure| {
                        if job.dispatch_started {
                            if known {
                                failure == Failure::ProviderTaskFailed
                            } else {
                                matches!(
                                    failure,
                                    Failure::ProviderUnauthorized | Failure::ProviderForbidden
                                )
                            }
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
            Status::Uncertain => {
                job.dispatch_started
                    && operation.result.is_none()
                    && operation.failure.is_some_and(|failure| {
                        if known {
                            matches!(
                                failure,
                                Failure::ProviderTaskUnconfirmed
                                    | Failure::RestoreStateUnconfirmed
                                    | Failure::ProcessInterrupted
                                    | Failure::ProviderScopeChanged
                                    | Failure::ProviderNotConfigured
                            )
                        } else {
                            failure == Failure::ProviderSubmissionUnconfirmed
                        }
                    })
            }
        };
    if !consistent {
        return Err(RestoreError::Unavailable);
    }
    if operation.status == Status::Succeeded {
        let evidence: ResultEvidence =
            serde_json::from_value(operation.result.clone().ok_or(RestoreError::Unavailable)?)
                .map_err(|_| RestoreError::Unavailable)?;
        if !operation.provider_task_known
            || operation.failure.is_some()
            || evidence.task_status != TaskStatus::Succeeded
            || evidence.instance_id != approval.instance_id
            || evidence.provider_scope_id != approval.provider_scope_id
            || evidence.node != approval.node
            || evidence.vmid != approval.vmid
            || evidence.snapshot_name != approval.request.snapshot_name
            || evidence.snapshot_config_hash != approval.snapshot_config_hash
            || evidence.observation.node != approval.node
            || evidence.observation.vmid != approval.vmid
            || evidence.observation.status != crate::proxmox::PowerState::Stopped
            || !matches!(
                evidence.observation.qmp_status,
                None | Some(crate::proxmox::QmpState::Stopped)
            )
        {
            return Err(RestoreError::Unavailable);
        }
        // Only the typed public evidence is forwarded, even for a damaged journal.
        operation.result =
            Some(serde_json::to_value(evidence).map_err(|_| RestoreError::Unavailable)?);
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
        request: approval.request,
        snapshot_config_hash: approval.snapshot_config_hash,
        submission_started: job.dispatch_started,
    })
}

impl Store {
    pub fn recover_restore(
        &self,
        instance_id: &str,
        operation_id: &str,
    ) -> Result<Option<Recovered>, RestoreError> {
        let mut connection = self.0.lock().map_err(|_| RestoreError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| RestoreError::Unavailable)?;
        // Resolve scope before decoding the journal; another instance's corrupted
        // record must not disclose information through this endpoint's errors.
        let matching=transaction.query_row("SELECT 1 FROM operations WHERE id=?1 AND instance_id=?2 AND kind='vm_snapshot_restore'",params![operation_id,instance_id],|_|Ok(())).optional().map_err(|_|RestoreError::Unavailable)?.is_some();
        if !matching {
            return Ok(None);
        }
        let job = read_job(&transaction, operation_id)?.ok_or(RestoreError::Unavailable)?;
        let recovered = checked_recovery(job)?;
        transaction
            .commit()
            .map_err(|_| RestoreError::Unavailable)?;
        Ok(Some(recovered))
    }

    pub fn restore_history(&self, instance_id: &str) -> Result<History, RestoreError> {
        let mut connection = self.0.lock().map_err(|_| RestoreError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| RestoreError::Unavailable)?;
        if !transaction
            .query_row("SELECT 1 FROM instances WHERE id=?1", [instance_id], |_| {
                Ok(())
            })
            .optional()
            .map_err(|_| RestoreError::Unavailable)?
            .is_some()
        {
            return Err(RestoreError::InstanceNotFound);
        }
        let ids = {
            let mut statement=transaction.prepare("SELECT id FROM operations WHERE instance_id=?1 AND kind='vm_snapshot_restore' ORDER BY created_at DESC,id DESC LIMIT 100").map_err(|_|RestoreError::Unavailable)?;
            let rows = statement
                .query_map([instance_id], |row| row.get::<_, String>(0))
                .map_err(|_| RestoreError::Unavailable)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| RestoreError::Unavailable)?
        };
        let mut operations = Vec::with_capacity(ids.len());
        for id in ids {
            let job = read_job(&transaction, &id)?.ok_or(RestoreError::Unavailable)?;
            let recovered = checked_recovery(job)?;
            operations.push(HistoryEntry {
                id: recovered.operation.id,
                snapshot_name: recovered.request.snapshot_name,
                status: recovered.operation.status,
                failure: recovered.operation.failure,
                provider_task_known: recovered.operation.provider_task_known,
                created_at: recovered.operation.created_at,
            });
        }
        transaction
            .commit()
            .map_err(|_| RestoreError::Unavailable)?;
        Ok(History {
            instance_id: instance_id.into(),
            operations,
        })
    }
}
