//! Read-only, instance-scoped journal recovery. Never claims or re-enqueues work.
use super::{lifecycle::read_job, LifecycleError, Store};
use crate::lifecycle::{History, HistoryEntry, Job, Recovered, RecoveryTarget, ResultEvidence};
use crate::{operation::Status, proxmox::TaskStatus};
use rusqlite::{params, OptionalExtension};

fn checked_recovery(job: Job) -> Result<Recovered, LifecycleError> {
    let mut operation = job.operation;
    let approval = job.approval;
    let success = operation.status == Status::Succeeded;
    let failed = matches!(
        operation.status,
        Status::Failed | Status::Interrupted | Status::Uncertain
    );
    if operation.id.len() != 32
        || !operation
            .id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || success != operation.result.is_some()
        || failed != operation.failure.is_some()
        || (operation.provider_task_known && !job.dispatch_started)
        || (operation.status == Status::Queued
            && (job.dispatch_started || operation.provider_task_known))
    {
        return Err(LifecycleError::Unavailable);
    }
    if success {
        let evidence: ResultEvidence = serde_json::from_value(
            operation
                .result
                .clone()
                .ok_or(LifecycleError::Unavailable)?,
        )
        .map_err(|_| LifecycleError::Unavailable)?;
        if !operation.provider_task_known
            || evidence.action != approval.action
            || evidence.task_status != TaskStatus::Succeeded
            || evidence.instance_id != approval.instance_id
            || evidence.provider_scope_id != approval.provider_scope_id
            || evidence.observation.node != approval.node
            || evidence.observation.vmid != approval.vmid
            || evidence.observation.status != approval.action.desired_power()
        {
            return Err(LifecycleError::Unavailable);
        }
        operation.result =
            Some(serde_json::to_value(evidence).map_err(|_| LifecycleError::Unavailable)?);
    }
    Ok(Recovered {
        operation,
        action: approval.action,
        target: RecoveryTarget {
            instance_id: approval.instance_id,
            provider_scope_id: approval.provider_scope_id,
            definition_hash: approval.definition_hash,
            node: approval.node,
            vmid: approval.vmid,
        },
        submission_started: job.dispatch_started,
    })
}

impl Store {
    pub fn recover_lifecycle(
        &self,
        instance_id: &str,
        operation_id: &str,
    ) -> Result<Option<Recovered>, LifecycleError> {
        let mut connection = self.0.lock().map_err(|_| LifecycleError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| LifecycleError::Unavailable)?;
        // Resolve scope before decoding the journal; another instance's corrupted
        // record must not disclose information through this endpoint's errors.
        let matching=transaction.query_row("SELECT 1 FROM operations WHERE id=?1 AND instance_id=?2 AND kind IN ('vm_start','vm_shutdown')",params![operation_id,instance_id],|_|Ok(())).optional().map_err(|_|LifecycleError::Unavailable)?.is_some();
        if !matching {
            return Ok(None);
        }
        let job = read_job(&transaction, operation_id)?.ok_or(LifecycleError::Unavailable)?;
        let recovered = checked_recovery(job)?;
        transaction
            .commit()
            .map_err(|_| LifecycleError::Unavailable)?;
        Ok(Some(recovered))
    }

    pub fn lifecycle_history(&self, instance_id: &str) -> Result<History, LifecycleError> {
        let mut connection = self.0.lock().map_err(|_| LifecycleError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| LifecycleError::Unavailable)?;
        if !transaction
            .query_row("SELECT 1 FROM instances WHERE id=?1", [instance_id], |_| {
                Ok(())
            })
            .optional()
            .map_err(|_| LifecycleError::Unavailable)?
            .is_some()
        {
            return Err(LifecycleError::InstanceNotFound);
        }
        let ids = {
            let mut statement=transaction.prepare("SELECT id FROM operations WHERE instance_id=?1 AND kind IN ('vm_start','vm_shutdown') ORDER BY created_at DESC,id DESC LIMIT 100").map_err(|_|LifecycleError::Unavailable)?;
            let rows = statement
                .query_map([instance_id], |row| row.get::<_, String>(0))
                .map_err(|_| LifecycleError::Unavailable)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| LifecycleError::Unavailable)?
        };
        let mut operations = Vec::with_capacity(ids.len());
        for id in ids {
            let job = read_job(&transaction, &id)?.ok_or(LifecycleError::Unavailable)?;
            let recovered = checked_recovery(job)?;
            operations.push(HistoryEntry {
                id: recovered.operation.id,
                action: recovered.action,
                status: recovered.operation.status,
                failure: recovered.operation.failure,
                provider_task_known: recovered.operation.provider_task_known,
                created_at: recovered.operation.created_at,
            });
        }
        transaction
            .commit()
            .map_err(|_| LifecycleError::Unavailable)?;
        Ok(History {
            instance_id: instance_id.into(),
            operations,
        })
    }
}
