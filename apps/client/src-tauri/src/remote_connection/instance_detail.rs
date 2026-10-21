//! Saved definition/association inspection, not current guest/resource evidence.
use super::{
    hex, identifier, lifecycle::Target, ReadPath, RemoteConnection, RemoteEnvironmentStatus,
    RemoteProvider,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    id: String,
    definition_name: String,
    definition_hash: String,
    definition_revision: u32,
    definition_yaml: String,
    provider_reference: RemoteProvider,
    status: RemoteEnvironmentStatus,
}
impl Detail {
    fn matches(&self, target: &Target, name: &str) -> bool {
        self.id == target.instance_id
            && self.definition_name == name
            && self.definition_revision > 0
            && self.provider_reference.provider == "proxmox"
            && self.provider_reference.scope_id == target.provider_scope_id
            && self.provider_reference.node == target.node
            && self.provider_reference.vmid == target.vmid
            && !self.definition_yaml.is_empty()
            && self.definition_yaml.len() <= 64 * 1024
            && hex(&self.definition_hash, 64)
            && format!("{:x}", Sha256::digest(self.definition_yaml.as_bytes()))
                == self.definition_hash
    }
}
impl RemoteConnection {
    async fn instance_detail(
        &self,
        id: u64,
        target: &Target,
        name: &str,
    ) -> Result<Detail, String> {
        if !target.valid() || !identifier(name, 63) {
            return Err(
                "Choose a valid saved instance, definition name and provider target.".into(),
            );
        }
        let profile = self.snapshot(id)?;
        let result: Result<Detail, String> = profile
            .get(ReadPath::Environment(&target.instance_id))
            .await;
        self.current(id)?;
        let detail = result?;
        if !detail.matches(target, name) {
            return Err(
                "The saved instance, definition hash or selected target does not match.".into(),
            );
        }
        Ok(detail)
    }
}
#[tauri::command]
pub async fn read_remote_instance_detail(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    target: Target,
    definition_name: String,
) -> Result<Detail, String> {
    state
        .instance_detail(connection_id, &target, &definition_name)
        .await
}
#[cfg(test)]
#[path = "instance_detail_tests.rs"]
mod tests;
