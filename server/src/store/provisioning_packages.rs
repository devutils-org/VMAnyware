//! One durable package mutation; only retained-PID observations may resume.
use super::{operations, provisioning::read, ProvisioningError, Store};
use crate::{
    operation::{Failure, Operation, Status},
    provisioning_intent::{Approval, Job, PackageJournal, PackageOutcome, PackageResult},
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
fn now(tx: &Transaction<'_>) -> Result<i64, ProvisioningError> {
    tx.query_row("SELECT unixepoch()", [], |r| r.get(0))
        .map_err(unavailable)
}
fn checked(tx: &Transaction<'_>, id: &str) -> Result<Job, ProvisioningError> {
    let job = read(tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
    if job.operation.status != Status::Running
        || !job
            .stages
            .iter()
            .any(|s| s.kind == "install_declared_packages" && s.status == Status::Running)
        || job.guest_readiness().is_none()
        || job
            .seed_journal
            .as_ref()
            .and_then(|j| j.packages.as_ref())
            .is_none()
    {
        return Err(ProvisioningError::InvalidOperation);
    }
    Ok(job)
}
impl Store {
    pub fn claim_provisioning_packages(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id:Option<String>=tx.query_row("SELECT o.id FROM operations o JOIN provisioning_stages s ON s.operation_id=o.id WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain','interrupted') AND s.kind='install_declared_packages' AND s.status IN ('queued','interrupted','uncertain') AND (s.status='queued' OR o.next_reconcile_at<=unixepoch()) AND NOT EXISTS(SELECT 1 FROM provisioning_stages p WHERE p.operation_id=o.id AND p.position<s.position AND p.status!='succeeded') ORDER BY o.created_at,o.id LIMIT 1",[],|r|r.get(0)).optional().map_err(unavailable)?;
        let Some(id) = id else {
            tx.commit().map_err(unavailable)?;
            return Ok(None);
        };
        let mut job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        job.guest_readiness()
            .ok_or(ProvisioningError::Unavailable)?;
        let time = now(&tx)?;
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::Unavailable)?;
        if journal.packages.is_none() {
            journal.packages = Some(PackageJournal {
                started_at: time,
                deadline: time + 960,
                pid: None,
            });
        }
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.execute("UPDATE provisioning_stages SET status='running' WHERE operation_id=?1 AND kind='install_declared_packages'",[&id]).map_err(unavailable)?;
        tx.execute(
            "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
            [&id],
        )
        .map_err(unavailable)?;
        let job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(Some(job))
    }
    pub fn begin_provisioning_packages(
        &self,
        id: &str,
        approval: &Approval,
    ) -> Result<bool, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = checked(&tx, id)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "install_declared_packages")
            .unwrap();
        let journal = job
            .seed_journal
            .as_ref()
            .unwrap()
            .packages
            .as_ref()
            .unwrap();
        if stage.dispatch_started || journal.pid.is_some() {
            return Err(ProvisioningError::InvalidOperation);
        }
        if !approval.valid()
            || approval.input_hash().map_err(unavailable)?
                != job.approval.input_hash().map_err(unavailable)?
        {
            return Err(ProvisioningError::TargetChanged);
        }
        let time = now(&tx)?;
        if time < journal.started_at || time >= journal.deadline {
            return Ok(false);
        }
        tx.execute("UPDATE provisioning_stages SET dispatch_started=1 WHERE operation_id=?1 AND kind='install_declared_packages'",[id]).map_err(unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(true)
    }
    pub fn link_provisioning_packages(&self, id: &str, pid: u32) -> Result<(), ProvisioningError> {
        if pid == 0 || pid > i32::MAX as u32 {
            return Err(ProvisioningError::InvalidOperation);
        }
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let mut job = checked(&tx, id)?;
        if !job
            .stages
            .iter()
            .any(|s| s.kind == "install_declared_packages" && s.dispatch_started)
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let journal = job.seed_journal.as_mut().unwrap();
        let packages = journal.packages.as_mut().unwrap();
        if packages.pid.is_some_and(|p| p != pid) {
            return Err(ProvisioningError::InvalidOperation);
        }
        packages.pid = Some(pid);
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }
    pub fn finish_provisioning_packages(
        &self,
        id: &str,
        outcome: PackageOutcome,
    ) -> Result<Operation, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let job = checked(&tx, id)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "install_declared_packages")
            .unwrap();
        let journal = job
            .seed_journal
            .as_ref()
            .unwrap()
            .packages
            .as_ref()
            .unwrap();
        let time = now(&tx)?;
        let expired = time < journal.started_at || time >= journal.deadline;
        let last = job.stages.iter().all(|s| s.position <= stage.position);
        let outcome = if expired {
            PackageOutcome::TimedOut
        } else {
            outcome
        };
        let (status, op_status, result, failure, next) = match outcome {
            PackageOutcome::Completed => {
                if !stage.dispatch_started || journal.pid.is_none() {
                    return Err(ProvisioningError::InvalidOperation);
                }
                let proof = PackageResult {
                    instance_id: job
                        .operation
                        .instance_id
                        .clone()
                        .ok_or(ProvisioningError::InvalidOperation)?,
                    provider_scope_id: job.approval.provider_scope_id.clone(),
                    plan_hash: job.approval.plan_hash.clone(),
                    package_count: job.definition.spec.provisioning.packages.len(),
                    installed: true,
                };
                (
                    "succeeded",
                    if last { "succeeded" } else { "running" },
                    Some(serde_json::to_string(&proof).map_err(unavailable)?),
                    None,
                    0,
                )
            }
            PackageOutcome::Failed(failure) => {
                if !matches!(
                    failure,
                    Failure::ProvisioningDisabled
                        | Failure::ProviderNotConfigured
                        | Failure::ProviderScopeChanged
                        | Failure::ProvisioningPlanChanged
                        | Failure::PackagesFailed
                ) || if matches!(failure, Failure::PackagesFailed) {
                    !stage.dispatch_started || journal.pid.is_none()
                } else {
                    stage.dispatch_started
                } {
                    return Err(ProvisioningError::InvalidOperation);
                }
                ("failed", "failed", None, Some(failure), 0)
            }
            PackageOutcome::Pending | PackageOutcome::Uncertain => (
                if stage.dispatch_started {
                    "uncertain"
                } else {
                    "interrupted"
                },
                "uncertain",
                None,
                Some(Failure::PackagesUnconfirmed),
                if stage.dispatch_started && journal.pid.is_none() {
                    journal.deadline
                } else {
                    time + 5
                },
            ),
            PackageOutcome::TimedOut => {
                if !expired {
                    return Err(ProvisioningError::InvalidOperation);
                }
                // A guest may still be running: never report a known failure or redispatch.
                (
                    "uncertain",
                    "uncertain",
                    None,
                    Some(Failure::PackagesTimedOut),
                    i64::MAX,
                )
            }
        };
        tx.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND kind='install_declared_packages'",params![id,status,result]).map_err(unavailable)?;
        let operation_result = if last && op_status == "succeeded" {
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
        tx.execute("UPDATE operations SET status=?2,result_json=?3,failure_code=?4,next_reconcile_at=?5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,op_status,operation_result,failure,next]).map_err(unavailable)?;
        let operation = operations::read(&tx, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(operation)
    }
}
