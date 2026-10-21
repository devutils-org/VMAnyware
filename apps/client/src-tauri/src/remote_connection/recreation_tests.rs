use super::*;
use crate::remote_connection::{
    create::prepare_tests::sequence,
    tests::{fixture, fixture_with_gate},
    Profile,
};
use serde_json::Value;
use std::sync::Arc;
const TOKEN: &str = "fixture-only-recreation-token-0123456789abcdef";
fn source() -> Value {
    json!({"id":"a".repeat(32),"definitionName":"mobile-lab","definitionHash":"b".repeat(64),"definitionRevision":1,"definitionYaml":"fixture retained YAML","status":"deleted"})
}
fn review() -> Value {
    json!({"sourceInstanceId":"a".repeat(32),"sourceDefinitionHash":"b".repeat(64),"sourceDefinitionRevision":1,"definitionYaml":"fixture retained YAML","plan":{"definitionName":"mobile-lab","definitionHash":"b".repeat(64),"planHash":"c".repeat(64),"providerScopeId":"d".repeat(64),"source":{"templateVmid":9000,"templateVersion":"fixture-v1"},"target":{"node":"pve-test","vmid":202,"storage":"local-lvm","bridge":"vmbr0","guestUser":"debian","resources":{"cpu":2,"memoryMiB":2048,"diskGiB":8}},"creationEnabled":true,"sshKeyCount":1,"packages":[],"snapshotAfterProvisioning":false},"restoresPreviousDiskContents":false,"requiresNewInstanceIdentity":true,"recreationEnabled":true,"preflight":{"planHash":"c".repeat(64),"creationEnabled":true,"vmidReserved":false,"checks":CHECKS.iter().enumerate().map(|(index,code)|json!({"code":code,"status":if index<7{"passed"}else{"unknown"},"message":"untrusted server text"})).collect::<Vec<_>>()}})
}
fn manager(endpoint: &str, ca: reqwest::Certificate) -> (Arc<RemoteConnection>, u64) {
    let manager = Arc::new(RemoteConnection::default());
    let id = manager.begin().unwrap();
    manager
        .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
        .unwrap();
    (manager, id)
}
#[tokio::test]
async fn native_recreation_review_reads_fixed_source_and_preflight_retains_then_clears() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", source().to_string()),
        ("200 OK", review().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let result = manager
        .prepare_recreation(id, "review-key".into(), "a".repeat(32), 202)
        .await
        .unwrap();
    assert!(result.recreation_enabled);
    let serialized = serde_json::to_value(result).unwrap();
    assert!(!serialized.to_string().contains("untrusted server text"));
    assert!(manager.clear_recreation(id, "wrong-key").is_err());
    assert!(manager.0.lock().unwrap().recreation.review.is_some());
    manager.clear_recreation(id, "review-key").unwrap();
    assert!(manager.0.lock().unwrap().recreation.review.is_none());
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!(
        "GET /api/v1/environments/{} HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[1].starts_with(&format!(
        "POST /api/v1/environments/{}/recreate-preflight HTTP/1.1",
        "a".repeat(32)
    )));
    assert_eq!(
        serde_json::from_str::<Value>(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap(),
        json!({"targetVmid":202})
    );
    assert!(requests.iter().all(|request| request
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {TOKEN}"))));
}
#[tokio::test]
async fn native_recreation_review_rejects_changed_source_plan_semantics_and_check_sets() {
    for pointer in [
        "/sourceInstanceId",
        "/sourceDefinitionHash",
        "/definitionYaml",
        "/plan/definitionHash",
        "/plan/target/vmid",
        "/preflight/planHash",
        "/preflight/checks/0/code",
        "/preflight/checks/0/status",
        "/restoresPreviousDiskContents",
        "/requiresNewInstanceIdentity",
        "/preflight/vmidReserved",
    ] {
        let mut changed = review();
        let replacement = match pointer {
            "/plan/target/vmid" => json!(203),
            "/restoresPreviousDiskContents" | "/preflight/vmidReserved" => json!(true),
            "/requiresNewInstanceIdentity" => json!(false),
            _ => json!("changed"),
        };
        *changed.pointer_mut(pointer).unwrap() = replacement;
        let (endpoint, ca, task) = sequence(vec![
            ("200 OK", source().to_string()),
            ("200 OK", changed.to_string()),
        ])
        .await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(
            manager
                .prepare_recreation(id, "review-key".into(), "a".repeat(32), 202)
                .await
                .is_err(),
            "{pointer}"
        );
        assert!(manager.0.lock().unwrap().recreation.review.is_none());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_review_preserves_disabled_failed_and_unknown_checks_for_display() {
    let mut value = review();
    value["recreationEnabled"] = json!(false);
    value["plan"]["creationEnabled"] = json!(false);
    value["preflight"]["creationEnabled"] = json!(false);
    value["preflight"]["checks"][6]["status"] = json!("failed");
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", source().to_string()),
        ("200 OK", value.to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let review = manager
        .prepare_recreation(id, "review-key".into(), "a".repeat(32), 202)
        .await
        .unwrap();
    assert!(!review.recreation_enabled);
    assert_eq!(review.preflight.checks[6].status, "failed");
    task.await.unwrap();
}
#[tokio::test]
async fn native_recreation_review_refuses_active_source_invalid_inputs_and_missing_auth_marker() {
    let mut active = source();
    active["status"] = json!("registered");
    for (body, headers) in [
        (active.to_string(), "x-vmanyware-authenticated: bearer\r\n"),
        (source().to_string(), ""),
    ] {
        let (endpoint, ca, task) = fixture("200 OK", &body, headers).await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(manager
            .prepare_recreation(id, "key".into(), "../bad".into(), 202)
            .await
            .is_err());
        assert!(manager
            .prepare_recreation(id, "key".into(), "a".repeat(32), 99)
            .await
            .is_err());
        assert!(manager
            .prepare_recreation(id, "bad key".into(), "a".repeat(32), 202)
            .await
            .is_err());
        assert!(manager
            .prepare_recreation(id, "key".into(), "a".repeat(32), 202)
            .await
            .is_err());
        assert!(manager.0.lock().unwrap().recreation.review.is_none());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_review_discards_response_after_disconnect() {
    let (seen, received) = tokio::sync::oneshot::channel();
    let (release, gate) = tokio::sync::oneshot::channel();
    let (endpoint, ca, task) = fixture_with_gate(
        "200 OK",
        &source().to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, gate)),
    )
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let request_manager = manager.clone();
    let request = tokio::spawn(async move {
        request_manager
            .prepare_recreation(id, "key".into(), "a".repeat(32), 202)
            .await
    });
    received.await.unwrap();
    manager.begin().unwrap();
    release.send(()).unwrap();
    assert!(request.await.unwrap().is_err());
    assert!(manager.0.lock().unwrap().recreation.review.is_none());
    task.await.unwrap();
}

fn queued() -> Value {
    json!({"id":"e".repeat(32),"kind":"vm_provision","instanceId":"f".repeat(32),"status":"queued","result":null})
}
fn succeeded() -> Value {
    let mut value = queued();
    value["status"] = json!("succeeded");
    value["result"] = json!({"instanceId":"f".repeat(32),"providerScopeId":"d".repeat(64),"planHash":"c".repeat(64),"guestReady":true,"guest":{"ipv4":"192.0.2.42","interface":"eth0","cloudInitExitCode":0,"cloudInitClean":true}});
    value
}
fn saved(operation: Value) -> Value {
    json!({"sourceInstanceId":"a".repeat(32),"operation":operation,"plan":review()["plan"],"restoresPreviousDiskContents":false,"requiresNewInstanceIdentity":true})
}
fn seed(manager: &RemoteConnection) {
    manager.0.lock().unwrap().recreation = RecreationState {
        epoch: 1,
        key: Some("fixed-key".into()),
        review: Some(serde_json::from_value(review()).unwrap()),
        prepared_at: Some(Instant::now()),
        ..Default::default()
    };
}
#[tokio::test]
async fn native_recreation_confirmation_uses_retained_inputs_and_reads_scoped_success() {
    let (endpoint, ca, task) = sequence(vec![
        ("202 Accepted", queued().to_string()),
        ("200 OK", saved(succeeded()).to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager);
    assert!(manager
        .confirm_recreation(id, "fixed-key", false)
        .await
        .is_err());
    assert!(manager
        .confirm_recreation(id, "wrong-key", true)
        .await
        .is_err());
    let operation = manager
        .confirm_recreation(id, "fixed-key", true)
        .await
        .unwrap();
    assert_eq!(operation.id(), "e".repeat(32));
    assert!(manager
        .confirm_recreation(id, "fixed-key", true)
        .await
        .is_err());
    assert!(manager.clear_recreation(id, "fixed-key").is_err());
    assert!(manager
        .prepare_recreation(id, "other".into(), "a".repeat(32), 203)
        .await
        .is_err());
    assert!(manager
        .recreation_operation(id, "fixed-key")
        .await
        .unwrap()
        .terminal());
    manager.clear_recreation(id, "fixed-key").unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!(
        "POST /api/v1/environments/{}/recreate HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[0]
        .to_ascii_lowercase()
        .contains("idempotency-key: fixed-key"));
    let value: Value = serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        value,
        json!({"sourceInstanceId":"a".repeat(32),"approval":serde_json::from_value::<Review>(review()).unwrap().plan.approval()})
    );
    assert!(requests[1].starts_with(&format!(
        "GET /api/v1/environments/{}/recreation-operations/{} HTTP/1.1",
        "a".repeat(32),
        "e".repeat(32)
    )));
}
#[tokio::test]
async fn native_recreation_unknown_reply_keeps_exact_key_body_and_allows_only_identical_retry() {
    let (endpoint, ca, task) = sequence(vec![
        ("202 Accepted", "PRIVATE_INVALID_REPLY".into()),
        ("202 Accepted", queued().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager);
    let error = manager
        .confirm_recreation(id, "fixed-key", true)
        .await
        .err()
        .unwrap();
    assert!(!error.contains("PRIVATE_INVALID_REPLY"));
    assert!(manager.clear_recreation(id, "fixed-key").is_err());
    assert!(manager.recreation_operation(id, "fixed-key").await.is_err());
    manager.0.lock().unwrap().recreation.prepared_at =
        Some(Instant::now() - Duration::from_secs(121));
    manager
        .confirm_recreation(id, "fixed-key", true)
        .await
        .unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
}
#[tokio::test]
async fn native_recreation_confirmation_refuses_expired_disabled_and_failed_reviews_without_http() {
    let manager = RemoteConnection::default();
    let id = manager.begin().unwrap();
    manager
        .complete(
            id,
            Profile::new("https://localhost:1", TOKEN, None).unwrap(),
        )
        .unwrap();
    for mode in 0..4 {
        seed(&manager);
        {
            let mut state = manager.0.lock().unwrap();
            match mode {
                0 => state.recreation.prepared_at = Some(Instant::now() - Duration::from_secs(121)),
                1 => state.recreation.review.as_mut().unwrap().recreation_enabled = false,
                2 => {
                    state.recreation.review.as_mut().unwrap().preflight.checks[6].status =
                        "failed".into()
                }
                _ => {
                    state.recreation.review.as_mut().unwrap().preflight.checks[8].status =
                        "failed".into()
                }
            }
        }
        assert!(manager
            .confirm_recreation(id, "fixed-key", true)
            .await
            .is_err());
        assert!(!manager.0.lock().unwrap().recreation.submitted);
    }
}
#[tokio::test]
async fn native_recreation_confirmation_rejects_source_identity_reuse_and_false_success() {
    for mode in 0..3 {
        let mut operation = if mode == 2 { succeeded() } else { queued() };
        match mode {
            0 => operation["instanceId"] = json!("a".repeat(32)),
            1 => operation["instanceId"] = Value::Null,
            _ => operation["result"]["guest"]["cloudInitClean"] = json!(false),
        };
        let (endpoint, ca, task) = sequence(vec![("202 Accepted", operation.to_string())]).await;
        let (manager, id) = manager(&endpoint, ca);
        seed(&manager);
        assert!(manager
            .confirm_recreation(id, "fixed-key", true)
            .await
            .is_err());
        let state = manager.0.lock().unwrap();
        assert!(state.recreation.submitted);
        assert!(state.recreation.operation.is_none());
        drop(state);
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_saved_status_rejects_changed_source_plan_operation_and_instance() {
    for pointer in [
        "/sourceInstanceId",
        "/plan/target/storage",
        "/operation/id",
        "/operation/instanceId",
        "/operation/result/providerScopeId",
        "/operation/result/planHash",
    ] {
        let mut response = saved(succeeded());
        *response.pointer_mut(pointer).unwrap() = json!("changed");
        let (endpoint, ca, task) = sequence(vec![
            ("202 Accepted", queued().to_string()),
            ("200 OK", response.to_string()),
        ])
        .await;
        let (manager, id) = manager(&endpoint, ca);
        seed(&manager);
        manager
            .confirm_recreation(id, "fixed-key", true)
            .await
            .unwrap();
        assert!(
            manager.recreation_operation(id, "fixed-key").await.is_err(),
            "{pointer}"
        );
        assert!(!manager.0.lock().unwrap().recreation.terminal);
        assert!(manager.clear_recreation(id, "fixed-key").is_err());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn native_recreation_disconnect_during_submission_discards_reply_without_replaying() {
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
    seed(&manager);
    let request_manager = manager.clone();
    let request = tokio::spawn(async move {
        request_manager
            .confirm_recreation(id, "fixed-key", true)
            .await
    });
    received.await.unwrap();
    assert!(manager.clear_recreation(id, "fixed-key").is_err());
    assert!(manager
        .confirm_recreation(id, "fixed-key", true)
        .await
        .is_err());
    manager.begin().unwrap();
    release.send(()).unwrap();
    assert!(request.await.unwrap().is_err());
    assert!(manager.0.lock().unwrap().recreation.review.is_none());
    let request = task.await.unwrap();
    assert!(request.starts_with("POST "));
}

#[tokio::test]
async fn native_recreation_cancel_pending_review_prevents_late_state_installation() {
    let (seen, received) = tokio::sync::oneshot::channel();
    let (release, gate) = tokio::sync::oneshot::channel();
    let (endpoint, ca, task) = fixture_with_gate(
        "200 OK",
        &source().to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, gate)),
    )
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let request_manager = manager.clone();
    let request = tokio::spawn(async move {
        request_manager
            .prepare_recreation(id, "key".into(), "a".repeat(32), 202)
            .await
    });
    received.await.unwrap();
    manager.clear_recreation(id, "key").unwrap();
    release.send(()).unwrap();
    assert!(request.await.unwrap().is_err());
    assert!(manager.0.lock().unwrap().recreation.review.is_none());
    task.await.unwrap();
}
