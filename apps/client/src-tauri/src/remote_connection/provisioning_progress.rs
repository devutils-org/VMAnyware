//! Fixed retained progress GET; never installs a prepared execution intent.
use super::{hex, Failure, RecoveredCreation, RemoteConnection, Status};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    operation_id: String,
    instance_id: String,
    source_instance_id: Option<String>,
    provider_scope_id: String,
    definition_hash: String,
    plan_hash: String,
    node: String,
    vmid: u32,
    status: Status,
    failure: Option<Failure>,
    stages: Vec<Stage>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stage {
    position: usize,
    kind: Kind,
    status: Status,
    submission_started: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    FullClone,
    ConfigureResources,
    ExpandDisk,
    ConfigureCloudInit,
    StartVm,
    WaitForGuestReadiness,
    InstallDeclaredPackages,
    SnapshotAfterProvisioning,
}
impl Progress {
    fn valid(&self, recovered: &RecoveredCreation) -> bool {
        let operation = &recovered.operation;
        let environment = &recovered.environment;
        let provider = &environment.provider_reference;
        if self.operation_id != operation.id
            || self.instance_id != environment.id
            || self.provider_scope_id != provider.scope_id
            || self.definition_hash != environment.definition_hash
            || self.node != provider.node
            || self.vmid != provider.vmid
            || !hex(&self.plan_hash, 64)
            || self
                .source_instance_id
                .as_ref()
                .is_some_and(|s| !hex(s, 32) || s == &self.instance_id)
            || self.status != operation.status
            || self.failure != operation.failure
            || operation
                .result
                .as_ref()
                .is_some_and(|r| r.plan_hash != self.plan_hash)
        {
            return false;
        }
        use Kind::*;
        let contains = |kind| self.stages.iter().any(|s| s.kind == kind);
        let mut expected = vec![FullClone, ConfigureResources];
        if contains(ExpandDisk) {
            expected.push(ExpandDisk);
        }
        expected.extend([ConfigureCloudInit, StartVm, WaitForGuestReadiness]);
        if contains(InstallDeclaredPackages) {
            expected.push(InstallDeclaredPackages);
        }
        if contains(SnapshotAfterProvisioning) {
            expected.push(SnapshotAfterProvisioning);
        }
        if self.stages.len() != expected.len()
            || self
                .stages
                .iter()
                .zip(expected)
                .enumerate()
                .any(|(i, (s, k))| s.position != i || s.kind != k)
        {
            return false;
        }
        let first = self
            .stages
            .iter()
            .position(|s| s.status != Status::Succeeded);
        if first.is_some_and(|i| {
            self.stages[i + 1..]
                .iter()
                .any(|s| s.status != Status::Queued || s.submission_started)
        }) {
            return false;
        }
        match (self.status, first.map(|i| self.stages[i].status)) {
            (Status::Succeeded, None) => true,
            (Status::Queued, Some(Status::Queued)) => self
                .stages
                .iter()
                .all(|s| s.status == Status::Queued && !s.submission_started),
            (Status::Running, Some(Status::Running | Status::Queued)) => true,
            (Status::Failed, Some(Status::Failed)) => true,
            (Status::Interrupted, Some(Status::Interrupted)) => true,
            (Status::Uncertain, Some(Status::Uncertain | Status::Interrupted)) => true,
            _ => false,
        }
    }
}
impl RemoteConnection {
    async fn provisioning_progress(&self, id: u64, operation_id: &str) -> Result<Progress, String> {
        // Reconstruct the association through existing strict GET-only recovery;
        // a concurrent worker transition can require an explicit refresh.
        let recovered = self.recover_creation(id, operation_id).await?;
        let profile = self.snapshot(id)?;
        let result: Result<Progress, String> = profile
            .create_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/operations/{operation_id}/provisioning"))
                        .map_err(|_| "Invalid provisioning progress path.")?,
                ),
            )
            .await;
        self.current(id)?;
        let progress = result?;
        if !progress.valid(&recovered) {
            return Err(
                "Recorded progress differs from its operation or target. Refresh to read it again."
                    .into(),
            );
        }
        Ok(progress)
    }
}
#[tauri::command]
pub async fn read_remote_provisioning_progress(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    operation_id: String,
) -> Result<Progress, String> {
    state
        .provisioning_progress(connection_id, &operation_id)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn operation() -> Value {
        json!({"id":"d".repeat(32),"instanceId":"e".repeat(32),"kind":"vm_provision","status":"running","result":null,"failure":null})
    }
    fn environment() -> Value {
        json!({"id":"e".repeat(32),"definitionName":"progress-lab","definitionHash":"a".repeat(64),"providerReference":{"provider":"proxmox","scopeId":"c".repeat(64),"node":"pve-test","vmid":202}})
    }
    fn progress() -> Value {
        json!({"operationId":"d".repeat(32),"instanceId":"e".repeat(32),"sourceInstanceId":null,"providerScopeId":"c".repeat(64),"definitionHash":"a".repeat(64),"planHash":"b".repeat(64),"node":"pve-test","vmid":202,"status":"running","failure":null,"stages":[
            {"position":0,"kind":"full_clone","status":"succeeded","submissionStarted":true},
            {"position":1,"kind":"configure_resources","status":"running","submissionStarted":true},
            {"position":2,"kind":"configure_cloud_init","status":"queued","submissionStarted":false},
            {"position":3,"kind":"start_vm","status":"queued","submissionStarted":false},
            {"position":4,"kind":"wait_for_guest_readiness","status":"queued","submissionStarted":false}
        ]})
    }
    fn recovered(op: Value) -> RecoveredCreation {
        RecoveredCreation {
            operation: serde_json::from_value(op).unwrap(),
            environment: serde_json::from_value(environment()).unwrap(),
        }
    }
    #[test]
    fn provisioning_progress_rejects_mismatched_identity_order_and_status() {
        let recovered = recovered(operation());
        for field in [
            "operationId",
            "instanceId",
            "providerScopeId",
            "definitionHash",
            "planHash",
            "node",
            "vmid",
            "sourceInstanceId",
            "status",
            "failure",
            "stage_order",
            "duplicate",
            "future_started",
            "future_status",
            "kind",
            "empty",
        ] {
            let mut value = progress();
            match field {
                "planHash" => value[field] = json!("bad"),
                "vmid" => value[field] = json!(203),
                "sourceInstanceId" => value[field] = value["instanceId"].clone(),
                "status" => value[field] = json!("succeeded"),
                "failure" => value[field] = json!("packages_failed"),
                "stage_order" => value["stages"][2]["position"] = json!(3),
                "duplicate" => value["stages"][2] = value["stages"][1].clone(),
                "future_started" => value["stages"][4]["submissionStarted"] = json!(true),
                "future_status" => value["stages"][4]["status"] = json!("succeeded"),
                "kind" => value["stages"][0]["kind"] = json!("configure_resources"),
                "empty" => value["stages"] = json!([]),
                _ => value[field] = json!("f".repeat(64)),
            }
            let value: Progress = serde_json::from_value(value).unwrap();
            assert!(!value.valid(&recovered), "{field}");
        }
    }
    #[test]
    fn provisioning_progress_accepts_optional_steps_and_terminal_evidence_only_when_consistent() {
        let mut value = progress();
        let stages = value["stages"].as_array_mut().unwrap();
        stages.insert(
            2,
            json!({"kind":"expand_disk","status":"succeeded","submissionStarted":true}),
        );
        stages.push(json!({"kind":"install_declared_packages","status":"succeeded","submissionStarted":true}));
        stages.push(json!({"kind":"snapshot_after_provisioning","status":"succeeded","submissionStarted":true}));
        for (i, stage) in stages.iter_mut().enumerate() {
            stage["position"] = json!(i);
            stage["status"] = json!("succeeded");
        }
        value["status"] = json!("succeeded");
        let mut op = operation();
        op["status"] = json!("succeeded");
        op["result"] = json!({"instanceId":"e".repeat(32),"providerScopeId":"c".repeat(64),"planHash":"b".repeat(64),"guestReady":true,"guest":{"ipv4":"192.0.2.42","interface":"ens18","cloudInitExitCode":0,"cloudInitClean":true}});
        let parsed: Progress = serde_json::from_value(value.clone()).unwrap();
        assert!(parsed.valid(&recovered(op.clone())));
        value["planHash"] = json!("f".repeat(64));
        assert!(!serde_json::from_value::<Progress>(value)
            .unwrap()
            .valid(&recovered(op)));
        for status in ["failed", "interrupted", "uncertain"] {
            let mut value = progress();
            value["status"] = json!(status);
            value["stages"][1]["status"] = json!(status);
            let mut op = operation();
            op["status"] = json!(status);
            assert!(serde_json::from_value::<Progress>(value)
                .unwrap()
                .valid(&recovered(op)));
        }
    }
    #[tokio::test]
    async fn provisioning_progress_uses_scoped_authenticated_gets_without_restoring_intent() {
        let mut value = progress();
        value["privateJournal"] = json!("PRIVATE_PROVIDER_OUTPUT");
        value["stages"][0]["result"] = json!({"pid":43,"stdout":"PRIVATE_GUEST_OUTPUT"});
        let (endpoint, ca, task) = super::super::prepare_tests::sequence(vec![
            ("200 OK", operation().to_string()),
            ("200 OK", environment().to_string()),
            ("200 OK", value.to_string()),
        ])
        .await;
        let manager = RemoteConnection::default();
        let id = manager.begin().unwrap();
        manager
            .complete(
                id,
                super::super::Profile::new(
                    &endpoint,
                    "owned-progress-token-0123456789abcdef",
                    Some(ca),
                )
                .unwrap(),
            )
            .unwrap();
        let progress = manager
            .provisioning_progress(id, &"d".repeat(32))
            .await
            .unwrap();
        let output = serde_json::to_string(&progress).unwrap();
        assert!(
            !output.contains("PRIVATE") && !output.contains("pid") && !output.contains("result")
        );
        assert!(manager.0.lock().unwrap().prepared.is_none());
        let requests = task.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with(&format!("GET /api/v1/operations/{} ", "d".repeat(32))));
        assert!(requests[1].starts_with(&format!("GET /api/v1/environments/{} ", "e".repeat(32))));
        assert!(requests[2].starts_with(&format!(
            "GET /api/v1/operations/{}/provisioning ",
            "d".repeat(32)
        )));
        assert!(requests.iter().all(|r| r
            .to_ascii_lowercase()
            .contains("authorization: bearer owned-progress-token-0123456789abcdef")));
        assert!(manager.provisioning_progress(id, "../bad").await.is_err());
        manager.begin().unwrap();
        assert!(manager
            .provisioning_progress(id, &"d".repeat(32))
            .await
            .is_err());
    }
}
