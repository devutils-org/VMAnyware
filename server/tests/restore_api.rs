use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    proxmox::Proxmox,
    router_with_restores, router_with_snapshots,
    snapshot_restore::{Approval, Request as RestoreRequest},
    store::Store,
};
fn setup() -> (Store, Proxmox, Approval) {
    let store = Store::memory().unwrap();
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    store.create(&definition).unwrap();
    // No network peer exists: these API tests must only enqueue into owned SQLite.
    let provider = Proxmox::new(
        "https://localhost:1",
        "fixture@pve!snapshot",
        "fixture-only-secret",
        None,
    )
    .unwrap();
    let instance = store
        .register_instance(
            &Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "fixture-node".into(),
                vmid: 202,
            },
            &provider.scope_id(),
        )
        .unwrap();
    let approval = Approval {
        instance_id: instance.summary.id,
        provider_scope_id: provider.scope_id(),
        definition_hash: definition.hash().unwrap(),
        node: "fixture-node".into(),
        vmid: 202,
        config_digest: "b".repeat(40),
        request: RestoreRequest {
            snapshot_name: "before-work".into(),
        },
        snapshot_config_hash: "c".repeat(64),
        acknowledge_data_loss: true,
        confirmed: true,
    };
    (store, provider, approval)
}
fn request(approval: &Approval, key: &str) -> Request<Body> {
    Request::post(format!(
        "/api/v1/environments/{}/snapshots/{}/restore",
        approval.instance_id, approval.request.snapshot_name
    ))
    .header("content-type", "application/json")
    .header("idempotency-key", key)
    .body(Body::from(serde_json::to_vec(approval).unwrap()))
    .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}
#[tokio::test]
async fn restore_api_enqueues_complete_confirmation_and_same_key_retains_operation() {
    let (store, provider, approval) = setup();
    let app = router_with_restores(
        Some(provider),
        store.clone(),
        None,
        false,
        false,
        false,
        true,
    );
    let response = app
        .clone()
        .oneshot(request(&approval, "same-key"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let location = response.headers()["location"].to_str().unwrap().to_string();
    let first = body(response).await;
    assert_eq!(first["kind"], "vm_snapshot_restore");
    assert_eq!(first["status"], "queued");
    assert_eq!(first["instanceId"], approval.instance_id);
    assert!(first["result"].is_null());
    let original = store
        .restore_job(first["id"].as_str().unwrap())
        .unwrap()
        .unwrap();
    let response = app
        .clone()
        .oneshot(request(&approval, "same-key"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(body(response).await, first);
    assert_eq!(
        store
            .restore_job(&original.operation.id)
            .unwrap()
            .unwrap()
            .confirmed_at,
        original.confirmed_at
    );
    assert!(!original.dispatch_started);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    let response = app
        .oneshot(Request::get(location).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, first);
}
#[tokio::test]
async fn restore_api_requires_independent_capability_even_with_other_writes_enabled() {
    let (store, provider, approval) = setup();
    let app = router_with_snapshots(Some(provider), store.clone(), None, true, true, true);
    let response = app.oneshot(request(&approval, "disabled")).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(body(response).await["error"]["code"], "restores_disabled");
    assert!(store.recent_operations().unwrap().is_empty());
    let response = router_with_restores(None, store.clone(), None, false, false, false, true)
        .oneshot(request(&approval, "no-provider"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(store.recent_operations().unwrap().is_empty());
}
#[tokio::test]
async fn restore_api_refuses_unconfirmed_unknown_or_changed_targets_without_enqueuing() {
    let (store, provider, approval) = setup();
    let app = router_with_restores(
        Some(provider),
        store.clone(),
        None,
        false,
        false,
        false,
        true,
    );
    for field in [
        "confirm",
        "data-loss",
        "scope",
        "vmid",
        "node",
        "hash",
        "instance",
        "name",
    ] {
        let mut changed = approval.clone();
        match field {
            "confirm" => changed.confirmed = false,
            "data-loss" => changed.acknowledge_data_loss = false,
            "scope" => changed.provider_scope_id = "c".repeat(64),
            "vmid" => changed.vmid += 1,
            "node" => changed.node = "other-node".into(),
            "hash" => changed.definition_hash = "d".repeat(64),
            "instance" => changed.instance_id = "f".repeat(32),
            _ => changed.request.snapshot_name = "current".into(),
        }
        let response = app.clone().oneshot(request(&changed, field)).await.unwrap();
        assert_eq!(
            response.status(),
            match field {
                "confirm" | "data-loss" | "name" => StatusCode::BAD_REQUEST,
                "instance" => StatusCode::NOT_FOUND,
                _ => StatusCode::CONFLICT,
            },
            "{field}"
        );
    }
    let mut wrong_path = request(&approval, "wrong-path");
    *wrong_path.uri_mut() = format!(
        "/api/v1/environments/{}/snapshots/{}/restore",
        "e".repeat(32),
        approval.request.snapshot_name
    )
    .parse()
    .unwrap();
    assert_eq!(
        app.clone().oneshot(wrong_path).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let mut unknown = serde_json::to_value(&approval).unwrap();
    unknown["includesRam"] = json!(true);
    let req = Request::post(format!(
        "/api/v1/environments/{}/snapshots/{}/restore",
        approval.instance_id, approval.request.snapshot_name
    ))
    .header("content-type", "application/json")
    .header("idempotency-key", "unknown-option")
    .body(Body::from(unknown.to_string()))
    .unwrap();
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(store.recent_operations().unwrap().is_empty());
}
#[tokio::test]
async fn restore_api_rejects_origins_missing_or_duplicate_keys_and_oversized_bodies() {
    let (store, provider, approval) = setup();
    let app = router_with_restores(
        Some(provider),
        store.clone(),
        None,
        false,
        false,
        false,
        true,
    );
    for variant in ["origin", "missing", "duplicate", "oversized"] {
        let mut req = request(&approval, "request-key");
        match variant {
            "origin" => {
                req.headers_mut()
                    .insert("origin", "https://browser.example".parse().unwrap());
            }
            "missing" => {
                req.headers_mut().remove("idempotency-key");
            }
            "duplicate" => {
                req.headers_mut()
                    .append("idempotency-key", "other-key".parse().unwrap());
            }
            _ => {
                *req.body_mut() = Body::from("x".repeat(4097));
            }
        }
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            match variant {
                "origin" => StatusCode::FORBIDDEN,
                "oversized" => StatusCode::PAYLOAD_TOO_LARGE,
                _ => StatusCode::BAD_REQUEST,
            },
            "{variant}"
        );
    }
    assert!(store.recent_operations().unwrap().is_empty());
}
#[tokio::test]
async fn restore_api_conflicts_on_changed_request_new_key_or_lifecycle_overlap() {
    let (store, provider, approval) = setup();
    let app = router_with_restores(
        Some(provider),
        store.clone(),
        None,
        true,
        false,
        false,
        true,
    );
    assert_eq!(
        app.clone()
            .oneshot(request(&approval, "same-key"))
            .await
            .unwrap()
            .status(),
        StatusCode::ACCEPTED
    );
    let mut changed = approval.clone();
    changed.snapshot_config_hash = "d".repeat(64);
    for req in [request(&changed, "same-key"), request(&approval, "new-key")] {
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(body(response).await["error"]["code"], "operation_conflict");
    }
    let mut action = serde_json::to_value(&approval).unwrap();
    action.as_object_mut().unwrap().remove("request");
    action.as_object_mut().unwrap().remove("snapshotConfigHash");
    action
        .as_object_mut()
        .unwrap()
        .remove("acknowledgeDataLoss");
    action["action"] = json!("start");
    let req = Request::post(format!(
        "/api/v1/environments/{}/start",
        approval.instance_id
    ))
    .header("content-type", "application/json")
    .header("idempotency-key", "start")
    .body(Body::from(action.to_string()))
    .unwrap();
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    assert_eq!(store.recent_operations().unwrap().len(), 1);
}
#[tokio::test]
async fn restore_api_concurrent_retries_have_one_identity() {
    let (store, provider, approval) = setup();
    let app = router_with_restores(
        Some(provider),
        store.clone(),
        None,
        false,
        false,
        false,
        true,
    );
    let (first, second) = tokio::join!(
        app.clone().oneshot(request(&approval, "retry")),
        app.oneshot(request(&approval, "retry"))
    );
    let first = first.unwrap();
    let second = second.unwrap();
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    assert_eq!(second.status(), StatusCode::ACCEPTED);
    assert_eq!(body(first).await["id"], body(second).await["id"]);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
}
#[tokio::test]
async fn restore_api_is_protected_by_bearer_authentication() {
    let (store, provider, approval) = setup();
    let app = vmanyware_server::auth::protect(
        router_with_restores(
            Some(provider),
            store.clone(),
            None,
            false,
            false,
            false,
            true,
        ),
        Some(
            vmanyware_server::auth::AccessToken::parse(
                "fixture-only-control-token-0123456789abcdef",
            )
            .unwrap(),
        ),
    );
    let response = app
        .clone()
        .oneshot(request(&approval, "auth"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(store.recent_operations().unwrap().is_empty());
    let mut req = request(&approval, "auth");
    req.headers_mut().insert(
        "authorization",
        "Bearer fixture-only-control-token-0123456789abcdef"
            .parse()
            .unwrap(),
    );
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.headers()["x-vmanyware-authenticated"], "bearer");
}

#[tokio::test]
async fn restore_api_binds_snapshot_path_and_rejects_missing_or_extra_consent_fields() {
    let (store, provider, approval) = setup();
    let app = router_with_restores(
        Some(provider),
        store.clone(),
        None,
        false,
        false,
        false,
        true,
    );
    for (variant, expected) in [
        ("wrong-snapshot", StatusCode::BAD_REQUEST),
        ("missing-ack", StatusCode::UNPROCESSABLE_ENTITY),
        ("missing-confirm", StatusCode::UNPROCESSABLE_ENTITY),
        ("missing-fingerprint", StatusCode::UNPROCESSABLE_ENTITY),
        ("start", StatusCode::UNPROCESSABLE_ENTITY),
        ("skiplock", StatusCode::UNPROCESSABLE_ENTITY),
        ("bad-fingerprint", StatusCode::BAD_REQUEST),
        ("malformed", StatusCode::BAD_REQUEST),
    ] {
        let mut value = serde_json::to_value(&approval).unwrap();
        match variant {
            "missing-ack" => {
                value.as_object_mut().unwrap().remove("acknowledgeDataLoss");
            }
            "missing-confirm" => {
                value.as_object_mut().unwrap().remove("confirmed");
            }
            "missing-fingerprint" => {
                value.as_object_mut().unwrap().remove("snapshotConfigHash");
            }
            "start" | "skiplock" => value[variant] = json!(true),
            "bad-fingerprint" => value["snapshotConfigHash"] = json!("private-invalid-fingerprint"),
            _ => {}
        }
        let mut req = request(&approval, variant);
        if variant == "wrong-snapshot" {
            *req.uri_mut() = format!(
                "/api/v1/environments/{}/snapshots/other-snapshot/restore",
                approval.instance_id
            )
            .parse()
            .unwrap();
        }
        *req.body_mut() = Body::from(if variant == "malformed" {
            "{private-invalid-json".into()
        } else {
            value.to_string()
        });
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), expected, "{variant}");
        let response = body(response).await;
        assert_eq!(response["error"]["code"], "invalid_restore_approval");
        assert!(!response.to_string().contains("private-invalid"));
    }
    assert!(store.recent_operations().unwrap().is_empty());
}
