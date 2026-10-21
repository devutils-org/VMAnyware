use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition, instance::Registration, provisioning::Catalog, router_with_catalog,
    store::Store,
};

const YAML: &str = include_str!("../../examples/kernel-lab.yaml");
const CATALOG: &str = include_str!("../../examples/templates.yaml");

fn setup(deleted: bool) -> (tempfile::TempDir, Store, String) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fixture.sqlite");
    let store = Store::open(&path).unwrap();
    let definition = Definition::parse(YAML).unwrap();
    store.create(&definition).unwrap();
    let instance = store
        .register_instance(
            &Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-test".into(),
                vmid: 202,
            },
            &"b".repeat(64),
        )
        .unwrap();
    if deleted {
        // Fixture retirement only; production retirement requires task/absence proof.
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE instances SET status='deleted' WHERE id=?1",
                [&instance.summary.id],
            )
            .unwrap();
    }
    (directory, store, instance.summary.id)
}
fn request(id: &str, value: Value) -> Request<Body> {
    Request::post(format!("/api/v1/environments/{id}/recreate-plan"))
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap()
}
#[tokio::test]
async fn recreation_preview_uses_exact_retained_definition_without_provider_or_writes() {
    let (_directory, store, id) = setup(true);
    let before = serde_json::to_value(store.instance(&id).unwrap()).unwrap();
    let catalog = Catalog::parse(CATALOG).unwrap();
    let expected = serde_json::to_value(
        catalog
            .plan(&Definition::parse(YAML).unwrap(), 203)
            .unwrap(),
    )
    .unwrap();
    let app = router_with_catalog(None, store.clone(), Some(catalog));
    let response = app
        .clone()
        .oneshot(request(&id, json!({"targetVmid":203})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value = body(response).await;
    assert_eq!(value["sourceInstanceId"], id);
    assert_eq!(value["sourceDefinitionHash"], before["definitionHash"]);
    assert_eq!(value["definitionYaml"], before["definitionYaml"]);
    assert_eq!(value["plan"], expected);
    assert_eq!(value["requiresNewInstanceIdentity"], true);
    for flag in ["recreationEnabled", "restoresPreviousDiskContents"] {
        assert_eq!(value[flag], false);
    }
    for flag in ["liveVerified", "vmidReserved", "creationEnabled"] {
        assert_eq!(value["plan"][flag], false);
    }
    let retry = body(
        app.clone()
            .oneshot(request(&id, json!({"targetVmid":203})))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(retry, value);
    // Even the retired VMID is only a proposal; no availability/reservation claim.
    assert_eq!(
        app.oneshot(request(&id, json!({"targetVmid":202})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        serde_json::to_value(store.instance(&id).unwrap()).unwrap(),
        before
    );
    assert_eq!(store.recent_instances().unwrap().len(), 1);
    assert!(store.recent_operations().unwrap().is_empty());
    assert_eq!(
        store.get("kernel-lab").unwrap().unwrap().hash,
        before["definitionHash"]
    );
}
#[tokio::test]
async fn recreation_preview_refuses_active_unknown_invalid_origin_and_missing_catalog() {
    let (_directory, store, id) = setup(false);
    let app = router_with_catalog(None, store, Some(Catalog::parse(CATALOG).unwrap()));
    assert_eq!(
        app.clone()
            .oneshot(request(&id, json!({"targetVmid":203})))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    for unknown in ["bad".to_string(), "a".repeat(32)] {
        assert_eq!(
            app.clone()
                .oneshot(request(&unknown, json!({"targetVmid":203})))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let (_directory, store, id) = setup(true);
    let app = router_with_catalog(None, store.clone(), Some(Catalog::parse(CATALOG).unwrap()));
    for value in [json!({"targetVmid":0}), json!({"targetVmid":9000})] {
        assert_eq!(
            app.clone()
                .oneshot(request(&id, value))
                .await
                .unwrap()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for value in [
        json!({"targetVmid":203,"yaml":YAML}),
        json!({}),
        json!({"targetVmid":"203"}),
    ] {
        assert!(!app
            .clone()
            .oneshot(request(&id, value))
            .await
            .unwrap()
            .status()
            .is_success());
    }
    let mut origin = request(&id, json!({"targetVmid":203}));
    origin
        .headers_mut()
        .insert("origin", "https://example.invalid".parse().unwrap());
    assert_eq!(
        app.oneshot(origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        router_with_catalog(None, store, None)
            .oneshot(request(&id, json!({"targetVmid":203})))
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn recreation_preview_rejects_changed_retained_yaml_and_oversized_body() {
    let (directory, store, id) = setup(false);
    let app = router_with_catalog(None, store.clone(), Some(Catalog::parse(CATALOG).unwrap()));
    let oversized = request(&id, json!({"targetVmid":203,"unexpected":"x".repeat(2048)}));
    assert_eq!(
        app.clone().oneshot(oversized).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    rusqlite::Connection::open(directory.path().join("fixture.sqlite"))
        .unwrap()
        .execute(
            "UPDATE instances SET definition_yaml=?1, status='deleted' WHERE id=?2",
            [YAML.replace("8192", "4096"), id.clone()],
        )
        .unwrap();
    let response = app
        .oneshot(request(&id, json!({"targetVmid":203})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["error"]["code"],
        "retained_definition_invalid"
    );
    assert!(store.recent_operations().unwrap().is_empty());
}

#[tokio::test]
async fn recreation_live_preflight_validates_source_inputs_and_missing_provider_before_observation()
{
    let (_directory, store, id) = setup(false);
    let app = router_with_catalog(None, store.clone(), Some(Catalog::parse(CATALOG).unwrap()));
    let live = |id: &str, value: Value| {
        let mut request = request(id, value);
        *request.uri_mut() = format!("/api/v1/environments/{id}/recreate-preflight")
            .parse()
            .unwrap();
        request
    };
    assert_eq!(
        app.clone()
            .oneshot(live(&id, json!({"targetVmid":203})))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        app.clone()
            .oneshot(live(&"a".repeat(32), json!({"targetVmid":203})))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let (_directory, store, id) = setup(true);
    let app = router_with_catalog(None, store.clone(), Some(Catalog::parse(CATALOG).unwrap()));
    let response = app
        .clone()
        .oneshot(live(&id, json!({"targetVmid":203})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body(response).await["error"]["code"],
        "provider_not_configured"
    );
    assert!(!app
        .clone()
        .oneshot(live(&id, json!({"targetVmid":203,"yaml":YAML})))
        .await
        .unwrap()
        .status()
        .is_success());
    let mut origin = live(&id, json!({"targetVmid":203}));
    origin
        .headers_mut()
        .insert("origin", "https://example.invalid".parse().unwrap());
    assert_eq!(
        app.oneshot(origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert!(store.recent_operations().unwrap().is_empty());
}
