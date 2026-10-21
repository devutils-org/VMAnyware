use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition, provisioning::Catalog, proxmox::Proxmox, router_with_execution,
    router_with_snapshots, store::Store,
};
fn inputs(attested: bool, packages: bool) -> (Catalog, Definition, Proxmox, Value) {
    let mut yaml = include_str!("../../examples/templates.yaml").to_owned();
    if attested {
        yaml.push_str("    imageDefaultUser: {name: debian, templateVersion: debian-13-cloud-20261016, passwordLocked: true, noAuthorizedKeys: true}\n");
    }
    let catalog = Catalog::parse(&yaml).unwrap();
    let mut definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    if !packages {
        definition.spec.provisioning.packages.clear();
    }
    let provider = Proxmox::new(
        "https://localhost:1",
        "fixture@pve!create",
        "fixture-only-secret",
        None,
    )
    .unwrap();
    let plan = catalog.plan(&definition, 202).unwrap();
    let approval = json!({"definitionName":"kernel-lab","definitionHash":plan.definition_hash,"planHash":plan.plan_hash,"providerScopeId":provider.scope_id(),"targetNode":"pve-test","targetVmid":202,"sourceVmid":9000,"storage":"local-lvm","bridge":"vmbr0","confirmed":true});
    (catalog, definition, provider, approval)
}
fn request(body: Value) -> Request<Body> {
    Request::post("/api/v1/provisioning/create")
        .header("content-type", "application/json")
        .header("idempotency-key", "create-fixture")
        .body(Body::from(body.to_string()))
        .unwrap()
}
async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}
#[tokio::test]
async fn create_queues_complete_confirmation_idempotently_without_provider_calls() {
    let (catalog, definition, provider, approval) = inputs(true, false);
    let store = Store::memory().unwrap();
    store.create(&definition).unwrap();
    let app = router_with_execution(Some(provider), store.clone(), Some(catalog), false, true);
    let first = app
        .clone()
        .oneshot(request(approval.clone()))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    assert!(first
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("/api/v1/operations/"));
    let first = json_body(first).await;
    let second = app.clone().oneshot(request(approval)).await.unwrap();
    assert_eq!(second.status(), StatusCode::ACCEPTED);
    assert_eq!(json_body(second).await["id"], first["id"]);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    assert_eq!(store.recent_instances().unwrap().len(), 1);
    let job = store
        .provisioning_job(first["id"].as_str().unwrap())
        .unwrap()
        .unwrap();
    assert!(job
        .stages
        .iter()
        .all(|s| s.status == vmanyware_server::operation::Status::Queued && !s.dispatch_started));
    let status = app
        .oneshot(
            Request::get(format!(
                "/api/v1/operations/{}",
                first["id"].as_str().unwrap()
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status.status(), StatusCode::OK);
}
#[tokio::test]
async fn create_rejects_disabled_unconfigured_changed_or_unsupported_inputs_without_reservations() {
    for variant in [
        "disabled",
        "provider",
        "catalog",
        "hash",
        "scope",
        "confirmation",
        "packages",
        "snapshot",
        "attestation",
        "unknown",
        "origin",
        "duplicate",
        "oversized",
        "missing_definition",
    ] {
        let (catalog, mut definition, provider, mut approval) =
            inputs(variant != "attestation", variant == "packages");
        if variant == "packages" || variant == "snapshot" {
            if variant == "packages" {
                definition.spec.provisioning.packages = vec!["git-".into()];
            }
            definition.spec.lifecycle.snapshot_after_provisioning = variant == "snapshot";
            let plan = catalog.plan(&definition, 202).unwrap();
            approval["definitionHash"] = json!(plan.definition_hash);
            approval["planHash"] = json!(plan.plan_hash);
        }
        let store = Store::memory().unwrap();
        if variant != "missing_definition" {
            store.create(&definition).unwrap();
        }
        match variant {
            "hash" => approval["planHash"] = json!("f".repeat(64)),
            "scope" => approval["providerScopeId"] = json!("e".repeat(64)),
            "confirmation" => approval["confirmed"] = json!(false),
            "unknown" => approval["unexpected"] = json!("private-value"),
            "oversized" => approval["unexpected"] = json!("x".repeat(5000)),
            _ => {}
        }
        let app = router_with_execution(
            if variant == "provider" {
                None
            } else {
                Some(provider)
            },
            store.clone(),
            if variant == "catalog" {
                None
            } else {
                Some(catalog)
            },
            false,
            variant != "disabled",
        );
        let mut req = request(approval);
        if variant == "origin" {
            req.headers_mut()
                .insert("origin", "https://untrusted.test".parse().unwrap());
        }
        if variant == "duplicate" {
            req.headers_mut()
                .append("idempotency-key", "second-key".parse().unwrap());
        }
        let response = app.oneshot(req).await.unwrap();
        let expected = match variant {
            "disabled" | "origin" | "snapshot" => StatusCode::FORBIDDEN,
            "provider" | "catalog" => StatusCode::SERVICE_UNAVAILABLE,
            "hash" | "scope" => StatusCode::CONFLICT,
            "confirmation" | "duplicate" => StatusCode::BAD_REQUEST,
            "oversized" => StatusCode::PAYLOAD_TOO_LARGE,
            "missing_definition" => StatusCode::NOT_FOUND,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        };
        assert_eq!(response.status(), expected, "{variant}");
        assert!(!json_body(response)
            .await
            .to_string()
            .contains("private-value"));
        assert!(store.recent_operations().unwrap().is_empty(), "{variant}");
        assert!(store.recent_instances().unwrap().is_empty(), "{variant}");
    }
}
#[tokio::test]
async fn create_preview_reports_current_scope_and_capability_without_claiming_live_checks() {
    let (catalog, definition, provider, _) = inputs(true, false);
    let scope = provider.scope_id();
    let app = router_with_execution(
        Some(provider),
        Store::memory().unwrap(),
        Some(catalog),
        false,
        true,
    );
    let response = app
        .oneshot(
            Request::post("/api/v1/provisioning/plan")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"yaml":definition.canonical_yaml().unwrap(),"targetVmid":202})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let plan = json_body(response).await;
    assert_eq!(plan["providerScopeId"], scope);
    assert_eq!(plan["creationEnabled"], true);
    assert_eq!(plan["liveVerified"], false);
    assert_eq!(plan["vmidReserved"], false);
}
#[test]
fn provisioning_runtime_opt_in_is_exact_and_invalid_startup_precedes_database_creation() {
    use vmanyware_server::config::Config;
    let config = Config::from_port(None).unwrap();
    assert!(!config.provisioning_enabled());
    for (input, enabled) in [(None, false), (Some("false"), false), (Some("true"), true)] {
        assert_eq!(
            config
                .with_provisioning(input)
                .unwrap()
                .provisioning_enabled(),
            enabled
        );
    }
    for value in ["", "1", "TRUE", " true", "private-value"] {
        assert_eq!(
            config
                .with_provisioning(Some(value))
                .unwrap_err()
                .to_string(),
            "VMANYWARE_PROVISIONING_ENABLED must be true or false"
        );
    }
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("never-created.sqlite");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vmanyware-server"))
        .env_clear()
        .env("VMANYWARE_DATABASE", &database)
        .env("VMANYWARE_PROVISIONING_ENABLED", "private-value")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let logs = String::from_utf8_lossy(&output.stderr);
    assert!(logs.contains("invalid_provisioning_configuration"));
    assert!(!logs.contains("private-value"));
    assert!(!database.exists());
}

#[tokio::test]
async fn create_worker_rechecks_disabled_capability_before_any_provider_request() {
    let (catalog, definition, provider, approval) = inputs(true, false);
    let store = Store::memory().unwrap();
    store.create(&definition).unwrap();
    let app = router_with_execution(
        Some(provider.clone()),
        store.clone(),
        Some(catalog.clone()),
        false,
        true,
    );
    let response = app.oneshot(request(approval)).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let operation = json_body(response).await;
    let id = operation["id"].as_str().unwrap();
    assert!(vmanyware_server::worker::run_execution_round(
        &store,
        Some(&provider),
        Some(&catalog),
        false,
        false
    )
    .await
    .unwrap());
    let result = store.operation(id).unwrap().unwrap();
    assert_eq!(
        result.failure,
        Some(vmanyware_server::operation::Failure::ProvisioningDisabled)
    );
    assert!(store
        .provisioning_job(id)
        .unwrap()
        .unwrap()
        .stages
        .iter()
        .all(|s| !s.dispatch_started));
}

#[tokio::test]
async fn declared_package_snapshot_plan_and_enqueue_obey_independent_snapshot_gate() {
    for (snapshot, enabled) in [(false, false), (true, false), (true, true)] {
        let (catalog, mut definition, provider, mut approval) = inputs(true, true);
        definition.spec.lifecycle.snapshot_after_provisioning = snapshot;
        let plan = catalog.plan(&definition, 202).unwrap();
        approval["definitionHash"] = json!(plan.definition_hash);
        approval["planHash"] = json!(plan.plan_hash);
        let store = Store::memory().unwrap();
        store.create(&definition).unwrap();
        let app = router_with_snapshots(
            Some(provider),
            store.clone(),
            Some(catalog),
            false,
            true,
            enabled,
        );
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/provisioning/plan")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"yaml":definition.canonical_yaml().unwrap(),"targetVmid":202})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let plan = json_body(response).await;
        assert_eq!(plan["creationEnabled"], !snapshot || enabled);
        assert_eq!(
            plan["packages"],
            json!(definition.spec.provisioning.packages)
        );
        assert_eq!(plan["snapshotAfterProvisioning"], snapshot);
        let response = app.oneshot(request(approval)).await.unwrap();
        assert_eq!(
            response.status(),
            if snapshot && !enabled {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::ACCEPTED
            }
        );
        if snapshot && !enabled {
            assert!(store.recent_operations().unwrap().is_empty());
            assert!(store.recent_instances().unwrap().is_empty());
        }
    }
}
