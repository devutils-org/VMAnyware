use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition,
    provisioning::{Catalog, Error},
    router_with_catalog,
    store::Store,
};

const CATALOG: &str = include_str!("../../examples/templates.yaml");
const DEFINITION: &str = include_str!("../../examples/kernel-lab.yaml");

#[test]
fn preview_binds_explicit_mapping_definition_and_target_without_success_claims() {
    let catalog = Catalog::parse(CATALOG).unwrap();
    let definition = Definition::parse(DEFINITION).unwrap();
    let plan = catalog.plan(&definition, 201).unwrap();
    assert_eq!(plan.definition_hash, definition.hash().unwrap());
    assert_eq!(
        plan.plan_hash,
        catalog.plan(&definition, 201).unwrap().plan_hash
    );
    assert_ne!(
        plan.plan_hash,
        catalog.plan(&definition, 202).unwrap().plan_hash
    );
    assert_ne!(
        plan.plan_hash,
        Catalog::parse(&CATALOG.replace("local-lvm", "test-storage"))
            .unwrap()
            .plan(&definition, 201)
            .unwrap()
            .plan_hash
    );
    assert_ne!(
        plan.plan_hash,
        Catalog::parse(&CATALOG.replace("20261016", "20261017"))
            .unwrap()
            .plan(&definition, 201)
            .unwrap()
            .plan_hash
    );
    let value = serde_json::to_value(plan).unwrap();
    assert_eq!(value["target"]["node"], "pve-test");
    assert_eq!(value["source"]["templateVmid"], 9000);
    assert_eq!(value["target"]["resources"]["memoryMiB"], 8192);
    assert_eq!(value["target"]["resources"]["diskGiB"], 40);
    assert_eq!(value["source"]["sourceDiskGiB"], 8);
    assert_eq!(value["target"]["ipv4"], "dhcp");
    for field in ["liveVerified", "vmidReserved", "creationEnabled"] {
        assert_eq!(value[field], false);
    }
    assert!(value["steps"]
        .as_array()
        .unwrap()
        .contains(&json!("expand_disk")));
    assert_eq!(value["sshKeyCount"], 1);
    assert!(!value.to_string().contains("AAAAC3"));
}

#[test]
fn incomplete_ambiguous_or_invalid_catalogs_are_refused_without_echoing_values() {
    let duplicate = format!("{CATALOG}{}", CATALOG.split("images:\n").nth(1).unwrap());
    for yaml in [
        CATALOG.replace("    guestUser: debian\n", ""),
        CATALOG.replace(
            "guestUser: debian",
            "guestUser: debian\n    secret: never-echo-me",
        ),
        CATALOG.replace("vmanyware.dev/v1", "vmanyware.dev/v99"),
        CATALOG.replace("templateVmid: 9000", "templateVmid: 0"),
        CATALOG.replace(
            "templateVersion: debian-13-cloud-20261016",
            "templateVersion: ''",
        ),
        CATALOG.replace("node: pve-test", "node: ../pve"),
        CATALOG.replace("bridge: vmbr0", "bridge: 'vmbr0; command'"),
        CATALOG.replace("sourceDiskGiB: 8", "sourceDiskGiB: 0"),
        CATALOG.replace("diskDevice: scsi0", "diskDevice: arbitrary"),
        CATALOG.replace("guestUser: debian", "guestUser: 'root; command'"),
        duplicate,
        "x".repeat(65537),
    ] {
        let error = Catalog::parse(&yaml).unwrap_err();
        assert_eq!(error, Error::InvalidCatalog);
        assert!(!error.to_string().contains("never-echo-me"));
    }
}

#[test]
fn plan_refuses_unknown_images_shrinking_and_reusing_the_source_vmid() {
    let catalog = Catalog::parse(CATALOG).unwrap();
    let definition = Definition::parse(DEFINITION).unwrap();
    for vmid in [0, 99, 9000, 1_000_000_000] {
        assert_eq!(
            catalog.plan(&definition, vmid).unwrap_err(),
            Error::InvalidVmid
        );
    }
    let missing =
        Definition::parse(&DEFINITION.replace("image: debian-13", "image: other")).unwrap();
    assert_eq!(
        catalog.plan(&missing, 201).unwrap_err(),
        Error::ImageNotMapped
    );
    let smaller = Definition::parse(&DEFINITION.replace("diskGiB: 40", "diskGiB: 4")).unwrap();
    assert_eq!(catalog.plan(&smaller, 201).unwrap_err(), Error::DiskShrink);
    let same = Definition::parse(&DEFINITION.replace("diskGiB: 40", "diskGiB: 8")).unwrap();
    let plan = serde_json::to_value(catalog.plan(&same, 201).unwrap()).unwrap();
    assert!(!plan["steps"]
        .as_array()
        .unwrap()
        .contains(&json!("expand_disk")));
}

#[test]
fn canonical_plan_hash_ignores_yaml_formatting_but_tracks_declared_inputs() {
    let catalog = Catalog::parse(CATALOG).unwrap();
    let definition = Definition::parse(DEFINITION).unwrap();
    let canonical = Definition::parse(&definition.canonical_yaml().unwrap()).unwrap();
    let plan = catalog.plan(&definition, 201).unwrap();
    assert_eq!(
        plan.plan_hash,
        catalog.plan(&canonical, 201).unwrap().plan_hash
    );
    let changed = Definition::parse(&DEFINITION.replace("cpu: 4", "cpu: 8")).unwrap();
    assert_ne!(
        plan.plan_hash,
        catalog.plan(&changed, 201).unwrap().plan_hash
    );
}

fn request(yaml: &str, vmid: u32) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/provisioning/plan")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"yaml":yaml,"targetVmid":vmid}).to_string(),
        ))
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}

#[tokio::test]
async fn api_previews_without_provider_or_database_mutations_and_exposes_mapping() {
    let store = Store::memory().unwrap();
    let app = router_with_catalog(None, store.clone(), Some(Catalog::parse(CATALOG).unwrap()));
    let response = app.clone().oneshot(request(DEFINITION, 201)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let plan = body(response).await;
    assert_eq!(plan["target"]["vmid"], 201);
    assert_eq!(
        plan["source"]["templateVersion"],
        "debian-13-cloud-20261016"
    );
    assert!(store.list().unwrap().is_empty());
    assert!(store.claim_inventory().unwrap().is_none());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/images")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(body(response).await["images"][0]["image"], "debian-13");
}

#[tokio::test]
async fn api_returns_specific_safe_errors_for_unconfigured_or_invalid_plans() {
    let app = router_with_catalog(None, Store::memory().unwrap(), None);
    let response = app.oneshot(request(DEFINITION, 201)).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body(response).await["error"]["code"],
        "template_catalog_not_configured"
    );
    let app = router_with_catalog(
        None,
        Store::memory().unwrap(),
        Some(Catalog::parse(CATALOG).unwrap()),
    );
    for (yaml, vmid, code) in [
        (DEFINITION.to_string(), 9000, "invalid_target_vmid"),
        (
            DEFINITION.replace("diskGiB: 40", "diskGiB: 4"),
            201,
            "disk_shrink_unsupported",
        ),
        (
            DEFINITION.replace("image: debian-13", "image: other"),
            201,
            "image_not_mapped",
        ),
        (
            DEFINITION.replace("cpu: 4", "cpu: 0"),
            201,
            "invalid_definition",
        ),
        ("x".repeat(65537), 201, "invalid_definition"),
    ] {
        let response = app.clone().oneshot(request(&yaml, vmid)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body(response).await["error"]["code"], code);
    }
    let mut browser = request(DEFINITION, 201);
    browser
        .headers_mut()
        .insert("origin", "https://untrusted.invalid".parse().unwrap());
    assert_eq!(
        app.oneshot(browser).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn json_envelope_and_encoded_yaml_have_separate_size_limits() {
    let app = router_with_catalog(
        None,
        Store::memory().unwrap(),
        Some(Catalog::parse(CATALOG).unwrap()),
    );
    // Comments are legal YAML and can exceed 64 KiB after JSON escaping.
    let padded = format!("# {}\n{}", "\\".repeat(40_000), DEFINITION);
    assert!(padded.len() < 65536);
    assert!(json!({"yaml":padded,"targetVmid":201}).to_string().len() > 65536);
    assert_eq!(
        app.clone()
            .oneshot(request(&padded, 201))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/provisioning/plan")
                .header("content-type", "application/json")
                .body(Body::from("x".repeat(6 * 65536 + 1025)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        body(response).await["error"]["code"],
        "invalid_plan_request"
    );
}

#[tokio::test]
async fn preflight_requires_explicit_provider_and_rejects_origins_and_invalid_requests() {
    let app = router_with_catalog(
        None,
        Store::memory().unwrap(),
        Some(Catalog::parse(CATALOG).unwrap()),
    );
    let request = |body: Value| {
        Request::post("/api/v1/provisioning/preflight")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    let response = app
        .clone()
        .oneshot(request(json!({"yaml":DEFINITION,"targetVmid":201})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(body["error"]["code"], "provider_not_configured");
    let mut origin = request(json!({"yaml":DEFINITION,"targetVmid":201}));
    origin
        .headers_mut()
        .insert("origin", "http://localhost".parse().unwrap());
    assert_eq!(
        app.clone().oneshot(origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(request(json!({"yaml":DEFINITION,"targetVmid":9000})))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        app.oneshot(request(
            json!({"yaml":DEFINITION,"targetVmid":201,"secret":"never-echo"})
        ))
        .await
        .unwrap()
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
