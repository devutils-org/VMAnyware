//! Bounded source-scoped discovery; never claims or dispatches provisioning work.
use super::{provisioning, ProvisioningError, Store};
use crate::{
    instance::Status,
    provisioning_intent::{RecreationHistory, RecreationSummary},
};

impl Store {
    pub fn recreation_history(
        &self,
        source_id: &str,
    ) -> Result<Option<RecreationHistory>, ProvisioningError> {
        if source_id.len() != 32
            || !source_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Ok(None);
        }
        let connection = self.0.lock().map_err(|_| ProvisioningError::Unavailable)?;
        let Some(source) = super::instances::read(&connection, source_id)
            .map_err(|_| ProvisioningError::Unavailable)?
        else {
            return Ok(None);
        };
        if source.summary.status != Status::Deleted {
            return Ok(None);
        }
        let mut statement=connection.prepare("SELECT o.id FROM recreation_sources r JOIN operations o ON o.id=r.operation_id WHERE r.source_instance_id=?1 ORDER BY o.created_at DESC,o.id DESC LIMIT 100").map_err(|_|ProvisioningError::Unavailable)?;
        let ids = statement
            .query_map([source_id], |row| row.get::<_, String>(0))
            .map_err(|_| ProvisioningError::Unavailable)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ProvisioningError::Unavailable)?;
        let mut operations = Vec::with_capacity(ids.len());
        for id in ids {
            // Bound saved payloads before the shared job reader parses them.
            let bounded:bool=connection.query_row(
                "SELECT length(CAST(o.approval_json AS BLOB))<=4096 AND length(CAST(p.plan_json AS BLOB))<=65536 AND length(CAST(p.catalog_yaml AS BLOB))<=65536 AND length(CAST(i.definition_yaml AS BLOB))<=65536 AND length(o.created_at)<=64 AND (o.result_json IS NULL OR length(CAST(o.result_json AS BLOB))<=131072) AND NOT EXISTS (SELECT 1 FROM provisioning_stages s WHERE s.operation_id=o.id AND s.result_json IS NOT NULL AND length(CAST(s.result_json AS BLOB))>131072) FROM operations o JOIN provisioning_intents p ON p.operation_id=o.id JOIN instances i ON i.id=o.instance_id WHERE o.id=?1",
                [&id],|row|row.get(0)).map_err(|_|ProvisioningError::Unavailable)?;
            if !bounded {
                return Err(ProvisioningError::Unavailable);
            }
            let job =
                provisioning::read(&connection, &id)?.ok_or(ProvisioningError::Unavailable)?;
            if job.recreated_from.as_deref() != Some(source_id) {
                return Err(ProvisioningError::Unavailable);
            }
            let operation = job.operation;
            operations.push(RecreationSummary {
                id: operation.id,
                instance_id: operation
                    .instance_id
                    .ok_or(ProvisioningError::Unavailable)?,
                status: operation.status,
                created_at: operation.created_at,
            });
        }
        Ok(Some(RecreationHistory {
            source_instance_id: source_id.into(),
            operations,
        }))
    }
}
