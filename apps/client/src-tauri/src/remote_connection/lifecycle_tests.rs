use super::*;
use crate::remote_connection::{
    create::prepare_tests::sequence, tests::fixture_with_gate, Profile,
};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-lifecycle-token-0123456789abcdef";
fn target() -> Target {
    Target {
        instance_id: "a".repeat(32),
        provider_scope_id: "b".repeat(64),
        node: "pve-test".into(),
        vmid: 202,
    }
}
fn instance() -> Value {
    json!({"id":"a".repeat(32),"definitionHash":"c".repeat(64),"status":"registered","providerReference":{"provider":"proxmox","scopeId":"b".repeat(64),"node":"pve-test","vmid":202}})
}
fn report() -> Value {
    json!({"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"definitionHash":"c".repeat(64),"expectedMarker":format!("vmanyware-{}","a".repeat(32)),"configDigest":"d".repeat(40),"observation":{"node":"pve-test","vmid":202,"status":"stopped","qmpStatus":null},"checks":CHECKS.iter().map(|code|json!({"code":code,"status":"passed","message":"PRIVATE_SERVER_TEXT"})).collect::<Vec<_>>(),"startPreconditions":"passed","shutdownPreconditions":"failed","executionEnabled":true})
}
fn manager(endpoint: &str, ca: reqwest::Certificate) -> (Arc<RemoteConnection>, u64) {
    let state = Arc::new(RemoteConnection::default());
    let id = state.begin().unwrap();
    state
        .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
        .unwrap();
    (state, id)
}
#[tokio::test]
async fn remote_lifecycle_review_retains_only_fixed_reads_and_clears_exact_key() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", instance().to_string()),
        ("200 OK", report().to_string()),
    ])
    .await;
    let (state, id) = manager(&endpoint, ca);
    let review = state
        .prepare_lifecycle(id, "review-key".into(), target(), Action::Start)
        .await
        .unwrap();
    assert_eq!(review.preflight.start_preconditions, Status::Passed);
    assert!(!serde_json::to_string(&review)
        .unwrap()
        .contains("PRIVATE_SERVER_TEXT"));
    assert!(state.0.lock().unwrap().lifecycle.prepared_at.is_some());
    assert!(state.clear_lifecycle(id, "wrong-key").is_err());
    assert!(state.0.lock().unwrap().lifecycle.review.is_some());
    state.clear_lifecycle(id, "review-key").unwrap();
    assert!(state.0.lock().unwrap().lifecycle.review.is_none());
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!("GET /api/v1/environments/{} ", "a".repeat(32))));
    assert!(requests[1].starts_with(&format!(
        "GET /api/v1/environments/{}/lifecycle-preflight ",
        "a".repeat(32)
    )));
    assert!(requests
        .iter()
        .all(|r| !r.to_lowercase().contains("idempotency-key:")));
}
#[tokio::test]
async fn remote_lifecycle_review_rejects_unrelated_or_inconsistent_preflight() {
    for field in [
        "instanceId",
        "providerScopeId",
        "definitionHash",
        "expectedMarker",
        "configDigest",
        "checks",
        "observation",
        "startPreconditions",
        "shutdownPreconditions",
    ] {
        let mut value = report();
        match field {
            "checks" => {
                value[field][1] = value[field][0].clone();
            }
            "observation" => value[field]["vmid"] = json!(203),
            "startPreconditions" => value[field] = json!("failed"),
            "shutdownPreconditions" => value[field] = json!("passed"),
            _ => value[field] = json!("unrelated"),
        }
        let (endpoint, ca, task) = sequence(vec![
            ("200 OK", instance().to_string()),
            ("200 OK", value.to_string()),
        ])
        .await;
        let (state, id) = manager(&endpoint, ca);
        assert!(state
            .prepare_lifecycle(id, "review-key".into(), target(), Action::Start)
            .await
            .is_err());
        assert!(state.0.lock().unwrap().lifecycle.review.is_none());
        assert!(!state.0.lock().unwrap().lifecycle.reading);
        assert_eq!(task.await.unwrap().len(), 2);
    }
}
#[tokio::test]
async fn remote_lifecycle_review_rejects_changed_or_deleted_instance_before_preflight() {
    for field in ["id", "status", "definitionHash", "providerReference"] {
        let mut value = instance();
        if field == "providerReference" {
            value[field]["scopeId"] = json!("e".repeat(64));
        } else {
            value[field] = json!("deleted");
        }
        let (endpoint, ca, task) = sequence(vec![("200 OK", value.to_string())]).await;
        let (state, id) = manager(&endpoint, ca);
        assert!(state
            .prepare_lifecycle(id, "review-key".into(), target(), Action::Shutdown)
            .await
            .is_err());
        assert_eq!(task.await.unwrap().len(), 1);
    }
}
#[tokio::test]
async fn remote_lifecycle_review_keeps_failed_unknown_and_disabled_states_as_review_not_consent() {
    let mut value = report();
    value["checks"][0]["status"] = json!("unknown");
    value["startPreconditions"] = json!("unknown");
    value["executionEnabled"] = json!(false);
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", instance().to_string()),
        ("200 OK", value.to_string()),
    ])
    .await;
    let (state, id) = manager(&endpoint, ca);
    let review = state
        .prepare_lifecycle(id, "review-key".into(), target(), Action::Shutdown)
        .await
        .unwrap();
    assert_eq!(review.preflight.start_preconditions, Status::Unknown);
    assert_eq!(review.preflight.shutdown_preconditions, Status::Failed);
    assert!(!review.preflight.execution_enabled);
    assert_eq!(task.await.unwrap().len(), 2);
    state.begin().unwrap();
    assert!(state.0.lock().unwrap().lifecycle.review.is_none());
}
#[tokio::test]
async fn remote_lifecycle_review_clear_or_disconnect_discards_inflight_response() {
    for disconnect in [false, true] {
        let (seen, observed) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let (endpoint, ca, server) = fixture_with_gate(
            "200 OK",
            &instance().to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
            Some((seen, wait)),
        )
        .await;
        let (state, id) = manager(&endpoint, ca);
        let worker = state.clone();
        let task = tokio::spawn(async move {
            worker
                .prepare_lifecycle(id, "review-key".into(), target(), Action::Start)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(3), observed)
            .await
            .unwrap()
            .unwrap();
        assert!(state
            .prepare_lifecycle(id, "other-key".into(), target(), Action::Start)
            .await
            .is_err());
        if disconnect {
            state.begin().unwrap();
        } else {
            state.clear_lifecycle(id, "review-key").unwrap();
        }
        release.send(()).unwrap();
        assert!(task.await.unwrap().is_err());
        assert!(state.0.lock().unwrap().lifecycle.review.is_none());
        assert!(server.await.unwrap().starts_with("GET "));
    }
}

fn retained(state: &RemoteConnection, action: Action) {
    let mut preflight: Preflight = serde_json::from_value(report()).unwrap();
    if action == Action::Shutdown {
        preflight.observation.status = "running".into();
        preflight.observation.qmp_status = Some("running".into());
        preflight.start_preconditions = Status::Failed;
        preflight.shutdown_preconditions = Status::Passed;
    }
    state.0.lock().unwrap().lifecycle = LifecycleState {
        key: Some("review-key".into()),
        review: Some(Review {
            target: target(),
            action,
            preflight,
        }),
        prepared_at: Some(Instant::now()),
        ..Default::default()
    };
}
fn operation(action: Action, status: &str) -> Value {
    let success = status == "succeeded";
    json!({"id":"e".repeat(32),"kind":if action==Action::Start{"vm_start"}else{"vm_shutdown"},"instanceId":"a".repeat(32),"status":status,"providerTaskKnown":success,"result":if success {json!({"action":action,"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"taskStatus":"succeeded","observation":{"node":"pve-test","vmid":202,"status":if action==Action::Start{"running"}else{"stopped"},"qmpStatus":null}})}else{Value::Null},"failure":null,"createdAt":"2026-10-21T00:00:00Z","updatedAt":"2026-10-21T00:00:01Z"})
}
#[tokio::test]
async fn remote_lifecycle_confirm_retries_identical_unknown_request_then_only_reads_known_operation(
) {
    for action in [Action::Start, Action::Shutdown] {
        let (endpoint, ca, server) = sequence(vec![
            ("INVALID", String::new()),
            ("202 Accepted", operation(action, "queued").to_string()),
            ("200 OK", operation(action, "queued").to_string()),
            ("200 OK", operation(action, "succeeded").to_string()),
        ])
        .await;
        let (state, id) = manager(&endpoint, ca);
        retained(&state, action);
        assert!(state
            .confirm_lifecycle(id, "review-key", true)
            .await
            .is_err());
        {
            let inner = state.0.lock().unwrap();
            assert!(inner.lifecycle.submitted && !inner.lifecycle.reading);
            assert!(inner.lifecycle.operation.is_none());
        }
        assert!(state.clear_lifecycle(id, "review-key").is_err());
        assert!(state
            .prepare_lifecycle(id, "replacement".into(), target(), action)
            .await
            .is_err());
        assert!(state.lifecycle_operation(id, "review-key").await.is_err());
        // Expiry prevents a new submission, but must not prevent identical unknown-outcome recovery.
        state.0.lock().unwrap().lifecycle.prepared_at =
            Some(Instant::now() - Duration::from_secs(121));
        assert_eq!(
            state
                .confirm_lifecycle(id, "review-key", true)
                .await
                .unwrap()
                .id,
            "e".repeat(32)
        );
        assert!(state.clear_lifecycle(id, "review-key").is_err());
        assert_eq!(
            state
                .confirm_lifecycle(id, "review-key", true)
                .await
                .unwrap()
                .status,
            OperationStatus::Queued
        );
        assert_eq!(
            state
                .lifecycle_operation(id, "review-key")
                .await
                .unwrap()
                .status,
            OperationStatus::Succeeded
        );
        state.clear_lifecycle(id, "review-key").unwrap();
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        let suffix = if action == Action::Start {
            "start"
        } else {
            "shutdown"
        };
        assert!(requests[..2].iter().all(|r| r.starts_with(&format!(
            "POST /api/v1/environments/{}/{suffix} ",
            "a".repeat(32)
        )) && r
            .to_lowercase()
            .contains("idempotency-key: review-key")));
        let first = requests[0].split_once("\r\n\r\n").unwrap().1;
        assert_eq!(first, requests[1].split_once("\r\n\r\n").unwrap().1);
        let approval: LifecycleApproval = serde_json::from_str(first).unwrap();
        assert!(approval.valid());
        assert_eq!(
            approval.action,
            if action == Action::Start {
                LifecycleAction::Start
            } else {
                LifecycleAction::Shutdown
            }
        );
        assert!(requests[2].starts_with(&format!("GET /api/v1/operations/{} ", "e".repeat(32))));
        assert!(!requests[2].to_lowercase().contains("idempotency-key:"));
        assert!(requests[3].starts_with(&format!("GET /api/v1/operations/{} ", "e".repeat(32))));
        assert!(!requests[3].to_lowercase().contains("idempotency-key:"));
    }
}
#[tokio::test]
async fn remote_lifecycle_confirm_refuses_unconfirmed_stale_blocked_disabled_or_wrong_key() {
    let state = RemoteConnection::default();
    let id = state.begin().unwrap();
    state
        .complete(
            id,
            Profile::new("https://fixture.invalid", TOKEN, None).unwrap(),
        )
        .unwrap();
    for variant in 0..5 {
        retained(&state, Action::Start);
        {
            let mut inner = state.0.lock().unwrap();
            match variant {
                1 => inner.lifecycle.prepared_at = Some(Instant::now() - Duration::from_secs(121)),
                2 => {
                    inner
                        .lifecycle
                        .review
                        .as_mut()
                        .unwrap()
                        .preflight
                        .start_preconditions = Status::Failed
                }
                3 => {
                    inner
                        .lifecycle
                        .review
                        .as_mut()
                        .unwrap()
                        .preflight
                        .execution_enabled = false
                }
                _ => {}
            }
        }
        assert!(state
            .confirm_lifecycle(
                id,
                if variant == 4 {
                    "wrong-key"
                } else {
                    "review-key"
                },
                variant != 0
            )
            .await
            .is_err());
        assert!(!state.0.lock().unwrap().lifecycle.submitted);
    }
}
#[tokio::test]
async fn remote_lifecycle_operation_rejects_unrelated_id_scope_result_or_impossible_queue() {
    for variant in 0..4 {
        let mut op = operation(
            Action::Start,
            if variant == 3 { "queued" } else { "succeeded" },
        );
        match variant {
            0 => op["id"] = json!("f".repeat(32)),
            1 => op["result"]["providerScopeId"] = json!("f".repeat(64)),
            2 => op["result"]["observation"]["vmid"] = json!(203),
            _ => op["providerTaskKnown"] = json!(true),
        }
        let (endpoint, ca, server) = sequence(vec![("200 OK", op.to_string())]).await;
        let (state, id) = manager(&endpoint, ca);
        retained(&state, Action::Start);
        {
            let mut inner = state.0.lock().unwrap();
            inner.lifecycle.submitted = true;
            inner.lifecycle.operation = Some("e".repeat(32));
        }
        assert!(state.lifecycle_operation(id, "review-key").await.is_err());
        assert!(!state.0.lock().unwrap().lifecycle.terminal);
        assert!(!state.0.lock().unwrap().lifecycle.reading);
        assert_eq!(server.await.unwrap().len(), 1);
    }
}
#[tokio::test]
async fn remote_lifecycle_submission_overlap_and_disconnect_never_restore_intent() {
    let (seen, observed) = tokio::sync::oneshot::channel();
    let (release, wait) = tokio::sync::oneshot::channel();
    let (endpoint, ca, server) = fixture_with_gate(
        "202 Accepted",
        &operation(Action::Start, "queued").to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
        Some((seen, wait)),
    )
    .await;
    let (state, id) = manager(&endpoint, ca);
    retained(&state, Action::Start);
    let worker = state.clone();
    let task = tokio::spawn(async move { worker.confirm_lifecycle(id, "review-key", true).await });
    tokio::time::timeout(Duration::from_secs(3), observed)
        .await
        .unwrap()
        .unwrap();
    assert!(state
        .confirm_lifecycle(id, "review-key", true)
        .await
        .is_err());
    assert!(state.clear_lifecycle(id, "review-key").is_err());
    state.begin().unwrap();
    release.send(()).unwrap();
    assert!(task.await.unwrap().is_err());
    assert!(state.0.lock().unwrap().lifecycle.review.is_none());
    assert!(!state.0.lock().unwrap().lifecycle.submitted);
    assert!(server.await.unwrap().starts_with("POST "));
}
