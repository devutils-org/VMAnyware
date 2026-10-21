use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition,
    deletion::{Approval, Outcome},
    instance::Registration,
    operation::{Failure, Status},
    proxmox::TaskReference,
    router_with_services,
    store::Store,
};
fn setup(store: &Store, vmid: u32) -> Approval {
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    if store.get(&definition.metadata.name).unwrap().is_none() {
        store.create(&definition).unwrap();
    }
    let instance = store
        .register_instance(
            &Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-test".into(),
                vmid,
            },
            &"b".repeat(64),
        )
        .unwrap();
    let mut approval: Approval =
        serde_json::from_str(include_str!("../../fixtures/deletion-approval.json")).unwrap();
    approval.instance_id = instance.summary.id;
    approval.definition_hash = definition.hash().unwrap();
    approval.vmid = vmid;
    approval
}
fn task() -> TaskReference {
    TaskReference::new(
        "pve-test",
        "UPID:pve-test:00000001:00000002:00000003:qmdestroy:202:fixture@pve:",
    )
    .unwrap()
}
fn get(instance: &str, operation: &str) -> Request<Body> {
    Request::get(format!(
        "/api/v1/environments/{instance}/deletion-operations/{operation}"
    ))
    .body(Body::empty())
    .unwrap()
}
fn history(instance: &str) -> Request<Body> {
    Request::get(format!(
        "/api/v1/environments/{instance}/deletion-operations"
    ))
    .body(Body::empty())
    .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap()
}
#[tokio::test]
async fn deletion_recovery_reopens_without_provider_flags_or_claiming_or_forwarding_consent() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let operation = store
        .enqueue_deletion("private-request-key", &approval)
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    let app = router_with_services(None, store.clone());
    let before = serde_json::to_value(
        store
            .deletion_job(&operation.id)
            .unwrap()
            .unwrap()
            .operation,
    )
    .unwrap();
    let response = app
        .clone()
        .oneshot(get(&approval.instance_id, &operation.id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let recovered = body(response).await;
    assert_eq!(recovered["operation"], before);
    assert_eq!(
        recovered["plan"],
        serde_json::to_value(&approval.plan).unwrap()
    );
    assert_eq!(recovered["approvalHash"], approval.input_hash().unwrap());
    assert_eq!(recovered["submissionStarted"], false);
    assert_eq!(recovered["configDigest"], approval.config_digest);
    assert_eq!(
        recovered["target"]["providerScopeId"],
        approval.provider_scope_id
    );
    for private in [
        "confirmed",
        "acknowledgeDataLoss",
        "requestKey",
        "providerUpid",
        "approvalJson",
    ] {
        assert!(recovered.get(private).is_none());
    }
    assert!(!recovered.to_string().contains("private-request-key"));
    let response = app.oneshot(history(&approval.instance_id)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let saved = body(response).await;
    assert_eq!(saved["operations"][0]["id"], operation.id);
    assert_eq!(
        saved["operations"][0]["approvalHash"],
        approval.input_hash().unwrap()
    );
    assert!(saved["operations"][0].get("plan").is_none());
    let job = store.deletion_job(&operation.id).unwrap().unwrap();
    assert_eq!(serde_json::to_value(job.operation).unwrap(), before);
    assert!(!job.dispatch_started && job.provider_task.is_none());
}
#[tokio::test]
async fn deletion_recovery_preserves_queued_running_unknown_partial_interrupted_and_failed_states()
{
    for variant in [
        "queued",
        "running",
        "unknown",
        "known-running",
        "partial-failed-task",
        "interrupted",
        "failed",
        "explicit-refusal",
    ] {
        let store = Store::memory().unwrap();
        let approval = setup(&store, 202);
        let operation = store.enqueue_deletion("retained", &approval).unwrap();
        if variant != "queued" {
            store.claim_deletion().unwrap().unwrap();
        }
        if [
            "unknown",
            "known-running",
            "partial-failed-task",
            "explicit-refusal",
        ]
        .contains(&variant)
        {
            store
                .begin_deletion_dispatch(&operation.id, &approval)
                .unwrap();
        }
        if ["known-running", "partial-failed-task"].contains(&variant) {
            store.link_deletion_task(&operation.id, &task()).unwrap();
        }
        match variant {
            "unknown" => {
                store
                    .finish_deletion(
                        &operation.id,
                        Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed),
                    )
                    .unwrap();
            }
            "partial-failed-task" => {
                store
                    .finish_deletion(
                        &operation.id,
                        Outcome::Uncertain(Failure::ProviderTaskFailed),
                    )
                    .unwrap();
            }
            "interrupted" => {
                store.recover_interrupted_operations().unwrap();
            }
            "failed" => {
                store
                    .finish_deletion(
                        &operation.id,
                        Outcome::Failed(Failure::DeletionPreconditionsFailed),
                    )
                    .unwrap();
            }
            "explicit-refusal" => {
                store
                    .finish_deletion(&operation.id, Outcome::Failed(Failure::ProviderForbidden))
                    .unwrap();
            }
            _ => {}
        }
        let before =
            serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap();
        let app = router_with_services(None, store.clone());
        let response = app
            .clone()
            .oneshot(get(&approval.instance_id, &operation.id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{variant}");
        assert_eq!(body(response).await["operation"], before, "{variant}");
        assert_eq!(
            app.oneshot(history(&approval.instance_id))
                .await
                .unwrap()
                .status(),
            StatusCode::OK,
            "{variant}"
        );
        assert_eq!(
            serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap(),
            before
        );
    }
}
#[tokio::test]
async fn deletion_recovery_resolves_scope_and_kind_before_decoding_damaged_journals() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scope.sqlite");
    let store = Store::open(&path).unwrap();
    let owner = setup(&store, 202);
    let other = setup(&store, 203);
    let operation = store.enqueue_deletion("damaged", &owner).unwrap();
    let inventory = store.enqueue_inventory("other-kind").unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    let mut bad = serde_json::to_value(&owner).unwrap();
    bad["node"] = json!("private-node");
    connection
        .execute(
            "UPDATE operations SET approval_json=?1 WHERE id=?2",
            rusqlite::params![bad.to_string(), operation.id],
        )
        .unwrap();
    let app = router_with_services(None, store.clone());
    for (instance, id) in [
        (&other.instance_id, operation.id.as_str()),
        (&owner.instance_id, inventory.id.as_str()),
        (&owner.instance_id, "missing"),
        (&owner.instance_id, "../secret"),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(get(instance, id))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let response = app
        .clone()
        .oneshot(get(&owner.instance_id, &operation.id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!body(response).await.to_string().contains("private-node"));
    assert_eq!(
        app.clone()
            .oneshot(history(&other.instance_id))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.oneshot(history(&owner.instance_id))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}
#[tokio::test]
async fn deletion_history_filters_then_limits_orders_and_keeps_old_detail() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let other = setup(&store, 203);
    let mut ids = vec![];
    for index in 0..103 {
        let op = store
            .enqueue_deletion(&format!("saved-{index}"), &approval)
            .unwrap();
        store.claim_deletion().unwrap().unwrap();
        store
            .finish_deletion(&op.id, Outcome::Failed(Failure::DeletionDisabled))
            .unwrap();
        ids.push(op.id);
    }
    for index in 0..3 {
        let op = store
            .enqueue_deletion(&format!("unrelated-{index}"), &other)
            .unwrap();
        store.claim_deletion().unwrap().unwrap();
        store
            .finish_deletion(&op.id, Outcome::Failed(Failure::DeletionDisabled))
            .unwrap();
    }
    let app = router_with_services(None, store.clone());
    let response = app
        .clone()
        .oneshot(history(&approval.instance_id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let saved = body(response).await;
    let entries = saved["operations"].as_array().unwrap();
    assert_eq!(entries.len(), 100);
    for pair in entries.windows(2) {
        assert!(
            (pair[0]["createdAt"].as_str(), pair[0]["id"].as_str())
                > (pair[1]["createdAt"].as_str(), pair[1]["id"].as_str())
        );
    }
    assert!(entries
        .iter()
        .all(|entry| ids.contains(&entry["id"].as_str().unwrap().to_owned())));
    assert_eq!(
        app.oneshot(get(&approval.instance_id, &ids[0]))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(store.recent_operations().unwrap().len(), 100);
}
#[tokio::test]
async fn deletion_recovery_auth_and_missing_history_are_read_only() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let op = store.enqueue_deletion("protected", &approval).unwrap();
    let app = vmanyware_server::auth::protect(
        router_with_services(None, store.clone()),
        Some(
            vmanyware_server::auth::AccessToken::parse(
                "fixture-only-access-token-0123456789abcdef",
            )
            .unwrap(),
        ),
    );
    assert_eq!(
        app.clone()
            .oneshot(get(&approval.instance_id, &op.id))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.oneshot(history(&approval.instance_id))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let app = router_with_services(None, store.clone());
    assert_eq!(
        app.oneshot(history(&"f".repeat(32)))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        store.operation(&op.id).unwrap().unwrap().status,
        Status::Queued
    );
}

fn saved_success(
    store: &Store,
    path: &std::path::Path,
    approval: &Approval,
    retire: bool,
) -> (String, Value) {
    // Synthetic persisted journal encoding, not provider-completion evidence.
    let operation = store.enqueue_deletion("saved-success", approval).unwrap();
    store.claim_deletion().unwrap().unwrap();
    store
        .begin_deletion_dispatch(&operation.id, approval)
        .unwrap();
    store.link_deletion_task(&operation.id, &task()).unwrap();
    let evidence = json!({"instanceId":approval.instance_id,"providerScopeId":approval.provider_scope_id,"node":approval.node,"vmid":approval.vmid,"approvalHash":approval.input_hash().unwrap(),"taskStatus":"succeeded","vmAbsenceVerified":true,"volumesAbsenceVerified":true,"definitionRetained":true,"associationRetired":true});
    let mut connection = rusqlite::Connection::open(path).unwrap();
    let transaction = connection.transaction().unwrap();
    transaction
        .execute(
            "UPDATE operations SET status='succeeded',result_json=?1,failure_code=NULL WHERE id=?2",
            rusqlite::params![evidence.to_string(), operation.id],
        )
        .unwrap();
    if retire {
        transaction
            .execute(
                "UPDATE instances SET status='deleted' WHERE id=?1",
                [&approval.instance_id],
            )
            .unwrap();
    }
    transaction.commit().unwrap();
    (operation.id, evidence)
}
#[tokio::test]
async fn deletion_recovery_requires_exact_saved_absence_task_hash_and_retired_association() {
    for variant in [
        "valid",
        "hash",
        "vmid",
        "vm-absence",
        "volume-absence",
        "definition",
        "retirement-flag",
        "task-status",
        "unknown-task",
        "missing-marker",
        "wrong-task",
        "not-retired",
        "extra",
        "oversized-result",
        "oversized-approval",
        "foreign-failure",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("success.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store, 202);
        let (id, mut evidence) = saved_success(&store, &path, &approval, variant != "not-retired");
        let connection = rusqlite::Connection::open(&path).unwrap();
        match variant {
            "hash" => evidence["approvalHash"] = json!("f".repeat(64)),
            "vmid" => evidence["vmid"] = json!(203),
            "vm-absence" => evidence["vmAbsenceVerified"] = json!(false),
            "volume-absence" => evidence["volumesAbsenceVerified"] = json!(false),
            "definition" => evidence["definitionRetained"] = json!(false),
            "retirement-flag" => evidence["associationRetired"] = json!(false),
            "task-status" => evidence["taskStatus"] = json!("failed"),
            "extra" => evidence["privateExtra"] = json!("private-detail"),
            "oversized-result" => evidence["privateExtra"] = json!("private-detail".repeat(1024)),
            _ => {}
        }
        connection
            .execute(
                "UPDATE operations SET result_json=?1 WHERE id=?2",
                rusqlite::params![evidence.to_string(), id],
            )
            .unwrap();
        match variant {
            "unknown-task" => {
                connection
                    .execute(
                        "UPDATE operations SET provider_upid=NULL WHERE id=?1",
                        [&id],
                    )
                    .unwrap();
            }
            "missing-marker" => {
                connection
                    .execute(
                        "UPDATE operations SET provider_upid=NULL,dispatch_started=0 WHERE id=?1",
                        [&id],
                    )
                    .unwrap();
            }
            "wrong-task" => {
                connection.execute("UPDATE operations SET provider_upid='UPID:pve-test:00000001:00000002:00000003:qmstart:202:fixture@pve:' WHERE id=?1",[&id]).unwrap();
            }
            "oversized-approval" => {
                connection
                    .execute(
                        "UPDATE operations SET approval_json=?1 WHERE id=?2",
                        rusqlite::params![
                            format!(
                                "{}{}",
                                " ".repeat(65536),
                                serde_json::to_string(&approval).unwrap()
                            ),
                            id
                        ],
                    )
                    .unwrap();
            }
            "foreign-failure" => {
                connection.execute("UPDATE operations SET status='failed',result_json=NULL,failure_code='restore_state_unconfirmed',provider_upid=NULL,dispatch_started=0 WHERE id=?1",[&id]).unwrap();
            }
            _ => {}
        }
        let version: u32 = connection
            .pragma_query_value(None, "data_version", |row| row.get(0))
            .unwrap();
        let original: (String,Option<String>,bool,Option<String>)=connection.query_row("SELECT status,result_json,dispatch_started,provider_upid FROM operations WHERE id=?1",[&id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
        let app = router_with_services(None, store.clone());
        let response = app
            .clone()
            .oneshot(get(&approval.instance_id, &id))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            if variant == "valid" {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            "{variant}"
        );
        assert!(!body(response).await.to_string().contains("private-detail"));
        assert_eq!(
            app.oneshot(history(&approval.instance_id))
                .await
                .unwrap()
                .status(),
            if variant == "valid" {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            "{variant}"
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "data_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            version
        );
        assert_eq!(connection.query_row("SELECT status,result_json,dispatch_started,provider_upid FROM operations WHERE id=?1",[&id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,bool>(2)?,row.get::<_,Option<String>>(3)?))).unwrap(),original);
    }
}
#[tokio::test]
async fn deletion_recovery_keeps_old_failed_and_deleted_history_separate_from_reused_vmid() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reuse.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let failed = store.enqueue_deletion("earlier-failed", &approval).unwrap();
    store.claim_deletion().unwrap().unwrap();
    store
        .finish_deletion(
            &failed.id,
            Outcome::Failed(Failure::DeletionPreconditionsFailed),
        )
        .unwrap();
    let (id, _) = saved_success(&store, &path, &approval, true);
    let replacement = setup(&store, 202);
    assert_ne!(replacement.instance_id, approval.instance_id);
    let app = router_with_services(None, store.clone());
    let response = app
        .clone()
        .oneshot(history(&approval.instance_id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await["operations"].as_array().unwrap().len(),
        2
    );
    let response = app
        .clone()
        .oneshot(history(&replacement.instance_id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body(response).await["operations"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        app.clone()
            .oneshot(get(&replacement.instance_id, &id))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.oneshot(get(&approval.instance_id, &id))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        store
            .instance(&approval.instance_id)
            .unwrap()
            .unwrap()
            .summary
            .status,
        vmanyware_server::instance::Status::Deleted
    );
}
#[tokio::test]
async fn deletion_recovery_rejects_saved_target_mismatch_without_reading_current_provider() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("target.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let operation = store.enqueue_deletion("target-binding", &approval).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE instances SET node='private-retarget' WHERE id=?1",
            [&approval.instance_id],
        )
        .unwrap();
    let app = router_with_services(None, store.clone());
    let response = app
        .oneshot(get(&approval.instance_id, &operation.id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!body(response)
        .await
        .to_string()
        .contains("private-retarget"));
    assert_eq!(
        store.operation(&operation.id).unwrap().unwrap().status,
        Status::Queued
    );
}
