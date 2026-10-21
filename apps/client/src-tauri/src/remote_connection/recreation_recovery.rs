//! Fixed saved recreation GETs; no review, request key or consent is installed.
use super::{
    create::Status,
    hex,
    recreation::{Saved, Source},
    RemoteConnection,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    source_instance_id: String,
    operations: Vec<Entry>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    instance_id: String,
    status: Status,
    created_at: String,
}
impl History {
    fn valid(&self, source: &str) -> bool {
        let mut operations = HashSet::new();
        let mut instances = HashSet::new();
        self.source_instance_id == source
            && self.operations.len() <= 100
            && self.operations.iter().all(|entry| {
                hex(&entry.id, 32)
                    && hex(&entry.instance_id, 32)
                    && entry.instance_id != source
                    && operations.insert(&entry.id)
                    && instances.insert(&entry.instance_id)
                    && !entry.created_at.is_empty()
                    && entry.created_at.len() <= 64
                    && !entry.created_at.chars().any(char::is_control)
            })
            && self.operations.windows(2).all(|entries| {
                (&entries[0].created_at, &entries[0].id) >= (&entries[1].created_at, &entries[1].id)
            })
    }
}
impl RemoteConnection {
    async fn recreation_history(&self, id: u64, source_id: &str) -> Result<History, String> {
        if !hex(source_id, 32) {
            return Err("Choose a valid retained source instance.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<History, String> = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{source_id}/recreation-operations"
                        ))
                        .map_err(|_| "Invalid saved recreation path.")?,
                ),
            )
            .await;
        self.current(id)?;
        let history = result?;
        if !history.valid(source_id) {
            return Err("The server returned invalid saved recreation history.".into());
        }
        Ok(history)
    }
    async fn recover_recreation(
        &self,
        id: u64,
        source_id: &str,
        operation_id: &str,
    ) -> Result<Saved, String> {
        if !hex(source_id, 32) || !hex(operation_id, 32) {
            return Err("Choose canonical saved source and operation IDs.".into());
        }
        let profile = self.snapshot(id)?;
        let source: Source = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/environments/{source_id}"))
                        .map_err(|_| "Invalid source path.")?,
                ),
            )
            .await?;
        self.current(id)?;
        if !source.valid(source_id) {
            return Err("The server did not return the selected retired source.".into());
        }
        let result: Result<Saved, String> = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{source_id}/recreation-operations/{operation_id}"
                        ))
                        .map_err(|_| "Invalid saved recreation path.")?,
                ),
            )
            .await;
        self.current(id)?;
        let saved = result?;
        saved.valid(&source, operation_id)?;
        Ok(saved)
    }
}
#[tauri::command]
pub async fn read_remote_recreation_history(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    source_instance_id: String,
) -> Result<History, String> {
    state
        .recreation_history(connection_id, &source_instance_id)
        .await
}
#[tauri::command]
pub async fn recover_remote_recreation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    source_instance_id: String,
    operation_id: String,
) -> Result<Saved, String> {
    state
        .recover_recreation(connection_id, &source_instance_id, &operation_id)
        .await
}
#[cfg(test)]
#[path = "recreation_recovery_tests.rs"]
mod tests;
