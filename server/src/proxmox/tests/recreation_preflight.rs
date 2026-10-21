//! Source-bound read-only recreation observations over generated verified HTTPS.
use super::*;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

fn source(
    store: &crate::store::Store,
    path: &std::path::Path,
    provider: &Proxmox,
) -> (String, crate::definition::Definition) {
    let mut definition =
        crate::definition::Definition::parse(include_str!("../../../../examples/kernel-lab.yaml"))
            .unwrap();
    definition.spec.provisioning.packages.clear();
    store.create(&definition).unwrap();
    let instance = store
        .register_instance(
            &crate::instance::Registration {
                definition_name: "kernel-lab".into(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-test".into(),
                vmid: 202,
            },
            &provider.scope_id(),
        )
        .unwrap();
    // Owned retired fixture; this read-only test performs no deletion.
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "UPDATE instances SET status='deleted' WHERE id=?1",
            [&instance.summary.id],
        )
        .unwrap();
    (instance.summary.id, definition)
}
fn catalog() -> crate::provisioning::Catalog {
    crate::provisioning::Catalog::parse(&(include_str!("../../../../examples/templates.yaml").to_string()+"    imageDefaultUser: {name: debian, templateVersion: debian-13-cloud-20261016, passwordLocked: true, noAuthorizedKeys: true}\n")).unwrap()
}
fn request(id: &str) -> Request<Body> {
    Request::post(format!("/api/v1/environments/{id}/recreate-preflight"))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"targetVmid":203}"#))
        .unwrap()
}
async fn body(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap()
}
async fn fixture(available: bool) -> Fixture {
    Fixture::routes("200 OK",&[
        ("/api2/json/nodes/pve-test/qemu/9000/config?current=1",r#"{"data":{"template":1,"digest":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","scsi0":"local-lvm:base-9000-disk-0,size=8G","ide2":"local-lvm:vm-9000-cloudinit,media=cdrom"}}"#),
        ("/api2/json/nodes/pve-test/storage",r#"{"data":[{"storage":"local-lvm","content":"images","active":1,"enabled":1,"avail":100000000000}]}"#),
        ("/api2/json/nodes/pve-test/network",r#"{"data":[{"iface":"vmbr0","type":"bridge","active":1}]}"#),
        ("/api2/json/cluster/nextid?vmid=203",if available {r#"{"data":203}"#}else{r#"{"data":204}"#}),
    ],"",Duration::ZERO).await
}
#[tokio::test]
async fn recreation_preflight_binds_saved_source_plan_and_runtime_without_mutating() {
    for enabled in [false, true] {
        let mut fixture = fixture(true).await;
        let provider = fixture.client();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db.sqlite");
        let store = crate::store::Store::open(&path).unwrap();
        let (id, definition) = source(&store, &path, &provider);
        let before = serde_json::to_value(store.instance(&id).unwrap()).unwrap();
        let catalog = catalog();
        let expected = catalog.plan(&definition, 203).unwrap().plan_hash;
        let app = crate::router_with_execution(
            Some(provider.clone()),
            store.clone(),
            Some(catalog),
            false,
            enabled,
        );
        let response = app.oneshot(request(&id)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value = body(response).await;
        assert_eq!(value["sourceInstanceId"], id);
        assert_eq!(value["definitionYaml"], before["definitionYaml"]);
        assert_eq!(value["sourceDefinitionHash"], before["definitionHash"]);
        assert_eq!(value["plan"]["planHash"], expected);
        assert_eq!(value["preflight"]["planHash"], expected);
        assert_eq!(value["plan"]["providerScopeId"], provider.scope_id());
        assert_eq!(value["recreationEnabled"], enabled);
        assert_eq!(value["plan"]["creationEnabled"], enabled);
        assert_eq!(value["preflight"]["creationEnabled"], enabled);
        assert_eq!(value["preflight"]["vmidReserved"], false);
        assert_eq!(value["plan"]["liveVerified"], false);
        assert_eq!(value["restoresPreviousDiskContents"], false);
        assert_eq!(value["requiresNewInstanceIdentity"], true);
        let checks = value["preflight"]["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 10);
        assert!(checks[..7].iter().all(|check| check["status"] == "passed"));
        assert!(checks[7..].iter().all(|check| check["status"] == "unknown"));
        for _ in 0..4 {
            assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        }
        assert!(fixture.requests.try_recv().is_err());
        assert_eq!(
            serde_json::to_value(store.instance(&id).unwrap()).unwrap(),
            before
        );
        assert!(store.recent_operations().unwrap().is_empty());
        assert_eq!(store.recent_instances().unwrap().len(), 1);
    }
}
#[tokio::test]
async fn recreation_preflight_busy_target_fails_observation_without_granting_reservation() {
    let mut fixture = fixture(false).await;
    let provider = fixture.client();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite");
    let store = crate::store::Store::open(&path).unwrap();
    let (id, _) = source(&store, &path, &provider);
    let app =
        crate::router_with_execution(Some(provider), store.clone(), Some(catalog()), false, true);
    let value = body(app.oneshot(request(&id)).await.unwrap()).await;
    assert_eq!(value["preflight"]["checks"][6]["code"], "target_vmid");
    assert_eq!(value["preflight"]["checks"][6]["status"], "failed");
    assert_eq!(value["preflight"]["vmidReserved"], false);
    // Capability denotes runtime support, not passing checks or permission to execute.
    assert_eq!(value["recreationEnabled"], true);
    assert!(store.recent_operations().unwrap().is_empty());
    for _ in 0..4 {
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
    }
}
#[tokio::test]
async fn recreation_preflight_denied_reads_are_unknown_and_redacted() {
    let mut fixture = Fixture::new(
        "403 Forbidden",
        r#"{"data":"fixture-private-provider-detail"}"#,
        "",
        Duration::ZERO,
    )
    .await;
    let provider = fixture.client();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite");
    let store = crate::store::Store::open(&path).unwrap();
    let (id, _) = source(&store, &path, &provider);
    let app =
        crate::router_with_execution(Some(provider), store.clone(), Some(catalog()), false, true);
    let response = app.oneshot(request(&id)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value = body(response).await;
    assert!(value["preflight"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|check| check["status"] == "unknown"));
    assert!(!value
        .to_string()
        .contains("fixture-private-provider-detail"));
    assert!(store.recent_operations().unwrap().is_empty());
    for _ in 0..4 {
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
    }
}
