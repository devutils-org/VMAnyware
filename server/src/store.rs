use crate::definition::Definition;
mod deletion;
mod deletion_recovery;
mod instances;
mod lifecycle;
mod lifecycle_recovery;
mod restore_recovery;
pub use lifecycle::LifecycleError as DeletionError;
mod snapshot;
mod snapshot_recovery;
mod snapshot_restore;
pub use lifecycle::LifecycleError as RestoreError;
pub use lifecycle::LifecycleError as SnapshotError;
mod provisioning;
mod provisioning_cloud_init;
mod provisioning_disk;
mod provisioning_packages;
mod provisioning_progress;
mod provisioning_readiness;
mod provisioning_seed;
mod provisioning_snapshot;
mod provisioning_start;
mod recreation_history;
pub use provisioning::ProvisioningError;
mod operations;
pub use instances::RegistrationError;
pub use lifecycle::LifecycleError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    fmt,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Unavailable,
    Conflict,
    InvalidDefinition,
    InvalidOperation,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "Definition storage is unavailable.",
            Self::Conflict => "A definition with this name already exists.",
            Self::InvalidDefinition => "Definition is invalid.",
            Self::InvalidOperation => "Invalid operation or transition.",
        })
    }
}
impl std::error::Error for Error {}

#[derive(Clone)]
pub struct Store(Arc<Mutex<Connection>>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub name: String,
    pub yaml: String,
    pub hash: String,
    pub revision: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let connection = Connection::open(path).map_err(|_| Error::Unavailable)?;
        Self::initialize(connection)
    }

    pub fn memory() -> Result<Self, Error> {
        Self::initialize(Connection::open_in_memory().map_err(|_| Error::Unavailable)?)
    }

    fn initialize(mut connection: Connection) -> Result<Self, Error> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|_| Error::Unavailable)?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|_| Error::Unavailable)?;
        // SQLite requires foreign keys disabled outside the table-rebuild transaction.
        // No application writes occur until the complete graph has been checked and
        // enforcement is restored below; failure drops this unopened store.
        let initial_version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| Error::Unavailable)?;
        if initial_version < 16 {
            connection
                .pragma_update(None, "foreign_keys", false)
                .map_err(|_| Error::Unavailable)?;
        }
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Unavailable)?;
        let version: u32 = transaction
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| Error::Unavailable)?;
        if version > 16 {
            return Err(Error::Unavailable);
        }
        if version < 1 {
            transaction
                .execute_batch(include_str!("../../migrations/0001_definitions.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 1)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 2 {
            transaction
                .execute_batch(include_str!("../../migrations/0002_operations.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 2)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 3 {
            transaction
                .execute_batch(include_str!("../../migrations/0003_instances.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 3)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 4 {
            transaction
                .execute_batch(include_str!(
                    "../../migrations/0004_instance_observations.sql"
                ))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 4)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 5 {
            transaction
                .execute_batch(include_str!(
                    "../../migrations/0005_lifecycle_operations.sql"
                ))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 5)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 6 {
            transaction
                .execute_batch(include_str!(
                    "../../migrations/0006_provisioning_intent.sql"
                ))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 6)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 7 {
            transaction
                .execute_batch(include_str!("../../migrations/0007_resource_dispatch.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 7)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 8 {
            transaction
                .execute_batch(include_str!("../../migrations/0008_disk_dispatch.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 8)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 9 {
            transaction
                .execute_batch(include_str!(
                    "../../migrations/0009_cloud_init_dispatch.sql"
                ))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 9)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 10 {
            transaction
                .execute_batch(include_str!("../../migrations/0010_seed_regeneration.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 10)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 11 {
            transaction
                .execute_batch(include_str!("../../migrations/0011_snapshot_creation.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 11)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 12 {
            transaction
                .execute_batch(include_str!("../../migrations/0012_snapshot_restore.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 12)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 13 {
            transaction
                .execute_batch(include_str!("../../migrations/0013_deletion_intent.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 13)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 14 {
            transaction
                .execute_batch(include_str!(
                    "../../migrations/0014_instance_retirement.sql"
                ))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 14)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 15 {
            transaction
                .execute_batch(include_str!("../../migrations/0015_recreation_lineage.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 15)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 16 {
            transaction
                .execute_batch(include_str!("../../migrations/0016_instance_release.sql"))
                .map_err(|_| Error::Unavailable)?;
            transaction
                .pragma_update(None, "user_version", 16)
                .map_err(|_| Error::Unavailable)?;
        }
        if version < 16 {
            let foreign_key_problem = transaction
                .prepare("PRAGMA foreign_key_check")
                .map_err(|_| Error::Unavailable)?
                .exists([])
                .map_err(|_| Error::Unavailable)?;
            if foreign_key_problem {
                return Err(Error::Unavailable);
            }
        }
        transaction.commit().map_err(|_| Error::Unavailable)?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|_| Error::Unavailable)?;
        Ok(Self(Arc::new(Mutex::new(connection))))
    }

    pub fn create(&self, definition: &Definition) -> Result<Record, Error> {
        let yaml = definition
            .canonical_yaml()
            .map_err(|_| Error::InvalidDefinition)?;
        let hash = definition.hash().map_err(|_| Error::InvalidDefinition)?;
        let connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        connection.execute("INSERT INTO definitions (name, definition_yaml, definition_hash, revision) VALUES (?1, ?2, ?3, 1)", params![definition.metadata.name, yaml, hash]).map_err(|e| match e {
            rusqlite::Error::SqliteFailure(code, _) if code.code == rusqlite::ErrorCode::ConstraintViolation => Error::Conflict,
            _ => Error::Unavailable,
        })?;
        read(&connection, &definition.metadata.name)?.ok_or(Error::Unavailable)
    }

    pub fn get(&self, name: &str) -> Result<Option<Record>, Error> {
        let connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        read(&connection, name)
    }

    pub fn list(&self) -> Result<Vec<Record>, Error> {
        let connection = self.0.lock().map_err(|_| Error::Unavailable)?;
        let mut statement = connection.prepare("SELECT name, definition_yaml, definition_hash, revision, created_at, updated_at FROM definitions ORDER BY name").map_err(|_| Error::Unavailable)?;
        let records = statement
            .query_map([], record)
            .map_err(|_| Error::Unavailable)?
            .collect::<Result<_, _>>()
            .map_err(|_| Error::Unavailable);
        records
    }
}

fn record(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        name: row.get(0)?,
        yaml: row.get(1)?,
        hash: row.get(2)?,
        revision: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn read(connection: &Connection, name: &str) -> Result<Option<Record>, Error> {
    connection.query_row("SELECT name, definition_yaml, definition_hash, revision, created_at, updated_at FROM definitions WHERE name = ?1", [name], record).optional().map_err(|_| Error::Unavailable)
}
