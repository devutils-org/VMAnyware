use super::{
    operations,
    provisioning::{read, valid_digest},
    ProvisioningError, Store,
};
use crate::{
    operation::{Failure, Operation, Status},
    provisioning_intent::{Approval, DiskOutcome, Job},
    proxmox::{PowerState, QmpState, TaskReference, TaskStatus, VmReference},
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}

impl Store {
    /// Unknown resize submissions cannot be reclaimed; known tasks permit GET-only recovery.
    pub fn claim_provisioning_disk(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id:Option<String>=transaction.query_row(
            "UPDATE provisioning_stages SET status='running' WHERE operation_id=(SELECT o.id FROM operations o JOIN provisioning_stages r ON r.operation_id=o.id AND r.position=1 JOIN provisioning_stages s ON s.operation_id=o.id AND s.position=2 WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain') AND r.kind='configure_resources' AND r.status='succeeded' AND s.kind='expand_disk' AND ((s.status IN ('queued','interrupted') AND s.dispatch_started=0) OR (s.status='uncertain' AND s.provider_upid IS NOT NULL AND o.next_reconcile_at<=unixepoch())) ORDER BY o.created_at,o.id LIMIT 1) AND position=2 RETURNING operation_id",
            [],|r|r.get(0)).optional().map_err(unavailable)?;
        let job = if let Some(id) = id {
            transaction
                .execute(
                    "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
                    [&id],
                )
                .map_err(unavailable)?;
            let job = read(&transaction, &id)?.ok_or(ProvisioningError::Unavailable)?;
            let digest = job
                .resource_config_digest()
                .ok_or(ProvisioningError::Unavailable)?;
            if job
                .disk_dispatch
                .as_ref()
                .is_some_and(|d| d.config_digest != digest)
            {
                return Err(ProvisioningError::Unavailable);
            }
            Some(job)
        } else {
            None
        };
        transaction.commit().map_err(unavailable)?;
        Ok(job)
    }
    pub fn begin_provisioning_disk(
        &self,
        id: &str,
        expected: &Approval,
        digest: &str,
        volume: &str,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .get(2)
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.kind != "expand_disk"
            || stage.status != Status::Running
            || stage.dispatch_started
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !expected.valid()
            || expected.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
            || job.resource_config_digest().as_deref() != Some(digest)
            || job
                .plan
                .disk_request(
                    job.operation
                        .instance_id
                        .as_deref()
                        .ok_or(ProvisioningError::Unavailable)?,
                    digest,
                    volume,
                )
                .is_err()
        {
            return Err(ProvisioningError::TargetChanged);
        }
        transaction.execute("INSERT INTO provisioning_disk_dispatch(operation_id,config_digest,volume) VALUES (?1,?2,?3)",params![id,digest,volume]).map_err(unavailable)?;
        transaction.execute("UPDATE provisioning_stages SET dispatch_started=1 WHERE operation_id=?1 AND position=2",[id]).map_err(unavailable)?;
        transaction
            .execute("UPDATE operations SET provider_upid=NULL WHERE id=?1", [id])
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)
    }
    pub fn link_provisioning_disk(
        &self,
        id: &str,
        task: &TaskReference,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .get(2)
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.kind != "expand_disk"
            || stage.status != Status::Running
            || !stage.dispatch_started
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !task.matches_resize(
            &VmReference::new(&job.approval.target_node, job.approval.target_vmid)
                .map_err(unavailable)?,
        ) {
            return Err(ProvisioningError::TargetChanged);
        }
        if stage.task.as_ref().is_some_and(|old| old != task) {
            return Err(ProvisioningError::Conflict);
        }
        transaction.execute("UPDATE provisioning_stages SET provider_upid=?2 WHERE operation_id=?1 AND position=2",params![id,task.upid()]).map_err(unavailable)?;
        transaction
            .execute(
                "UPDATE operations SET provider_upid=?2 WHERE id=?1",
                params![id, task.upid()],
            )
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)
    }
    pub fn finish_provisioning_disk(
        &self,
        id: &str,
        outcome: DiskOutcome,
    ) -> Result<Operation, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .get(2)
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.kind != "expand_disk"
            || stage.status != Status::Running
            || job.resource_config_digest().is_none()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let (stage_status, operation_status, result, failure) = match outcome {
            DiskOutcome::Completed(evidence) => {
                let retained = job
                    .disk_dispatch
                    .as_ref()
                    .ok_or(ProvisioningError::InvalidOperation)?;
                let target = &evidence.disk.resources.target;
                let device = match job.plan.source.disk_device {
                    crate::provisioning::DiskDevice::Scsi0 => "scsi0",
                    crate::provisioning::DiskDevice::Virtio0 => "virtio0",
                    crate::provisioning::DiskDevice::Sata0 => "sata0",
                };
                if !stage.dispatch_started
                    || stage.task.is_none()
                    || Some(evidence.instance_id.as_str()) != job.operation.instance_id.as_deref()
                    || evidence.provider_scope_id != job.approval.provider_scope_id
                    || evidence.plan_hash != job.approval.plan_hash
                    || evidence.task_status != TaskStatus::Succeeded
                    || evidence.disk.device != device
                    || evidence.disk.volume != retained.volume
                    || evidence.disk.size_bytes
                        != u64::from(job.plan.target.resources.disk_gib) * 1024_u64.pow(3)
                    || evidence.disk.resources.cpu != job.plan.target.resources.cpu
                    || evidence.disk.resources.memory_mib != job.plan.target.resources.memory_mib
                    || target.observation.node != job.approval.target_node
                    || target.observation.vmid != job.approval.target_vmid
                    || target.observation.status != PowerState::Stopped
                    || !matches!(
                        target.observation.qmp_status,
                        None | Some(QmpState::Stopped)
                    )
                    || !valid_digest(&target.config_digest)
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                (
                    "succeeded",
                    "running",
                    Some(serde_json::to_string(&evidence).map_err(unavailable)?),
                    None,
                )
            }
            DiskOutcome::Failed(code) => {
                if (code == Failure::ProviderTaskFailed && stage.task.is_none())
                    || !matches!(
                        code,
                        Failure::ProvisioningDisabled
                            | Failure::ProvisioningPlanChanged
                            | Failure::ProvisioningPreconditionsFailed
                            | Failure::ProviderNotConfigured
                            | Failure::ProviderScopeChanged
                            | Failure::ProviderUnauthorized
                            | Failure::ProviderForbidden
                            | Failure::ProviderTaskFailed
                    )
                    || (stage.dispatch_started
                        && !matches!(
                            (&stage.task, code),
                            (
                                None,
                                Failure::ProviderUnauthorized | Failure::ProviderForbidden
                            ) | (Some(_), Failure::ProviderTaskFailed)
                        ))
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("failed", "failed", None, Some(code))
            }
            DiskOutcome::Uncertain(code) => {
                if !stage.dispatch_started
                    || !matches!(
                        code,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::ProviderTaskUnconfirmed
                            | Failure::DiskStateUnconfirmed
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
        transaction.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND position=2",params![id,stage_status,result]).map_err(unavailable)?;
        let failure = failure
            .map(serde_json::to_value)
            .transpose()
            .map_err(unavailable)?
            .map(|v| v.as_str().unwrap_or_default().to_owned());
        transaction.execute("UPDATE operations SET status=?2,failure_code=?3,next_reconcile_at=unixepoch()+5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,operation_status,failure]).map_err(unavailable)?;
        let operation = operations::read(&transaction, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        transaction.commit().map_err(unavailable)?;
        Ok(operation)
    }
}
