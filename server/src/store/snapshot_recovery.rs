//! Read-only, instance-scoped journal recovery. Never claims or re-enqueues work.
use super::{snapshot::read_job, SnapshotError, Store};
use crate::snapshot::{History, HistoryEntry, Job, Recovered, RecoveryTarget, ResultEvidence};
use crate::{operation::Status, proxmox::TaskStatus};
use rusqlite::{params, OptionalExtension};

fn checked_recovery(job: Job) -> Result<Recovered, SnapshotError> {
    let mut operation = job.operation;
    let approval = job.approval;
    if operation.status == Status::Succeeded {
        let evidence: ResultEvidence =
            serde_json::from_value(operation.result.clone().ok_or(SnapshotError::Unavailable)?)
                .map_err(|_| SnapshotError::Unavailable)?;
        if !operation.provider_task_known
            || operation.failure.is_some()
            || evidence.task_status != TaskStatus::Succeeded
            || evidence.instance_id != approval.instance_id
            || evidence.provider_scope_id != approval.provider_scope_id
            || evidence.node != approval.node
            || evidence.vmid != approval.vmid
            || evidence.snapshot.name != approval.request.name
            || evidence.snapshot.description != approval.request.description
            || evidence.snapshot.includes_ram != Some(false)
            || evidence.snapshot.provider_state.is_some()
        {
            return Err(SnapshotError::Unavailable);
        }
        // Only the typed public evidence is forwarded, even for a damaged journal.
        operation.result =
            Some(serde_json::to_value(evidence).map_err(|_| SnapshotError::Unavailable)?);
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
        submission_started: job.dispatch_started,
    })
}

impl Store {
    pub fn recover_snapshot(
        &self,
        instance_id: &str,
        operation_id: &str,
    ) -> Result<Option<Recovered>, SnapshotError> {
        let mut connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| SnapshotError::Unavailable)?;
        // Resolve scope before decoding the journal; another instance's corrupted
        // record must not disclose information through this endpoint's errors.
        let matching=transaction.query_row("SELECT 1 FROM operations WHERE id=?1 AND instance_id=?2 AND kind='vm_snapshot_create'",params![operation_id,instance_id],|_|Ok(())).optional().map_err(|_|SnapshotError::Unavailable)?.is_some();
        if !matching {
            return Ok(None);
        }
        let job = read_job(&transaction, operation_id)?.ok_or(SnapshotError::Unavailable)?;
        let recovered = checked_recovery(job)?;
        transaction
            .commit()
            .map_err(|_| SnapshotError::Unavailable)?;
        Ok(Some(recovered))
    }

    pub fn snapshot_history(&self, instance_id: &str) -> Result<History, SnapshotError> {
        let mut connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        let transaction = connection
            .transaction()
            .map_err(|_| SnapshotError::Unavailable)?;
        if !transaction
            .query_row("SELECT 1 FROM instances WHERE id=?1", [instance_id], |_| {
                Ok(())
            })
            .optional()
            .map_err(|_| SnapshotError::Unavailable)?
            .is_some()
        {
            return Err(SnapshotError::InstanceNotFound);
        }
        let ids = {
            let mut statement=transaction.prepare("SELECT id FROM operations WHERE instance_id=?1 AND kind='vm_snapshot_create' ORDER BY created_at DESC,id DESC LIMIT 100").map_err(|_|SnapshotError::Unavailable)?;
            let rows = statement
                .query_map([instance_id], |row| row.get::<_, String>(0))
                .map_err(|_| SnapshotError::Unavailable)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| SnapshotError::Unavailable)?
        };
        let mut operations = Vec::with_capacity(ids.len());
        for id in ids {
            let job = read_job(&transaction, &id)?.ok_or(SnapshotError::Unavailable)?;
            let recovered = checked_recovery(job)?;
            operations.push(HistoryEntry {
                id: recovered.operation.id,
                name: recovered.request.name,
                status: recovered.operation.status,
                failure: recovered.operation.failure,
                provider_task_known: recovered.operation.provider_task_known,
                created_at: recovered.operation.created_at,
            });
        }
        transaction
            .commit()
            .map_err(|_| SnapshotError::Unavailable)?;
        Ok(History {
            instance_id: instance_id.into(),
            operations,
        })
    }
}
