//! Regeneration is the first durable substep of start_vm; its marker is never replayed.
use super::{operations, provisioning::read, ProvisioningError, Store};
use crate::{
    operation::{Failure, Operation, Status},
    provisioning_intent::{Approval, Job, SeedJournal, SeedOutcome},
    proxmox::SeedPreflight,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
impl Store {
    pub fn claim_provisioning_seed(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id:Option<String>=tx.query_row("UPDATE provisioning_stages SET status='running' WHERE kind='start_vm' AND operation_id=(SELECT o.id FROM operations o JOIN provisioning_stages s ON s.operation_id=o.id WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain','interrupted') AND s.kind='start_vm' AND s.status IN ('queued','interrupted') AND s.dispatch_started=0 AND s.result_json IS NULL AND NOT EXISTS(SELECT 1 FROM provisioning_stages p WHERE p.operation_id=o.id AND p.position<s.position AND p.status!='succeeded') ORDER BY o.created_at,o.id LIMIT 1) RETURNING operation_id",[],|r|r.get(0)).optional().map_err(unavailable)?;
        let job = if let Some(id) = id {
            tx.execute(
                "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
                [&id],
            )
            .map_err(unavailable)?;
            let job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
            job.cloud_init_settings()
                .ok_or(ProvisioningError::Unavailable)?;
            Some(job)
        } else {
            None
        };
        tx.commit().map_err(unavailable)?;
        Ok(job)
    }
    pub fn begin_provisioning_seed(
        &self,
        id: &str,
        approval: &Approval,
        expected: SeedPreflight,
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
        let settings = job
            .cloud_init_settings()
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || stage.dispatch_started
            || job.seed_journal.is_some()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !approval.valid()
            || approval.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
            || !job.valid_seed(&expected)
            || expected.config_digest != settings.settings.resources.target.config_digest
        {
            return Err(ProvisioningError::TargetChanged);
        }
        let journal = SeedJournal {
            expected,
            completed: None,
            power: None,
            readiness: None,
        };
        tx.execute(
            "INSERT INTO provisioning_seed_regeneration VALUES (?1,?2)",
            params![id, serde_json::to_string(&journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.execute("UPDATE provisioning_stages SET dispatch_started=1 WHERE operation_id=?1 AND kind='start_vm'",[id]).map_err(unavailable)?;
        tx.execute("UPDATE operations SET provider_upid=NULL WHERE id=?1", [id])
            .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }
    pub fn finish_provisioning_seed(
        &self,
        id: &str,
        outcome: SeedOutcome,
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
            || job.cloud_init_settings().is_none()
            || job
                .seed_journal
                .as_ref()
                .is_some_and(|j| j.completed.is_some())
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let (status, operation_status, failure, result) = match outcome {
            SeedOutcome::Completed(proof) => {
                let mut journal = job
                    .seed_journal
                    .as_ref()
                    .ok_or(ProvisioningError::InvalidOperation)?
                    .clone();
                if !stage.dispatch_started
                    || journal.completed.is_some()
                    || !job.valid_seed(&proof)
                    || !job.valid_seed(&journal.expected)
                    || proof.user_data_hash != journal.expected.user_data_hash
                    || proof.network_data_hash != journal.expected.network_data_hash
                    || proof.user_format != journal.expected.user_format
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                journal.completed = Some(proof);
                // start_vm remains pending; dispatch_started records its seed substep.
                (
                    "running",
                    "running",
                    None,
                    Some(serde_json::to_string(&journal).map_err(unavailable)?),
                )
            }
            SeedOutcome::Failed(code) => {
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
                ("failed", "failed", Some(code), None)
            }
            SeedOutcome::Uncertain(code) => {
                if !stage.dispatch_started
                    || !matches!(
                        code,
                        Failure::SeedRegenerationUnconfirmed
                            | Failure::ProviderSubmissionUnconfirmed
                            | Failure::ProcessInterrupted
                    )
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("uncertain", "uncertain", Some(code), None)
            }
        };
        if let Some(result) = result {
            tx.execute(
                "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
                params![id, result],
            )
            .map_err(unavailable)?;
        }
        tx.execute(
            "UPDATE provisioning_stages SET status=?2 WHERE operation_id=?1 AND kind='start_vm'",
            params![id, status],
        )
        .map_err(unavailable)?;
        let failure = failure
            .map(serde_json::to_value)
            .transpose()
            .map_err(unavailable)?
            .map(|v| v.as_str().unwrap_or_default().to_owned());
        tx.execute("UPDATE operations SET status=?2,failure_code=?3,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,operation_status,failure]).map_err(unavailable)?;
        let operation = operations::read(&tx, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(operation)
    }
}
