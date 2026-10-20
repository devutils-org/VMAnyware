use super::*;
use crate::remote_connection::{
    create::prepare_tests::sequence, tests::fixture_with_gate, Profile,
};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-snapshot-token-0123456789abcdef";
fn request() -> Request {
    Request {
        snapshot_name: "before-work".into(),
    }
}
fn target() -> String {
    json!({"id":"a".repeat(32),"definitionHash":"c".repeat(64),"providerReference":{"provider":"proxmox","scopeId":"b".repeat(64),"node":"pve-test","vmid":202}}).to_string()
}
fn review() -> Review {
    serde_json::from_value(json!({"instanceId":"a".repeat(32),"definitionHash":"c".repeat(64),"providerScopeId":"b".repeat(64),"node":"pve-test","vmid":202,"configDigest":"d".repeat(40),"request":request(),"observation":{"node":"pve-test","vmid":202,"status":"stopped","qmpStatus":"stopped"},"selectedSnapshot":{"name":"before-work","description":"Before","createdAt":1700000000,"includesRam":false,"parent":null,"providerState":null},"snapshotConfigHash":"f".repeat(64),"destructive":true,"startsVm":false,"executionEnabled":true,"restorePreconditions":"passed","checks":CHECKS.iter().map(|code|json!({"code":code,"status":"passed","message":"ignored-server-message"})).collect::<Vec<_>>()})).unwrap()
}
fn queued() -> Value {
    json!({"id":"e".repeat(32),"kind":"vm_snapshot_restore","instanceId":"a".repeat(32),"status":"queued","result":null,"failure":null})
}
fn succeeded() -> Value {
    json!({"id":"e".repeat(32),"kind":"vm_snapshot_restore","instanceId":"a".repeat(32),"status":"succeeded","providerTaskKnown":true,"failure":null,"result":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"node":"pve-test","vmid":202,"taskStatus":"succeeded","snapshotName":request().snapshot_name,"snapshotConfigHash":"f".repeat(64),"observation":{"node":"pve-test","vmid":202,"status":"stopped","qmpStatus":"stopped"}}})
}
fn manager(endpoint: &str, ca: reqwest::Certificate) -> (Arc<RemoteConnection>, u64) {
    let manager = Arc::new(RemoteConnection::default());
    let id = manager.begin().unwrap();
    manager
        .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
        .unwrap();
    (manager, id)
}
fn seed(manager: &RemoteConnection, operation: bool) {
    manager.0.lock().unwrap().snapshot_restore.prepared = Some(Prepared {
        key: "snapshot-key".into(),
        review: review(),
        submitted: operation,
        inflight: false,
        operation: operation.then(|| "e".repeat(32)),
        terminal: false,
    });
}

#[tokio::test]
async fn remote_snapshot_restore_retains_review_and_posts_only_confirmed_inputs_then_gets_status() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", target()),
        ("200 OK", serde_json::to_string(&review()).unwrap()),
        ("202 Accepted", queued().to_string()),
        ("200 OK", succeeded().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let result = manager
        .prepare_restore(id, "snapshot-key".into(), "a".repeat(32), request())
        .await
        .unwrap();
    assert!(result.eligible());
    assert!(manager
        .confirm_restore(id, "snapshot-key", false, true)
        .await
        .is_err());
    assert!(manager
        .confirm_restore(id, "wrong-key", true, true)
        .await
        .is_err());
    assert!(manager
        .confirm_restore(id, "snapshot-key", true, false)
        .await
        .is_err());
    let operation = manager
        .confirm_restore(id, "snapshot-key", true, true)
        .await
        .unwrap();
    assert_eq!(operation.status, Status::Queued);
    assert!(manager
        .confirm_restore(id, "snapshot-key", true, true)
        .await
        .is_err());
    assert!(manager.clear_restore(id, "snapshot-key").is_err());
    assert_eq!(
        manager
            .restore_operation(id, "snapshot-key")
            .await
            .unwrap()
            .status,
        Status::Succeeded
    );
    manager.clear_restore(id, "snapshot-key").unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with(&format!(
        "GET /api/v1/environments/{} HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[1].starts_with(&format!(
        "POST /api/v1/environments/{}/snapshot-restore-preflight HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[2].starts_with(&format!(
        "POST /api/v1/environments/{}/snapshots/before-work/restore HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[3].starts_with(&format!(
        "GET /api/v1/operations/{} HTTP/1.1",
        "e".repeat(32)
    )));
    for request in &requests {
        assert!(request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {TOKEN}")));
    }
    assert!(requests[2]
        .to_ascii_lowercase()
        .contains("idempotency-key: snapshot-key"));
    let approval: Value =
        serde_json::from_str(requests[2].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(approval, review().approval());
    assert_eq!(approval["acknowledgeDataLoss"], true);
    assert_eq!(approval["confirmed"], true);
    assert!(approval.get("start").is_none());
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains("ignored-server-message"));
}
#[tokio::test]
async fn remote_snapshot_restore_lost_reply_keeps_same_key_body_and_blocks_replacement() {
    let (endpoint, ca, task) = sequence(vec![
        (
            "500 Internal Server Error",
            "private-provider-detail".into(),
        ),
        ("202 Accepted", queued().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager, false);
    let failure = manager
        .confirm_restore(id, "snapshot-key", true, true)
        .await
        .unwrap_err();
    assert!(!failure.contains("private-provider-detail") && !failure.contains(TOKEN));
    assert!(manager.clear_restore(id, "snapshot-key").is_err());
    assert!(manager
        .prepare_restore(id, "another-key".into(), "a".repeat(32), request())
        .await
        .is_err());
    assert!(manager.restore_operation(id, "snapshot-key").await.is_err());
    manager
        .confirm_restore(id, "snapshot-key", true, true)
        .await
        .unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
}
#[tokio::test]
async fn remote_snapshot_restore_blocks_disabled_or_failed_checks_without_submission() {
    for enabled in [false, true] {
        let mut report = review();
        report.execution_enabled = enabled;
        if enabled {
            report.restore_preconditions = CheckStatus::Failed;
            report.checks[5].status = CheckStatus::Failed;
        }
        let (endpoint, ca, task) = sequence(vec![
            ("200 OK", target()),
            ("200 OK", serde_json::to_string(&report).unwrap()),
        ])
        .await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(!manager
            .prepare_restore(id, "snapshot-key".into(), "a".repeat(32), request())
            .await
            .unwrap()
            .eligible());
        assert!(manager
            .confirm_restore(id, "snapshot-key", true, true)
            .await
            .is_err());
        manager.clear_restore(id, "snapshot-key").unwrap();
        assert_eq!(task.await.unwrap().len(), 2);
    }
}
#[tokio::test]
async fn remote_snapshot_restore_rejects_mismatched_request_target_and_inconsistent_checks() {
    for variant in [
        "name",
        "target",
        "scope",
        "ram",
        "check",
        "status",
        "fingerprint",
        "missing-hash",
        "missing-snapshot",
        "selected-name",
        "destructive",
        "start",
        "power",
        "qmp",
        "power-target",
    ] {
        let mut report = serde_json::to_value(review()).unwrap();
        match variant {
            "name" => report["request"]["snapshotName"] = json!("other-work"),
            "target" => report["vmid"] = json!(203),
            "scope" => report["providerScopeId"] = json!("f".repeat(64)),
            "ram" => report["selectedSnapshot"]["includesRam"] = json!(true),
            "check" => report["checks"][0]["code"] = json!("untrusted-code"),
            "fingerprint" => report["snapshotConfigHash"] = json!("bad"),
            "missing-hash" => report["snapshotConfigHash"] = Value::Null,
            "missing-snapshot" => report["selectedSnapshot"] = Value::Null,
            "selected-name" => report["selectedSnapshot"]["name"] = json!("other-snapshot"),
            "destructive" => report["destructive"] = json!(false),
            "start" => report["startsVm"] = json!(true),
            "power" => report["observation"]["status"] = json!("running"),
            "qmp" => report["observation"]["qmpStatus"] = json!("paused"),
            "power-target" => report["observation"]["vmid"] = json!(203),
            _ => report["checks"][0]["status"] = json!("failed"),
        }
        let (endpoint, ca, task) =
            sequence(vec![("200 OK", target()), ("200 OK", report.to_string())]).await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(
            manager
                .prepare_restore(id, "snapshot-key".into(), "a".repeat(32), request())
                .await
                .is_err(),
            "{variant}"
        );
        assert!(manager
            .confirm_restore(id, "snapshot-key", true, true)
            .await
            .is_err());
        assert_eq!(task.await.unwrap().len(), 2);
    }
}
#[tokio::test]
async fn remote_snapshot_restore_disconnect_during_submission_discards_reply_without_replay() {
    let (seen, received) = tokio::sync::oneshot::channel();
    let (release, gate) = tokio::sync::oneshot::channel();
    let (endpoint, ca, task) = fixture_with_gate(
        "202 Accepted",
        &queued().to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, gate)),
    )
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager, false);
    let active = manager.clone();
    let submit =
        tokio::spawn(async move { active.confirm_restore(id, "snapshot-key", true, true).await });
    received.await.unwrap();
    assert!(manager
        .confirm_restore(id, "snapshot-key", true, true)
        .await
        .unwrap_err()
        .contains("progress"));
    assert!(manager.clear_restore(id, "snapshot-key").is_err());
    manager.begin().unwrap();
    assert!(manager.snapshot_restore_is_empty());
    release.send(()).unwrap();
    assert!(submit.await.unwrap().unwrap_err().contains("may continue"));
    assert!(manager
        .0
        .lock()
        .unwrap()
        .snapshot_restore
        .prepared
        .is_none());
    task.await.unwrap();
}
#[tokio::test]
async fn remote_snapshot_restore_stale_preparation_stops_before_followup_request() {
    for disconnected in [false, true] {
        let (seen, received) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let (endpoint, ca, task) = fixture_with_gate(
            "200 OK",
            &target(),
            "x-vmanyware-authenticated: bearer\r\n",
            Some((seen, gate)),
        )
        .await;
        let (manager, id) = manager(&endpoint, ca);
        let active = manager.clone();
        let prepare = tokio::spawn(async move {
            active
                .prepare_restore(id, "snapshot-key".into(), "a".repeat(32), request())
                .await
        });
        received.await.unwrap();
        if disconnected {
            manager.begin().unwrap();
        } else {
            manager.0.lock().unwrap().snapshot_restore.revision += 1;
        }
        release.send(()).unwrap();
        assert!(prepare.await.unwrap().is_err());
        let request = task.await.unwrap();
        assert!(request.starts_with("GET "));
        assert!(manager
            .0
            .lock()
            .unwrap()
            .snapshot_restore
            .prepared
            .is_none());
    }
}
#[test]
fn remote_snapshot_restore_success_requires_exact_snapshot_and_stopped_evidence() {
    let review = review();
    let operation: Operation = serde_json::from_value(succeeded()).unwrap();
    assert!(operation.valid(&review, Some(&"e".repeat(32))));
    for variant in [
        "id",
        "instance",
        "name",
        "description",
        "scope",
        "task",
        "task-marker",
        "ram",
        "incomplete",
        "kind",
    ] {
        let mut operation = succeeded();
        match variant {
            "id" => operation["id"] = json!("f".repeat(32)),
            "instance" => operation["instanceId"] = json!("f".repeat(32)),
            "name" => operation["result"]["snapshotName"] = json!("another"),
            "description" => operation["result"]["snapshotConfigHash"] = json!("other"),
            "scope" => operation["result"]["providerScopeId"] = json!("f".repeat(64)),
            "task" => operation["result"]["taskStatus"] = json!("running"),
            "task-marker" => operation["providerTaskKnown"] = json!(false),
            "ram" => operation["result"]["observation"]["status"] = json!("running"),
            "incomplete" => operation["result"]["observation"]["qmpStatus"] = json!("paused"),
            _ => operation["kind"] = json!("vm_provision"),
        }
        let parsed = serde_json::from_value::<Operation>(operation);
        assert!(
            parsed.is_err() || !parsed.unwrap().valid(&review, Some(&"e".repeat(32))),
            "{variant}"
        );
    }
}
#[tokio::test]
async fn remote_snapshot_restore_invalid_input_never_contacts_server() {
    let manager = RemoteConnection::default();
    for name in ["current", "../bad", "x", "1bad"] {
        assert!(manager
            .prepare_restore(
                1,
                "key".into(),
                "a".repeat(32),
                Request {
                    snapshot_name: name.into()
                }
            )
            .await
            .unwrap_err()
            .contains("valid snapshot"));
    }
}

#[tokio::test]
async fn remote_snapshot_restore_rejects_missing_auth_marker_invalid_status_and_oversized_reply() {
    for (status, body, headers) in [
        (
            "403 Forbidden",
            "private-provider-detail".into(),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
        ("202 Accepted", queued().to_string(), ""),
        (
            "200 OK",
            queued().to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
        (
            "202 Accepted",
            "private-provider-detail".into(),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
        (
            "202 Accepted",
            "x".repeat(256 * 1024 + 1),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
    ] {
        let (endpoint, ca, task) =
            crate::remote_connection::tests::fixture(status, &body, headers).await;
        let (manager, id) = manager(&endpoint, ca);
        seed(&manager, false);
        let failure = manager
            .confirm_restore(id, "snapshot-key", true, true)
            .await
            .unwrap_err();
        assert!(!failure.contains(TOKEN) && !failure.contains("private-provider-detail"));
        assert!(
            manager
                .0
                .lock()
                .unwrap()
                .snapshot_restore
                .prepared
                .as_ref()
                .unwrap()
                .submitted
        );
        assert!(manager.clear_restore(id, "snapshot-key").is_err());
        task.await.unwrap();
    }
}

#[tokio::test]
async fn remote_snapshot_restore_socket_loss_retains_unknown_submission() {
    let (seen, received) = tokio::sync::oneshot::channel();
    let (_release, gate) = tokio::sync::oneshot::channel();
    let (endpoint, ca, task) = fixture_with_gate(
        "202 Accepted",
        &queued().to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, gate)),
    )
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager, false);
    let active = manager.clone();
    let submission =
        tokio::spawn(async move { active.confirm_restore(id, "snapshot-key", true, true).await });
    received.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(submission
        .await
        .unwrap()
        .unwrap_err()
        .contains("interrupted"));
    let state = manager.0.lock().unwrap();
    let intent = state.snapshot_restore.prepared.as_ref().unwrap();
    assert!(intent.submitted && !intent.inflight && intent.operation.is_none());
    drop(state);
    assert!(manager.clear_restore(id, "snapshot-key").is_err());
}

#[test]
fn remote_snapshot_restore_status_validation_preserves_uncertainty() {
    let review = review();
    for (status, known, failure, valid) in [
        ("queued", false, None, true),
        ("queued", true, None, false),
        ("running", true, None, true),
        (
            "uncertain",
            false,
            Some("provider_submission_unconfirmed"),
            true,
        ),
        ("uncertain", true, Some("restore_state_unconfirmed"), true),
        ("uncertain", true, Some("provider_task_unconfirmed"), true),
        ("uncertain", true, Some("provider_scope_changed"), true),
        ("uncertain", true, Some("provider_not_configured"), true),
        ("uncertain", false, Some("restore_state_unconfirmed"), false),
        ("uncertain", true, Some("provider_task_failed"), false),
        ("failed", false, Some("provider_forbidden"), true),
        ("failed", true, Some("provider_task_failed"), true),
        ("failed", true, Some("restore_state_unconfirmed"), false),
        ("failed", false, Some("process_interrupted"), false),
        (
            "failed",
            false,
            Some("provider_submission_unconfirmed"),
            false,
        ),
        ("interrupted", false, Some("process_interrupted"), true),
        ("interrupted", true, Some("process_interrupted"), false),
    ] {
        let mut value = queued();
        value["status"] = json!(status);
        value["providerTaskKnown"] = json!(known);
        value["failure"] = json!(failure);
        let operation: Operation = serde_json::from_value(value).unwrap();
        assert_eq!(
            operation.valid(&review, None),
            valid,
            "{status}/{known}/{failure:?}"
        );
    }
}

#[tokio::test]
async fn remote_snapshot_restore_failed_missing_snapshot_report_is_reviewable_but_cannot_submit() {
    let mut report = review();
    report.selected_snapshot = None;
    report.snapshot_config_hash = None;
    report.restore_preconditions = CheckStatus::Failed;
    report.checks[6].status = CheckStatus::Failed;
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", target()),
        ("200 OK", serde_json::to_string(&report).unwrap()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let report = manager
        .prepare_restore(id, "snapshot-key".into(), "a".repeat(32), request())
        .await
        .unwrap();
    assert!(!report.eligible());
    assert!(manager
        .confirm_restore(id, "snapshot-key", true, true)
        .await
        .is_err());
    manager.clear_restore(id, "snapshot-key").unwrap();
    assert_eq!(task.await.unwrap().len(), 2);
}

#[tokio::test]
async fn remote_snapshot_restore_known_uncertain_operation_only_reads_until_verified_terminal() {
    let uncertain = json!({"id":"e".repeat(32),"kind":"vm_snapshot_restore","instanceId":"a".repeat(32),"status":"uncertain","providerTaskKnown":true,"failure":"restore_state_unconfirmed","result":null});
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", uncertain.to_string()),
        ("200 OK", succeeded().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager, true);
    assert_eq!(
        manager
            .restore_operation(id, "snapshot-key")
            .await
            .unwrap()
            .status,
        Status::Uncertain
    );
    assert!(manager.clear_restore(id, "snapshot-key").is_err());
    assert!(manager
        .confirm_restore(id, "snapshot-key", true, true)
        .await
        .is_err());
    assert!(manager
        .prepare_restore(id, "other-key".into(), "a".repeat(32), request())
        .await
        .is_err());
    assert_eq!(
        manager
            .restore_operation(id, "snapshot-key")
            .await
            .unwrap()
            .status,
        Status::Succeeded
    );
    manager.clear_restore(id, "snapshot-key").unwrap();
    let all = task.await.unwrap();
    assert_eq!(all.len(), 2);
    assert!(all
        .iter()
        .all(|request| request.starts_with("GET /api/v1/operations/")));
}

#[tokio::test]
async fn remote_snapshot_restore_disconnected_status_read_cannot_restore_local_intent() {
    let (seen, received) = tokio::sync::oneshot::channel();
    let (release, gate) = tokio::sync::oneshot::channel();
    let (endpoint, ca, task) = fixture_with_gate(
        "200 OK",
        &succeeded().to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, gate)),
    )
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager, true);
    let active = manager.clone();
    let read = tokio::spawn(async move { active.restore_operation(id, "snapshot-key").await });
    received.await.unwrap();
    let next = manager.begin().unwrap();
    assert_ne!(next, id);
    release.send(()).unwrap();
    assert!(read.await.unwrap().is_err());
    assert!(manager.snapshot_restore_is_empty());
    let request = task.await.unwrap();
    assert!(request.starts_with("GET /api/v1/operations/"));
}
