use super::*;
use crate::remote_connection::{
    create::prepare_tests::sequence,
    tests::{fixture, fixture_with_gate},
    Profile,
};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-recovery-token-0123456789abcdef";
fn history() -> Value {
    json!({"instanceId":"a".repeat(32),"operations":[{"id":"e".repeat(32),"name":"before-work","status":"succeeded","failure":null,"providerTaskKnown":true,"createdAt":"2026-10-20T10:00:00Z"}]})
}
fn recovered() -> Value {
    json!({"operation":{"id":"e".repeat(32),"kind":"vm_snapshot_create","instanceId":"a".repeat(32),"status":"succeeded","providerTaskKnown":true,"failure":null,"result":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"node":"saved-node","vmid":202,"taskStatus":"succeeded","snapshot":{"name":"before-work","description":"Saved request","createdAt":1700000000,"includesRam":false,"parent":null,"providerState":null}}},"target":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"definitionHash":"c".repeat(64),"node":"saved-node","vmid":202},"request":{"name":"before-work","description":"Saved request"},"submissionStarted":true})
}
fn connected_manager(endpoint: &str, ca: reqwest::Certificate) -> (Arc<RemoteConnection>, u64) {
    let manager = Arc::new(RemoteConnection::default());
    let id = manager.begin().unwrap();
    manager
        .complete(id, Profile::new(endpoint, TOKEN, Some(ca)).unwrap())
        .unwrap();
    (manager, id)
}
#[tokio::test]
async fn remote_snapshot_recovery_reads_fixed_scoped_authenticated_gets_without_intent() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", history().to_string()),
        ("200 OK", recovered().to_string()),
    ])
    .await;
    let (manager, id) = connected_manager(&endpoint, ca);
    assert_eq!(
        manager
            .snapshot_history(id, &"a".repeat(32))
            .await
            .unwrap()
            .operations
            .len(),
        1
    );
    let result = manager
        .recover_snapshot(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["request"]["name"],
        "before-work"
    );
    let state = manager.0.lock().unwrap();
    assert!(state.prepared.is_none());
    drop(state);
    // Recovery never installs a prepared snapshot approval, even after reconnect.
    assert!(manager.snapshot_creation_is_empty());
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!(
        "GET /api/v1/environments/{}/snapshot-operations HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[1].starts_with(&format!(
        "GET /api/v1/environments/{}/snapshot-operations/{} HTTP/1.1",
        "a".repeat(32),
        "e".repeat(32)
    )));
    assert!(requests.iter().all(|request| request
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {TOKEN}"))
        && !request.to_ascii_lowercase().contains("idempotency-key")));
}
#[tokio::test]
async fn remote_snapshot_recovery_accepts_saved_queued_running_and_uncertain_states() {
    for status in ["queued", "running", "uncertain", "failed", "interrupted"] {
        let mut record = recovered();
        record["operation"]["status"] = json!(status);
        record["operation"]["result"] = Value::Null;
        record["operation"]["providerTaskKnown"] = json!(false);
        record["submissionStarted"] = json!(status == "uncertain");
        record["operation"]["failure"] = if ["queued", "running"].contains(&status) {
            Value::Null
        } else {
            json!("provider_submission_unconfirmed")
        };
        let (endpoint, ca, task) = fixture(
            "200 OK",
            &record.to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
        )
        .await;
        let (manager, id) = connected_manager(&endpoint, ca);
        let result = manager
            .recover_snapshot(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap()["operation"]["status"],
            status
        );
        assert!(manager.snapshot_creation_is_empty());
        assert!(task.await.unwrap().starts_with("GET "));
    }
}
#[tokio::test]
async fn remote_snapshot_recovery_rejects_cross_target_identity_and_inconsistent_evidence() {
    for variant in [
        "operation",
        "instance",
        "scope",
        "request",
        "ram",
        "started",
        "kind",
    ] {
        let mut record = recovered();
        match variant {
            "operation" => record["operation"]["id"] = json!("f".repeat(32)),
            "instance" => record["target"]["instanceId"] = json!("f".repeat(32)),
            "scope" => record["target"]["providerScopeId"] = json!("f".repeat(64)),
            "request" => record["request"]["name"] = json!("other-snapshot"),
            "ram" => record["operation"]["result"]["snapshot"]["includesRam"] = json!(true),
            "started" => record["submissionStarted"] = json!(false),
            _ => record["operation"]["kind"] = json!("vm_provision"),
        }
        let (endpoint, ca, task) = fixture(
            "200 OK",
            &record.to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
        )
        .await;
        let (manager, id) = connected_manager(&endpoint, ca);
        assert!(
            manager
                .recover_snapshot(id, &"a".repeat(32), &"e".repeat(32))
                .await
                .is_err(),
            "{variant}"
        );
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_snapshot_recovery_rejects_unbounded_duplicate_or_wrong_instance_history() {
    for variant in ["scope", "duplicate", "count", "name", "status"] {
        let mut records = history();
        match variant {
            "scope" => records["instanceId"] = json!("f".repeat(32)),
            "duplicate" => {
                let row = records["operations"][0].clone();
                records["operations"].as_array_mut().unwrap().push(row);
            }
            "count" => {
                records["operations"] = json!((0..101)
                    .map(|index| {
                        let mut row = history()["operations"][0].clone();
                        row["id"] = json!(format!("{index:032x}"));
                        row
                    })
                    .collect::<Vec<_>>())
            }
            "name" => records["operations"][0]["name"] = json!("current"),
            _ => records["operations"][0]["failure"] = json!("snapshot_state_unconfirmed"),
        }
        let (endpoint, ca, task) = fixture(
            "200 OK",
            &records.to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
        )
        .await;
        let (manager, id) = connected_manager(&endpoint, ca);
        assert!(
            manager.snapshot_history(id, &"a".repeat(32)).await.is_err(),
            "{variant}"
        );
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_snapshot_recovery_discards_reply_after_disconnect_or_reconnect() {
    for detail in [false, true] {
        let (seen, received) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let body = if detail { recovered() } else { history() };
        let (endpoint, ca, task) = fixture_with_gate(
            "200 OK",
            &body.to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
            Some((seen, gate)),
        )
        .await;
        let next_profile = Profile::new(&endpoint, TOKEN, Some(ca.clone())).unwrap();
        let (manager, id) = connected_manager(&endpoint, ca);
        let active = manager.clone();
        let read = tokio::spawn(async move {
            if detail {
                active
                    .recover_snapshot(id, &"a".repeat(32), &"e".repeat(32))
                    .await
                    .map(|_| ())
            } else {
                active
                    .snapshot_history(id, &"a".repeat(32))
                    .await
                    .map(|_| ())
            }
        });
        received.await.unwrap();
        let next_id = manager.begin().unwrap();
        if detail {
            manager.complete(next_id, next_profile).unwrap();
        }
        release.send(()).unwrap();
        assert!(read.await.unwrap().unwrap_err().contains("changed"));
        assert!(manager.snapshot_creation_is_empty());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_snapshot_recovery_drops_unknown_fields_and_keeps_raw_errors_private() {
    let mut record = recovered();
    record["privateUnexpected"] = json!("private-result-detail");
    record["target"]["configDigest"] = json!("private-target-detail");
    let (endpoint, ca, task) = fixture(
        "200 OK",
        &record.to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
    )
    .await;
    let (manager, id) = connected_manager(&endpoint, ca);
    let result = manager
        .recover_snapshot(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    assert!(!serde_json::to_string(&result).unwrap().contains("private-"));
    task.await.unwrap();
    for (status, body, headers) in [
        (
            "503 Service Unavailable",
            "private-provider-detail",
            "x-vmanyware-authenticated: bearer\r\n",
        ),
        (
            "200 OK",
            "private-invalid-json",
            "x-vmanyware-authenticated: bearer\r\n",
        ),
        ("200 OK", "{}", ""),
    ] {
        let (endpoint, ca, task) = fixture(status, body, headers).await;
        let (manager, id) = connected_manager(&endpoint, ca);
        let failure = manager
            .snapshot_history(id, &"a".repeat(32))
            .await
            .unwrap_err();
        assert!(!failure.contains("private-") && !failure.contains(TOKEN));
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_snapshot_recovery_invalid_ids_are_rejected_before_connection_or_request() {
    let manager = RemoteConnection::default();
    for id in ["../other", "invalid", &"A".repeat(32)] {
        assert!(manager
            .snapshot_history(1, id)
            .await
            .unwrap_err()
            .contains("valid"));
        assert!(manager
            .recover_snapshot(1, &"a".repeat(32), id)
            .await
            .unwrap_err()
            .contains("valid"));
    }
}
