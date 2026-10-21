//! Explicit instance-scoped power read. Never infers guest/SSH readiness.
use super::{lifecycle::Target, ReadPath, RemoteConnection};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    instance_id: String,
    provider_scope_id: String,
    observation: Observation,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    node: String,
    vmid: u32,
    status: String,
    qmp_status: Option<String>,
}
impl Status {
    fn matches(&self, target: &Target) -> bool {
        self.instance_id == target.instance_id
            && self.provider_scope_id == target.provider_scope_id
            && self.observation.node == target.node
            && self.observation.vmid == target.vmid
            && matches!(self.observation.status.as_str(), "running" | "stopped")
            && self.observation.qmp_status.as_deref().is_none_or(|qmp| {
                [
                    "running",
                    "stopped",
                    "paused",
                    "prelaunch",
                    "shutdown",
                    "suspended",
                    "guest-panicked",
                    "unknown",
                ]
                .contains(&qmp)
            })
    }
}
impl RemoteConnection {
    async fn instance_status(&self, id: u64, target: &Target) -> Result<Status, String> {
        if !target.valid() {
            return Err("Choose a valid registered instance and provider target.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<Status, String> = profile
            .get(ReadPath::InstanceStatus(&target.instance_id))
            .await;
        self.current(id)?;
        let status = result?;
        if !status.matches(target) {
            return Err("The server returned an unrelated or invalid instance observation.".into());
        }
        Ok(status)
    }
}
#[tauri::command]
pub async fn read_remote_instance_status(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    target: Target,
) -> Result<Status, String> {
    state.instance_status(connection_id, &target).await
}
#[cfg(test)]
#[path = "instance_status_tests.rs"]
mod tests;
