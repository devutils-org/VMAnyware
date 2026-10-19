//! Power is a separate durable substep from the seed marker on start_vm.
use super::{operations, provisioning::read, ProvisioningError, Store};
use crate::{
    lifecycle::Action,
    operation::{Failure, Operation, Status},
    provisioning_intent::{Approval, Job, PowerDispatch, StartOutcome},
    proxmox::{PowerState, QmpState, TaskReference, TaskStatus, VmReference},
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
impl Store {
    pub fn claim_provisioning_start(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id:Option<String>=tx.query_row("SELECT o.id FROM operations o JOIN provisioning_stages s ON s.operation_id=o.id JOIN provisioning_seed_regeneration j ON j.operation_id=o.id WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain','interrupted') AND s.kind='start_vm' AND s.status IN ('running','uncertain','interrupted') AND json_extract(j.journal_json,'$.completed') IS NOT NULL AND (json_extract(j.journal_json,'$.power') IS NULL OR (json_extract(j.journal_json,'$.power.configDigest') IS NULL AND s.status IN ('uncertain','interrupted')) OR (json_extract(j.journal_json,'$.power.configDigest') IS NOT NULL AND s.status='uncertain' AND s.provider_upid IS NOT NULL AND o.next_reconcile_at<=unixepoch())) ORDER BY o.created_at,o.id LIMIT 1",[],|r|r.get(0)).optional().map_err(unavailable)?;
        let Some(id) = id else {
            tx.commit().map_err(unavailable)?;
            return Ok(None);
        };
        let mut job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        job.regenerated_seed()
            .ok_or(ProvisioningError::Unavailable)?;
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::Unavailable)?;
        if journal.power.is_none() {
            journal.power = Some(PowerDispatch {
                config_digest: None,
            });
        }
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(&journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.execute("UPDATE provisioning_stages SET status='running' WHERE operation_id=?1 AND kind='start_vm'",[&id]).map_err(unavailable)?;
        tx.execute(
            "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
            [&id],
        )
        .map_err(unavailable)?;
        let job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(Some(job))
    }
    pub fn begin_provisioning_start(
        &self,
        id: &str,
        approval: &Approval,
        digest: &str,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let mut job = read(&tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "start_vm")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || stage.task.is_some()
            || job.power_dispatch_started()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !approval.valid()
            || approval.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
            || job
                .regenerated_seed()
                .is_none_or(|p| p.config_digest != digest)
        {
            return Err(ProvisioningError::TargetChanged);
        }
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        let power = journal
            .power
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        power.config_digest = Some(digest.into());
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }
    pub fn link_provisioning_start(
        &self,
        id: &str,
        task: &TaskReference,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "start_vm")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || !job.power_dispatch_started()
            || !task.matches_action(
                &VmReference::new(&job.approval.target_node, job.approval.target_vmid)
                    .map_err(unavailable)?,
                Action::Start,
            )
            || stage.task.as_ref().is_some_and(|t| t != task)
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        tx.execute("UPDATE provisioning_stages SET provider_upid=?2 WHERE operation_id=?1 AND kind='start_vm'",params![id,task.upid()]).map_err(unavailable)?;
        tx.execute(
            "UPDATE operations SET provider_upid=?2 WHERE id=?1",
            params![id, task.upid()],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }
    pub fn finish_provisioning_start(
        &self,
        id: &str,
        outcome: StartOutcome,
    ) -> Result<Operation, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "start_vm")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || job.regenerated_seed().is_none()
            || job
                .seed_journal
                .as_ref()
                .and_then(|j| j.power.as_ref())
                .is_none()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let marked = job.power_dispatch_started();
        let (status, op_status, result, failure) = match outcome {
            StartOutcome::Completed(proof) => {
                if !marked
                    || stage.task.is_none()
                    || Some(proof.instance_id.as_str()) != job.operation.instance_id.as_deref()
                    || proof.provider_scope_id != job.approval.provider_scope_id
                    || proof.plan_hash != job.approval.plan_hash
                    || proof.task_status != TaskStatus::Succeeded
                    || proof.observation.node != job.approval.target_node
                    || proof.observation.vmid != job.approval.target_vmid
                    || proof.observation.status != PowerState::Running
                    || !matches!(proof.observation.qmp_status, None | Some(QmpState::Running))
                    || proof.guest_ready
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                (
                    "succeeded",
                    "running",
                    Some(serde_json::to_string(&proof).map_err(unavailable)?),
                    None,
                )
            }
            StartOutcome::Failed(code) => {
                if if marked {
                    code != Failure::ProviderTaskFailed || stage.task.is_none()
                } else {
                    !matches!(
                        code,
                        Failure::ProvisioningDisabled
                            | Failure::ProvisioningPlanChanged
                            | Failure::ProvisioningPreconditionsFailed
                            | Failure::ProviderNotConfigured
                            | Failure::ProviderScopeChanged
                    )
                } {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("failed", "failed", None, Some(code))
            }
            StartOutcome::Uncertain(code) => {
                if !marked
                    || !matches!(
                        code,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::ProviderTaskUnconfirmed
                            | Failure::PowerStateUnconfirmed
                            | Failure::ProviderNotConfigured
                            | Failure::ProviderScopeChanged
                            | Failure::ProcessInterrupted
                    )
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("uncertain", "uncertain", None, Some(code))
            }
        };
        tx.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND kind='start_vm'",params![id,status,result]).map_err(unavailable)?;
        let failure = failure
            .map(serde_json::to_value)
            .transpose()
            .map_err(unavailable)?
            .map(|v| v.as_str().unwrap_or_default().to_owned());
        tx.execute("UPDATE operations SET status=?2,failure_code=?3,next_reconcile_at=unixepoch()+5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,op_status,failure]).map_err(unavailable)?;
        let operation = operations::read(&tx, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(operation)
    }
}
