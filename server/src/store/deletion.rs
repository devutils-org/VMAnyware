//! Durable deletion intent and dispatch guards. No provider request is made here.
use super::DeletionError;
use super::{operations, Store};
use crate::{
    deletion::{Approval, Job, Outcome, ResultEvidence},
    operation::{Failure, Operation, Status},
    proxmox::{TaskReference, VmReference},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
const APPROVAL_TTL_SECONDS: i64 = 120;

pub(super) fn read_job(connection: &Connection, id: &str) -> Result<Option<Job>, DeletionError> {
    let Some(operation) =
        operations::read(connection, id).map_err(|_| DeletionError::Unavailable)?
    else {
        return Ok(None);
    };
    if operation.kind != "vm_delete" {
        return Err(DeletionError::InvalidOperation);
    }
    let (json, hash, confirmed_at, dispatch_started, upid): (String, String, i64, bool, Option<String>) = connection.query_row(
        "SELECT approval_json, input_hash, confirmed_at, dispatch_started, provider_upid FROM operations WHERE id=?1", [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).map_err(|_| DeletionError::Unavailable)?;
    let approval: Approval = serde_json::from_str(&json).map_err(|_| DeletionError::Unavailable)?;
    if !approval.is_valid()
        || approval
            .input_hash()
            .map_err(|_| DeletionError::Unavailable)?
            != hash
        || operation.instance_id.as_deref() != Some(approval.instance_id.as_str())
    {
        return Err(DeletionError::Unavailable);
    }
    let provider_task = upid
        .map(|upid| {
            let task = TaskReference::new(&approval.node, &upid)
                .map_err(|_| DeletionError::Unavailable)?;
            let target = VmReference::new(&approval.node, approval.vmid)
                .map_err(|_| DeletionError::Unavailable)?;
            if !task.matches_deletion(&target) {
                return Err(DeletionError::Unavailable);
            }
            Ok(task)
        })
        .transpose()?;
    Ok(Some(Job {
        operation,
        approval,
        confirmed_at,
        dispatch_started,
        provider_task,
    }))
}

impl Store {
    /// Same key and identical confirmed inputs retain the original record and expiry.
    pub fn enqueue_deletion(
        &self,
        key: &str,
        approval: &Approval,
    ) -> Result<Operation, DeletionError> {
        if !crate::operation::valid_request_key(key) || !approval.is_valid() {
            return Err(DeletionError::InvalidApproval);
        }
        let hash = approval
            .input_hash()
            .map_err(|_| DeletionError::Unavailable)?;
        let mut connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| DeletionError::Unavailable)?;
        let existing: Option<(String, Option<String>)> = transaction
            .query_row(
                "SELECT id, input_hash FROM operations WHERE request_key=?1",
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| DeletionError::Unavailable)?;
        if let Some((id, previous_hash)) = existing {
            if previous_hash.as_deref() != Some(hash.as_str()) {
                return Err(DeletionError::Conflict);
            }
            let job = read_job(&transaction, &id)?.ok_or(DeletionError::Unavailable)?;
            return Ok(job.operation);
        }
        let instance = super::instances::read(&transaction, &approval.instance_id)
            .map_err(|_| DeletionError::Unavailable)?
            .ok_or(DeletionError::InstanceNotFound)?;
        if !approval.matches_instance(&instance.summary) {
            return Err(DeletionError::TargetMismatch);
        }
        let json = serde_json::to_string(approval).map_err(|_| DeletionError::Unavailable)?;
        let id: String = transaction.query_row(
            "INSERT INTO operations (request_key, kind, instance_id, status, approval_json, input_hash, confirmed_at) VALUES (?1,?2,?3,'queued',?4,?5,unixepoch()) RETURNING id",
            params![key, "vm_delete", approval.instance_id, json, hash], |row| row.get(0),
        ).map_err(|error| match error {
            rusqlite::Error::SqliteFailure(code, _) if matches!(code.extended_code, rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE | rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER) => DeletionError::Conflict,
            _ => DeletionError::Unavailable,
        })?;
        let operation = operations::read(&transaction, &id)
            .map_err(|_| DeletionError::Unavailable)?
            .ok_or(DeletionError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| DeletionError::Unavailable)?;
        Ok(operation)
    }

    pub fn deletion_job(&self, id: &str) -> Result<Option<Job>, DeletionError> {
        let connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        read_job(&connection, id)
    }

    /// Uncertain work with a known task can only be claimed for read-only reconciliation.
    pub fn claim_deletion(&self) -> Result<Option<Job>, DeletionError> {
        let mut connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| DeletionError::Unavailable)?;
        let id: Option<String> = transaction.query_row(
            "UPDATE operations SET status='running', failure_code=NULL, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=(SELECT id FROM operations WHERE kind='vm_delete' AND (status='queued' OR (status='uncertain' AND provider_upid IS NOT NULL AND next_reconcile_at<=unixepoch())) ORDER BY created_at,id LIMIT 1) RETURNING id",
            [], |row| row.get(0),
        ).optional().map_err(|_| DeletionError::Unavailable)?;
        let job = id
            .map(|id| read_job(&transaction, &id)?.ok_or(DeletionError::Unavailable))
            .transpose()?;
        transaction
            .commit()
            .map_err(|_| DeletionError::Unavailable)?;
        Ok(job)
    }

    /// Commit exactly once before attempting the provider write; never reset this marker.
    pub fn begin_deletion_dispatch(
        &self,
        id: &str,
        expected: &Approval,
    ) -> Result<(), DeletionError> {
        let mut connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| DeletionError::Unavailable)?;
        let job = read_job(&transaction, id)?.ok_or(DeletionError::InvalidOperation)?;
        if job.operation.status != Status::Running || job.dispatch_started {
            return Err(DeletionError::InvalidOperation);
        }
        if !expected.is_valid()
            || expected
                .input_hash()
                .map_err(|_| DeletionError::Unavailable)?
                != job
                    .approval
                    .input_hash()
                    .map_err(|_| DeletionError::Unavailable)?
        {
            return Err(DeletionError::TargetMismatch);
        }
        let now: i64 = transaction
            .query_row("SELECT unixepoch()", [], |row| row.get(0))
            .map_err(|_| DeletionError::Unavailable)?;
        if !now
            .checked_sub(job.confirmed_at)
            .is_some_and(|age| (0..=APPROVAL_TTL_SECONDS).contains(&age))
        {
            return Err(DeletionError::ApprovalExpired);
        }
        let changed = transaction.execute("UPDATE operations SET dispatch_started=1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1 AND status='running' AND dispatch_started=0", [id]).map_err(|_| DeletionError::Unavailable)?;
        if changed != 1 {
            return Err(DeletionError::InvalidOperation);
        }
        transaction.commit().map_err(|_| DeletionError::Unavailable)
    }

    /// Idempotently retain only a task whose node, worker kind and VMID match approval.
    pub fn link_deletion_task(&self, id: &str, task: &TaskReference) -> Result<(), DeletionError> {
        let mut connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| DeletionError::Unavailable)?;
        let job = read_job(&transaction, id)?.ok_or(DeletionError::InvalidOperation)?;
        if job.operation.status != Status::Running || !job.dispatch_started {
            return Err(DeletionError::InvalidOperation);
        }
        let target = VmReference::new(&job.approval.node, job.approval.vmid)
            .map_err(|_| DeletionError::Unavailable)?;
        if !task.matches_deletion(&target) {
            return Err(DeletionError::TargetMismatch);
        }
        if job.provider_task.as_ref().is_some_and(|old| old != task) {
            return Err(DeletionError::Conflict);
        }
        transaction.execute("UPDATE operations SET provider_upid=?2, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", params![id, task.upid()]).map_err(|_| DeletionError::Unavailable)?;
        transaction.commit().map_err(|_| DeletionError::Unavailable)
    }

    pub fn finish_deletion(&self, id: &str, outcome: Outcome) -> Result<Operation, DeletionError> {
        let mut connection = self.0.lock().map_err(|_| DeletionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| DeletionError::Unavailable)?;
        let job = read_job(&transaction, id)?.ok_or(DeletionError::InvalidOperation)?;
        if job.operation.status != Status::Running {
            return Err(DeletionError::InvalidOperation);
        }
        let (status, result, failure): (&str, Option<String>, Option<Failure>) = match outcome {
            Outcome::Succeeded(evidence) => {
                let task = job
                    .provider_task
                    .as_ref()
                    .ok_or(DeletionError::InvalidOperation)?;
                if !job.dispatch_started || !evidence.matches(&job.approval, task) {
                    return Err(DeletionError::InvalidOperation);
                }
                let instance = super::instances::read(&transaction, &job.approval.instance_id)
                    .map_err(|_| DeletionError::Unavailable)?
                    .ok_or(DeletionError::InstanceNotFound)?;
                if instance.summary.status != crate::instance::Status::Registered
                    || !job.approval.matches_instance(&instance.summary)
                {
                    return Err(DeletionError::TargetMismatch);
                }
                let changed = transaction
                    .execute(
                        "UPDATE instances SET status='deleted' WHERE id=?1 AND status='registered'",
                        [&job.approval.instance_id],
                    )
                    .map_err(|_| DeletionError::Unavailable)?;
                if changed != 1 {
                    return Err(DeletionError::InvalidOperation);
                }
                let result = ResultEvidence {
                    instance_id: job.approval.instance_id.clone(),
                    provider_scope_id: job.approval.provider_scope_id.clone(),
                    node: job.approval.node.clone(),
                    vmid: job.approval.vmid,
                    approval_hash: job
                        .approval
                        .input_hash()
                        .map_err(|_| DeletionError::Unavailable)?,
                    task_status: crate::proxmox::TaskStatus::Succeeded,
                    vm_absence_verified: true,
                    volumes_absence_verified: true,
                    definition_retained: true,
                    association_retired: true,
                };
                (
                    "succeeded",
                    Some(serde_json::to_string(&result).map_err(|_| DeletionError::Unavailable)?),
                    None,
                )
            }
            Outcome::Failed(code) => {
                if matches!(
                    code,
                    Failure::ProviderSubmissionUnconfirmed
                        | Failure::ProviderTaskUnconfirmed
                        | Failure::DeletionStateUnconfirmed
                        | Failure::ProviderTaskFailed
                ) || (job.dispatch_started
                    && !matches!(
                        (&job.provider_task, code),
                        (
                            None,
                            Failure::ProviderUnauthorized | Failure::ProviderForbidden
                        )
                    ))
                {
                    return Err(DeletionError::InvalidOperation);
                }
                ("failed", None, Some(code))
            }
            Outcome::Uncertain(code) => {
                if !job.dispatch_started
                    || (job.provider_task.is_none()
                        != matches!(code, Failure::ProviderSubmissionUnconfirmed))
                    || !matches!(
                        code,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::ProviderTaskUnconfirmed
                            | Failure::DeletionStateUnconfirmed
                            | Failure::ProcessInterrupted
                            | Failure::ProviderScopeChanged
                            | Failure::ProviderNotConfigured
                            | Failure::ProviderTaskFailed
                    )
                {
                    return Err(DeletionError::InvalidOperation);
                }
                ("uncertain", None, Some(code))
            }
        };
        let failure = failure
            .map(|code| serde_json::to_value(code).map_err(|_| DeletionError::Unavailable))
            .transpose()?
            .map(|value| value.as_str().unwrap_or_default().to_owned());
        transaction.execute("UPDATE operations SET status=?2,result_json=?3,failure_code=?4,next_reconcile_at=unixepoch()+5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", params![id,status,result,failure]).map_err(|_| DeletionError::Unavailable)?;
        let operation = operations::read(&transaction, id)
            .map_err(|_| DeletionError::Unavailable)?
            .ok_or(DeletionError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| DeletionError::Unavailable)?;
        Ok(operation)
    }
}
