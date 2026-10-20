use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    operation::{Failure, Status},
    router_with_services,
    snapshot_restore::{Approval, Outcome, Request as RestoreRequest},
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
                node: "saved-node".into(),
                vmid,
            },
            &"a".repeat(64),
        )
        .unwrap();
    Approval {
        instance_id: instance.summary.id,
        provider_scope_id: "a".repeat(64),
        definition_hash: definition.hash().unwrap(),
        node: "saved-node".into(),
        vmid,
        config_digest: "b".repeat(40),
        request: RestoreRequest {
            snapshot_name: "before-work".into(),
        },
        snapshot_config_hash: "c".repeat(64),
        acknowledge_data_loss: true,
        confirmed: true,
    }
}
fn get(instance: &str, operation: &str) -> Request<Body> {
    Request::get(format!(
        "/api/v1/environments/{instance}/restore-operations/{operation}"
    ))
    .body(Body::empty())
    .unwrap()
}
fn history(instance: &str) -> Request<Body> {
    Request::get(format!(
        "/api/v1/environments/{instance}/restore-operations"
    ))
    .body(Body::empty())
    .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}

#[tokio::test]
async fn restore_recovery_survives_reopen_without_provider_capability_or_claiming_work() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let original = store.enqueue_restore("saved-key", &approval).unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    let before = serde_json::to_value(store.operation(&original.id).unwrap().unwrap()).unwrap();
    let app = router_with_services(None, store.clone());
    let response = app
        .clone()
        .oneshot(get(&approval.instance_id, &original.id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let recovered = body(response).await;
    assert_eq!(recovered["operation"], before);
    assert_eq!(
        recovered["request"],
        serde_json::to_value(&approval.request).unwrap()
    );
    assert_eq!(
        recovered["target"],
        json!({"instanceId":approval.instance_id,"providerScopeId":approval.provider_scope_id,"definitionHash":approval.definition_hash,"node":"saved-node","vmid":202})
    );
    assert_eq!(recovered["submissionStarted"], false);
    for excluded in [
        "confirmed",
        "acknowledgeDataLoss",
        "configDigest",
        "inputHash",
        "providerUpid",
        "requestKey",
    ] {
        assert!(recovered.get(excluded).is_none());
        assert!(recovered["target"].get(excluded).is_none());
    }
    let response = app.oneshot(history(&approval.instance_id)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let history = body(response).await;
    assert_eq!(history["instanceId"], approval.instance_id);
    assert_eq!(history["operations"].as_array().unwrap().len(), 1);
    assert_eq!(history["operations"][0]["snapshotName"], "before-work");
    assert_eq!(
        serde_json::to_value(store.operation(&original.id).unwrap().unwrap()).unwrap(),
        before
    );
    assert!(
        !store
            .restore_job(&original.id)
            .unwrap()
            .unwrap()
            .dispatch_started
    );
}

#[tokio::test]
async fn restore_recovery_preserves_running_and_unknown_submission_without_reenqueue() {
    for dispatched in [false, true] {
        let store = Store::memory().unwrap();
        let approval = setup(&store, 202);
        let operation = store.enqueue_restore("pending", &approval).unwrap();
        store.claim_restore().unwrap().unwrap();
        if dispatched {
            store
                .begin_restore_dispatch(&operation.id, &approval)
                .unwrap();
            store
                .finish_restore(
                    &operation.id,
                    Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed),
                )
                .unwrap();
        }
        let before =
            serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap();
        let app = router_with_services(None, store.clone());
        let response = app
            .oneshot(get(&approval.instance_id, &operation.id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let recovered = body(response).await;
        assert_eq!(recovered["operation"], before);
        assert_eq!(recovered["submissionStarted"], dispatched);
        assert_eq!(
            serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap(),
            before
        );
        assert_eq!(store.recent_operations().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn restore_recovery_refuses_cross_instance_wrong_kind_and_invalid_ids() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let other = setup(&store, 203);
    let operation = store.enqueue_restore("right-instance", &approval).unwrap();
    let read = store.enqueue_inventory("read").unwrap();
    let app = router_with_services(None, store.clone());
    for (instance, operation) in [
        (&other.instance_id[..], &operation.id[..]),
        (&approval.instance_id, &read.id),
        (&approval.instance_id, "invalid"),
        ("invalid", &operation.id),
        (&approval.instance_id, &"f".repeat(32)),
    ] {
        let response = app.clone().oneshot(get(instance, operation)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body(response).await["error"]["code"],
            "restore_operation_not_found"
        );
    }
    let response = app
        .clone()
        .oneshot(history(&other.instance_id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await["operations"], json!([]));
    assert_eq!(
        app.oneshot(history(&"f".repeat(32)))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn restore_recovery_history_limits_after_instance_filter_and_orders_records() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let mut other = setup(&store, 203);
    let old = store.enqueue_restore("old-instance", &approval).unwrap();
    store.claim_restore().unwrap().unwrap();
    store
        .finish_restore(&old.id, Outcome::Failed(Failure::RestoreDisabled))
        .unwrap();
    for index in 0..130 {
        other.request.snapshot_name = format!("snapshot-{index:03}");
        let operation = store
            .enqueue_restore(&format!("other-{index}"), &other)
            .unwrap();
        store.claim_restore().unwrap().unwrap();
        store
            .finish_restore(&operation.id, Outcome::Failed(Failure::RestoreDisabled))
            .unwrap();
    }
    let app = router_with_services(None, store.clone());
    let response = app
        .clone()
        .oneshot(history(&approval.instance_id))
        .await
        .unwrap();
    let first = body(response).await;
    assert_eq!(first["operations"].as_array().unwrap().len(), 1);
    assert_eq!(first["operations"][0]["id"], old.id);
    let response = app.oneshot(history(&other.instance_id)).await.unwrap();
    let history = body(response).await;
    let rows = history["operations"].as_array().unwrap();
    assert_eq!(rows.len(), 100);
    assert!(rows.windows(2).all(|pair| (
        pair[0]["createdAt"].as_str().unwrap(),
        pair[0]["id"].as_str().unwrap()
    ) > (
        pair[1]["createdAt"].as_str().unwrap(),
        pair[1]["id"].as_str().unwrap()
    )));
    assert!(rows.iter().all(|row| row["snapshotName"]
        .as_str()
        .unwrap()
        .starts_with("snapshot-")
        && row.get("request").is_none()
        && row.get("result").is_none()));
}

#[tokio::test]
async fn restore_recovery_corrupt_journal_fails_closed_without_cross_instance_leak() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("corrupt.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let other = setup(&store, 203);
    let operation = store.enqueue_restore("corrupt", &approval).unwrap();
    rusqlite::Connection::open(&path).unwrap().execute("UPDATE operations SET approval_json=json_set(approval_json,'$.request.snapshotName','private-corrupt-detail') WHERE id=?1",[&operation.id]).unwrap();
    let app = router_with_services(None, store.clone());
    let response = app
        .clone()
        .oneshot(get(&other.instance_id, &operation.id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    for request in [
        get(&approval.instance_id, &operation.id),
        history(&approval.instance_id),
    ] {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(!body(response)
            .await
            .to_string()
            .contains("private-corrupt-detail"));
    }
    assert_eq!(
        store.operation(&operation.id).unwrap().unwrap().status,
        Status::Queued
    );
}

#[tokio::test]
async fn restore_recovery_routes_remain_behind_configured_bearer_authentication() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let operation = store.enqueue_restore("auth", &approval).unwrap();
    let token = "fixture-only-recovery-token-0123456789abcdef";
    let app = vmanyware_server::auth::protect(
        router_with_services(None, store.clone()),
        Some(vmanyware_server::auth::AccessToken::parse(token).unwrap()),
    );
    for mut request in [
        get(&approval.instance_id, &operation.id),
        history(&approval.instance_id),
    ] {
        let unauthorized = app
            .clone()
            .oneshot(if request.uri().path().ends_with(&operation.id) {
                get(&approval.instance_id, &operation.id)
            } else {
                history(&approval.instance_id)
            })
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
        request
            .headers_mut()
            .insert("authorization", format!("Bearer {token}").parse().unwrap());
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["x-vmanyware-authenticated"], "bearer");
    }
}

#[tokio::test]
async fn restore_recovery_validates_saved_success_and_does_not_forward_extra_result_fields() {
    use vmanyware_server::{
        proxmox::{PowerState, TaskReference, TaskStatus, VmObservation},
        snapshot_restore::ResultEvidence,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("success.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let operation = store.enqueue_restore("success", &approval).unwrap();
    store.claim_restore().unwrap().unwrap();
    store
        .begin_restore_dispatch(&operation.id, &approval)
        .unwrap();
    store
        .link_restore_task(
            &operation.id,
            &TaskReference::new(
                "saved-node",
                "UPID:saved-node:00000001:00000002:00000003:qmrollback:202:fixture@pve:",
            )
            .unwrap(),
        )
        .unwrap();
    store
        .finish_restore(
            &operation.id,
            Outcome::Succeeded(ResultEvidence {
                instance_id: approval.instance_id.clone(),
                provider_scope_id: approval.provider_scope_id.clone(),
                node: approval.node.clone(),
                vmid: 202,
                task_status: TaskStatus::Succeeded,
                snapshot_name: approval.request.snapshot_name.clone(),
                snapshot_config_hash: approval.snapshot_config_hash.clone(),
                observation: VmObservation {
                    node: approval.node.clone(),
                    vmid: 202,
                    status: PowerState::Stopped,
                    qmp_status: None,
                },
            }),
        )
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute("UPDATE operations SET result_json=json_set(result_json,'$.unexpected','private-extra-field','$.observation.private','private-nested-field') WHERE id=?1",[&operation.id]).unwrap();
    let app = router_with_services(None, store.clone());
    let response = app
        .clone()
        .oneshot(get(&approval.instance_id, &operation.id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let recovered = body(response).await;
    assert_eq!(recovered["operation"]["status"], "succeeded");
    assert_eq!(
        recovered["snapshotConfigHash"],
        approval.snapshot_config_hash
    );
    assert_eq!(
        recovered["operation"]["result"]["observation"]["status"],
        "stopped"
    );
    assert!(!recovered.to_string().contains("private-extra-field"));
    assert!(!recovered.to_string().contains("private-nested-field"));
    let original: String = connection
        .query_row(
            "SELECT result_json FROM operations WHERE id=?1",
            [&operation.id],
            |r| r.get(0),
        )
        .unwrap();
    for (path, value) in [
        ("$.snapshotName", json!("wrong-snapshot")),
        ("$.snapshotConfigHash", json!("d".repeat(64))),
        ("$.observation.vmid", json!(203)),
        ("$.observation.status", json!("running")),
        ("$.observation.qmpStatus", json!("paused")),
        ("$.taskStatus", json!("running")),
    ] {
        connection
            .execute(
                "UPDATE operations SET result_json=json_set(?1,?2,json(?3)) WHERE id=?4",
                rusqlite::params![original, path, value.to_string(), operation.id],
            )
            .unwrap();
        for request in [
            get(&approval.instance_id, &operation.id),
            history(&approval.instance_id),
        ] {
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
            assert!(!body(response).await.to_string().contains("private"));
        }
    }
}

fn task() -> vmanyware_server::proxmox::TaskReference {
    vmanyware_server::proxmox::TaskReference::new(
        "saved-node",
        "UPID:saved-node:00000001:00000002:00000003:qmrollback:202:fixture@pve:",
    )
    .unwrap()
}

#[tokio::test]
async fn restore_recovery_retains_known_running_uncertain_interrupted_and_failed_states() {
    for variant in [
        "running",
        "known-uncertain",
        "interrupted",
        "auth-refusal",
        "task-failed",
    ] {
        let store = Store::memory().unwrap();
        let approval = setup(&store, 202);
        let operation = store.enqueue_restore("saved-state", &approval).unwrap();
        store.claim_restore().unwrap().unwrap();
        if variant == "interrupted" {
            store.recover_interrupted_operations().unwrap();
        } else {
            store
                .begin_restore_dispatch(&operation.id, &approval)
                .unwrap();
            if variant != "auth-refusal" {
                store.link_restore_task(&operation.id, &task()).unwrap();
            }
            match variant {
                "known-uncertain" => {
                    store
                        .finish_restore(
                            &operation.id,
                            Outcome::Uncertain(Failure::RestoreStateUnconfirmed),
                        )
                        .unwrap();
                }
                "auth-refusal" => {
                    store
                        .finish_restore(&operation.id, Outcome::Failed(Failure::ProviderForbidden))
                        .unwrap();
                }
                "task-failed" => {
                    store
                        .finish_restore(&operation.id, Outcome::Failed(Failure::ProviderTaskFailed))
                        .unwrap();
                }
                _ => {}
            }
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
        let response = app.oneshot(history(&approval.instance_id)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{variant}");
        assert_eq!(
            body(response).await["operations"][0]["status"],
            before["status"]
        );
        assert_eq!(
            serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap(),
            before
        );
        assert_eq!(store.recent_operations().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn restore_recovery_rejects_inconsistent_dispatch_and_task_journals_without_writes() {
    for variant in [
        "unknown-task-evidence",
        "known-false-failure",
        "wrong-task-kind",
        "known-false-interruption",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inconsistent.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store, 202);
        let other = setup(&store, 203);
        let operation = store.enqueue_restore("bad-state", &approval).unwrap();
        store.claim_restore().unwrap().unwrap();
        store
            .begin_restore_dispatch(&operation.id, &approval)
            .unwrap();
        if variant != "unknown-task-evidence" {
            store.link_restore_task(&operation.id, &task()).unwrap();
        }
        store
            .finish_restore(
                &operation.id,
                Outcome::Uncertain(if variant == "unknown-task-evidence" {
                    Failure::ProviderSubmissionUnconfirmed
                } else {
                    Failure::RestoreStateUnconfirmed
                }),
            )
            .unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        match variant {
            "unknown-task-evidence" => {
                connection.execute("UPDATE operations SET failure_code='restore_state_unconfirmed' WHERE id=?1",[&operation.id]).unwrap();
            }
            "known-false-failure" => {
                connection
                    .execute(
                        "UPDATE operations SET status='failed' WHERE id=?1",
                        [&operation.id],
                    )
                    .unwrap();
            }
            "known-false-interruption" => {
                connection.execute("UPDATE operations SET status='interrupted',failure_code='process_interrupted' WHERE id=?1",[&operation.id]).unwrap();
            }
            _ => {
                connection.execute("UPDATE operations SET provider_upid='UPID:saved-node:00000001:00000002:00000003:qmstart:202:fixture@pve:' WHERE id=?1",[&operation.id]).unwrap();
            }
        }
        let before =
            serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap();
        let app = router_with_services(None, store.clone());
        let response = app
            .clone()
            .oneshot(get(&other.instance_id, &operation.id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        for request in [
            get(&approval.instance_id, &operation.id),
            history(&approval.instance_id),
        ] {
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                StatusCode::SERVICE_UNAVAILABLE,
                "{variant}"
            );
            assert!(!body(response).await.to_string().contains("UPID:"));
        }
        assert_eq!(
            serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap(),
            before
        );
    }
}

#[tokio::test]
async fn restore_recovery_filters_snapshot_creation_on_the_same_instance() {
    use vmanyware_server::snapshot::{Approval as CreateApproval, CreateRequest};
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let operation = store.enqueue_restore("restore", &approval).unwrap();
    store.claim_restore().unwrap().unwrap();
    store
        .finish_restore(&operation.id, Outcome::Failed(Failure::RestoreDisabled))
        .unwrap();
    let create = CreateApproval {
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        definition_hash: approval.definition_hash.clone(),
        node: approval.node.clone(),
        vmid: 202,
        config_digest: approval.config_digest.clone(),
        confirmed: true,
        request: CreateRequest {
            name: "other-snapshot".into(),
            description: "".into(),
        },
    };
    let creation = store.enqueue_snapshot("create", &create).unwrap();
    let app = router_with_services(None, store.clone());
    assert_eq!(
        app.clone()
            .oneshot(get(&approval.instance_id, &creation.id))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let response = app.oneshot(history(&approval.instance_id)).await.unwrap();
    let rows = body(response).await;
    assert_eq!(rows["operations"].as_array().unwrap().len(), 1);
    assert_eq!(rows["operations"][0]["id"], operation.id);
}

#[tokio::test]
async fn restore_recovery_refuses_unrelated_failure_codes_in_a_saved_restore() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wrong-failure.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let operation = store.enqueue_restore("wrong-failure", &approval).unwrap();
    store.claim_restore().unwrap().unwrap();
    store
        .finish_restore(&operation.id, Outcome::Failed(Failure::RestoreDisabled))
        .unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE operations SET failure_code='snapshot_disabled' WHERE id=?1",
            [&operation.id],
        )
        .unwrap();
    let before = serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap();
    let app = router_with_services(None, store.clone());
    for request in [
        get(&approval.instance_id, &operation.id),
        history(&approval.instance_id),
    ] {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    assert_eq!(
        serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap(),
        before
    );
}
