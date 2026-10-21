use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    auth::{self, AccessToken},
    definition::Definition,
    operation::Failure,
    provisioning::Catalog,
    provisioning_intent::{Approval, CloneOutcome, CloneResult},
    proxmox::{CloneObservation, PowerState, TaskReference, TaskStatus, VmObservation},
    router_with_services,
    store::Store,
};
const TOKEN: &str = "owned-progress-test-token-000000000000";
fn setup(store: &Store) -> (Approval, Catalog, String) {
    let mut definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    definition.metadata.description = "PRIVATE_DEFINITION_NOTE".into();
    definition.spec.lifecycle.snapshot_after_provisioning = true;
    store.create(&definition).unwrap();
    let catalog = Catalog::parse(include_str!("../../examples/templates.yaml")).unwrap();
    let plan = catalog.plan(&definition, 202).unwrap();
    let approval = Approval {
        definition_name: definition.metadata.name,
        definition_hash: plan.definition_hash,
        provider_scope_id: "a".repeat(64),
        plan_hash: plan.plan_hash,
        target_node: "pve-test".into(),
        target_vmid: 202,
        source_vmid: 9000,
        storage: "local-lvm".into(),
        bridge: "vmbr0".into(),
        confirmed: true,
    };
    let operation = store
        .enqueue_provisioning("PRIVATE_REQUEST_KEY", &approval, &catalog)
        .unwrap();
    (approval, catalog, operation.id)
}
fn app(store: Store) -> axum::Router {
    auth::protect(
        router_with_services(None, store),
        Some(AccessToken::parse(TOKEN).unwrap()),
    )
}
fn request(id: &str) -> Request<Body> {
    Request::get(format!("/api/v1/operations/{id}/provisioning"))
        .header("authorization", format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}
fn assert_projection(value: &Value) {
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "definitionHash",
            "failure",
            "instanceId",
            "node",
            "operationId",
            "planHash",
            "providerScopeId",
            "sourceInstanceId",
            "stages",
            "status",
            "vmid"
        ]
    );
    for stage in value["stages"].as_array().unwrap() {
        let mut keys = stage
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            vec!["kind", "position", "status", "submissionStarted"]
        );
    }
    let text = value.to_string();
    for private in [
        "PRIVATE_",
        "requestKey",
        "approval",
        "UPID:",
        "pid",
        "seed",
        "catalogYaml",
        "resultJson",
        "definitionYaml",
    ] {
        assert!(!text.contains(private), "Leaked {private}");
    }
}
#[tokio::test]
async fn progress_reports_ordered_safe_recorded_steps_without_provider_or_write_gates() {
    let store = Store::memory().unwrap();
    let (approval, _, id) = setup(&store);
    let before = serde_json::to_value(store.operation(&id).unwrap().unwrap()).unwrap();
    let response = app(store.clone()).oneshot(request(&id)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value = body(response).await;
    assert_projection(&value);
    assert_eq!(value["operationId"], id);
    assert_eq!(value["definitionHash"], approval.definition_hash);
    assert_eq!(value["planHash"], approval.plan_hash);
    assert_eq!(value["providerScopeId"], approval.provider_scope_id);
    assert_eq!(value["node"], "pve-test");
    assert_eq!(value["vmid"], 202);
    assert!(value["sourceInstanceId"].is_null());
    let expected = [
        "full_clone",
        "configure_resources",
        "expand_disk",
        "configure_cloud_init",
        "start_vm",
        "wait_for_guest_readiness",
        "install_declared_packages",
        "snapshot_after_provisioning",
    ];
    let stages = value["stages"].as_array().unwrap();
    assert_eq!(stages.len(), expected.len());
    for (i, stage) in stages.iter().enumerate() {
        assert_eq!(stage["position"], i);
        assert_eq!(stage["kind"], expected[i]);
        assert_eq!(stage["status"], "queued");
        assert_eq!(stage["submissionStarted"], false);
    }
    assert_eq!(
        serde_json::to_value(store.operation(&id).unwrap().unwrap()).unwrap(),
        before
    );
}
#[tokio::test]
async fn progress_is_authenticated_get_only_and_unknown_or_nonprovision_ids_are_not_found() {
    let store = Store::memory().unwrap();
    let (_, _, id) = setup(&store);
    let inventory = store.enqueue_inventory("other-kind").unwrap();
    let router = app(store);
    let response = router
        .clone()
        .oneshot(
            Request::get(format!("/api/v1/operations/{id}/provisioning"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    for id in ["invalid".into(), "f".repeat(32), inventory.id] {
        let response = router.clone().oneshot(request(&id)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let mut post = request(&id);
    *post.method_mut() = axum::http::Method::POST;
    assert_eq!(
        router.oneshot(post).await.unwrap().status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
}
#[tokio::test]
async fn progress_retains_uncertainty_after_restart_without_repairing_or_replaying() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fixture.sqlite");
    let store = Store::open(&path).unwrap();
    let (approval, _, id) = setup(&store);
    store.claim_provisioning_clone().unwrap().unwrap();
    store.begin_provisioning_clone(&id, &approval).unwrap();
    drop(store);
    let reopened = Store::open(&path).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    let before = serde_json::to_value(reopened.operation(&id).unwrap().unwrap()).unwrap();
    let response = app(reopened.clone()).oneshot(request(&id)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value = body(response).await;
    assert_projection(&value);
    assert_eq!(value["status"], "uncertain");
    assert_eq!(value["stages"][0]["status"], "uncertain");
    assert_eq!(value["stages"][0]["submissionStarted"], true);
    assert_eq!(
        serde_json::to_value(reopened.operation(&id).unwrap().unwrap()).unwrap(),
        before
    );
}
#[tokio::test]
async fn progress_exposes_failure_and_completed_prefix_but_not_private_stage_results() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fixture.sqlite");
    let store = Store::open(&path).unwrap();
    let (approval, _, id) = setup(&store);
    let job = store.claim_provisioning_clone().unwrap().unwrap();
    store.begin_provisioning_clone(&id, &approval).unwrap();
    let task = TaskReference::new(
        "pve-test",
        "UPID:pve-test:00000001:00000002:00000003:qmclone:9000:fixture@pve:",
    )
    .unwrap();
    store.link_provisioning_clone(&id, &task).unwrap();
    store
        .finish_provisioning_clone(
            &id,
            CloneOutcome::Completed(CloneResult {
                instance_id: job.operation.instance_id.unwrap(),
                provider_scope_id: approval.provider_scope_id,
                plan_hash: approval.plan_hash,
                task_status: TaskStatus::Succeeded,
                target: CloneObservation {
                    config_digest: "d".repeat(40),
                    observation: VmObservation {
                        node: "pve-test".into(),
                        vmid: 202,
                        status: PowerState::Stopped,
                        qmp_status: None,
                    },
                },
            }),
        )
        .unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    let result: String = db
        .query_row(
            "SELECT result_json FROM provisioning_stages WHERE operation_id=?1 AND position=0",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    let mut result: Value = serde_json::from_str(&result).unwrap();
    result["stderr"] = json!("PRIVATE_GUEST_OUTPUT");
    db.execute(
        "UPDATE provisioning_stages SET result_json=?2 WHERE operation_id=?1 AND position=0",
        rusqlite::params![id, result.to_string()],
    )
    .unwrap();
    let value = body(app(store.clone()).oneshot(request(&id)).await.unwrap()).await;
    assert_projection(&value);
    assert_eq!(value["status"], "running");
    assert_eq!(value["stages"][0]["status"], "succeeded");
    assert_eq!(value["stages"][1]["status"], "queued");
    // A second independent fixture records a failure before any dispatch.
    let failed = Store::memory().unwrap();
    let (_, _, id) = setup(&failed);
    failed.claim_provisioning_clone().unwrap().unwrap();
    failed
        .finish_provisioning_clone(&id, CloneOutcome::Failed(Failure::ProvisioningDisabled))
        .unwrap();
    let value = body(app(failed).oneshot(request(&id)).await.unwrap()).await;
    assert_eq!(value["status"], "failed");
    assert_eq!(value["failure"], "provisioning_disabled");
    assert_eq!(value["stages"][0]["submissionStarted"], false);
}
#[tokio::test]
async fn progress_refuses_corrupt_order_and_contradictory_completion_without_changing_rows() {
    for variant in ["order", "completion"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture.sqlite");
        let store = Store::open(&path).unwrap();
        let (_, _, id) = setup(&store);
        let db = rusqlite::Connection::open(&path).unwrap();
        if variant == "order" {
            db.execute(
                "UPDATE provisioning_stages SET position=99 WHERE operation_id=?1 AND position=7",
                [&id],
            )
            .unwrap();
        } else {
            db.execute("UPDATE provisioning_stages SET status='succeeded',result_json='{}' WHERE operation_id=?1 AND position=0",[&id]).unwrap();
        }
        let before = serde_json::to_value(store.operation(&id).unwrap().unwrap()).unwrap();
        let response = app(store.clone()).oneshot(request(&id)).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let error = body(response).await;
        assert_eq!(error["error"]["code"], "storage_unavailable");
        assert!(!error.to_string().contains("PRIVATE_"));
        assert_eq!(
            serde_json::to_value(store.operation(&id).unwrap().unwrap()).unwrap(),
            before
        );
    }
}
