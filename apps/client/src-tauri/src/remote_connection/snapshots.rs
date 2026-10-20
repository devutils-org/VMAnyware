//! Fixed read-only path on the active authenticated HTTPS profile.
use super::{hex, identifier, ReadPath, RemoteConnection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotList {
    instance_id: String,
    provider_scope_id: String,
    node: String,
    vmid: u32,
    snapshots: Vec<Snapshot>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Snapshot {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) created_at: Option<u64>,
    pub(super) includes_ram: Option<bool>,
    pub(super) parent: Option<String>,
    pub(super) provider_state: Option<String>,
}
impl RemoteConnection {
    async fn snapshots(&self, id: u64, instance_id: &str) -> Result<SnapshotList, String> {
        if !hex(instance_id, 32) {
            return Err("Choose a valid registered instance.".into());
        }
        let profile = self.snapshot(id)?;
        let result: Result<SnapshotList, String> =
            profile.get(ReadPath::Snapshots(instance_id)).await;
        self.current(id)?;
        let list = result?;
        let mut names = HashSet::new();
        if list.instance_id != instance_id
            || !hex(&list.provider_scope_id, 64)
            || !identifier(&list.node, 63)
            || !(100..=999999999).contains(&list.vmid)
            || list.snapshots.iter().any(|snapshot| {
                let valid_name = |name: &str| {
                    !name.is_empty()
                        && name.len() <= 255
                        && name
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                };
                snapshot.name == "current"
                    || !valid_name(&snapshot.name)
                    || !names.insert(&snapshot.name)
                    || snapshot.description.len() > 65536
                    || snapshot
                        .parent
                        .as_deref()
                        .is_some_and(|name| !valid_name(name) || name == "current")
                    || snapshot.provider_state.as_deref().is_some_and(|state| {
                        state.is_empty() || state.len() > 64 || state.chars().any(char::is_control)
                    })
            })
        {
            return Err("The server returned an invalid snapshot list.".into());
        }
        Ok(list)
    }
}
#[tauri::command]
pub async fn read_remote_snapshots(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    instance_id: String,
) -> Result<SnapshotList, String> {
    state.snapshots(connection_id, &instance_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_connection::{
        tests::{fixture, fixture_with_gate},
        Profile,
    };
    use serde_json::json;
    const AUTH: &str = "x-vmanyware-authenticated: bearer\r\n";
    const TOKEN: &str = "fixture-only-access-token-0123456789abcdef";
    fn body() -> String {
        json!({"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"node":"fixture-node","vmid":202,
            "snapshots":[{"name":"before-work","description":"Before work","createdAt":null,"includesRam":null,"parent":null,"providerState":"prepare"}]}).to_string()
    }
    #[tokio::test]
    async fn remote_snapshot_read_uses_fixed_authenticated_path_and_preserves_unknown_state() {
        let (endpoint, ca, task) = fixture("200 OK", &body(), AUTH).await;
        let manager = RemoteConnection::default();
        let id = manager.begin().unwrap();
        manager
            .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
            .unwrap();
        let listing = manager.snapshots(id, &"a".repeat(32)).await.unwrap();
        assert_eq!(listing.snapshots[0].includes_ram, None);
        assert_eq!(
            listing.snapshots[0].provider_state.as_deref(),
            Some("prepare")
        );
        let request = task.await.unwrap();
        assert!(request.starts_with(&format!(
            "GET /api/v1/environments/{}/snapshots HTTP/1.1\r\n",
            "a".repeat(32)
        )));
        assert!(request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {TOKEN}")));
    }
    #[tokio::test]
    async fn remote_snapshot_read_rejects_invalid_data_auth_and_path_without_raw_errors() {
        for (body, headers) in [
            (body().replace(&"a".repeat(32), &"c".repeat(32)), AUTH),
            (body().replace("before-work", "current"), AUTH),
            (body().replace("before-work", "../bad"), AUTH),
            (
                body().replace("\"includesRam\":null", "\"includesRam\":2"),
                AUTH,
            ),
            (body(), ""),
            ("private-provider-detail".into(), AUTH),
        ] {
            let (endpoint, ca, task) = fixture("200 OK", &body, headers).await;
            let manager = RemoteConnection::default();
            let id = manager.begin().unwrap();
            manager
                .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
                .unwrap();
            assert!(manager.snapshots(id, "../secret").await.is_err());
            let failure = manager.snapshots(id, &"a".repeat(32)).await.unwrap_err();
            assert!(!failure.contains(TOKEN) && !failure.contains("private-provider-detail"));
            task.await.unwrap();
        }
    }
    #[tokio::test]
    async fn remote_snapshot_read_discards_response_after_disconnect() {
        let (seen, received) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let (endpoint, ca, task) =
            fixture_with_gate("200 OK", &body(), AUTH, Some((seen, gate))).await;
        let manager = std::sync::Arc::new(RemoteConnection::default());
        let id = manager.begin().unwrap();
        manager
            .complete(id, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
            .unwrap();
        let active = manager.clone();
        let read = tokio::spawn(async move { active.snapshots(id, &"a".repeat(32)).await });
        received.await.unwrap();
        manager.begin().unwrap();
        release.send(()).unwrap();
        assert!(read.await.unwrap().unwrap_err().contains("changed"));
        task.await.unwrap();
    }
}
