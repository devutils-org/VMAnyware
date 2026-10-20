use super::{lifecycle_config, registered_fixture_instance, Fixture, DROP_REPLY};
use crate::{
    operation::{Failure, Status},
    proxmox::{TaskReference, VmReference},
    snapshot::{Approval, CreateRequest},
    store::Store,
};
use serde_json::{json, Value};
use std::time::Duration;
const UPID: &str = "UPID:pve-lab:00000001:00000002:00000003:qmsnapshot:202:fixture@pve:";
fn request() -> CreateRequest {
    CreateRequest {
        name: "before-work".into(),
        description: "Disk & work + café\nOnly".into(),
    }
}
fn snapshot() -> Value {
    json!({"data":[{"name":"before-work","description":request().description,"snaptime":1700000000,"vmstate":0},{"name":"current"}]})
}
async fn fixture(variant: &str, recovery: bool) -> Fixture {
    let config = lifecycle_config();
    let mut final_config = config.clone();
    if variant == "identity" {
        final_config["data"]["tags"] = json!("foreign");
    }
    let mut completed = snapshot();
    if variant == "metadata" {
        completed["data"][0]["vmstate"] = json!(1);
    }
    let initial = if variant == "duplicate" {
        snapshot()
    } else {
        json!({"data":[{"name":"current"}]})
    };
    let feature = json!({"data":{"hasFeature":variant!="unsupported"}});
    let poll = json!({"data":{"node":"pve-lab","upid":UPID,"status":"stopped","exitstatus":if variant=="task-failed" {"ERROR: private-detail"} else {"OK"}}});
    let mut routes = Vec::<(String, String)>::new();
    if !recovery {
        for (path, body) in [
            ("config?current=1", config.to_string()),
            ("pending", json!({"data":[]}).to_string()),
            (
                "status/current",
                json!({"data":{"vmid":202,"status":"running","qmpstatus":"running"}}).to_string(),
            ),
            ("config?current=1", config.to_string()),
            ("feature?feature=snapshot", feature.to_string()),
            ("snapshot", initial.to_string()),
            ("pending", json!({"data":[]}).to_string()),
            ("config?current=1", config.to_string()),
        ] {
            routes.push((format!("/api2/json/nodes/pve-lab/qemu/202/{path}"), body));
        }
        routes.push((
            "/api2/json/nodes/pve-lab/qemu/202/snapshot".into(),
            if variant == "lost" {
                DROP_REPLY.into()
            } else if variant == "wrong-task" {
                json!({"data":UPID.replace("qmsnapshot","qmstart")}).to_string()
            } else {
                json!({"data":UPID}).to_string()
            },
        ));
    }
    routes.push((
        format!("/api2/json/nodes/pve-lab/tasks/{UPID}/status"),
        poll.to_string(),
    ));
    routes.push((
        "/api2/json/nodes/pve-lab/qemu/202/config?current=1".into(),
        final_config.to_string(),
    ));
    routes.push((
        "/api2/json/nodes/pve-lab/qemu/202/snapshot".into(),
        completed.to_string(),
    ));
    let refs: Vec<_> = routes
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect();
    Fixture::responses("200 OK", &refs, "", Duration::ZERO, true).await
}
fn prepare(path: &std::path::Path, provider: &super::Proxmox) -> (Store, Approval, String) {
    let store = Store::open(path).unwrap();
    let old = registered_fixture_instance(&store, &provider.scope_id());
    // Owned fixture identity matches the generated provider tag.
    let id = "a".repeat(32);
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "UPDATE instances SET id=?1 WHERE id=?2",
            rusqlite::params![id, old],
        )
        .unwrap();
    let instance = store.instance(&id).unwrap().unwrap();
    let approval = Approval {
        instance_id: id,
        provider_scope_id: provider.scope_id(),
        definition_hash: instance.summary.definition_hash,
        node: "pve-lab".into(),
        vmid: 202,
        config_digest: "b".repeat(40),
        request: request(),
        confirmed: true,
    };
    let op = store
        .enqueue_snapshot("snapshot-execution", &approval)
        .unwrap();
    (store, approval, op.id)
}
fn requests(fixture: &mut Fixture) -> Vec<String> {
    let mut requests = vec![];
    while let Ok(request) = fixture.requests.try_recv() {
        requests.push(request);
    }
    requests
}

#[tokio::test]
async fn snapshot_execution_submits_one_encoded_disk_only_request_and_requires_task_and_metadata() {
    let mut fixture = fixture("valid", false).await;
    let provider = fixture.client();
    let dir = tempfile::tempdir().unwrap();
    let (store, approval, id) = prepare(&dir.path().join("success.sqlite"), &provider);
    assert!(
        crate::snapshot_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap()
    );
    let op = store.operation(&id).unwrap().unwrap();
    assert_eq!(op.status, Status::Succeeded);
    let result = op.result.unwrap();
    assert_eq!(result["snapshot"]["name"], approval.request.name);
    assert_eq!(result["snapshot"]["includesRam"], false);
    assert!(
        !crate::snapshot_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap()
    );
    let requests = requests(&mut fixture);
    assert_eq!(requests.len(), 12);
    let posts: Vec<_> = requests.iter().filter(|r| r.starts_with("POST ")).collect();
    assert_eq!(posts.len(), 1);
    assert!(posts[0].starts_with("POST /api2/json/nodes/pve-lab/qemu/202/snapshot HTTP/1.1\r\n"));
    let body = posts[0].split("\r\n\r\n").nth(1).unwrap();
    let form: Vec<_> = form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(
        form,
        vec![
            ("snapname".into(), approval.request.name),
            ("description".into(), approval.request.description),
            ("vmstate".into(), "0".into())
        ]
    );
    assert!(store.snapshot_job(&id).unwrap().unwrap().dispatch_started);
}

#[tokio::test]
async fn snapshot_execution_rejects_disabled_scope_expired_or_failed_preflight_before_post() {
    for variant in ["disabled", "scope", "expired", "duplicate", "unsupported"] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("guard.sqlite");
        let (store, _, id) = prepare(&path, &provider);
        if variant == "scope" {
            let mut approval = store.snapshot_job(&id).unwrap().unwrap().approval;
            approval.provider_scope_id = "c".repeat(64);
            rusqlite::Connection::open(&path)
                .unwrap()
                .execute(
                    "UPDATE operations SET approval_json=?1,input_hash=?2 WHERE id=?3",
                    rusqlite::params![
                        serde_json::to_string(&approval).unwrap(),
                        approval.input_hash().unwrap(),
                        id
                    ],
                )
                .unwrap();
        }
        if variant == "expired" {
            rusqlite::Connection::open(&path)
                .unwrap()
                .execute(
                    "UPDATE operations SET confirmed_at=unixepoch()-121 WHERE id=?1",
                    [&id],
                )
                .unwrap();
        }
        crate::snapshot_executor::run_one(&store, Some(&provider), variant != "disabled")
            .await
            .unwrap();
        let op = store.operation(&id).unwrap().unwrap();
        assert_eq!(op.status, Status::Failed, "{variant}");
        assert_eq!(
            op.failure,
            Some(match variant {
                "disabled" => Failure::SnapshotDisabled,
                "scope" => Failure::ProviderScopeChanged,
                "expired" => Failure::ApprovalExpired,
                _ => Failure::SnapshotPreconditionsFailed,
            })
        );
        assert!(!store.snapshot_job(&id).unwrap().unwrap().dispatch_started);
        assert!(requests(&mut fixture).iter().all(|r| r.starts_with("GET ")));
    }
}

#[tokio::test]
async fn snapshot_execution_lost_or_invalid_task_reply_never_replays_after_restart() {
    for variant in ["lost", "wrong-task"] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lost.sqlite");
        let (store, _, id) = prepare(&path, &provider);
        crate::snapshot_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap();
        assert_eq!(
            store.operation(&id).unwrap().unwrap().failure,
            Some(Failure::ProviderSubmissionUnconfirmed)
        );
        drop(store);
        let store = Store::open(&path).unwrap();
        store.recover_interrupted_operations().unwrap();
        assert!(
            !crate::snapshot_executor::run_one(&store, Some(&provider), true)
                .await
                .unwrap()
        );
        let job = store.snapshot_job(&id).unwrap().unwrap();
        assert!(job.dispatch_started && job.provider_task.is_none());
        assert_eq!(job.operation.status, Status::Uncertain);
        let requests = requests(&mut fixture);
        assert_eq!(requests.len(), 9);
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            1
        );
    }
}

#[tokio::test]
async fn snapshot_execution_known_task_restart_is_get_only_even_when_disabled() {
    let mut fixture = fixture("valid", true).await;
    let provider = fixture.client();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restart.sqlite");
    let (store, approval, id) = prepare(&path, &provider);
    store.claim_snapshot().unwrap().unwrap();
    store.begin_snapshot_dispatch(&id, &approval).unwrap();
    store
        .link_snapshot_task(&id, &TaskReference::new("pve-lab", UPID).unwrap())
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    store.recover_interrupted_operations().unwrap();
    crate::snapshot_executor::run_one(&store, Some(&provider), false)
        .await
        .unwrap();
    assert_eq!(
        store.operation(&id).unwrap().unwrap().status,
        Status::Succeeded
    );
    let requests = requests(&mut fixture);
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
}

#[tokio::test]
async fn snapshot_execution_task_success_alone_cannot_hide_incomplete_or_foreign_snapshot() {
    for variant in ["metadata", "identity", "task-failed"] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let dir = tempfile::tempdir().unwrap();
        let (store, _, id) = prepare(&dir.path().join("unconfirmed.sqlite"), &provider);
        crate::snapshot_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap();
        let op = store.operation(&id).unwrap().unwrap();
        assert_eq!(
            op.status,
            if variant == "task-failed" {
                Status::Failed
            } else {
                Status::Uncertain
            }
        );
        assert_eq!(
            op.failure,
            Some(if variant == "task-failed" {
                Failure::ProviderTaskFailed
            } else {
                Failure::SnapshotStateUnconfirmed
            })
        );
        assert!(!serde_json::to_string(&op)
            .unwrap()
            .contains("private-detail"));
        assert_eq!(
            requests(&mut fixture)
                .iter()
                .filter(|r| r.starts_with("POST "))
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn snapshot_execution_transport_invalid_request_is_rejected_without_network() {
    let mut fixture = fixture("valid", false).await;
    let provider = fixture.client();
    let mut invalid = request();
    invalid.name = "current".into();
    assert_eq!(
        provider
            .dispatch_snapshot(&VmReference::new("pve-lab", 202).unwrap(), &invalid)
            .await
            .unwrap_err(),
        super::Error::Configuration
    );
    assert!(requests(&mut fixture).is_empty());
}

#[tokio::test]
async fn snapshot_execution_production_round_services_reads_and_defaults_new_writes_off() {
    let mut fixture = fixture("valid", false).await;
    let provider = fixture.client();
    let dir = tempfile::tempdir().unwrap();
    let (store, _, id) = prepare(&dir.path().join("round.sqlite"), &provider);
    let read = store.enqueue_inventory("read-before-snapshot").unwrap();
    assert!(
        crate::worker::run_snapshot_round(&store, None, None, false, false, false)
            .await
            .unwrap()
    );
    assert_eq!(
        store.operation(&read.id).unwrap().unwrap().failure,
        Some(Failure::ProviderNotConfigured)
    );
    assert_eq!(
        store.operation(&id).unwrap().unwrap().failure,
        Some(Failure::SnapshotDisabled)
    );
    assert!(requests(&mut fixture).is_empty());
    assert!(
        !crate::worker::run_snapshot_round(&store, None, None, false, false, false)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn snapshot_execution_http_enqueue_to_worker_verifies_success_and_stale_confirmation() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    for stale in [false, true] {
        let mut fixture = fixture("valid", false).await;
        let provider = fixture.client();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("http.sqlite");
        let (store, mut approval, old_id) = prepare(&path, &provider);
        // Setup helper returns an owned pending intent; remove it so this test's
        // only submission is through the actual HTTP route below.
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute("DELETE FROM operations WHERE id=?1", [old_id])
            .unwrap();
        if stale {
            approval.config_digest = "c".repeat(40);
        }
        let app = crate::router_with_snapshots(
            Some(provider.clone()),
            store.clone(),
            None,
            false,
            false,
            true,
        );
        let request = Request::post(format!(
            "/api/v1/environments/{}/snapshots",
            approval.instance_id
        ))
        .header("content-type", "application/json")
        .header("idempotency-key", "http-create")
        .body(Body::from(serde_json::to_vec(&approval).unwrap()))
        .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let location = response.headers()["location"].to_str().unwrap().to_owned();
        let accepted: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(accepted["status"], "queued");
        assert!(requests(&mut fixture).is_empty());
        crate::worker::run_snapshot_round(&store, Some(&provider), None, false, false, true)
            .await
            .unwrap();
        let response = app
            .oneshot(Request::get(location).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let result: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(result["id"], accepted["id"]);
        assert_eq!(result["status"], if stale { "failed" } else { "succeeded" });
        if stale {
            assert_eq!(result["failure"], "snapshot_target_changed");
        }
        assert_eq!(
            requests(&mut fixture)
                .iter()
                .filter(|r| r.starts_with("POST "))
                .count(),
            if stale { 0 } else { 1 }
        );
    }
}
