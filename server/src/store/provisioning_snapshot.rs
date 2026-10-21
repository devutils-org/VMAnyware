//! The optional final snapshot is part of the original provisioning operation.
use super::{operations, provisioning::read, ProvisioningError, Store};
use crate::{
    operation::{Failure, Operation, Status},
    provisioning_intent::{Approval, Job},
    proxmox::{TaskReference, VmReference},
    snapshot::Outcome,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
fn checked(tx: &Transaction<'_>, id: &str) -> Result<Job, ProvisioningError> {
    let job = read(tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
    if job.operation.status != Status::Running
        || !job.final_snapshot_prerequisites()
        || !job
            .stages
            .iter()
            .any(|s| s.kind == "snapshot_after_provisioning" && s.status == Status::Running)
    {
        return Err(ProvisioningError::InvalidOperation);
    }
    Ok(job)
}
impl Store {
    pub fn claim_provisioning_snapshot(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id:Option<String>=tx.query_row("SELECT o.id FROM operations o JOIN provisioning_stages s ON s.operation_id=o.id WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain','interrupted') AND s.kind='snapshot_after_provisioning' AND (s.status='queued' OR (s.status IN ('uncertain','interrupted') AND (s.dispatch_started=0 OR s.provider_upid IS NOT NULL) AND o.next_reconcile_at<=unixepoch())) AND NOT EXISTS(SELECT 1 FROM provisioning_stages p WHERE p.operation_id=o.id AND p.position<s.position AND p.status!='succeeded') ORDER BY o.created_at,o.id LIMIT 1",[],|r|r.get(0)).optional().map_err(unavailable)?;
        let Some(id) = id else {
            tx.commit().map_err(unavailable)?;
            return Ok(None);
        };
        let job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        if !job.final_snapshot_prerequisites() {
            return Err(ProvisioningError::Unavailable);
        }
        tx.execute("UPDATE provisioning_stages SET status='running' WHERE operation_id=?1 AND kind='snapshot_after_provisioning'",[&id]).map_err(unavailable)?;
        tx.execute(
            "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
            [&id],
        )
        .map_err(unavailable)?;
        let job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(Some(job))
    }
    pub fn begin_provisioning_snapshot(
        &self,
        id: &str,
        approval: &Approval,
        digest: &str,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let mut job = checked(&tx, id)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "snapshot_after_provisioning")
            .unwrap();
        if stage.dispatch_started || stage.task.is_some() {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !approval.valid()
            || approval.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
            || job
                .regenerated_seed()
                .is_none_or(|s| s.config_digest != digest)
        {
            return Err(ProvisioningError::TargetChanged);
        }
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        journal.snapshot_digest = Some(digest.into());
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.execute("UPDATE provisioning_stages SET dispatch_started=1 WHERE operation_id=?1 AND kind='snapshot_after_provisioning'",[id]).map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }
    pub fn link_provisioning_snapshot(
        &self,
        id: &str,
        task: &TaskReference,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = checked(&tx, id)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "snapshot_after_provisioning")
            .unwrap();
        if !stage.dispatch_started
            || stage.task.as_ref().is_some_and(|t| t != task)
            || !task.matches_snapshot(
                &VmReference::new(&job.approval.target_node, job.approval.target_vmid)
                    .map_err(unavailable)?,
            )
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        tx.execute("UPDATE provisioning_stages SET provider_upid=?2 WHERE operation_id=?1 AND kind='snapshot_after_provisioning'",params![id,task.upid()]).map_err(unavailable)?;
        tx.execute(
            "UPDATE operations SET provider_upid=?2 WHERE id=?1",
            params![id, task.upid()],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }
    pub fn finish_provisioning_snapshot(
        &self,
        id: &str,
        outcome: Outcome,
    ) -> Result<Operation, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = checked(&tx, id)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "snapshot_after_provisioning")
            .unwrap();
        if job.stages.iter().any(|s| s.position > stage.position) {
            return Err(ProvisioningError::InvalidOperation);
        }
        let (status, result, failure) = match outcome {
            Outcome::Succeeded(proof) => {
                if !stage.dispatch_started
                    || stage.task.is_none()
                    || !job.valid_final_snapshot(&proof)
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                (
                    "succeeded",
                    Some(serde_json::to_string(&proof).map_err(unavailable)?),
                    None,
                )
            }
            Outcome::Failed(failure) => {
                if matches!(
                    failure,
                    Failure::ProviderSubmissionUnconfirmed
                        | Failure::ProviderTaskUnconfirmed
                        | Failure::SnapshotStateUnconfirmed
                ) || (stage.dispatch_started
                    && !(stage.task.is_some() && failure == Failure::ProviderTaskFailed))
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("failed", None, Some(failure))
            }
            Outcome::Uncertain(failure) => {
                if !stage.dispatch_started
                    || !matches!(
                        failure,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::ProviderTaskUnconfirmed
                            | Failure::SnapshotStateUnconfirmed
                            | Failure::ProviderScopeChanged
                            | Failure::ProviderNotConfigured
                    )
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("uncertain", None, Some(failure))
            }
        };
        tx.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND kind='snapshot_after_provisioning'",params![id,status,result]).map_err(unavailable)?;
        let operation_result = if status == "succeeded" {
            Some(
                serde_json::to_string(
                    &job.guest_readiness()
                        .ok_or(ProvisioningError::InvalidOperation)?,
                )
                .map_err(unavailable)?,
            )
        } else {
            None
        };
        let failure = failure
            .map(serde_json::to_value)
            .transpose()
            .map_err(unavailable)?
            .and_then(|v| v.as_str().map(str::to_owned));
        tx.execute("UPDATE operations SET status=?2,result_json=?3,failure_code=?4,next_reconcile_at=unixepoch()+5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,status,operation_result,failure]).map_err(unavailable)?;
        let operation = operations::read(&tx, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(operation)
    }
}
