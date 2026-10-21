use super::*;
use crate::remote_connection::{create::prepare_tests::sequence, Profile};
use serde_json::{json, Value};
use std::sync::Arc;
const TOKEN: &str = "fixture-only-deletion-token-0123456789abcdef";
fn review() -> Review {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/deletion-approval.json"
    ))
    .unwrap();
    let plan = value.as_object_mut().unwrap().remove("plan").unwrap();
    value
        .as_object_mut()
        .unwrap()
        .extend(plan.as_object().unwrap().clone());
    value["observation"] =
        json!({"node":"pve-test","vmid":202,"status":"stopped","qmpStatus":"stopped"});
    value["haManaged"] = json!(false);
    value["replicationJobs"] = json!([]);
    value["destructive"] = json!(true);
    value["definitionRetained"] = json!(true);
    value["purge"] = json!(false);
    value["destroyUnreferencedDisks"] = json!(false);
    value["deletionPreconditions"] = json!("passed");
    value["executionEnabled"] = json!(true);
    value["checks"] = json!(CHECKS
        .iter()
        .map(|code| json!({"code":code,"status":"passed"}))
        .collect::<Vec<_>>());
    serde_json::from_value(value).unwrap()
}
fn target() -> String {
    json!({"id":"a".repeat(32),"definitionHash":"c".repeat(64),"status":"registered","providerReference":{"provider":"proxmox","scopeId":"b".repeat(64),"node":"pve-test","vmid":202}}).to_string()
}
fn queued() -> Value {
    json!({"id":"e".repeat(32),"kind":"vm_delete","instanceId":"a".repeat(32),"status":"queued","result":null,"failure":null})
}
fn succeeded() -> Value {
    json!({"id":"e".repeat(32),"kind":"vm_delete","instanceId":"a".repeat(32),"status":"succeeded","providerTaskKnown":true,"failure":null,"result":{"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"node":"pve-test","vmid":202,"approvalHash":include_str!("../../../../../fixtures/deletion-approval.sha256").trim(),"taskStatus":"succeeded","vmAbsenceVerified":true,"volumesAbsenceVerified":true,"definitionRetained":true,"associationRetired":true}})
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
    manager.0.lock().unwrap().deletion.prepared = Some(Prepared {
        key: "delete-key".into(),
        review: review(),
        submitted: operation,
        inflight: false,
        operation: operation.then(|| "e".repeat(32)),
        terminal: false,
        prepared_at: Instant::now(),
    });
}
#[test]
fn remote_deletion_hash_matches_server_golden_contract() {
    let review = review();
    assert!(review.valid() && review.eligible());
    assert_eq!(
        review.approval_hash().unwrap(),
        include_str!("../../../../../fixtures/deletion-approval.sha256").trim()
    );
    assert_eq!(
        serde_json::to_value(review.approval()).unwrap(),
        serde_json::from_str::<Value>(include_str!(
            "../../../../../fixtures/deletion-approval.json"
        ))
        .unwrap()
    );
}
#[tokio::test]
async fn remote_deletion_retains_full_review_dual_consent_and_get_only_tracking() {
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", target()),
        ("200 OK", serde_json::to_string(&review()).unwrap()),
        ("202 Accepted", queued().to_string()),
        ("200 OK", succeeded().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    let prepared = manager
        .prepare_deletion(id, "delete-key".into(), "a".repeat(32))
        .await
        .unwrap();
    assert!(prepared.eligible());
    for (confirmed, loss) in [(false, false), (true, false), (false, true)] {
        assert!(manager
            .confirm_deletion(id, "delete-key", confirmed, loss)
            .await
            .is_err());
    }
    assert!(manager
        .confirm_deletion(id, "wrong-key", true, true)
        .await
        .is_err());
    manager
        .confirm_deletion(id, "delete-key", true, true)
        .await
        .unwrap();
    assert!(manager
        .confirm_deletion(id, "delete-key", true, true)
        .await
        .is_err());
    assert!(manager.clear_deletion(id, "delete-key").is_err());
    let result = manager.deletion_operation(id, "delete-key").await.unwrap();
    assert_eq!(result.status, Status::Succeeded);
    manager.clear_deletion(id, "delete-key").unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with(&format!("GET /api/v1/environments/{} ", "a".repeat(32))));
    assert!(requests[1].starts_with(&format!(
        "GET /api/v1/environments/{}/delete-preflight ",
        "a".repeat(32)
    )));
    assert!(requests[2].starts_with(&format!(
        "POST /api/v1/environments/{}/delete ",
        "a".repeat(32)
    )));
    let body = requests[2].split("\r\n\r\n").nth(1).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        serde_json::to_value(prepared.approval()).unwrap()
    );
    assert!(requests[2]
        .to_ascii_lowercase()
        .contains("idempotency-key: delete-key\r\n"));
    assert!(requests[3].starts_with(&format!("GET /api/v1/operations/{} ", "e".repeat(32))));
    assert!(requests.iter().all(|request| request
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {TOKEN}\r\n"))));
}

#[tokio::test]
async fn remote_deletion_unknown_reply_retains_same_key_body_and_blocks_replacement() {
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
    let error = manager
        .confirm_deletion(id, "delete-key", true, true)
        .await
        .unwrap_err();
    assert!(!error.contains(TOKEN) && !error.contains("private-provider-detail"));
    assert!(manager.clear_deletion(id, "delete-key").is_err());
    assert!(manager
        .prepare_deletion(id, "new-key".into(), "a".repeat(32))
        .await
        .is_err());
    assert!(manager.deletion_operation(id, "delete-key").await.is_err());
    manager
        .0
        .lock()
        .unwrap()
        .deletion
        .prepared
        .as_mut()
        .unwrap()
        .prepared_at = Instant::now() - std::time::Duration::from_secs(121);
    manager
        .confirm_deletion(id, "delete-key", true, true)
        .await
        .unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
}
#[tokio::test]
async fn remote_deletion_disabled_failed_or_expired_review_never_submits() {
    for variant in ["disabled", "failed", "expired"] {
        let manager = RemoteConnection::default();
        let id = manager.begin().unwrap();
        manager
            .complete(
                id,
                Profile::new("https://localhost:1", TOKEN, None).unwrap(),
            )
            .unwrap();
        seed(&manager, false);
        {
            let mut state = manager.0.lock().unwrap();
            let intent = state.deletion.prepared.as_mut().unwrap();
            match variant {
                "disabled" => intent.review.execution_enabled = false,
                "failed" => {
                    intent.review.deletion_preconditions = CheckStatus::Failed;
                    intent.review.checks[5].status = CheckStatus::Failed;
                }
                _ => intent.prepared_at = Instant::now() - std::time::Duration::from_secs(121),
            }
        }
        let error = manager
            .confirm_deletion(id, "delete-key", true, true)
            .await
            .unwrap_err();
        assert!(error.contains(if variant == "expired" {
            "expired"
        } else {
            "permit"
        }));
        assert!(
            !manager
                .0
                .lock()
                .unwrap()
                .deletion
                .prepared
                .as_ref()
                .unwrap()
                .submitted
        );
        manager.clear_deletion(id, "delete-key").unwrap();
    }
}
#[test]
fn remote_deletion_review_rejects_unsafe_unreviewed_or_inconsistent_scope() {
    for variant in [
        "path",
        "owner",
        "effect",
        "effect-version",
        "overlap",
        "duplicate",
        "references",
        "snapshot-only",
        "slot",
        "checks",
        "power",
        "qmp",
        "ha",
        "replication",
        "definition-retained",
        "purge",
    ] {
        let mut value = serde_json::to_value(review()).unwrap();
        match variant {
            "path" => value["volumes"][0]["volume"] = json!("storage:../disk"),
            "owner" => value["volumes"][0]["ownerVerified"] = json!(false),
            "effect" => value["effects"]["cleanupIpam"] = json!(false),
            "effect-version" => value["effects"]["version"] = json!(2),
            "overlap" => value["preservedOwnedVolumes"] = json!(["storage:vm-202-disk-0"]),
            "duplicate" => value["volumes"][1] = value["volumes"][0].clone(),
            "references" => {
                value["volumes"][0]["references"] = json!(["snapshot/unreviewed:scsi0"])
            }
            "snapshot-only" => value["volumes"][0]["effect"] = json!("snapshot_reference"),
            "slot" => value["volumes"][0]["references"] = json!(["live:scsi999"]),
            "checks" => value["checks"][1] = value["checks"][0].clone(),
            "power" => value["observation"]["status"] = json!("running"),
            "qmp" => value["observation"]["qmpStatus"] = json!("paused"),
            "ha" => value["haManaged"] = json!(true),
            "replication" => value["replicationJobs"] = json!(["202-0"]),
            "definition-retained" => value["definitionRetained"] = json!(false),
            _ => value["purge"] = json!(true),
        }
        assert!(
            !serde_json::from_value::<Review>(value).unwrap().valid(),
            "{variant}"
        );
    }
    let mut unknown = serde_json::to_value(review()).unwrap();
    unknown["effects"]["futureCleanup"] = json!(true);
    assert!(serde_json::from_value::<Review>(unknown).is_err());
}
#[tokio::test]
async fn remote_deletion_rejects_target_scope_mismatch_and_deleted_identity() {
    for variant in ["scope", "target", "deleted"] {
        let mut report = review();
        if variant == "scope" {
            report.provider_scope_id = "f".repeat(64);
        } else {
            report.vmid = 203;
            report.observation.vmid = 203;
        }
        let mut responses = vec![(
            "200 OK",
            if variant == "deleted" {
                target().replace("registered", "deleted")
            } else {
                target()
            },
        )];
        if variant != "deleted" {
            responses.push(("200 OK", serde_json::to_string(&report).unwrap()));
        }
        let (endpoint, ca, task) = sequence(responses).await;
        let (manager, id) = manager(&endpoint, ca);
        assert!(manager
            .prepare_deletion(id, "delete-key".into(), "a".repeat(32))
            .await
            .is_err());
        assert!(manager.0.lock().unwrap().deletion.prepared.is_none());
        assert!(task.await.unwrap().iter().all(|r| r.starts_with("GET ")));
    }
}
#[test]
fn remote_deletion_success_requires_full_hash_absence_and_retirement_evidence() {
    for variant in [
        "kind",
        "instance",
        "task",
        "hash",
        "node",
        "vmid",
        "absence",
        "volumes",
        "definition",
        "retired",
        "status",
        "result-instance",
        "result-scope",
    ] {
        let mut value = succeeded();
        match variant {
            "kind" => value["kind"] = json!("vm_snapshot_restore"),
            "instance" => value["instanceId"] = json!("f".repeat(32)),
            "task" => value["providerTaskKnown"] = json!(false),
            "hash" => value["result"]["approvalHash"] = json!("f".repeat(64)),
            "node" => value["result"]["node"] = json!("other-node"),
            "vmid" => value["result"]["vmid"] = json!(203),
            "absence" => value["result"]["vmAbsenceVerified"] = json!(false),
            "volumes" => value["result"]["volumesAbsenceVerified"] = json!(false),
            "definition" => value["result"]["definitionRetained"] = json!(false),
            "retired" => value["result"]["associationRetired"] = json!(false),
            "status" => value["result"]["taskStatus"] = json!("failed"),
            "result-instance" => value["result"]["instanceId"] = json!("f".repeat(32)),
            _ => value["result"]["providerScopeId"] = json!("f".repeat(64)),
        }
        assert!(
            serde_json::from_value::<Operation>(value)
                .map_or(true, |operation| !operation.valid(&review(), None)),
            "{variant}"
        );
    }
    let valid: Operation = serde_json::from_value(succeeded()).unwrap();
    assert!(!valid.valid(&review(), Some(&"f".repeat(32))));
    for (known, status, failure, accepted) in [
        (true, "uncertain", "provider_task_failed", true),
        (false, "uncertain", "provider_task_failed", false),
        (false, "uncertain", "provider_submission_unconfirmed", true),
        (true, "failed", "provider_task_failed", false),
        (true, "uncertain", "deletion_state_unconfirmed", true),
    ] {
        let mut value = queued();
        value["providerTaskKnown"] = json!(known);
        value["status"] = json!(status);
        value["failure"] = json!(failure);
        assert_eq!(
            serde_json::from_value::<Operation>(value)
                .unwrap()
                .valid(&review(), None),
            accepted
        );
    }
}
#[tokio::test]
async fn remote_deletion_stale_preparation_stops_before_followup_read() {
    use crate::remote_connection::tests::fixture_with_gate;
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
                .prepare_deletion(id, "delete-key".into(), "a".repeat(32))
                .await
        });
        received.await.unwrap();
        if disconnected {
            manager.begin().unwrap();
        } else {
            manager.0.lock().unwrap().deletion.revision += 1;
        }
        release.send(()).unwrap();
        assert!(prepare.await.unwrap().is_err());
        assert!(task.await.unwrap().starts_with("GET "));
        assert!(manager.0.lock().unwrap().deletion.prepared.is_none());
    }
}
#[tokio::test]
async fn remote_deletion_disconnect_during_submission_clears_state_without_replaying() {
    use crate::remote_connection::tests::fixture_with_gate;
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
        tokio::spawn(async move { active.confirm_deletion(id, "delete-key", true, true).await });
    received.await.unwrap();
    assert!(manager
        .confirm_deletion(id, "delete-key", true, true)
        .await
        .unwrap_err()
        .contains("progress"));
    assert!(manager.clear_deletion(id, "delete-key").is_err());
    manager.begin().unwrap();
    release.send(()).unwrap();
    assert!(submit.await.unwrap().unwrap_err().contains("may continue"));
    assert!(manager.0.lock().unwrap().deletion.prepared.is_none());
    assert!(task.await.unwrap().starts_with("POST "));
}
#[tokio::test]
async fn remote_deletion_transport_refuses_missing_auth_bad_status_and_oversized_response() {
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
            "x".repeat(256 * 1024 + 1),
            "x-vmanyware-authenticated: bearer\r\n",
        ),
    ] {
        let (endpoint, ca, task) =
            crate::remote_connection::tests::fixture(status, &body, headers).await;
        let (manager, id) = manager(&endpoint, ca);
        seed(&manager, false);
        let error = manager
            .confirm_deletion(id, "delete-key", true, true)
            .await
            .unwrap_err();
        assert!(!error.contains(TOKEN) && !error.contains("private-provider-detail"));
        assert!(
            manager
                .0
                .lock()
                .unwrap()
                .deletion
                .prepared
                .as_ref()
                .unwrap()
                .submitted
        );
        assert!(manager.clear_deletion(id, "delete-key").is_err());
        task.await.unwrap();
    }
}

#[tokio::test]
async fn remote_deletion_socket_loss_retains_unknown_submission_without_automatic_retry() {
    use crate::remote_connection::tests::fixture_with_gate;
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
    let submit =
        tokio::spawn(async move { active.confirm_deletion(id, "delete-key", true, true).await });
    received.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(submit.await.unwrap().unwrap_err().contains("interrupted"));
    {
        let state = manager.0.lock().unwrap();
        let intent = state.deletion.prepared.as_ref().unwrap();
        assert!(intent.submitted && !intent.inflight && intent.operation.is_none());
    }
    assert!(manager.clear_deletion(id, "delete-key").is_err());
}
#[tokio::test]
async fn remote_deletion_known_partial_task_only_reads_until_verified_terminal() {
    let mut uncertain = queued();
    uncertain["status"] = json!("uncertain");
    uncertain["providerTaskKnown"] = json!(true);
    uncertain["failure"] = json!("deletion_state_unconfirmed");
    let (endpoint, ca, task) = sequence(vec![
        ("200 OK", uncertain.to_string()),
        ("200 OK", succeeded().to_string()),
    ])
    .await;
    let (manager, id) = manager(&endpoint, ca);
    seed(&manager, true);
    manager
        .0
        .lock()
        .unwrap()
        .deletion
        .prepared
        .as_mut()
        .unwrap()
        .review
        .execution_enabled = false;
    assert_eq!(
        manager
            .deletion_operation(id, "delete-key")
            .await
            .unwrap()
            .status,
        Status::Uncertain
    );
    assert!(manager
        .confirm_deletion(id, "delete-key", true, true)
        .await
        .is_err());
    assert!(manager.clear_deletion(id, "delete-key").is_err());
    assert_eq!(
        manager
            .deletion_operation(id, "delete-key")
            .await
            .unwrap()
            .status,
        Status::Succeeded
    );
    manager.clear_deletion(id, "delete-key").unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
}
#[tokio::test]
async fn remote_deletion_disconnected_read_cannot_restore_cleared_intent() {
    use crate::remote_connection::tests::fixture_with_gate;
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
    let read = tokio::spawn(async move { active.deletion_operation(id, "delete-key").await });
    received.await.unwrap();
    manager.begin().unwrap();
    release.send(()).unwrap();
    assert!(read.await.unwrap().is_err());
    assert!(manager.0.lock().unwrap().deletion.prepared.is_none());
    assert!(task.await.unwrap().starts_with("GET "));
}
#[tokio::test]
async fn remote_deletion_invalid_local_inputs_do_not_create_an_intent() {
    let manager = RemoteConnection::default();
    let id = manager.begin().unwrap();
    manager
        .complete(
            id,
            Profile::new("https://localhost:1", TOKEN, None).unwrap(),
        )
        .unwrap();
    for (key, instance) in [
        ("unsafe key".into(), "a".repeat(32)),
        ("valid-key".into(), "../202".into()),
        ("".into(), "a".repeat(32)),
    ] {
        assert!(manager
            .prepare_deletion(id, key, instance)
            .await
            .unwrap_err()
            .contains("valid"));
        assert!(manager.0.lock().unwrap().deletion.prepared.is_none());
    }
}
