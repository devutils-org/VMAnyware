//! Durable disk-only snapshot creation intent and dispatch guards. No provider request is made here.
use super::SnapshotError;
use super::{operations, Store};
use crate::{
    operation::{Failure, Operation, Status},
    proxmox::{TaskReference, TaskStatus, VmReference},
    snapshot::{Approval, Job, Outcome},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
const APPROVAL_TTL_SECONDS: i64 = 120;

pub(super) fn read_job(connection: &Connection, id: &str) -> Result<Option<Job>, SnapshotError> {
    let Some(operation) =
        operations::read(connection, id).map_err(|_| SnapshotError::Unavailable)?
    else {
        return Ok(None);
    };
    if operation.kind != "vm_snapshot_create" {
        return Err(SnapshotError::InvalidOperation);
    }
    let (json, hash, confirmed_at, dispatch_started, upid): (String, String, i64, bool, Option<String>) = connection.query_row(
        "SELECT approval_json, input_hash, confirmed_at, dispatch_started, provider_upid FROM operations WHERE id=?1", [id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).map_err(|_| SnapshotError::Unavailable)?;
    let approval: Approval = serde_json::from_str(&json).map_err(|_| SnapshotError::Unavailable)?;
    if !approval.is_valid()
        || approval
            .input_hash()
            .map_err(|_| SnapshotError::Unavailable)?
            != hash
        || operation.instance_id.as_deref() != Some(approval.instance_id.as_str())
    {
        return Err(SnapshotError::Unavailable);
    }
    let provider_task = upid
        .map(|upid| {
            let task = TaskReference::new(&approval.node, &upid)
                .map_err(|_| SnapshotError::Unavailable)?;
            let target = VmReference::new(&approval.node, approval.vmid)
                .map_err(|_| SnapshotError::Unavailable)?;
            if !task.matches_snapshot(&target) {
                return Err(SnapshotError::Unavailable);
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
    pub fn enqueue_snapshot(
        &self,
        key: &str,
        approval: &Approval,
    ) -> Result<Operation, SnapshotError> {
        if !crate::operation::valid_request_key(key) || !approval.is_valid() {
            return Err(SnapshotError::InvalidApproval);
        }
        let hash = approval
            .input_hash()
            .map_err(|_| SnapshotError::Unavailable)?;
        let mut connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SnapshotError::Unavailable)?;
        let existing: Option<(String, Option<String>)> = transaction
            .query_row(
                "SELECT id, input_hash FROM operations WHERE request_key=?1",
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| SnapshotError::Unavailable)?;
        if let Some((id, previous_hash)) = existing {
            if previous_hash.as_deref() != Some(hash.as_str()) {
                return Err(SnapshotError::Conflict);
            }
            let job = read_job(&transaction, &id)?.ok_or(SnapshotError::Unavailable)?;
            return Ok(job.operation);
        }
        let instance = super::instances::read(&transaction, &approval.instance_id)
            .map_err(|_| SnapshotError::Unavailable)?
            .ok_or(SnapshotError::InstanceNotFound)?;
        if !approval.matches_instance(&instance.summary) {
            return Err(SnapshotError::TargetMismatch);
        }
        let json = serde_json::to_string(approval).map_err(|_| SnapshotError::Unavailable)?;
        let id: String = transaction.query_row(
            "INSERT INTO operations (request_key, kind, instance_id, status, approval_json, input_hash, confirmed_at) VALUES (?1,?2,?3,'queued',?4,?5,unixepoch()) RETURNING id",
            params![key, "vm_snapshot_create", approval.instance_id, json, hash], |row| row.get(0),
        ).map_err(|error| match error {
            rusqlite::Error::SqliteFailure(code, _) if matches!(code.extended_code, rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE | rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER) => SnapshotError::Conflict,
            _ => SnapshotError::Unavailable,
        })?;
        let operation = operations::read(&transaction, &id)
            .map_err(|_| SnapshotError::Unavailable)?
            .ok_or(SnapshotError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| SnapshotError::Unavailable)?;
        Ok(operation)
    }

    pub fn snapshot_job(&self, id: &str) -> Result<Option<Job>, SnapshotError> {
        let connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        read_job(&connection, id)
    }

    /// Uncertain work with a known task can only be claimed for read-only reconciliation.
    pub fn claim_snapshot(&self) -> Result<Option<Job>, SnapshotError> {
        let mut connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SnapshotError::Unavailable)?;
        let id: Option<String> = transaction.query_row(
            "UPDATE operations SET status='running', failure_code=NULL, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=(SELECT id FROM operations WHERE kind='vm_snapshot_create' AND (status='queued' OR (status='uncertain' AND provider_upid IS NOT NULL AND next_reconcile_at<=unixepoch())) ORDER BY created_at,id LIMIT 1) RETURNING id",
            [], |row| row.get(0),
        ).optional().map_err(|_| SnapshotError::Unavailable)?;
        let job = id
            .map(|id| read_job(&transaction, &id)?.ok_or(SnapshotError::Unavailable))
            .transpose()?;
        transaction
            .commit()
            .map_err(|_| SnapshotError::Unavailable)?;
        Ok(job)
    }

    /// Commit exactly once before attempting the provider write; never reset this marker.
    pub fn begin_snapshot_dispatch(
        &self,
        id: &str,
        expected: &Approval,
    ) -> Result<(), SnapshotError> {
        let mut connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SnapshotError::Unavailable)?;
        let job = read_job(&transaction, id)?.ok_or(SnapshotError::InvalidOperation)?;
        if job.operation.status != Status::Running || job.dispatch_started {
            return Err(SnapshotError::InvalidOperation);
        }
        if !expected.is_valid()
            || expected
                .input_hash()
                .map_err(|_| SnapshotError::Unavailable)?
                != job
                    .approval
                    .input_hash()
                    .map_err(|_| SnapshotError::Unavailable)?
        {
            return Err(SnapshotError::TargetMismatch);
        }
        let now: i64 = transaction
            .query_row("SELECT unixepoch()", [], |row| row.get(0))
            .map_err(|_| SnapshotError::Unavailable)?;
        if !now
            .checked_sub(job.confirmed_at)
            .is_some_and(|age| (0..=APPROVAL_TTL_SECONDS).contains(&age))
        {
            return Err(SnapshotError::ApprovalExpired);
        }
        let changed = transaction.execute("UPDATE operations SET dispatch_started=1, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1 AND status='running' AND dispatch_started=0", [id]).map_err(|_| SnapshotError::Unavailable)?;
        if changed != 1 {
            return Err(SnapshotError::InvalidOperation);
        }
        transaction.commit().map_err(|_| SnapshotError::Unavailable)
    }

    /// Idempotently retain only a task whose node, worker kind and VMID match approval.
    pub fn link_snapshot_task(&self, id: &str, task: &TaskReference) -> Result<(), SnapshotError> {
        let mut connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SnapshotError::Unavailable)?;
        let job = read_job(&transaction, id)?.ok_or(SnapshotError::InvalidOperation)?;
        if job.operation.status != Status::Running || !job.dispatch_started {
            return Err(SnapshotError::InvalidOperation);
        }
        let target = VmReference::new(&job.approval.node, job.approval.vmid)
            .map_err(|_| SnapshotError::Unavailable)?;
        if !task.matches_snapshot(&target) {
            return Err(SnapshotError::TargetMismatch);
        }
        if job.provider_task.as_ref().is_some_and(|old| old != task) {
            return Err(SnapshotError::Conflict);
        }
        transaction.execute("UPDATE operations SET provider_upid=?2, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", params![id, task.upid()]).map_err(|_| SnapshotError::Unavailable)?;
        transaction.commit().map_err(|_| SnapshotError::Unavailable)
    }

    pub fn finish_snapshot(&self, id: &str, outcome: Outcome) -> Result<Operation, SnapshotError> {
        let mut connection = self.0.lock().map_err(|_| SnapshotError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SnapshotError::Unavailable)?;
        let job = read_job(&transaction, id)?.ok_or(SnapshotError::InvalidOperation)?;
        if job.operation.status != Status::Running {
            return Err(SnapshotError::InvalidOperation);
        }
        let (status, result, failure) = match outcome {
            Outcome::Succeeded(evidence) => {
                if !job.dispatch_started
                    || job.provider_task.is_none()
                    || evidence.instance_id != job.approval.instance_id
                    || evidence.provider_scope_id != job.approval.provider_scope_id
                    || evidence.node != job.approval.node
                    || evidence.vmid != job.approval.vmid
                    || evidence.task_status != TaskStatus::Succeeded
                    || evidence.snapshot.name != job.approval.request.name
                    || evidence.snapshot.description != job.approval.request.description
                    || evidence.snapshot.includes_ram != Some(false)
                    || evidence.snapshot.provider_state.is_some()
                {
                    return Err(SnapshotError::InvalidOperation);
                }
                (
                    "succeeded",
                    Some(serde_json::to_string(&evidence).map_err(|_| SnapshotError::Unavailable)?),
                    None,
                )
            }
            Outcome::Failed(code) => {
                if matches!(
                    code,
                    Failure::ProviderSubmissionUnconfirmed
                        | Failure::ProviderTaskUnconfirmed
                        | Failure::SnapshotStateUnconfirmed
                ) || (job.dispatch_started
                    && !matches!(
                        (&job.provider_task, code),
                        (
                            None,
                            Failure::ProviderUnauthorized | Failure::ProviderForbidden
                        ) | (Some(_), Failure::ProviderTaskFailed)
                    ))
                {
                    return Err(SnapshotError::InvalidOperation);
                }
                ("failed", None, Some(code))
            }
            Outcome::Uncertain(code) => {
                if !job.dispatch_started
                    || !matches!(
                        code,
                        Failure::ProviderSubmissionUnconfirmed
                            | Failure::ProviderTaskUnconfirmed
                            | Failure::SnapshotStateUnconfirmed
                            | Failure::ProcessInterrupted
                            | Failure::ProviderScopeChanged
                            | Failure::ProviderNotConfigured
                    )
                {
                    return Err(SnapshotError::InvalidOperation);
                }
                ("uncertain", None, Some(code))
            }
        };
        let failure = failure
            .map(|code| serde_json::to_value(code).map_err(|_| SnapshotError::Unavailable))
            .transpose()?
            .map(|value| value.as_str().unwrap_or_default().to_owned());
        transaction.execute("UPDATE operations SET status=?2,result_json=?3,failure_code=?4,next_reconcile_at=unixepoch()+5,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", params![id,status,result,failure]).map_err(|_| SnapshotError::Unavailable)?;
        let operation = operations::read(&transaction, id)
            .map_err(|_| SnapshotError::Unavailable)?
            .ok_or(SnapshotError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| SnapshotError::Unavailable)?;
        Ok(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_schema_keeps_foreign_key_enforcement_after_fresh_and_migrated_open() {
        let migrations = [
            include_str!("../../../migrations/0001_definitions.sql"),
            include_str!("../../../migrations/0002_operations.sql"),
            include_str!("../../../migrations/0003_instances.sql"),
            include_str!("../../../migrations/0004_instance_observations.sql"),
            include_str!("../../../migrations/0005_lifecycle_operations.sql"),
            include_str!("../../../migrations/0006_provisioning_intent.sql"),
            include_str!("../../../migrations/0007_resource_dispatch.sql"),
            include_str!("../../../migrations/0008_disk_dispatch.sql"),
            include_str!("../../../migrations/0009_cloud_init_dispatch.sql"),
            include_str!("../../../migrations/0010_seed_regeneration.sql"),
            include_str!("../../../migrations/0011_snapshot_creation.sql"),
            include_str!("../../../migrations/0012_snapshot_restore.sql"),
            include_str!("../../../migrations/0013_deletion_intent.sql"),
            include_str!("../../../migrations/0014_instance_retirement.sql"),
        ];
        for version in [0, 10, 11, 12, 13, 14] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("fk.sqlite");
            let fixture = rusqlite::Connection::open(&path).unwrap();
            // Build the actual historical schema, rather than relabeling a newer one.
            for migration in &migrations[..version] {
                fixture.execute_batch(migration).unwrap();
            }
            fixture
                .pragma_update(None, "user_version", version as i64)
                .unwrap();
            drop(fixture);
            let store = Store::open(&path).unwrap();
            let connection = store.0.lock().unwrap();
            assert!(connection
                .pragma_query_value(None, "foreign_keys", |r| r.get::<_, bool>(0))
                .unwrap());
            assert_eq!(
                connection
                    .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                15
            );
            assert!(!connection
                .prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap());
            assert!(connection.execute("INSERT INTO provisioning_intents(operation_id,catalog_yaml,plan_json) VALUES ('orphan','test','{}')",[]).is_err());
        }
    }
}
