use super::{lifecycle_config, registered_fixture_instance, Fixture, DROP_REPLY};
use crate::{
    operation::{Failure, Status},
    proxmox::{TaskReference, VmReference},
    snapshot_restore::{Approval, Request},
    store::Store,
};
use serde_json::{json, Value};
use std::time::Duration;
const UPID: &str = "UPID:pve-lab:00000001:00000002:00000003:qmrollback:202:fixture@pve:";
fn request() -> Request {
    Request {
        snapshot_name: "before-work".into(),
    }
}
fn saved_config() -> Value {
    json!({"tags":format!("vmanyware-{}","a".repeat(32)),"template":0,"scsi0":"storage:vm-202-disk-0,size=8G","snaptime":1700000000,"description":"Before"})
}
fn saved_hash() -> String {
    use sha2::{Digest, Sha256};
    let config = saved_config();
    let sorted: std::collections::BTreeMap<_, _> = config.as_object().unwrap().iter().collect();
    let mut material = b"vmanyware-restore-snapshot-config-v1\0before-work\0".to_vec();
    material.extend(serde_json::to_vec(&sorted).unwrap());
    format!("{:x}", Sha256::digest(material))
}
fn preflight(routes: &mut Vec<(String, String)>, variant: &str, after: bool) {
    let mut live = lifecycle_config();
    if after {
        live["data"]["digest"] = json!("c".repeat(40));
    }
    if variant == "foreign" {
        live["data"]["tags"] = json!("foreign");
    }
    if variant == "live-locked" {
        live["data"]["lock"] = json!("backup");
    }
    let mut saved = saved_config();
    if variant == "saved-change" {
        saved["scsi0"] = json!("other:disk,size=8G");
    }
    if variant == "ram-config" {
        saved["vmstate"] = json!("storage:vm-202-state");
    }
    if variant == "saved-locked" {
        saved["lock"] = json!("backup");
    }
    let mut entry =
        json!({"name":"before-work","description":"Before","snaptime":1700000000,"vmstate":0});
    if variant == "ram-list" {
        entry["vmstate"] = json!(1);
    }
    if variant == "incomplete" {
        entry["snapstate"] = json!("prepare");
    }
    let list = if variant == "missing" {
        json!({"data":[{"name":"current"}]})
    } else {
        json!({"data":[entry,{"name":"current"}]})
    };
    let power = json!({"data":{"vmid":202,"status":if variant=="running"{"running"}else{"stopped"},"qmpstatus":if variant=="paused"{Some("paused")}else{None}}});
    let pending = if variant == "pending" {
        json!({"data":[{"key":"memory","pending":"4096"}]})
    } else {
        json!({"data":[]})
    };
    let mut add = |path: &str, value: Value| {
        routes.push((
            format!("/api2/json/nodes/pve-lab/qemu/202/{path}"),
            value.to_string(),
        ))
    };
    add("config?current=1", live.clone());
    add("pending", json!({"data":[]}));
    add("status/current", power.clone());
    add("config?current=1", live.clone());
    add("snapshot", list.clone());
    if variant != "missing" {
        add("snapshot/before-work/config", json!({"data":saved.clone()}));
        add(
            "feature?feature=snapshot",
            json!({"data":{"hasFeature":variant!="unsupported"}}),
        );
        add("snapshot/before-work/config", json!({"data":saved}));
    }
    add("snapshot", list);
    add("pending", pending);
    add("config?current=1", live);
    add("status/current", power);
}
async fn fixture(variant: &str, recovery: bool) -> Fixture {
    let mut routes = vec![];
    if !recovery {
        let before = variant.strip_prefix("before-").unwrap_or("valid");
        preflight(&mut routes, before, false);
        routes.push((
            "/api2/json/nodes/pve-lab/qemu/202/snapshot/before-work/rollback".into(),
            match variant {
                "lost" => DROP_REPLY.into(),
                "wrong-task" => json!({"data":UPID.replace("qmrollback","qmsnapshot")}).to_string(),
                "wrong-vmid" => json!({"data":UPID.replace(":202:",":203:")}).to_string(),
                "wrong-node" => json!({"data":UPID.replace("pve-lab","other-node")}).to_string(),
                _ => json!({"data":UPID}).to_string(),
            },
        ));
    }
    routes.push((format!("/api2/json/nodes/pve-lab/tasks/{UPID}/status"),if variant=="poll-lost"{DROP_REPLY.into()}else{json!({"data":{"node":"pve-lab","upid":UPID,"status":"stopped","exitstatus":if variant=="task-failed"{"ERROR: private-detail"}else{"OK"}}}).to_string()}));
    preflight(
        &mut routes,
        variant.strip_prefix("after-").unwrap_or("valid"),
        true,
    );
    let refs: Vec<_> = routes
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_str()))
        .collect();
    Fixture::responses("200 OK", &refs, "", Duration::ZERO, true).await
}
fn prepare(path: &std::path::Path, provider: &super::Proxmox) -> (Store, Approval, String) {
    let store = Store::open(path).unwrap();
    let old = registered_fixture_instance(&store, &provider.scope_id());
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
        snapshot_config_hash: saved_hash(),
        request: request(),
        confirmed: true,
        acknowledge_data_loss: true,
    };
    let op = store.enqueue_restore("owned-restore", &approval).unwrap();
    (store, approval, op.id)
}
fn requests(fixture: &mut Fixture) -> Vec<String> {
    let mut all = vec![];
    while let Ok(request) = fixture.requests.try_recv() {
        all.push(request);
    }
    all
}
#[tokio::test]
async fn restore_execution_one_fixed_post_then_task_and_independent_stopped_snapshot_checks() {
    let mut fixture = fixture("valid", false).await;
    let provider = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let (store, approval, id) = prepare(&directory.path().join("success.sqlite"), &provider);
    assert!(crate::worker::run_restore_round(
        &store,
        Some(&provider),
        None,
        false,
        false,
        false,
        true
    )
    .await
    .unwrap());
    let result = store.operation(&id).unwrap().unwrap();
    assert_eq!(result.status, Status::Succeeded);
    let evidence = result.result.unwrap();
    assert_eq!(evidence["snapshotName"], approval.request.snapshot_name);
    assert_eq!(
        evidence["snapshotConfigHash"],
        approval.snapshot_config_hash
    );
    assert_eq!(evidence["observation"]["status"], "stopped");
    assert!(!serde_json::to_string(&evidence)
        .unwrap()
        .contains("storage:vm"));
    assert!(
        !crate::restore_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap()
    );
    let all = requests(&mut fixture);
    assert_eq!(all.len(), 26);
    let posts: Vec<_> = all.iter().filter(|r| r.starts_with("POST ")).collect();
    assert_eq!(posts.len(), 1);
    assert!(posts[0].starts_with(
        "POST /api2/json/nodes/pve-lab/qemu/202/snapshot/before-work/rollback HTTP/1.1\r\n"
    ));
    assert_eq!(posts[0].split("\r\n\r\n").nth(1), Some("start=0"));
    let job = store.restore_job(&id).unwrap().unwrap();
    assert!(job.dispatch_started && job.provider_task.is_some());
}
#[tokio::test]
async fn restore_execution_blocks_changed_or_unsafe_approval_before_dispatch() {
    for variant in [
        "disabled",
        "scope",
        "expired",
        "digest",
        "hash",
        "before-running",
        "before-paused",
        "before-ram-list",
        "before-ram-config",
        "before-incomplete",
        "before-saved-change",
        "before-saved-locked",
        "before-foreign",
        "before-live-locked",
        "before-pending",
        "before-missing",
        "before-unsupported",
    ] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("guard.sqlite");
        let (store, mut approval, id) = prepare(&path, &provider);
        let connection = rusqlite::Connection::open(&path).unwrap();
        match variant {
            "scope" => approval.provider_scope_id = "c".repeat(64),
            "digest" => approval.config_digest = "d".repeat(40),
            "hash" => approval.snapshot_config_hash = "e".repeat(64),
            _ => {}
        }
        if ["scope", "digest", "hash"].contains(&variant) {
            connection
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
            connection
                .execute(
                    "UPDATE operations SET confirmed_at=unixepoch()-121 WHERE id=?1",
                    [&id],
                )
                .unwrap();
        }
        crate::restore_executor::run_one(&store, Some(&provider), variant != "disabled")
            .await
            .unwrap();
        let job = store.restore_job(&id).unwrap().unwrap();
        assert_eq!(job.operation.status, Status::Failed, "{variant}");
        assert!(!job.dispatch_started, "{variant}");
        assert!(
            requests(&mut fixture).iter().all(|r| r.starts_with("GET ")),
            "{variant}"
        );
    }
}
#[tokio::test]
async fn restore_execution_unknown_submission_never_repeats_after_reopen() {
    for variant in ["lost", "wrong-task", "wrong-node", "wrong-vmid"] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lost.sqlite");
        let (store, _, id) = prepare(&path, &provider);
        crate::restore_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap();
        let job = store.restore_job(&id).unwrap().unwrap();
        assert_eq!(
            job.operation.failure,
            Some(Failure::ProviderSubmissionUnconfirmed),
            "{variant}"
        );
        assert!(job.dispatch_started && job.provider_task.is_none());
        drop(store);
        let store = Store::open(&path).unwrap();
        store.recover_interrupted_operations().unwrap();
        assert!(
            !crate::restore_executor::run_one(&store, Some(&provider), true)
                .await
                .unwrap()
        );
        let all = requests(&mut fixture);
        assert_eq!(all.len(), 13);
        assert_eq!(all.iter().filter(|r| r.starts_with("POST ")).count(), 1);
    }
}
#[tokio::test]
async fn restore_execution_known_task_restart_only_reads_even_with_writes_disabled() {
    let mut fixture = fixture("valid", true).await;
    let provider = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("restart.sqlite");
    let (store, approval, id) = prepare(&path, &provider);
    store.claim_restore().unwrap().unwrap();
    store.begin_restore_dispatch(&id, &approval).unwrap();
    store
        .link_restore_task(&id, &TaskReference::new("pve-lab", UPID).unwrap())
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    store.recover_interrupted_operations().unwrap();
    crate::worker::run_restore_round(&store, Some(&provider), None, false, false, false, false)
        .await
        .unwrap();
    assert_eq!(
        store.operation(&id).unwrap().unwrap().status,
        Status::Succeeded
    );
    let all = requests(&mut fixture);
    assert_eq!(all.len(), 13);
    assert!(all.iter().all(|r| r.starts_with("GET ")));
}
#[tokio::test]
async fn restore_execution_task_success_alone_cannot_hide_failed_postchecks() {
    for variant in [
        "poll-lost",
        "task-failed",
        "after-running",
        "after-paused",
        "after-ram-list",
        "after-ram-config",
        "after-incomplete",
        "after-saved-change",
        "after-saved-locked",
        "after-foreign",
        "after-live-locked",
        "after-pending",
        "after-missing",
        "after-unsupported",
    ] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let (store, _, id) = prepare(&directory.path().join("post.sqlite"), &provider);
        crate::restore_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap();
        let op = store.operation(&id).unwrap().unwrap();
        assert_eq!(
            op.status,
            if variant == "task-failed" {
                Status::Failed
            } else {
                Status::Uncertain
            },
            "{variant}"
        );
        assert_eq!(
            op.failure,
            Some(match variant {
                "poll-lost" => Failure::ProviderTaskUnconfirmed,
                "task-failed" => Failure::ProviderTaskFailed,
                _ => Failure::RestoreStateUnconfirmed,
            }),
            "{variant}"
        );
        assert!(op.provider_task_known);
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
async fn restore_execution_invalid_transport_input_and_default_worker_never_write() {
    let mut fixture = fixture("valid", false).await;
    let provider = fixture.client();
    let target = VmReference::new("pve-lab", 202).unwrap();
    for name in ["current", "../other", "a/b", "x"] {
        assert_eq!(
            provider
                .dispatch_restore(
                    &target,
                    &Request {
                        snapshot_name: name.into()
                    }
                )
                .await
                .unwrap_err(),
            super::Error::Configuration
        );
    }
    let directory = tempfile::tempdir().unwrap();
    let (store, _, id) = prepare(&directory.path().join("disabled.sqlite"), &provider);
    let read = store.enqueue_inventory("read").unwrap();
    crate::worker::run_restore_round(&store, None, None, false, false, false, false)
        .await
        .unwrap();
    assert_eq!(
        store.operation(&id).unwrap().unwrap().failure,
        Some(Failure::RestoreDisabled)
    );
    assert_eq!(
        store.operation(&read.id).unwrap().unwrap().failure,
        Some(Failure::ProviderNotConfigured)
    );
    assert!(requests(&mut fixture).is_empty());
}

#[tokio::test]
async fn restore_execution_transport_rejects_errors_without_redirect_or_retry() {
    for (status, body, expected) in [
        (
            "401 Unauthorized",
            "private-provider-error",
            super::Error::Unauthorized,
        ),
        (
            "403 Forbidden",
            "private-provider-error",
            super::Error::Forbidden,
        ),
        (
            "302 Found",
            "private-provider-error",
            super::Error::Provider,
        ),
        (
            "500 Internal Server Error",
            "private-provider-error",
            super::Error::Provider,
        ),
        (
            "200 OK",
            "not-json-private-error",
            super::Error::InvalidResponse,
        ),
    ] {
        let mut fixture =
            Fixture::new(status, body, "Location: /unexpected\r\n", Duration::ZERO).await;
        let provider = fixture.client();
        let error = provider
            .dispatch_restore(&VmReference::new("pve-lab", 202).unwrap(), &request())
            .await
            .unwrap_err();
        assert_eq!(error, expected, "{status}");
        assert!(!format!("{error:?}").contains("private"));
        let all = requests(&mut fixture);
        assert_eq!(all.len(), 1, "{status}");
        assert!(all[0]
            .starts_with("POST /api2/json/nodes/pve-lab/qemu/202/snapshot/before-work/rollback "));
    }
}

#[tokio::test]
async fn restore_http_submission_to_worker_binds_revisions_and_never_replays_unknown_submission() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request as HttpRequest, StatusCode},
    };
    use tower::ServiceExt;
    for variant in ["valid", "digest", "hash", "lost"] {
        let mut fixture = fixture(if variant == "lost" { "lost" } else { "valid" }, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("http.sqlite");
        let (store, mut approval, old) = prepare(&path, &provider);
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute("DELETE FROM operations WHERE id=?1", [old])
            .unwrap();
        if variant == "digest" {
            approval.config_digest = "d".repeat(40);
        }
        if variant == "hash" {
            approval.snapshot_config_hash = "e".repeat(64);
        }
        let app = crate::router_with_restores(
            Some(provider.clone()),
            store.clone(),
            None,
            false,
            false,
            false,
            true,
        );
        let submit = || {
            HttpRequest::post(format!(
                "/api/v1/environments/{}/snapshots/{}/restore?node=other&vmid=999",
                approval.instance_id, approval.request.snapshot_name
            ))
            .header("content-type", "application/json")
            .header("idempotency-key", "http-restore")
            .body(Body::from(serde_json::to_vec(&approval).unwrap()))
            .unwrap()
        };
        let response = app.clone().oneshot(submit()).await.unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let location = response.headers()["location"].to_str().unwrap().to_owned();
        let queued: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(queued["status"], "queued");
        assert!(requests(&mut fixture).is_empty());
        crate::worker::run_restore_round(&store, Some(&provider), None, false, false, false, true)
            .await
            .unwrap();
        let response = app
            .clone()
            .oneshot(HttpRequest::get(location).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let result: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(result["id"], queued["id"]);
        assert_eq!(
            result["status"],
            match variant {
                "valid" => "succeeded",
                "lost" => "uncertain",
                _ => "failed",
            }
        );
        if ["digest", "hash"].contains(&variant) {
            assert_eq!(result["failure"], "restore_target_changed");
        }
        if variant == "lost" {
            assert_eq!(result["failure"], "provider_submission_unconfirmed");
        }
        let retry = app.oneshot(submit()).await.unwrap();
        assert_eq!(retry.status(), StatusCode::ACCEPTED);
        let retry: Value =
            serde_json::from_slice(&to_bytes(retry.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(retry, result);
        assert!(!crate::worker::run_restore_round(
            &store,
            Some(&provider),
            None,
            false,
            false,
            false,
            true
        )
        .await
        .unwrap());
        let all = requests(&mut fixture);
        assert_eq!(
            all.iter().filter(|r| r.starts_with("POST ")).count(),
            if ["valid", "lost"].contains(&variant) {
                1
            } else {
                0
            }
        );
        assert!(all
            .iter()
            .all(|r| !r.contains("/qemu/999/") && !r.contains("/nodes/other/")));
    }
}

#[tokio::test]
async fn restore_http_preflight_capability_is_independent_of_eligibility_and_has_no_write() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request as HttpRequest, StatusCode},
    };
    use tower::ServiceExt;
    for (enabled, variant, eligibility) in [
        (false, "valid", "passed"),
        (true, "valid", "passed"),
        (true, "before-running", "failed"),
        (true, "before-ram-list", "failed"),
    ] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preflight-http.sqlite");
        let (store, approval, old) = prepare(&path, &provider);
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute("DELETE FROM operations WHERE id=?1", [old])
            .unwrap();
        let app = crate::router_with_restores(
            Some(provider),
            store.clone(),
            None,
            false,
            false,
            false,
            enabled,
        );
        let response = app
            .oneshot(
                HttpRequest::post(format!(
                    "/api/v1/environments/{}/snapshot-restore-preflight",
                    approval.instance_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"snapshotName":"before-work"}).to_string(),
                ))
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let report: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
        assert_eq!(report["executionEnabled"], enabled);
        assert_eq!(report["restorePreconditions"], eligibility);
        assert_eq!(report["destructive"], true);
        assert_eq!(report["startsVm"], false);
        assert_eq!(report["instanceId"], approval.instance_id);
        assert_eq!(report["snapshotConfigHash"], approval.snapshot_config_hash);
        let all = requests(&mut fixture);
        assert_eq!(all.len(), 12);
        assert!(all.iter().all(|r| r.starts_with("GET ")));
        assert!(store.recent_operations().unwrap().is_empty());
    }
}
