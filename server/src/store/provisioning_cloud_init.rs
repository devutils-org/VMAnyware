use super::{
    operations,
    provisioning::{read, valid_digest},
    ProvisioningError, Store,
};
use crate::{
    operation::{Failure, Operation, Status},
    provisioning_intent::{Approval, CloudInitOutcome, Job},
    proxmox::{PowerState, QmpState},
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
impl Store {
    pub fn claim_provisioning_cloud_init(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id:Option<String>=transaction.query_row("UPDATE provisioning_stages SET status='running' WHERE operation_id=(SELECT o.id FROM operations o JOIN provisioning_stages s ON s.operation_id=o.id WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain') AND s.kind='configure_cloud_init' AND NOT EXISTS(SELECT 1 FROM provisioning_stages p WHERE p.operation_id=o.id AND p.position<s.position AND p.status!='succeeded') AND ((s.status IN ('queued','interrupted') AND s.dispatch_started=0) OR (s.status='uncertain' AND s.dispatch_started=1 AND o.next_reconcile_at<=unixepoch())) ORDER BY o.created_at,o.id LIMIT 1) AND kind='configure_cloud_init' RETURNING operation_id",[],|r|r.get(0)).optional().map_err(unavailable)?;
        let job = if let Some(id) = id {
            transaction
                .execute(
                    "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
                    [&id],
                )
                .map_err(unavailable)?;
            let job = read(&transaction, &id)?.ok_or(ProvisioningError::Unavailable)?;
            job.cloud_init_source()
                .ok_or(ProvisioningError::Unavailable)?;
            Some(job)
        } else {
            None
        };
        transaction.commit().map_err(unavailable)?;
        Ok(job)
    }
    pub fn begin_provisioning_cloud_init(
        &self,
        id: &str,
        expected: &Approval,
        digest: &str,
        volume: &str,
        network: &str,
    ) -> Result<(), ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "configure_cloud_init")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || stage.dispatch_started
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let (source, retained_volume) = job
            .cloud_init_source()
            .ok_or(ProvisioningError::InvalidOperation)?;
        if !expected.valid()
            || expected.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
            || digest != source
            || retained_volume.as_ref().is_some_and(|v| v != volume)
            || job
                .plan
                .cloud_init_request(
                    &job.definition,
                    job.operation
                        .instance_id
                        .as_deref()
                        .ok_or(ProvisioningError::Unavailable)?,
                    digest,
                    volume,
                    network,
                )
                .is_err()
        {
            return Err(ProvisioningError::TargetChanged);
        }
        transaction
            .execute(
                "INSERT INTO provisioning_cloud_init_dispatch VALUES (?1,?2,?3,?4)",
                params![id, digest, volume, network],
            )
            .map_err(unavailable)?;
        transaction.execute("UPDATE provisioning_stages SET dispatch_started=1 WHERE operation_id=?1 AND kind='configure_cloud_init'",[id]).map_err(unavailable)?;
        transaction
            .execute("UPDATE operations SET provider_upid=NULL WHERE id=?1", [id])
            .map_err(unavailable)?;
        transaction.commit().map_err(unavailable)
    }
    pub fn finish_provisioning_cloud_init(
        &self,
        id: &str,
        outcome: CloudInitOutcome,
    ) -> Result<Operation, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = read(&transaction, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "configure_cloud_init")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || job.cloud_init_source().is_none()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let (stage_status, operation_status, result, failure) = match outcome {
            CloudInitOutcome::Completed(evidence) => {
                let retained = job
                    .cloud_init_dispatch
                    .as_ref()
                    .ok_or(ProvisioningError::InvalidOperation)?;
                let request = job
                    .plan
                    .cloud_init_request(
                        &job.definition,
                        job.operation
                            .instance_id
                            .as_deref()
                            .ok_or(ProvisioningError::Unavailable)?,
                        &retained.config_digest,
                        &retained.volume,
                        &retained.network,
                    )
                    .map_err(unavailable)?;
                let settings = &evidence.settings;
                let target = &settings.resources.target;
                if !stage.dispatch_started
                    || Some(evidence.instance_id.as_str()) != job.operation.instance_id.as_deref()
                    || evidence.provider_scope_id != job.approval.provider_scope_id
                    || evidence.plan_hash != job.approval.plan_hash
                    || settings.primary_volume != retained.volume
                    || settings.disk_size_bytes
                        != u64::from(job.plan.target.resources.disk_gib) * 1024_u64.pow(3)
                    || settings.network != request.expected_network()
                    || settings.guest_user != job.plan.target.guest_user
                    || settings.ssh_key_count
                        != job.definition.spec.provisioning.ssh_authorized_keys.len()
                    || settings.guest_ready
                    || settings.resources.cpu != job.plan.target.resources.cpu
                    || settings.resources.memory_mib != job.plan.target.resources.memory_mib
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
            CloudInitOutcome::Failed(code) => {
                if stage.dispatch_started
                    || !matches!(
                        code,
                        Failure::ProvisioningDisabled
                            | Failure::ProvisioningPlanChanged
                            | Failure::ProvisioningPreconditionsFailed
                            | Failure::ProviderNotConfigured
                            | Failure::ProviderScopeChanged
                    )
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("failed", "failed", None, Some(code))
            }
            CloudInitOutcome::Uncertain(code) => {
                if !stage.dispatch_started
                    || !matches!(
                        code,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::CloudInitStateUnconfirmed
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
        transaction.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND kind='configure_cloud_init'",params![id,stage_status,result]).map_err(unavailable)?;
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
