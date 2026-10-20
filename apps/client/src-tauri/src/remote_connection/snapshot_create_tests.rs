use super::*;
use crate::remote_connection::{create::prepare_tests::sequence, tests::fixture_with_gate};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-snapshot-token-0123456789abcdef";
fn request() -> Request {
    Request {
        name: "before-work".into(),
        description: "Disk & café + work\nOnly".into(),
    }
}
fn target() -> String {
    json!({"id":"a".repeat(32),"definitionHash":"c".repeat(64),"providerReference":{"provider":"proxmox","scopeId":"b".repeat(64),"node":"pve-test","vmid":202}}).to_string()
}
fn review() -> Review {
    serde_json::from_value(json!({"instanceId":"a".repeat(32),"definitionHash":"c".repeat(64),"providerScopeId":"b".repeat(64),"node":"pve-test","vmid":202,"configDigest":"d".repeat(40),"request":request(),"includesRam":false,"executionEnabled":true,"creationPreconditions":"passed","checks":CHECKS.iter().map(|code|json!({"code":code,"status":"passed","message":"ignored-server-message"})).collect::<Vec<_>>()})).unwrap()
}
fn queued() -> Value {
    json!({"id":"e".repeat(32),"kind":"vm_snapshot_create","instanceId":"a".repeat(32),"status":"queued","result":null,"failure":null})
}
fn succeeded() -> Value {
    json!({"id":"e".repeat(32),"kind":"vm_snapshot_create","instanceId":"a".repeat(32),"status":"succeeded","providerTaskKnown":true,"failure":null,"result":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"node":"pve-test","vmid":202,"taskStatus":"succeeded","snapshot":{"name":request().name,"description":request().description,"createdAt":1700000000,"includesRam":false,"parent":null,"providerState":null}}})
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
    manager.0.lock().unwrap().snapshot_creation.prepared = Some(Prepared {
        key: "snapshot-key".into(),
        review: review(),
        submitted: operation,
        inflight: false,
        operation: operation.then(|| "e".repeat(32)),
        terminal: false,
    });
}

#[tokio::test]
async fn remote_snapshot_create_retains_review_and_posts_only_confirmed_inputs_then_gets_status() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", target()),
        ("200 OK", serde_json::to_string(&review()).unwrap()),
        ("202 Accepted", queued().to_string()),
        ("200 OK", succeeded().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let result = manager
        .prepare_snapshot(id, "snapshot-key".into(), "a".repeat(32), request())
        .await
        .unwrap();
    assert!(result.eligible());
    assert!(manager
        .confirm_snapshot(id, "snapshot-key", false)
        .await
        .is_err());
    assert!(manager
        .confirm_snapshot(id, "wrong-key", true)
        .await
        .is_err());
    let operation = manager
        .confirm_snapshot(id, "snapshot-key", true)
        .await
        .unwrap();
    assert_eq!(operation.status, Status::Queued);
    assert!(manager
        .confirm_snapshot(id, "snapshot-key", true)
        .await
        .is_err());
    assert!(manager.clear_snapshot(id, "snapshot-key").is_err());
    assert_eq!(
        manager
            .snapshot_operation(id, "snapshot-key")
            .await
            .unwrap()
            .status,
        Status::Succeeded
    );
    manager.clear_snapshot(id, "snapshot-key").unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with(&format!(
        "GET /api/v1/environments/{} HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[1].starts_with(&format!(
        "POST /api/v1/environments/{}/snapshot-preflight HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[2].starts_with(&format!(
        "POST /api/v1/environments/{}/snapshots HTTP/1.1",
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
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains("ignored-server-message"));
}
#[tokio::test]
async fn remote_snapshot_create_lost_reply_keeps_same_key_body_and_blocks_replacement() {
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
        .confirm_snapshot(id, "snapshot-key", true)
        .await
        .unwrap_err();
    assert!(!failure.contains("private-provider-detail") && !failure.contains(TOKEN));
    assert!(manager.clear_snapshot(id, "snapshot-key").is_err());
    assert!(manager
        .prepare_snapshot(id, "another-key".into(), "a".repeat(32), request())
        .await
        .is_err());
    assert!(manager
        .snapshot_operation(id, "snapshot-key")
        .await
        .is_err());
    manager
        .confirm_snapshot(id, "snapshot-key", true)
        .await
        .unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
}
#[tokio::test]
async fn remote_snapshot_create_blocks_disabled_or_failed_checks_without_submission() {
    for enabled in [false, true] {
        let mut report = review();
        report.execution_enabled = enabled;
        if enabled {
            report.creation_preconditions = CheckStatus::Failed;
            report.checks[5].status = CheckStatus::Failed;
        }
        let (endpoint, ca, task) = sequence(vec![
            ("200 OK", target()),
            ("200 OK", serde_json::to_string(&report).unwrap()),
        ])
        .await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(!manager
            .prepare_snapshot(id, "snapshot-key".into(), "a".repeat(32), request())
            .await
            .unwrap()
            .eligible());
        assert!(manager
            .confirm_snapshot(id, "snapshot-key", true)
            .await
            .is_err());
        manager.clear_snapshot(id, "snapshot-key").unwrap();
        assert_eq!(task.await.unwrap().len(), 2);
    }
}
#[tokio::test]
async fn remote_snapshot_create_rejects_mismatched_request_target_and_inconsistent_checks() {
    for variant in ["name", "target", "scope", "ram", "check", "status"] {
        let mut report = serde_json::to_value(review()).unwrap();
        match variant {
            "name" => report["request"]["name"] = json!("other-work"),
            "target" => report["vmid"] = json!(203),
            "scope" => report["providerScopeId"] = json!("f".repeat(64)),
            "ram" => report["includesRam"] = json!(true),
            "check" => report["checks"][0]["code"] = json!("untrusted-code"),
            _ => report["checks"][0]["status"] = json!("failed"),
        }
        let (endpoint, ca, task) =
            sequence(vec![("200 OK", target()), ("200 OK", report.to_string())]).await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(
            manager
                .prepare_snapshot(id, "snapshot-key".into(), "a".repeat(32), request())
                .await
                .is_err(),
            "{variant}"
        );
        assert!(manager
            .confirm_snapshot(id, "snapshot-key", true)
            .await
            .is_err());
        assert_eq!(task.await.unwrap().len(), 2);
    }
}
#[tokio::test]
async fn remote_snapshot_create_disconnect_during_submission_discards_reply_without_replay() {
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
        tokio::spawn(async move { active.confirm_snapshot(id, "snapshot-key", true).await });
    received.await.unwrap();
    assert!(manager
        .confirm_snapshot(id, "snapshot-key", true)
        .await
        .unwrap_err()
        .contains("progress"));
    assert!(manager.clear_snapshot(id, "snapshot-key").is_err());
    manager.begin().unwrap();
    release.send(()).unwrap();
    assert!(submit.await.unwrap().unwrap_err().contains("may continue"));
    assert!(manager
        .0
        .lock()
        .unwrap()
        .snapshot_creation
        .prepared
        .is_none());
    task.await.unwrap();
}
#[tokio::test]
async fn remote_snapshot_create_stale_preparation_stops_before_followup_request() {
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
                .prepare_snapshot(id, "snapshot-key".into(), "a".repeat(32), request())
                .await
        });
        received.await.unwrap();
        if disconnected {
            manager.begin().unwrap();
        } else {
            manager.0.lock().unwrap().snapshot_creation.revision += 1;
        }
        release.send(()).unwrap();
        assert!(prepare.await.unwrap().is_err());
        let request = task.await.unwrap();
        assert!(request.starts_with("GET "));
        assert!(manager
            .0
            .lock()
            .unwrap()
            .snapshot_creation
            .prepared
            .is_none());
    }
}
#[test]
fn remote_snapshot_create_success_requires_exact_complete_disk_only_evidence() {
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
            "name" => operation["result"]["snapshot"]["name"] = json!("another"),
            "description" => operation["result"]["snapshot"]["description"] = json!("other"),
            "scope" => operation["result"]["providerScopeId"] = json!("f".repeat(64)),
            "task" => operation["result"]["taskStatus"] = json!("running"),
            "task-marker" => operation["providerTaskKnown"] = json!(false),
            "ram" => operation["result"]["snapshot"]["includesRam"] = json!(true),
            "incomplete" => operation["result"]["snapshot"]["providerState"] = json!("prepare"),
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
async fn remote_snapshot_create_invalid_input_never_contacts_server() {
    let manager = RemoteConnection::default();
    for name in ["current", "../bad", "x", "1bad"] {
        assert!(manager
            .prepare_snapshot(
                1,
                "key".into(),
                "a".repeat(32),
                Request {
                    name: name.into(),
                    description: "".into()
                }
            )
            .await
            .unwrap_err()
            .contains("valid snapshot"));
    }
}

#[tokio::test]
async fn remote_snapshot_create_rejects_missing_auth_marker_invalid_status_and_oversized_reply() {
    for (status, body, headers) in [
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
            .confirm_snapshot(id, "snapshot-key", true)
            .await
            .unwrap_err();
        assert!(!failure.contains(TOKEN) && !failure.contains("private-provider-detail"));
        assert!(
            manager
                .0
                .lock()
                .unwrap()
                .snapshot_creation
                .prepared
                .as_ref()
                .unwrap()
                .submitted
        );
        assert!(manager.clear_snapshot(id, "snapshot-key").is_err());
        task.await.unwrap();
    }
}

#[tokio::test]
async fn remote_snapshot_create_socket_loss_retains_unknown_submission() {
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
        tokio::spawn(async move { active.confirm_snapshot(id, "snapshot-key", true).await });
    received.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(submission
        .await
        .unwrap()
        .unwrap_err()
        .contains("interrupted"));
    let state = manager.0.lock().unwrap();
    let intent = state.snapshot_creation.prepared.as_ref().unwrap();
    assert!(intent.submitted && !intent.inflight && intent.operation.is_none());
    drop(state);
    assert!(manager.clear_snapshot(id, "snapshot-key").is_err());
}
