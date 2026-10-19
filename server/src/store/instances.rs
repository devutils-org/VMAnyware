use super::Store;
use crate::{
    definition::Definition,
    instance::{valid_hash, Instance, ProviderReference, Registration, Status, Summary},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

const SUMMARY_COLUMNS: &str = "id, definition_name, definition_hash, definition_revision, provider_scope, node, vmid, status, created_at";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationError {
    Unavailable,
    InvalidRequest,
    DefinitionNotFound,
    InstanceNotFound,
    DefinitionChanged,
    Conflict,
}

fn summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<Summary> {
    let status: String = row.get(7)?;
    if status != "registered" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(Summary {
        id: row.get(0)?,
        definition_name: row.get(1)?,
        definition_hash: row.get(2)?,
        definition_revision: row.get(3)?,
        provider_reference: ProviderReference {
            provider: "proxmox",
            scope_id: row.get(4)?,
            node: row.get(5)?,
            vmid: row.get(6)?,
        },
        status: Status::Registered,
        created_at: row.get(8)?,
    })
}

pub(super) fn read(
    connection: &Connection,
    id: &str,
) -> Result<Option<Instance>, RegistrationError> {
    connection
        .query_row(
            &format!("SELECT {SUMMARY_COLUMNS}, definition_yaml FROM instances WHERE id=?1"),
            [id],
            |row| {
                Ok(Instance {
                    summary: summary(row)?,
                    definition_yaml: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(|_| RegistrationError::Unavailable)
}

impl Store {
    /// Records a local association only. No provider read/write or ownership claim.
    pub fn register_instance(
        &self,
        request: &Registration,
        scope: &str,
    ) -> Result<Instance, RegistrationError> {
        if !request.is_valid() || !valid_hash(scope) {
            return Err(RegistrationError::InvalidRequest);
        }
        let mut connection = self.0.lock().map_err(|_| RegistrationError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| RegistrationError::Unavailable)?;
        let definition = super::read(&transaction, &request.definition_name)
            .map_err(|_| RegistrationError::Unavailable)?
            .ok_or(RegistrationError::DefinitionNotFound)?;
        if definition.hash != request.definition_hash {
            return Err(RegistrationError::DefinitionChanged);
        }
        let parsed =
            Definition::parse(&definition.yaml).map_err(|_| RegistrationError::Unavailable)?;
        if parsed.hash().map_err(|_| RegistrationError::Unavailable)? != definition.hash
            || parsed.metadata.name != definition.name
        {
            return Err(RegistrationError::Unavailable);
        }
        let id: String = transaction.query_row(
            "INSERT INTO instances (definition_name, definition_hash, definition_revision, definition_yaml, provider_scope, node, vmid) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
            params![definition.name, definition.hash, definition.revision, definition.yaml, scope, request.node, request.vmid],
            |row| row.get(0),
        ).map_err(|error| match error {
            rusqlite::Error::SqliteFailure(code, _) if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE => RegistrationError::Conflict,
            _ => RegistrationError::Unavailable,
        })?;
        let instance = read(&transaction, &id)?.ok_or(RegistrationError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| RegistrationError::Unavailable)?;
        Ok(instance)
    }

    pub fn instance(&self, id: &str) -> Result<Option<Instance>, RegistrationError> {
        let connection = self.0.lock().map_err(|_| RegistrationError::Unavailable)?;
        read(&connection, id)
    }

    /// Recent 100 associations across scopes; results omit the YAML snapshot.
    pub fn recent_instances(&self) -> Result<Vec<Summary>, RegistrationError> {
        let connection = self.0.lock().map_err(|_| RegistrationError::Unavailable)?;
        let mut statement = connection
            .prepare(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM instances ORDER BY created_at DESC, id DESC LIMIT 100"
        ))
            .map_err(|_| RegistrationError::Unavailable)?;
        let rows = statement
            .query_map([], summary)
            .map_err(|_| RegistrationError::Unavailable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| RegistrationError::Unavailable)
    }
}
