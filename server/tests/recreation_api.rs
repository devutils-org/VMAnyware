use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition, instance::Registration, provisioning::Catalog, proxmox::Proxmox,
    router_with_execution, router_with_services, store::Store,
};

fn setup() -> (tempfile::TempDir, Store, Catalog, Proxmox, String, Value) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.sqlite");
    let store = Store::open(&path).unwrap();
    let catalog=Catalog::parse(&(include_str!("../../examples/templates.yaml").to_string()+"    imageDefaultUser: {name: debian, templateVersion: debian-13-cloud-20261016, passwordLocked: true, noAuthorizedKeys: true}\n")).unwrap();
    let mut definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    definition.spec.provisioning.packages.clear();
    store.create(&definition).unwrap();
    // Unreachable owned fixture endpoint: HTTP enqueue/recovery must never contact it.
    let provider = Proxmox::new(
        "https://localhost:1",
        "fixture@pve!recreate",
        "fixture-only-secret",
        None,
    )
    .unwrap();
    let instance = store
        .register_instance(
            &Registration {
                definition_name: "kernel-lab".into(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-test".into(),
                vmid: 202,
            },
            &provider.scope_id(),
        )
        .unwrap();
    // Test fixture only. Production retirement requires completed deletion evidence.
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE instances SET status='deleted' WHERE id=?1",
            [&instance.summary.id],
        )
        .unwrap();
    let plan = catalog.plan(&definition, 202).unwrap();
    let id = instance.summary.id;
    let value = json!({"sourceInstanceId":id,"approval":{"definitionName":"kernel-lab","definitionHash":plan.definition_hash,"planHash":plan.plan_hash,"providerScopeId":provider.scope_id(),"targetNode":"pve-test","targetVmid":202,"sourceVmid":9000,"storage":"local-lvm","bridge":"vmbr0","confirmed":true}});
    (dir, store, catalog, provider, id, value)
}
fn request(id: &str, value: Value) -> Request<Body> {
    Request::post(format!("/api/v1/environments/{id}/recreate"))
        .header("content-type", "application/json")
        .header("idempotency-key", "recreate-fixture")
        .body(Body::from(value.to_string()))
        .unwrap()
}
fn get(path: &str) -> Request<Body> {
    Request::get(path).body(Body::empty()).unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap()
}
#[tokio::test]
async fn recreation_api_confirms_once_and_recovers_scoped_after_restart_without_provider() {
    let (dir, store, catalog, provider, id, value) = setup();
    let before = serde_json::to_value(store.instance(&id).unwrap()).unwrap();
    let app = router_with_execution(Some(provider), store.clone(), Some(catalog), false, true);
    let response = app
        .clone()
        .oneshot(request(&id, value.clone()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let location = response.headers()["location"].to_str().unwrap().to_owned();
    let operation = body(response).await;
    let operation_id = operation["id"].as_str().unwrap();
    assert_eq!(operation["kind"], "vm_provision");
    assert_ne!(operation["instanceId"], id);
    let confirmed_at = store
        .provisioning_job(operation_id)
        .unwrap()
        .unwrap()
        .confirmed_at;
    assert_eq!(
        body(app.clone().oneshot(request(&id, value)).await.unwrap()).await,
        operation
    );
    assert_eq!(
        store
            .provisioning_job(operation_id)
            .unwrap()
            .unwrap()
            .confirmed_at,
        confirmed_at
    );
    let recovered = body(app.clone().oneshot(get(&location)).await.unwrap()).await;
    assert_eq!(recovered["sourceInstanceId"], id);
    assert_eq!(
        recovered["plan"]["providerScopeId"],
        before["providerReference"]["scopeId"]
    );
    assert_eq!(recovered["operation"], operation);
    assert_eq!(recovered["restoresPreviousDiskContents"], false);
    let serialized = recovered.to_string();
    for private in [
        "recreate-fixture",
        "fixture-only-secret",
        "confirmed",
        "UPID:",
    ] {
        assert!(!serialized.contains(private));
    }
    let foreign = location.replace(&id, &"a".repeat(32));
    assert_eq!(
        app.oneshot(get(&foreign)).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    drop(store);
    let store = Store::open(dir.path().join("fixture.sqlite")).unwrap();
    let app = router_with_services(None, store.clone());
    assert_eq!(
        body(app.oneshot(get(&location)).await.unwrap()).await,
        recovered
    );
    assert_eq!(
        serde_json::to_value(store.instance(&id).unwrap()).unwrap(),
        before
    );
    assert_eq!(store.recent_instances().unwrap().len(), 2);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
}
#[tokio::test]
async fn recreation_api_requires_runtime_provider_scope_and_catalog() {
    let (_dir, store, catalog, provider, id, value) = setup();
    let disabled = router_with_execution(
        Some(provider.clone()),
        store.clone(),
        Some(catalog.clone()),
        false,
        false,
    );
    assert_eq!(
        disabled
            .oneshot(request(&id, value.clone()))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let missing_provider =
        router_with_execution(None, store.clone(), Some(catalog.clone()), false, true);
    assert_eq!(
        missing_provider
            .oneshot(request(&id, value.clone()))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let missing_catalog =
        router_with_execution(Some(provider.clone()), store.clone(), None, false, true);
    assert_eq!(
        missing_catalog
            .oneshot(request(&id, value.clone()))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let mut changed = value;
    changed["approval"]["providerScopeId"] = json!("a".repeat(64));
    let app = router_with_execution(Some(provider), store.clone(), Some(catalog), false, true);
    assert_eq!(
        app.oneshot(request(&id, changed)).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    assert!(store.recent_operations().unwrap().is_empty());
}
#[tokio::test]
async fn recreation_api_refuses_missing_consent_path_mismatch_origin_duplicate_key_and_target_change(
) {
    let (_dir, store, catalog, provider, id, value) = setup();
    let app = router_with_execution(Some(provider), store.clone(), Some(catalog), false, true);
    let mut unconfirmed = value.clone();
    unconfirmed["approval"]["confirmed"] = json!(false);
    let mut wrong_source = value.clone();
    wrong_source["sourceInstanceId"] = json!("a".repeat(32));
    let mut changed = value.clone();
    changed["approval"]["planHash"] = json!("a".repeat(64));
    for (value, status) in [
        (unconfirmed, StatusCode::BAD_REQUEST),
        (wrong_source, StatusCode::BAD_REQUEST),
        (changed, StatusCode::CONFLICT),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(&id, value))
                .await
                .unwrap()
                .status(),
            status
        );
    }
    let mut origin = request(&id, value.clone());
    origin
        .headers_mut()
        .insert("origin", "https://example.invalid".parse().unwrap());
    assert_eq!(
        app.clone().oneshot(origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let mut duplicate = request(&id, value.clone());
    duplicate
        .headers_mut()
        .append("idempotency-key", "second".parse().unwrap());
    assert_eq!(
        app.clone().oneshot(duplicate).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let mut missing = request(&id, value);
    missing.headers_mut().remove("idempotency-key");
    assert_eq!(
        app.oneshot(missing).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    assert!(store.recent_operations().unwrap().is_empty());
    assert_eq!(store.recent_instances().unwrap().len(), 1);
}

#[tokio::test]
async fn recreation_api_enforces_auth_body_limit_retirement_and_minimum_workflow() {
    use vmanyware_server::auth::{protect, AccessToken};
    let (_dir, store, catalog, provider, id, value) = setup();
    let app = router_with_execution(
        Some(provider.clone()),
        store.clone(),
        Some(catalog),
        false,
        true,
    );
    let protected = protect(
        app.clone(),
        Some(AccessToken::parse("fixture-access-token-0123456789abcdef").unwrap()),
    );
    assert_eq!(
        protected
            .clone()
            .oneshot(request(&id, value.clone()))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        protected
            .oneshot(get(&format!(
                "/api/v1/environments/{id}/recreation-operations/{}",
                "a".repeat(32)
            )))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let oversized = request(
        &id,
        json!({"sourceInstanceId":id,"approval":value["approval"],"extra":"x".repeat(4096)}),
    );
    assert_eq!(
        app.clone().oneshot(oversized).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let active = store
        .register_instance(
            &Registration {
                definition_name: "kernel-lab".into(),
                definition_hash: value["approval"]["definitionHash"].as_str().unwrap().into(),
                node: "pve-test".into(),
                vmid: 203,
            },
            &provider.scope_id(),
        )
        .unwrap();
    let mut active_value = value.clone();
    active_value["sourceInstanceId"] = json!(active.summary.id);
    assert_eq!(
        app.clone()
            .oneshot(request(&active.summary.id, active_value))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let unknown = "a".repeat(32);
    let mut unknown_value = value.clone();
    unknown_value["sourceInstanceId"] = json!(unknown);
    assert_eq!(
        app.oneshot(request(&unknown, unknown_value))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let catalog = Catalog::parse(include_str!("../../examples/templates.yaml")).unwrap();
    let source = store.instance(&id).unwrap().unwrap();
    let definition = Definition::parse(&source.definition_yaml).unwrap();
    let plan = catalog.plan(&definition, 202).unwrap();
    let mut unreviewed = value;
    unreviewed["approval"]["planHash"] = json!(plan.plan_hash);
    let app = router_with_execution(Some(provider), store.clone(), Some(catalog), false, true);
    let response = app.oneshot(request(&id, unreviewed)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body(response).await["error"]["code"],
        "template_user_attestation_required"
    );
    assert!(store.recent_operations().unwrap().is_empty());
}

fn history_path(id: &str) -> String {
    format!("/api/v1/environments/{id}/recreation-operations")
}
#[tokio::test]
async fn recreation_history_is_source_scoped_read_only_and_survives_restart_without_provider() {
    let (dir, store, catalog, _provider, id, value) = setup();
    let approval: vmanyware_server::provisioning_intent::Approval =
        serde_json::from_value(value["approval"].clone()).unwrap();
    let operation = store
        .enqueue_recreation("private-history-key", &id, &approval, &catalog)
        .unwrap();
    let before = serde_json::to_value(
        store
            .provisioning_job(&operation.id)
            .unwrap()
            .unwrap()
            .operation,
    )
    .unwrap();
    let source_before = serde_json::to_value(store.instance(&id).unwrap()).unwrap();
    let source = store.instance(&id).unwrap().unwrap();
    let other = store
        .register_instance(
            &Registration {
                definition_name: source.summary.definition_name,
                definition_hash: source.summary.definition_hash,
                node: "pve-test".into(),
                vmid: 203,
            },
            &approval.provider_scope_id,
        )
        .unwrap();
    let connection = rusqlite::Connection::open(dir.path().join("fixture.sqlite")).unwrap();
    connection
        .execute(
            "UPDATE instances SET status='deleted' WHERE id=?1",
            [&other.summary.id],
        )
        .unwrap();
    drop(connection);
    drop(store);
    let store = Store::open(dir.path().join("fixture.sqlite")).unwrap();
    let app = router_with_services(None, store.clone());
    let response = app.clone().oneshot(get(&history_path(&id))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let history = body(response).await;
    assert_eq!(history["sourceInstanceId"], id);
    assert_eq!(history["operations"].as_array().unwrap().len(), 1);
    assert_eq!(history["operations"][0]["id"], operation.id);
    assert_eq!(
        history["operations"][0]["instanceId"],
        operation.instance_id.unwrap()
    );
    assert_eq!(history["operations"][0]["status"], "queued");
    for private in [
        "private-history-key",
        "confirmed",
        "UPID:",
        "approval",
        "definitionYaml",
    ] {
        assert!(!history.to_string().contains(private));
    }
    let empty = body(
        app.clone()
            .oneshot(get(&history_path(&other.summary.id)))
            .await
            .unwrap(),
    )
    .await;
    assert!(empty["operations"].as_array().unwrap().is_empty());
    assert_eq!(
        app.clone()
            .oneshot(get(&history_path(&"a".repeat(32))))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let location = format!("{}/{}", history_path(&id), operation.id);
    assert_eq!(
        body(app.oneshot(get(&location)).await.unwrap()).await["operation"],
        before
    );
    assert_eq!(
        serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap(),
        before
    );
    assert_eq!(
        serde_json::to_value(store.instance(&id).unwrap()).unwrap(),
        source_before
    );
    assert!(store
        .provisioning_job(&operation.id)
        .unwrap()
        .unwrap()
        .stages
        .iter()
        .all(|stage| !stage.dispatch_started));
}
#[tokio::test]
async fn recreation_history_returns_at_most_one_hundred_newest_source_operations() {
    let (_dir, store, catalog, _provider, id, value) = setup();
    let source = store.instance(&id).unwrap().unwrap();
    let definition = Definition::parse(&source.definition_yaml).unwrap();
    let mut approval: vmanyware_server::provisioning_intent::Approval =
        serde_json::from_value(value["approval"].clone()).unwrap();
    let mut expected = Vec::new();
    for index in 0..103 {
        approval.target_vmid = 1000 + index;
        approval.plan_hash = catalog
            .plan(&definition, approval.target_vmid)
            .unwrap()
            .plan_hash;
        let operation = store
            .enqueue_recreation(&format!("history-{index}"), &id, &approval, &catalog)
            .unwrap();
        expected.push((operation.created_at, operation.id));
    }
    approval.target_vmid = 999;
    approval.plan_hash = catalog.plan(&definition, 999).unwrap().plan_hash;
    let ordinary = store
        .enqueue_provisioning("ordinary-history", &approval, &catalog)
        .unwrap();
    let history = store.recreation_history(&id).unwrap().unwrap();
    assert_eq!(history.operations.len(), 100);
    assert!(history
        .operations
        .iter()
        .all(|entry| entry.id != ordinary.id));
    assert!(history
        .operations
        .windows(2)
        .all(|rows| (&rows[0].created_at, &rows[0].id) >= (&rows[1].created_at, &rows[1].id)));
    expected.sort_by(|a, b| b.cmp(a));
    expected.truncate(100);
    assert_eq!(
        history
            .operations
            .iter()
            .map(|entry| (&entry.created_at, &entry.id))
            .collect::<Vec<_>>(),
        expected.iter().map(|(at, id)| (at, id)).collect::<Vec<_>>()
    );
}
#[tokio::test]
async fn recreation_history_hides_active_sources_and_refuses_corruption_and_missing_auth() {
    use vmanyware_server::auth::{protect, AccessToken};
    let (dir, store, catalog, _provider, id, value) = setup();
    let approval: vmanyware_server::provisioning_intent::Approval =
        serde_json::from_value(value["approval"].clone()).unwrap();
    let operation = store
        .enqueue_recreation("history-corruption", &id, &approval, &catalog)
        .unwrap();
    let app = router_with_services(None, store.clone());
    assert_eq!(
        app.clone()
            .oneshot(get(&history_path(operation.instance_id.as_ref().unwrap())))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let protected = protect(
        app.clone(),
        Some(AccessToken::parse("fixture-access-token-0123456789abcdef").unwrap()),
    );
    assert_eq!(
        protected
            .oneshot(get(&history_path(&id)))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let connection = rusqlite::Connection::open(dir.path().join("fixture.sqlite")).unwrap();
    connection
        .execute(
            "UPDATE operations SET input_hash=?1 WHERE id=?2",
            ["a".repeat(64), operation.id.clone()],
        )
        .unwrap();
    let response = app.clone().oneshot(get(&history_path(&id))).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body(response).await["error"]["code"], "storage_unavailable");
    connection
        .execute(
            "UPDATE operations SET input_hash=?1 WHERE id=?2",
            [approval.input_hash().unwrap(), operation.id.clone()],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE provisioning_intents SET plan_json=?1 WHERE operation_id=?2",
            [
                json!({"PRIVATE_CORRUPT_PLAN":"x".repeat(66000)}).to_string(),
                operation.id,
            ],
        )
        .unwrap();
    let response = app.oneshot(get(&history_path(&id))).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!body(response)
        .await
        .to_string()
        .contains("PRIVATE_CORRUPT_PLAN"));
}
