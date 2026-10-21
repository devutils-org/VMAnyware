use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    lifecycle::{Action, Approval, Outcome, ResultEvidence},
    operation::Failure,
    proxmox::{TaskReference, TaskStatus, VmObservation},
    router_with_services,
    store::Store,
};
fn setup(store: &Store, vmid: u32) -> Approval {
    let d = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    if store.get(&d.metadata.name).unwrap().is_none() {
        store.create(&d).unwrap();
    }
    let instance = store
        .register_instance(
            &Registration {
                definition_name: d.metadata.name.clone(),
                definition_hash: d.hash().unwrap(),
                node: "saved-node".into(),
                vmid,
            },
            &"a".repeat(64),
        )
        .unwrap();
    Approval {
        action: Action::Start,
        instance_id: instance.summary.id,
        provider_scope_id: "a".repeat(64),
        definition_hash: d.hash().unwrap(),
        node: "saved-node".into(),
        vmid,
        config_digest: "b".repeat(40),
        confirmed: true,
    }
}
fn get(instance: &str, operation: Option<&str>) -> Request<Body> {
    Request::get(format!(
        "/api/v1/environments/{instance}/lifecycle-operations{}",
        operation.map(|id| format!("/{id}")).unwrap_or_default()
    ))
    .body(Body::empty())
    .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
fn finish(store: &Store, approval: &Approval, key: &str) -> String {
    let op = store.enqueue_lifecycle(key, approval).unwrap();
    store.claim_lifecycle().unwrap().unwrap();
    store.begin_lifecycle_dispatch(&op.id, approval).unwrap();
    let task = TaskReference::new(
        &approval.node,
        &format!(
            "UPID:saved-node:00000001:00000002:00000003:qm{}:{}:fixture@pve:",
            if approval.action == Action::Start {
                "start"
            } else {
                "shutdown"
            },
            approval.vmid
        ),
    )
    .unwrap();
    store.link_lifecycle_task(&op.id, &task).unwrap();
    store
        .finish_lifecycle(
            &op.id,
            Outcome::Succeeded(ResultEvidence {
                action: approval.action,
                instance_id: approval.instance_id.clone(),
                provider_scope_id: approval.provider_scope_id.clone(),
                task_status: TaskStatus::Succeeded,
                observation: VmObservation {
                    node: approval.node.clone(),
                    vmid: approval.vmid,
                    status: approval.action.desired_power(),
                    qmp_status: None,
                },
            }),
        )
        .unwrap();
    op.id
}
#[tokio::test]
async fn lifecycle_recovery_reopens_with_writes_off_no_provider_and_no_restored_consent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.sqlite");
    let store = Store::open(&path).unwrap();
    let mut approval = setup(&store, 202);
    let first = finish(&store, &approval, "saved-start");
    approval.action = Action::Shutdown;
    let second = finish(&store, &approval, "saved-shutdown");
    let before = serde_json::to_value(store.recent_operations().unwrap()).unwrap();
    // Archived-association fixture only; no provider deletion is executed.
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE instances SET status='deleted' WHERE id=?1",
            [&approval.instance_id],
        )
        .unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    let app = router_with_services(None, store.clone());
    for id in [&first, &second] {
        let response = app
            .clone()
            .oneshot(get(&approval.instance_id, Some(id)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let recovered = body(response).await;
        assert_eq!(recovered["operation"]["id"], *id);
        assert_eq!(recovered["operation"]["status"], "succeeded");
        assert_eq!(
            recovered["target"]["providerScopeId"],
            approval.provider_scope_id
        );
        assert_eq!(recovered["target"]["vmid"], 202);
        assert_eq!(recovered["submissionStarted"], true);
        assert!(
            recovered.get("approval").is_none()
                && recovered.get("confirmed").is_none()
                && recovered["target"].get("configDigest").is_none()
        );
    }
    let history = body(app.oneshot(get(&approval.instance_id, None)).await.unwrap()).await;
    assert_eq!(history["operations"].as_array().unwrap().len(), 2);
    assert_eq!(
        serde_json::to_value(store.recent_operations().unwrap()).unwrap(),
        before
    );
}
#[tokio::test]
async fn lifecycle_recovery_keeps_queued_running_and_unknown_outcomes_without_claiming() {
    for mode in 0..3 {
        let store = Store::memory().unwrap();
        let approval = setup(&store, 202);
        let op = store.enqueue_lifecycle("retained", &approval).unwrap();
        if mode > 0 {
            store.claim_lifecycle().unwrap().unwrap();
        }
        if mode == 2 {
            store.begin_lifecycle_dispatch(&op.id, &approval).unwrap();
            store
                .finish_lifecycle(
                    &op.id,
                    Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed),
                )
                .unwrap();
        }
        let before = serde_json::to_value(store.operation(&op.id).unwrap().unwrap()).unwrap();
        let app = router_with_services(None, store.clone());
        let saved = body(
            app.oneshot(get(&approval.instance_id, Some(&op.id)))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(saved["operation"], before);
        assert_eq!(saved["submissionStarted"], mode == 2);
        assert_eq!(
            serde_json::to_value(store.operation(&op.id).unwrap().unwrap()).unwrap(),
            before
        );
    }
}
#[tokio::test]
async fn lifecycle_recovery_is_instance_and_kind_scoped_and_requires_auth_when_wrapped() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let other = setup(&store, 203);
    let op = store.enqueue_lifecycle("saved", &approval).unwrap();
    let inventory = store.enqueue_inventory("inventory").unwrap();
    let app = router_with_services(None, store);
    for (instance, id) in [
        (&other.instance_id[..], &op.id[..]),
        (&approval.instance_id, &inventory.id),
        ("invalid", &op.id),
        (&approval.instance_id, "invalid"),
        (&approval.instance_id, &"f".repeat(32)),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(get(instance, Some(id)))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        body(
            app.clone()
                .oneshot(get(&other.instance_id, None))
                .await
                .unwrap()
        )
        .await["operations"],
        json!([])
    );
    assert_eq!(
        app.clone()
            .oneshot(get(&"f".repeat(32), None))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let token =
        vmanyware_server::auth::AccessToken::parse("fixture-only-access-token-0123456789abcdef")
            .unwrap();
    let protected = vmanyware_server::auth::protect(app, Some(token));
    for id in [None, Some(op.id.as_str())] {
        assert_eq!(
            protected
                .clone()
                .oneshot(get(&approval.instance_id, id))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
#[tokio::test]
async fn lifecycle_recovery_history_filters_before_limit_and_sorts_latest_hundred() {
    let store = Store::memory().unwrap();
    let approval = setup(&store, 202);
    let mut other = setup(&store, 203);
    let old = store.enqueue_lifecycle("first", &approval).unwrap();
    store.claim_lifecycle().unwrap();
    store
        .finish_lifecycle(&old.id, Outcome::Failed(Failure::LifecycleDisabled))
        .unwrap();
    for index in 0..105 {
        other.action = if index % 2 == 0 {
            Action::Start
        } else {
            Action::Shutdown
        };
        let op = store
            .enqueue_lifecycle(&format!("other-{index}"), &other)
            .unwrap();
        store.claim_lifecycle().unwrap();
        store
            .finish_lifecycle(&op.id, Outcome::Failed(Failure::LifecycleDisabled))
            .unwrap();
    }
    let app = router_with_services(None, store);
    let first = body(
        app.clone()
            .oneshot(get(&approval.instance_id, None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first["operations"].as_array().unwrap().len(), 1);
    assert_eq!(first["operations"][0]["id"], old.id);
    let saved = body(app.oneshot(get(&other.instance_id, None)).await.unwrap()).await;
    let entries = saved["operations"].as_array().unwrap();
    assert_eq!(entries.len(), 100);
    assert!(entries.windows(2).all(|pair| (
        pair[0]["createdAt"].as_str().unwrap(),
        pair[0]["id"].as_str().unwrap()
    ) > (
        pair[1]["createdAt"].as_str().unwrap(),
        pair[1]["id"].as_str().unwrap()
    )));
}
#[tokio::test]
async fn lifecycle_recovery_corrupt_or_oversized_approval_fails_closed_without_cross_instance_leak()
{
    for oversized in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupt.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store, 202);
        let other = setup(&store, 203);
        let op = store.enqueue_lifecycle("corrupt", &approval).unwrap();
        let bad = if oversized {
            "private-detail".repeat(1000)
        } else {
            "private-detail".into()
        };
        rusqlite::Connection::open(path)
            .unwrap()
            .execute(
                "UPDATE operations SET approval_json=json_set(approval_json,'$.unexpected',?1) WHERE id=?2",
                rusqlite::params![bad, op.id],
            )
            .unwrap();
        let app = router_with_services(None, store);
        assert_eq!(
            app.clone()
                .oneshot(get(&other.instance_id, Some(&op.id)))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        for id in [None, Some(op.id.as_str())] {
            let response = app
                .clone()
                .oneshot(get(&approval.instance_id, id))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert!(!body(response).await.to_string().contains("private-detail"));
        }
    }
}
#[tokio::test]
async fn lifecycle_recovery_sanitizes_saved_success_and_refuses_changed_power_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("evidence.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store, 202);
    let id = finish(&store, &approval, "success");
    let connection = rusqlite::Connection::open(path).unwrap();
    connection.execute("UPDATE operations SET result_json=json_set(result_json,'$.unexpected','private-extra') WHERE id=?1",[&id]).unwrap();
    let app = router_with_services(None, store);
    let response = app
        .clone()
        .oneshot(get(&approval.instance_id, Some(&id)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!body(response).await.to_string().contains("private-extra"));
    connection.execute("UPDATE operations SET result_json=json_set(result_json,'$.observation.status','stopped') WHERE id=?1",[&id]).unwrap();
    for op in [None, Some(id.as_str())] {
        assert_eq!(
            app.clone()
                .oneshot(get(&approval.instance_id, op))
                .await
                .unwrap()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
