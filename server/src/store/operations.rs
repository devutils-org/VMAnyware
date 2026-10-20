use super::{Error, Store};
use crate::operation::{Failure, Operation, Status, Summary};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

const COLUMNS: &str =
    "id, kind, status, result_json, failure_code, created_at, updated_at, instance_id, provider_upid IS NOT NULL";

fn decode<T: serde::de::DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_str(&value).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Operation> {
    let status: String = row.get(2)?;
    let failure: Option<String> = row.get(4)?;
    Ok(Operation {
        id: row.get(0)?,
        kind: row.get(1)?,
        instance_id: row.get(7)?,
        provider_task_known: row.get(8)?,
        status: decode(serde_json::to_string(&status).unwrap_or_default())?,
        result: row.get::<_, Option<String>>(3)?.map(decode).transpose()?,
        failure: failure
            .map(|value| decode(serde_json::to_string(&value).unwrap_or_default()))
            .transpose()?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(super) fn read(connection: &Connection, id: &str) -> Result<Option<Operation>, Error> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM operations WHERE id=?1"),
            [id],
            row,
        )
        .optional()
        .map_err(|_| Error::Unavailable)
}

impl Store {
    pub fn enqueue_instance_observation(
        &self,
        key: &str,
        instance_id: &str,
    ) -> Result<Operation, super::RegistrationError> {
        use super::RegistrationError as Problem;
        if !crate::operation::valid_request_key(key)
            || instance_id.len() != 32
            || !instance_id.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Problem::InvalidRequest);
        }
        let mut connection = self.0.lock().map_err(|_| Problem::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Problem::Unavailable)?;
        let existing = transaction
            .query_row(
                &format!("SELECT {COLUMNS} FROM operations WHERE request_key=?1"),
                [key],
                row,
            )
            .optional()
            .map_err(|_| Problem::Unavailable)?;
        if let Some(operation) = existing {
            if operation.kind != "instance_observe"
                || operation.instance_id.as_deref() != Some(instance_id)
            {
                return Err(Problem::Conflict);
            }
            return Ok(operation);
        }
        let exists = transaction
            .query_row(
                "SELECT 1 FROM instances WHERE id=?1 AND status='registered'",
                [instance_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| Problem::Unavailable)?
            .is_some();
        if !exists {
            return Err(Problem::InstanceNotFound);
        }
        let id: String = transaction.query_row(
            "INSERT INTO operations (request_key, kind, instance_id, status) VALUES (?1, 'instance_observe', ?2, 'queued') RETURNING id",
            params![key, instance_id], |row| row.get(0),
        ).map_err(|error| match error {
            rusqlite::Error::SqliteFailure(code, _) if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE => Problem::Conflict,
            _ => Problem::Unavailable,
        })?;
        let operation = read(&transaction, &id)
            .map_err(|_| Problem::Unavailable)?
            .ok_or(Problem::Unavailable)?;
        transaction.commit().map_err(|_| Problem::Unavailable)?;
        Ok(operation)
    }

    /// Newest 100 records, with a deterministic tie-breaker and no result bodies.
    pub fn recent_operations(&self) -> Result<Vec<Summary>, Error> {
        let connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        let mut statement = connection.prepare(
            "SELECT id, kind, status, failure_code, created_at, updated_at, instance_id, provider_upid IS NOT NULL FROM operations ORDER BY created_at DESC, id DESC LIMIT 100"
        ).map_err(|_| Error::Unavailable)?;
        let rows = statement
            .query_map([], |row| {
                let status: String = row.get(2)?;
                let failure: Option<String> = row.get(3)?;
                Ok(Summary {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    instance_id: row.get(6)?,
                    provider_task_known: row.get(7)?,
                    status: decode(serde_json::to_string(&status).unwrap_or_default())?,
                    failure: failure
                        .map(|value| decode(serde_json::to_string(&value).unwrap_or_default()))
                        .transpose()?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|_| Error::Unavailable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| Error::Unavailable)
    }

    /// Same request key returns its original operation, including after restart.
    pub fn enqueue_inventory(&self, key: &str) -> Result<Operation, Error> {
        if !crate::operation::valid_request_key(key) {
            return Err(Error::InvalidOperation);
        }
        let mut connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let existing = transaction
            .query_row(
                &format!("SELECT {COLUMNS} FROM operations WHERE request_key=?1"),
                [key],
                row,
            )
            .optional()
            .map_err(|_| Error::Unavailable)?;
        if let Some(operation) = existing {
            if operation.kind != "inventory_refresh" {
                return Err(Error::Conflict);
            }
            return Ok(operation);
        }
        let id: String = transaction.query_row(
            "INSERT INTO operations (request_key, kind, status) VALUES (?1, 'inventory_refresh', 'queued') RETURNING id",
            [key], |row| row.get(0)).map_err(|e| match e {
                rusqlite::Error::SqliteFailure(code, _) if code.code == rusqlite::ErrorCode::ConstraintViolation => Error::Conflict,
                _ => Error::Unavailable,
            })?;
        let operation = read(&transaction, &id)?.ok_or(Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(operation)
    }

    pub fn operation(&self, id: &str) -> Result<Option<Operation>, Error> {
        let connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        read(&connection, id)
    }

    /// A single compare-and-set claims the oldest queued job across connections.
    pub fn claim_operation(&self) -> Result<Option<Operation>, Error> {
        let connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        connection.query_row(&format!("UPDATE operations SET status='running', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id=(SELECT id FROM operations WHERE status='queued' AND kind IN ('inventory_refresh','instance_observe') ORDER BY created_at, id LIMIT 1) AND status='queued' RETURNING {COLUMNS}"), [], row)
            .optional().map_err(|_| Error::Unavailable)
    }

    pub fn finish_operation(
        &self,
        id: &str,
        outcome: Result<serde_json::Value, Failure>,
    ) -> Result<Operation, Error> {
        let (status, result, failure) = match outcome {
            Ok(value) => ("succeeded", Some(value.to_string()), None),
            Err(code) => {
                let code = serde_json::to_value(code).map_err(|_| Error::InvalidOperation)?;
                (
                    "failed",
                    None,
                    Some(code.as_str().ok_or(Error::InvalidOperation)?.to_string()),
                )
            }
        };
        let connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        let current = read(&connection, id)?.ok_or(Error::InvalidOperation)?;
        let next = if result.is_some() {
            Status::Succeeded
        } else {
            Status::Failed
        };
        if !matches!(
            current.kind.as_str(),
            "inventory_refresh" | "instance_observe"
        ) || !current.status.permits(next)
        {
            return Err(Error::InvalidOperation);
        }
        let changed = connection.execute("UPDATE operations SET status=?2, result_json=?3, failure_code=?4, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id=?1 AND status='running'", params![id, status, result, failure]).map_err(|_| Error::Unavailable)?;
        if changed != 1 {
            return Err(Error::InvalidOperation);
        }
        read(&connection, id)?.ok_or(Error::Unavailable)
    }

    /// Call only at exclusive service startup, never on ordinary DB opens.
    pub fn recover_interrupted_operations(&self) -> Result<usize, Error> {
        let mut connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        let transaction = connection.transaction().map_err(|_| Error::Unavailable)?;
        transaction.execute("UPDATE provisioning_stages SET status=CASE WHEN dispatch_started=1 THEN 'uncertain' ELSE 'interrupted' END WHERE status='running'", []).map_err(|_| Error::Unavailable)?;
        let changed = transaction.execute(
            "UPDATE operations SET status=CASE WHEN dispatch_started=1 THEN 'uncertain' ELSE 'interrupted' END, failure_code=CASE WHEN dispatch_started=1 AND provider_upid IS NULL THEN 'provider_submission_unconfirmed' ELSE 'process_interrupted' END, next_reconcile_at=0, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE status='running'", []
        ).map_err(|_| Error::Unavailable)?;
        transaction.commit().map_err(|_| Error::Unavailable)?;
        Ok(changed)
    }
}
