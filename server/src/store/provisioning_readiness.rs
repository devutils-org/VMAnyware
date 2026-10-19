//! Durable fixed-PID observations with one retained five-minute deadline.
use super::{operations, provisioning::read, ProvisioningError, Store};
use crate::{
    operation::{Failure, Operation, Status},
    provisioning_intent::{Job, ReadinessJournal, ReadinessOutcome},
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
fn unavailable<T>(_: T) -> ProvisioningError {
    ProvisioningError::Unavailable
}
impl Store {
    pub fn claim_provisioning_readiness(&self) -> Result<Option<Job>, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let id:Option<String>=tx.query_row("SELECT o.id FROM operations o JOIN provisioning_stages s ON s.operation_id=o.id WHERE o.kind='vm_provision' AND o.status IN ('running','uncertain','interrupted') AND s.kind='wait_for_guest_readiness' AND s.status IN ('queued','interrupted','uncertain') AND (s.status='queued' OR o.next_reconcile_at<=unixepoch()) AND NOT EXISTS(SELECT 1 FROM provisioning_stages p WHERE p.operation_id=o.id AND p.position<s.position AND p.status!='succeeded') ORDER BY o.created_at,o.id LIMIT 1",[],|r|r.get(0)).optional().map_err(unavailable)?;
        let Some(id) = id else {
            tx.commit().map_err(unavailable)?;
            return Ok(None);
        };
        let mut job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        job.started().ok_or(ProvisioningError::Unavailable)?;
        let now: i64 = tx
            .query_row("SELECT unixepoch()", [], |r| r.get(0))
            .map_err(unavailable)?;
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::Unavailable)?;
        if journal.readiness.is_none() {
            journal.readiness = Some(ReadinessJournal {
                started_at: now,
                deadline: now + 300,
                probe_started: false,
                pid: None,
                attempts: 0,
            });
        }
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.execute("UPDATE provisioning_stages SET status='running' WHERE operation_id=?1 AND kind='wait_for_guest_readiness'",[&id]).map_err(unavailable)?;
        tx.execute(
            "UPDATE operations SET status='running',failure_code=NULL WHERE id=?1",
            [&id],
        )
        .map_err(unavailable)?;
        let job = read(&tx, &id)?.ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(Some(job))
    }
    pub fn begin_readiness_probe(&self, id: &str) -> Result<bool, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let mut job = read(&tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "wait_for_guest_readiness")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || job.started().is_none()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let now: i64 = tx
            .query_row("SELECT unixepoch()", [], |r| r.get(0))
            .map_err(unavailable)?;
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        let probe = journal
            .readiness
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        if probe.probe_started || probe.pid.is_some() {
            return Err(ProvisioningError::InvalidOperation);
        }
        if now < probe.started_at || now >= probe.deadline || probe.attempts >= 60 {
            return Ok(false);
        }
        probe.probe_started = true;
        probe.attempts += 1;
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.execute("UPDATE provisioning_stages SET dispatch_started=1 WHERE operation_id=?1 AND kind='wait_for_guest_readiness'",[id]).map_err(unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(true)
    }
    pub fn link_readiness_pid(&self, id: &str, pid: u32) -> Result<(), ProvisioningError> {
        if pid == 0 || pid > i32::MAX as u32 {
            return Err(ProvisioningError::InvalidOperation);
        }
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let mut job = read(&tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "wait_for_guest_readiness")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || !stage.dispatch_started
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        let probe = journal
            .readiness
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        if !probe.probe_started || probe.pid.is_some_and(|p| p != pid) {
            return Err(ProvisioningError::InvalidOperation);
        }
        probe.pid = Some(pid);
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)
    }
    pub fn finish_provisioning_readiness(
        &self,
        id: &str,
        outcome: ReadinessOutcome,
    ) -> Result<Operation, ProvisioningError> {
        let mut connection = self.0.lock().map_err(unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        let mut job = read(&tx, id)?.ok_or(ProvisioningError::InvalidOperation)?;
        let stage = job
            .stages
            .iter()
            .find(|s| s.kind == "wait_for_guest_readiness")
            .ok_or(ProvisioningError::InvalidOperation)?;
        if job.operation.status != Status::Running
            || stage.status != Status::Running
            || job.started().is_none()
        {
            return Err(ProvisioningError::InvalidOperation);
        }
        let marked = stage.dispatch_started;
        let last = job.stages.iter().all(|s| s.position <= stage.position);
        let now: i64 = tx
            .query_row("SELECT unixepoch()", [], |r| r.get(0))
            .map_err(unavailable)?;
        let journal = job
            .seed_journal
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        let probe = journal
            .readiness
            .as_mut()
            .ok_or(ProvisioningError::InvalidOperation)?;
        let expired = now < probe.started_at
            || now >= probe.deadline
            || (probe.attempts >= 60 && !probe.probe_started);
        let outcome = if expired {
            ReadinessOutcome::TimedOut
        } else {
            outcome
        };
        let (status, op_status, result, failure) = match outcome {
            ReadinessOutcome::Completed(proof) => {
                let ip = proof.guest.ipv4;
                if expired
                    || !marked
                    || !probe.probe_started
                    || probe.pid.is_none()
                    || Some(proof.instance_id.as_str()) != job.operation.instance_id.as_deref()
                    || proof.provider_scope_id != job.approval.provider_scope_id
                    || proof.plan_hash != job.approval.plan_hash
                    || !proof.guest_ready
                    || !proof.guest.cloud_init_clean
                    || proof.guest.cloud_init_exit_code != 0
                    || ip.is_unspecified()
                    || ip.is_loopback()
                    || ip.is_link_local()
                    || ip.is_broadcast()
                    || ip.octets()[0] == 0
                    || ip.octets()[0] >= 224
                    || proof.guest.interface.is_empty()
                    || proof.guest.interface.len() > 15
                    || !proof
                        .guest
                        .interface
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                {
                    return Err(ProvisioningError::InvalidOperation);
                }
                (
                    "succeeded",
                    if last { "succeeded" } else { "running" },
                    Some(serde_json::to_string(&proof).map_err(unavailable)?),
                    None,
                )
            }
            ReadinessOutcome::Pending { process_exited } => {
                if expired || (process_exited && (!probe.probe_started || probe.pid.is_none())) {
                    return Err(ProvisioningError::InvalidOperation);
                }
                if process_exited {
                    probe.probe_started = false;
                    probe.pid = None;
                }
                (
                    if marked { "uncertain" } else { "interrupted" },
                    "uncertain",
                    None,
                    Some(Failure::GuestReadinessUnconfirmed),
                )
            }
            ReadinessOutcome::Uncertain => (
                if marked { "uncertain" } else { "interrupted" },
                "uncertain",
                None,
                Some(Failure::GuestReadinessUnconfirmed),
            ),
            ReadinessOutcome::Rejected => (
                "failed",
                "failed",
                None,
                Some(Failure::GuestReadinessFailed),
            ),
            ReadinessOutcome::TimedOut => {
                if !expired {
                    return Err(ProvisioningError::InvalidOperation);
                }
                (
                    "failed",
                    "failed",
                    None,
                    Some(Failure::GuestReadinessTimedOut),
                )
            }
        };
        let next = if probe.probe_started && probe.pid.is_none() {
            probe.deadline
        } else {
            now + 5
        };
        tx.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            params![id, serde_json::to_string(journal).map_err(unavailable)?],
        )
        .map_err(unavailable)?;
        tx.execute("UPDATE provisioning_stages SET status=?2,result_json=?3 WHERE operation_id=?1 AND kind='wait_for_guest_readiness'",params![id,status,result]).map_err(unavailable)?;
        let operation_result = if last && op_status == "succeeded" {
            result
        } else {
            None
        };
        let failure = failure
            .map(serde_json::to_value)
            .transpose()
            .map_err(unavailable)?
            .map(|v| v.as_str().unwrap_or_default().to_owned());
        tx.execute("UPDATE operations SET status=?2,result_json=?3,failure_code=?4,next_reconcile_at=?5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,op_status,operation_result,failure,next]).map_err(unavailable)?;
        let operation = operations::read(&tx, id)
            .map_err(unavailable)?
            .ok_or(ProvisioningError::Unavailable)?;
        tx.commit().map_err(unavailable)?;
        Ok(operation)
    }
}
