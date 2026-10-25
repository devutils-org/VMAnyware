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
    json!({"instanceId":"a".repeat(32),"operations":[{"id":"e".repeat(32),"snapshotName":"before-work","status":"succeeded","failure":null,"providerTaskKnown":true,"createdAt":"2026-10-20T10:00:00Z"}]})
}
fn recovered() -> Value {
    json!({"operation":{"id":"e".repeat(32),"kind":"vm_snapshot_restore","instanceId":"a".repeat(32),"status":"succeeded","providerTaskKnown":true,"failure":null,"result":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"node":"saved-node","vmid":202,"taskStatus":"succeeded","snapshotName":"before-work","snapshotConfigHash":"d".repeat(64),"observation":{"node":"saved-node","vmid":202,"status":"stopped","qmpStatus":null}}},"target":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"definitionHash":"c".repeat(64),"node":"saved-node","vmid":202},"request":{"snapshotName":"before-work"},"snapshotConfigHash":"d".repeat(64),"submissionStarted":true})
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
async fn remote_restore_recovery_reads_fixed_scoped_authenticated_gets_without_intent() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", history().to_string()),
        ("200 OK", recovered().to_string()),
    ])
    .await;
    let (manager, id) = connected_manager(&endpoint, ca);
    assert_eq!(
        manager
            .restore_history(id, &"a".repeat(32))
            .await
            .unwrap()
            .operations
            .len(),
        1
    );
    let result = manager
        .recover_restore(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["request"]["snapshotName"],
        "before-work"
    );
    {
        let state = manager.0.lock().unwrap();
        assert!(state.prepared.is_none());
    }
    // Recovery never installs a prepared snapshot approval, even after reconnect.
    assert!(manager.snapshot_restore_is_empty());
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!(
        "GET /api/v1/environments/{}/restore-operations HTTP/1.1",
        "a".repeat(32)
    )));
    assert!(requests[1].starts_with(&format!(
        "GET /api/v1/environments/{}/restore-operations/{} HTTP/1.1",
        "a".repeat(32),
        "e".repeat(32)
    )));
    assert!(requests.iter().all(|request| request
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {TOKEN}"))
        && !request.to_ascii_lowercase().contains("idempotency-key")));
}
#[tokio::test]
async fn remote_restore_recovery_accepts_saved_queued_running_and_uncertain_states() {
    for status in ["queued", "running", "uncertain", "failed", "interrupted"] {
        let mut record = recovered();
        record["operation"]["status"] = json!(status);
        record["operation"]["result"] = Value::Null;
        record["operation"]["providerTaskKnown"] = json!(false);
        record["submissionStarted"] = json!(status == "uncertain");
        record["operation"]["failure"] = if ["queued", "running"].contains(&status) {
            Value::Null
        } else {
            json!(match status {
                "failed" => "restore_disabled",
                "interrupted" => "process_interrupted",
                _ => "provider_submission_unconfirmed",
            })
        };
        let (endpoint, ca, task) = fixture(
            "200 OK",
            &record.to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
        )
        .await;
        let (manager, id) = connected_manager(&endpoint, ca);
        let result = manager
            .recover_restore(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap()["operation"]["status"],
            status
        );
        assert!(manager.snapshot_restore_is_empty());
        assert!(task.await.unwrap().starts_with("GET "));
    }
}
#[tokio::test]
async fn remote_restore_recovery_rejects_cross_target_identity_and_inconsistent_evidence() {
    for variant in [
        "operation",
        "instance",
        "scope",
        "request",
        "ram",
        "started",
        "kind",
        "fingerprint",
        "bad-hash",
        "qmp",
        "power-target",
        "task-marker",
    ] {
        let mut record = recovered();
        match variant {
            "operation" => record["operation"]["id"] = json!("f".repeat(32)),
            "instance" => record["target"]["instanceId"] = json!("f".repeat(32)),
            "scope" => record["target"]["providerScopeId"] = json!("f".repeat(64)),
            "request" => record["request"]["snapshotName"] = json!("other-snapshot"),
            "ram" => record["operation"]["result"]["observation"]["status"] = json!("running"),
            "started" => record["submissionStarted"] = json!(false),
            "fingerprint" => record["snapshotConfigHash"] = json!("7".repeat(64)),
            "bad-hash" => record["snapshotConfigHash"] = json!("invalid"),
            "qmp" => record["operation"]["result"]["observation"]["qmpStatus"] = json!("paused"),
            "power-target" => record["operation"]["result"]["observation"]["vmid"] = json!(203),
            "task-marker" => record["operation"]["providerTaskKnown"] = json!(false),
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
                .recover_restore(id, &"a".repeat(32), &"e".repeat(32))
                .await
                .is_err(),
            "{variant}"
        );
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_restore_recovery_rejects_unbounded_duplicate_or_wrong_instance_history() {
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
            "name" => records["operations"][0]["snapshotName"] = json!("current"),
            _ => records["operations"][0]["failure"] = json!("restore_state_unconfirmed"),
        }
        let (endpoint, ca, task) = fixture(
            "200 OK",
            &records.to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
        )
        .await;
        let (manager, id) = connected_manager(&endpoint, ca);
        assert!(
            manager.restore_history(id, &"a".repeat(32)).await.is_err(),
            "{variant}"
        );
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_restore_recovery_discards_reply_after_disconnect_or_reconnect() {
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
                    .recover_restore(id, &"a".repeat(32), &"e".repeat(32))
                    .await
                    .map(|_| ())
            } else {
                active
                    .restore_history(id, &"a".repeat(32))
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
        assert!(manager.snapshot_restore_is_empty());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_restore_recovery_drops_unknown_fields_and_keeps_raw_errors_private() {
    let mut record = recovered();
    record["privateUnexpected"] = json!("private-result-detail");
    record["target"]["configDigest"] = json!("private-target-detail");
    record["operation"]["result"]["observation"]["privateExtra"] = json!("private-nested-detail");
    let (endpoint, ca, task) = fixture(
        "200 OK",
        &record.to_string(),
        "x-vmanyware-authenticated: bearer\r\n",
    )
    .await;
    let (manager, id) = connected_manager(&endpoint, ca);
    let result = manager
        .recover_restore(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    assert!(!serde_json::to_string(&result).unwrap().contains("private-"));
    task.await.unwrap();
    let oversized = "x".repeat(256 * 1024 + 1);
    for (status, body, headers) in [
        (
            "200 OK",
            oversized.as_str(),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
        ("401 Unauthorized", "private-denied", ""),
        (
            "202 Accepted",
            "{}",
            "x-vmanyware-authenticated: bearer\r\n",
        ),
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
            .restore_history(id, &"a".repeat(32))
            .await
            .unwrap_err();
        assert!(!failure.contains("private-") && !failure.contains(TOKEN));
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_restore_recovery_invalid_ids_are_rejected_before_connection_or_request() {
    let manager = RemoteConnection::default();
    for id in ["../other", "invalid", &"A".repeat(32)] {
        assert!(manager
            .restore_history(1, id)
            .await
            .unwrap_err()
            .contains("valid"));
        assert!(manager
            .recover_restore(1, &"a".repeat(32), id)
            .await
            .unwrap_err()
            .contains("valid"));
    }
}

#[tokio::test]
async fn remote_restore_recovery_checks_known_task_states_without_creating_approval() {
    for (status, failure) in [
        ("running", None),
        ("uncertain", Some("restore_state_unconfirmed")),
        ("failed", Some("provider_task_failed")),
    ] {
        let mut detail = recovered();
        detail["operation"]["status"] = json!(status);
        detail["operation"]["failure"] = json!(failure);
        detail["operation"]["result"] = Value::Null;
        let mut rows = history();
        rows["operations"][0]["status"] = json!(status);
        rows["operations"][0]["failure"] = json!(failure);
        let (endpoint, ca, task) = sequence(vec![
            ("200 OK", rows.to_string()),
            ("200 OK", detail.to_string()),
        ])
        .await;
        let (manager, id) = connected_manager(&endpoint, ca);
        manager.restore_history(id, &"a".repeat(32)).await.unwrap();
        manager
            .recover_restore(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .unwrap();
        assert!(manager.snapshot_restore_is_empty());
        assert!(manager.snapshot_creation_is_empty());
        assert!(task
            .await
            .unwrap()
            .iter()
            .all(|request| request.starts_with("GET ")));
    }
}

#[test]
fn remote_restore_recovery_rejects_status_dispatch_and_failure_inconsistency() {
    for variant in [
        "interrupted-started",
        "failed-unknown",
        "failed-interrupted",
        "known-missing-start",
        "unknown-claims-task",
    ] {
        let mut value = recovered();
        value["operation"]["result"] = Value::Null;
        match variant {
            "interrupted-started" => {
                value["operation"]["status"] = json!("interrupted");
                value["operation"]["providerTaskKnown"] = json!(false);
                value["operation"]["failure"] = json!("process_interrupted");
            }
            "failed-unknown" => {
                value["operation"]["status"] = json!("failed");
                value["operation"]["providerTaskKnown"] = json!(false);
                value["operation"]["failure"] = json!("provider_submission_unconfirmed");
            }
            "failed-interrupted" => {
                value["operation"]["status"] = json!("failed");
                value["submissionStarted"] = json!(false);
                value["operation"]["providerTaskKnown"] = json!(false);
                value["operation"]["failure"] = json!("process_interrupted");
            }
            "known-missing-start" => {
                value["operation"]["status"] = json!("running");
                value["submissionStarted"] = json!(false);
            }
            _ => {
                value["operation"]["status"] = json!("uncertain");
                value["operation"]["providerTaskKnown"] = json!(false);
                value["operation"]["failure"] = json!("restore_state_unconfirmed");
            }
        }
        let record: Recovered = serde_json::from_value(value).unwrap();
        assert!(!record.valid(&"a".repeat(32), &"e".repeat(32)), "{variant}");
    }
    for (status, known, failure) in [
        ("failed", false, "provider_submission_unconfirmed"),
        ("interrupted", true, "process_interrupted"),
        ("uncertain", false, "restore_state_unconfirmed"),
    ] {
        let mut value = history();
        value["operations"][0]["status"] = json!(status);
        value["operations"][0]["providerTaskKnown"] = json!(known);
        value["operations"][0]["failure"] = json!(failure);
        let rows: History = serde_json::from_value(value).unwrap();
        assert!(!rows.valid(&"a".repeat(32)), "{status}");
    }
}
