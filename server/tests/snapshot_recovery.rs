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
    snapshot::{Approval, CreateRequest, Outcome},
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
        request: CreateRequest {
            name: "before-work".into(),
            description: "Saved request\nDisk only".into(),
        },
        confirmed: true,
    }
}
fn get(instance: &str, operation: &str) -> Request<Body> {
    Request::get(format!(
        "/api/v1/environments/{instance}/snapshot-operations/{operation}"
    ))
    .body(Body::empty())
    .unwrap()
}
fn history(instance: &str) -> Request<Body> {
    Request::get(format!(
        "/api/v1/environments/{instance}/snapshot-operations"
    ))
    .body(Body::empty())
    .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}

#[tokio::test]
async fn snapshot_recovery_survives_reopen_without_provider_capability_or_claiming_work() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let original = store.enqueue_snapshot("saved-key", &approval).unwrap();
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
    assert_eq!(history["operations"][0]["name"], "before-work");
    assert_eq!(
        serde_json::to_value(store.operation(&original.id).unwrap().unwrap()).unwrap(),
        before
    );
    assert!(
        !store
            .snapshot_job(&original.id)
            .unwrap()
            .unwrap()
            .dispatch_started
    );
}

#[tokio::test]
async fn snapshot_recovery_preserves_running_and_unknown_submission_without_reenqueue() {
    for dispatched in [false, true] {
        let store = Store::memory().unwrap();
        let approval = setup(&store, 202);
        let operation = store.enqueue_snapshot("pending", &approval).unwrap();
        store.claim_snapshot().unwrap().unwrap();
        if dispatched {
            store
                .begin_snapshot_dispatch(&operation.id, &approval)
                .unwrap();
            store
                .finish_snapshot(
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
async fn snapshot_recovery_refuses_cross_instance_wrong_kind_and_invalid_ids() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let other = setup(&store, 203);
    let operation = store.enqueue_snapshot("right-instance", &approval).unwrap();
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
            "snapshot_operation_not_found"
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
async fn snapshot_recovery_history_limits_after_instance_filter_and_orders_records() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let mut other = setup(&store, 203);
    let old = store.enqueue_snapshot("old-instance", &approval).unwrap();
    store.claim_snapshot().unwrap().unwrap();
    store
        .finish_snapshot(&old.id, Outcome::Failed(Failure::SnapshotDisabled))
        .unwrap();
    for index in 0..130 {
        other.request.name = format!("snapshot-{index:03}");
        let operation = store
            .enqueue_snapshot(&format!("other-{index}"), &other)
            .unwrap();
        store.claim_snapshot().unwrap().unwrap();
        store
            .finish_snapshot(&operation.id, Outcome::Failed(Failure::SnapshotDisabled))
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
    assert!(rows
        .iter()
        .all(|row| row["name"].as_str().unwrap().starts_with("snapshot-")
            && row.get("request").is_none()
            && row.get("result").is_none()));
}

#[tokio::test]
async fn snapshot_recovery_corrupt_journal_fails_closed_without_cross_instance_leak() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("corrupt.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let other = setup(&store, 203);
    let operation = store.enqueue_snapshot("corrupt", &approval).unwrap();
    rusqlite::Connection::open(&path).unwrap().execute("UPDATE operations SET approval_json=json_set(approval_json,'$.request.description','private-corrupt-detail') WHERE id=?1",[&operation.id]).unwrap();
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
async fn snapshot_recovery_routes_remain_behind_configured_bearer_authentication() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let operation = store.enqueue_snapshot("auth", &approval).unwrap();
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
async fn snapshot_recovery_validates_saved_success_and_does_not_forward_extra_result_fields() {
    use vmanyware_server::{
        proxmox::{Snapshot, TaskReference, TaskStatus},
        snapshot::ResultEvidence,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("success.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let operation = store.enqueue_snapshot("success", &approval).unwrap();
    store.claim_snapshot().unwrap().unwrap();
    store
        .begin_snapshot_dispatch(&operation.id, &approval)
        .unwrap();
    store
        .link_snapshot_task(
            &operation.id,
            &TaskReference::new(
                "saved-node",
                "UPID:saved-node:00000001:00000002:00000003:qmsnapshot:202:fixture@pve:",
            )
            .unwrap(),
        )
        .unwrap();
    store
        .finish_snapshot(
            &operation.id,
            Outcome::Succeeded(ResultEvidence {
                instance_id: approval.instance_id.clone(),
                provider_scope_id: approval.provider_scope_id.clone(),
                node: approval.node.clone(),
                vmid: 202,
                task_status: TaskStatus::Succeeded,
                snapshot: Snapshot {
                    name: approval.request.name.clone(),
                    description: approval.request.description.clone(),
                    created_at: Some(1700000000),
                    includes_ram: Some(false),
                    parent: None,
                    provider_state: None,
                },
            }),
        )
        .unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute("UPDATE operations SET result_json=json_set(result_json,'$.unexpected','private-extra-field') WHERE id=?1",[&operation.id]).unwrap();
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
        recovered["operation"]["result"]["snapshot"]["includesRam"],
        false
    );
    assert!(!recovered.to_string().contains("private-extra-field"));
    connection.execute("UPDATE operations SET result_json=json_set(result_json,'$.snapshot.name','wrong-snapshot') WHERE id=?1",[&operation.id]).unwrap();
    for request in [
        get(&approval.instance_id, &operation.id),
        history(&approval.instance_id),
    ] {
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
