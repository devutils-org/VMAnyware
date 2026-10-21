use super::*;
use crate::remote_connection::{
    create::prepare_tests::sequence,
    tests::{fixture, fixture_with_gate},
    Profile,
};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-deletion-recovery-0123456789abcdef";
fn hash() -> &'static str {
    include_str!("../../../../../fixtures/deletion-approval.sha256").trim()
}
fn history() -> Value {
    json!({"instanceId":"a".repeat(32),"operations":[{"id":"e".repeat(32),"status":"succeeded","failure":null,"providerTaskKnown":true,"approvalHash":hash(),"createdAt":"2026-10-21T10:00:00Z"}]})
}
fn recovered() -> Value {
    let approval: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/deletion-approval.json"
    ))
    .unwrap();
    json!({"operation":{"id":"e".repeat(32),"kind":"vm_delete","instanceId":approval["instanceId"],"status":"succeeded","providerTaskKnown":true,"failure":null,"result":{"instanceId":approval["instanceId"],"providerScopeId":approval["providerScopeId"],"node":"pve-test","vmid":202,"approvalHash":hash(),"taskStatus":"succeeded","vmAbsenceVerified":true,"volumesAbsenceVerified":true,"definitionRetained":true,"associationRetired":true}},"target":{"instanceId":approval["instanceId"],"providerScopeId":approval["providerScopeId"],"definitionHash":approval["definitionHash"],"node":"pve-test","vmid":202},"plan":approval["plan"],"configDigest":approval["configDigest"],"approvalHash":hash(),"submissionStarted":true})
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
async fn remote_deletion_recovery_reads_fixed_authenticated_gets_without_preparing_consent() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", history().to_string()),
        ("200 OK", recovered().to_string()),
    ])
    .await;
    let (manager, id) = connected_manager(&endpoint, ca);
    assert_eq!(
        manager
            .deletion_history(id, &"a".repeat(32))
            .await
            .unwrap()
            .operations
            .len(),
        1
    );
    let saved = manager
        .recover_deletion(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    assert_eq!(saved.approval_hash, hash());
    assert!(manager.deletion_is_empty());
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with(&format!(
        "GET /api/v1/environments/{}/deletion-operations ",
        "a".repeat(32)
    )));
    assert!(requests[1].starts_with(&format!(
        "GET /api/v1/environments/{}/deletion-operations/{} ",
        "a".repeat(32),
        "e".repeat(32)
    )));
    assert!(requests.iter().all(|r| r
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {TOKEN}\r\n"))));
}
#[test]
fn remote_deletion_recovery_checks_entire_plan_hash_target_and_absence_evidence() {
    for variant in [
        "instance",
        "operation",
        "kind",
        "scope",
        "node",
        "digest",
        "hash",
        "volume",
        "preserved",
        "effect",
        "snapshot",
        "dispatch",
        "absence",
        "retirement",
        "task",
        "result-hash",
    ] {
        let mut value = recovered();
        match variant {
            "instance" => value["target"]["instanceId"] = json!("f".repeat(32)),
            "operation" => value["operation"]["id"] = json!("f".repeat(32)),
            "kind" => value["operation"]["kind"] = json!("vm_snapshot_restore"),
            "scope" => value["target"]["providerScopeId"] = json!("f".repeat(64)),
            "node" => value["target"]["node"] = json!("other-node"),
            "digest" => value["configDigest"] = json!("f".repeat(40)),
            "hash" => value["approvalHash"] = json!("f".repeat(64)),
            "volume" => value["plan"]["volumes"][0]["ownerVerified"] = json!(false),
            "preserved" => value["plan"]["preservedMedia"] = json!(["local:iso/changed.iso"]),
            "effect" => value["plan"]["effects"]["purgeJobs"] = json!(true),
            "snapshot" => value["plan"]["snapshots"] = json!(["other-work"]),
            "dispatch" => value["submissionStarted"] = json!(false),
            "absence" => value["operation"]["result"]["volumesAbsenceVerified"] = json!(false),
            "retirement" => value["operation"]["result"]["associationRetired"] = json!(false),
            "task" => value["operation"]["providerTaskKnown"] = json!(false),
            _ => value["operation"]["result"]["approvalHash"] = json!("f".repeat(64)),
        }
        assert!(
            serde_json::from_value::<Recovered>(value)
                .map_or(true, |saved| !saved.valid(&"a".repeat(32), &"e".repeat(32))),
            "{variant}"
        );
    }
}
#[test]
fn remote_deletion_recovery_preserves_partial_failure_and_dispatch_consistency() {
    for (status, known, started, failure, accepted) in [
        ("queued", false, false, None, true),
        ("queued", false, true, None, false),
        ("running", false, false, None, true),
        ("running", true, true, None, true),
        ("running", true, false, None, false),
        (
            "uncertain",
            false,
            true,
            Some("provider_submission_unconfirmed"),
            true,
        ),
        ("uncertain", true, true, Some("provider_task_failed"), true),
        (
            "uncertain",
            false,
            true,
            Some("provider_task_failed"),
            false,
        ),
        (
            "uncertain",
            true,
            true,
            Some("deletion_state_unconfirmed"),
            true,
        ),
        ("failed", true, true, Some("provider_task_failed"), false),
        ("failed", false, true, Some("deletion_disabled"), false),
        ("failed", false, true, Some("provider_forbidden"), true),
        (
            "interrupted",
            false,
            false,
            Some("process_interrupted"),
            true,
        ),
    ] {
        let mut value = recovered();
        value["operation"]["status"] = json!(status);
        value["operation"]["providerTaskKnown"] = json!(known);
        value["operation"]["result"] = Value::Null;
        value["operation"]["failure"] = json!(failure);
        value["submissionStarted"] = json!(started);
        let saved: Recovered = serde_json::from_value(value).unwrap();
        assert_eq!(
            saved.valid(&"a".repeat(32), &"e".repeat(32)),
            accepted,
            "{status}/{failure:?}/{started}"
        );
    }
}
#[tokio::test]
async fn remote_deletion_recovery_refuses_mismatched_or_unbounded_history() {
    for variant in [
        "scope",
        "duplicate",
        "oversize",
        "hash",
        "known-failed",
        "wrong-failure",
        "timestamp",
    ] {
        let mut value = history();
        match variant {
            "scope" => value["instanceId"] = json!("f".repeat(32)),
            "duplicate" => {
                value["operations"] = json!([
                    value["operations"][0].clone(),
                    value["operations"][0].clone()
                ])
            }
            "oversize" => value["operations"] = json!(vec![value["operations"][0].clone(); 101]),
            "hash" => value["operations"][0]["approvalHash"] = json!("wrong-hash"),
            "known-failed" => {
                value["operations"][0]["status"] = json!("failed");
                value["operations"][0]["failure"] = json!("provider_task_failed");
            }
            "wrong-failure" => {
                value["operations"][0]["status"] = json!("queued");
                value["operations"][0]["failure"] = json!("deletion_disabled");
            }
            _ => value["operations"][0]["createdAt"] = json!("private\nvalue"),
        }
        let (endpoint, ca, task) = sequence(vec![("200 OK", value.to_string())]).await;
        let (manager, id) = connected_manager(&endpoint, ca);
        assert!(
            manager.deletion_history(id, &"a".repeat(32)).await.is_err(),
            "{variant}"
        );
        assert!(manager.deletion_is_empty());
        assert_eq!(task.await.unwrap().len(), 1);
    }
}
#[tokio::test]
async fn remote_deletion_recovery_disconnect_or_reconnect_discards_old_reads() {
    for history_read in [false, true] {
        let (seen, received) = tokio::sync::oneshot::channel();
        let (release, gate) = tokio::sync::oneshot::channel();
        let payload = if history_read { history() } else { recovered() };
        let (endpoint, ca, task) = fixture_with_gate(
            "200 OK",
            &payload.to_string(),
            "x-vmanyware-authenticated: bearer\r\n",
            Some((seen, gate)),
        )
        .await;
        let (manager, id) = connected_manager(&endpoint, ca.clone());
        let active = manager.clone();
        let read = tokio::spawn(async move {
            if history_read {
                active
                    .deletion_history(id, &"a".repeat(32))
                    .await
                    .map(|_| ())
            } else {
                active
                    .recover_deletion(id, &"a".repeat(32), &"e".repeat(32))
                    .await
                    .map(|_| ())
            }
        });
        received.await.unwrap();
        let fresh = manager.begin().unwrap();
        if history_read {
            manager
                .complete(fresh, Profile::new(&endpoint, TOKEN, Some(ca)).unwrap())
                .unwrap();
        }
        release.send(()).unwrap();
        assert!(read.await.unwrap().is_err());
        assert!(manager.deletion_is_empty());
        assert!(task.await.unwrap().starts_with("GET "));
    }
}
#[tokio::test]
async fn remote_deletion_recovery_drops_extraneous_fields_and_keeps_errors_private() {
    let mut value = recovered();
    value["confirmed"] = json!(true);
    value["requestKey"] = json!("private-key");
    value["providerUpid"] = json!("private-upid");
    let (endpoint, ca, task) = sequence(vec![("200 OK", value.to_string())]).await;
    let (manager, id) = connected_manager(&endpoint, ca);
    let saved = manager
        .recover_deletion(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    let encoded = serde_json::to_string(&saved).unwrap();
    assert!(
        !encoded.contains("private-key")
            && !encoded.contains("private-upid")
            && !encoded.contains("confirmed")
    );
    assert!(manager.deletion_is_empty());
    task.await.unwrap();
    for (status, payload, headers) in [
        (
            "403 Forbidden",
            "private-detail".into(),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
        ("200 OK", recovered().to_string(), ""),
        (
            "200 OK",
            "x".repeat(256 * 1024 + 1),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
    ] {
        let (endpoint, ca, task) = fixture(status, &payload, headers).await;
        let (manager, id) = connected_manager(&endpoint, ca);
        let error = manager
            .recover_deletion(id, &"a".repeat(32), &"e".repeat(32))
            .await
            .unwrap_err();
        assert!(
            !error.contains("private-detail")
                && !error.contains(TOKEN)
                && !error.contains("request key")
        );
        assert!(manager.deletion_is_empty());
        task.await.unwrap();
    }
}
#[tokio::test]
async fn remote_deletion_recovery_invalid_ids_fail_before_profile_or_network() {
    let manager = RemoteConnection::default();
    assert!(manager
        .deletion_history(0, "../invalid")
        .await
        .unwrap_err()
        .contains("valid retained"));
    for (instance, operation) in [
        ("../invalid".into(), "e".repeat(32)),
        ("a".repeat(32), "../invalid".into()),
    ] {
        assert!(manager
            .recover_deletion(0, &instance, &operation)
            .await
            .unwrap_err()
            .contains("valid instance"));
    }
    assert!(manager.deletion_is_empty());
}

#[tokio::test]
async fn remote_deletion_recovery_reads_empty_and_partial_history_without_live_consent() {
    let empty = json!({"instanceId":"a".repeat(32),"operations":[]});
    let mut partial = history();
    partial["operations"][0]["status"] = json!("uncertain");
    partial["operations"][0]["failure"] = json!("provider_task_failed");
    let mut detail = recovered();
    detail["operation"]["status"] = json!("uncertain");
    detail["operation"]["result"] = Value::Null;
    detail["operation"]["failure"] = json!("provider_task_failed");
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", empty.to_string()),
        ("200 OK", partial.to_string()),
        ("200 OK", detail.to_string()),
    ])
    .await;
    let (manager, id) = connected_manager(&endpoint, ca);
    assert!(manager
        .deletion_history(id, &"a".repeat(32))
        .await
        .unwrap()
        .operations
        .is_empty());
    let history = manager.deletion_history(id, &"a".repeat(32)).await.unwrap();
    assert_eq!(history.operations[0].status, Status::Uncertain);
    let saved = manager
        .recover_deletion(id, &"a".repeat(32), &"e".repeat(32))
        .await
        .unwrap();
    assert!(saved.operation.consistent_submission(true));
    assert!(manager.deletion_is_empty());
    assert!(task
        .await
        .unwrap()
        .iter()
        .all(|request| request.starts_with("GET ")));
}
