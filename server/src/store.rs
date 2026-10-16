use crate::definition::Definition;
mod operations;
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
        let transaction = connection.transaction().map_err(|_| Error::Unavailable)?;
        let version: u32 = transaction
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| Error::Unavailable)?;
        if version > 2 {
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
        transaction.commit().map_err(|_| Error::Unavailable)?;
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
