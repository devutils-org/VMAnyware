use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use vmanyware_server::{
    auth::{protect, AccessToken},
    definition::Definition,
    deletion::Approval,
    instance::Registration,
    proxmox::Proxmox,
    router_with_deletions, router_with_restores,
    store::Store,
};
fn setup() -> (Store, Proxmox, Approval) {
    let store = Store::memory().unwrap();
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    store.create(&definition).unwrap();
    // An unreachable endpoint proves enqueue never reads/writes the provider.
    let provider = Proxmox::new(
        "https://localhost:1",
        "fixture@pve!delete",
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
    let approval=serde_json::from_value(json!({"instanceId":instance.summary.id,"providerScopeId":provider.scope_id(),"definitionHash":definition.hash().unwrap(),"node":"fixture-node","vmid":202,"configDigest":"b".repeat(40),"confirmed":true,"acknowledgeDataLoss":true,"plan":{"scopeHash":"c".repeat(64),"volumes":[{"volume":"storage:vm-202-disk-0","references":["live:scsi0"],"ownerVerified":true,"effect":"delete_owned_volume"}],"snapshots":[],"preservedMedia":["local:iso/debian.iso"],"preservedOwnedVolumes":["storage:vm-202-orphan"],"effects":{"version":1,"removeVmConfiguration":true,"removeSnapshotMetadata":true,"removeVmPermissions":true,"removeVmFirewall":true,"cleanupIpam":true,"cleanupRecordedFleecingImages":true,"purgeJobs":false,"destroyUnreferencedDisks":false,"retainDefinitionAndHistory":true}}})).unwrap();
    (store, provider, approval)
}
fn request(approval: &Approval, key: &str) -> Request<Body> {
    Request::post(format!(
        "/api/v1/environments/{}/delete",
        approval.instance_id
    ))
    .header("content-type", "application/json")
    .header("idempotency-key", key)
    .body(Body::from(serde_json::to_vec(approval).unwrap()))
    .unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap()
}
#[tokio::test]
async fn deletion_api_enqueues_full_consent_and_preserves_one_identity_and_expiry() {
    let (store, provider, approval) = setup();
    let before =
        serde_json::to_value(store.instance(&approval.instance_id).unwrap().unwrap()).unwrap();
    let app = router_with_deletions(
        Some(provider),
        store.clone(),
        None,
        false,
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
    let location = response.headers()["location"].to_str().unwrap().to_owned();
    let first = body(response).await;
    assert_eq!(first["kind"], "vm_delete");
    assert_eq!(first["status"], "queued");
    let id = first["id"].as_str().unwrap();
    let original = store.deletion_job(id).unwrap().unwrap();
    assert_eq!(
        original.approval.input_hash().unwrap(),
        approval.input_hash().unwrap()
    );
    let response = app
        .clone()
        .oneshot(request(&approval, "same-key"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(body(response).await, first);
    assert_eq!(
        store.deletion_job(id).unwrap().unwrap().confirmed_at,
        original.confirmed_at
    );
    let read = app
        .oneshot(Request::get(location).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::OK);
    assert_eq!(body(read).await, first);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    assert_eq!(
        serde_json::to_value(store.instance(&approval.instance_id).unwrap().unwrap()).unwrap(),
        before
    );
    assert!(!original.dispatch_started && original.provider_task.is_none());
}
#[tokio::test]
async fn deletion_api_requires_independent_capability_provider_scope_and_matching_saved_target() {
    for variant in [
        "disabled",
        "missing-provider",
        "scope",
        "definition",
        "node",
        "vmid",
        "missing-instance",
        "path",
    ] {
        let (store, provider, mut approval) = setup();
        let path_id = approval.instance_id.clone();
        let expected = match variant {
            "disabled" => StatusCode::FORBIDDEN,
            "missing-provider" => StatusCode::SERVICE_UNAVAILABLE,
            "missing-instance" => StatusCode::NOT_FOUND,
            "path" => StatusCode::BAD_REQUEST,
            _ => StatusCode::CONFLICT,
        };
        match variant {
            "scope" => approval.provider_scope_id = "d".repeat(64),
            "definition" => approval.definition_hash = "d".repeat(64),
            "node" => approval.node = "other-node".into(),
            "vmid" => approval.vmid = 203,
            "missing-instance" => approval.instance_id = "f".repeat(32),
            "path" => approval.instance_id = "e".repeat(32),
            _ => {}
        }
        let app = if variant == "disabled" {
            router_with_restores(Some(provider), store.clone(), None, true, true, true, true)
        } else {
            router_with_deletions(
                if variant == "missing-provider" {
                    None
                } else {
                    Some(provider)
                },
                store.clone(),
                None,
                false,
                false,
                false,
                false,
                true,
            )
        };
        let mut request = request(&approval, "refused");
        if variant == "path" {
            *request.uri_mut() = format!("/api/v1/environments/{path_id}/delete")
                .parse()
                .unwrap();
        }
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected, "{variant}");
        assert!(store.recent_operations().unwrap().is_empty(), "{variant}");
    }
}
#[tokio::test]
async fn deletion_api_rejects_missing_consents_unsafe_effects_unknown_fields_and_oversized_json() {
    for variant in [
        "confirmed",
        "loss",
        "missing-plan",
        "unverified",
        "purge",
        "unreferenced",
        "unknown",
        "malformed",
        "content-type",
        "oversize",
    ] {
        let (store, provider, approval) = setup();
        let mut value = serde_json::to_value(&approval).unwrap();
        let expected = match variant {
            "missing-plan" | "unknown" => StatusCode::UNPROCESSABLE_ENTITY,
            "malformed" => StatusCode::BAD_REQUEST,
            "oversize" => StatusCode::PAYLOAD_TOO_LARGE,
            "content-type" => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            _ => StatusCode::BAD_REQUEST,
        };
        match variant {
            "confirmed" => value["confirmed"] = json!(false),
            "loss" => value["acknowledgeDataLoss"] = json!(false),
            "missing-plan" => {
                value.as_object_mut().unwrap().remove("plan");
            }
            "unverified" => value["plan"]["volumes"][0]["ownerVerified"] = json!(false),
            "purge" => value["plan"]["effects"]["purgeJobs"] = json!(true),
            "unreferenced" => value["plan"]["effects"]["destroyUnreferencedDisks"] = json!(true),
            "unknown" => value["privateUnknown"] = json!("private-payload"),
            _ => {}
        }
        let payload = match variant {
            "malformed" => "{private-payload".into(),
            "oversize" => format!("{}{}", " ".repeat(65536), value),
            _ => value.to_string(),
        };
        let mut req = request(&approval, "invalid-body");
        *req.body_mut() = Body::from(payload);
        if variant == "content-type" {
            req.headers_mut()
                .insert("content-type", "text/plain".parse().unwrap());
        }
        let response = router_with_deletions(
            Some(provider),
            store.clone(),
            None,
            false,
            false,
            false,
            false,
            true,
        )
        .oneshot(req)
        .await
        .unwrap();
        assert_eq!(response.status(), expected, "{variant}");
        assert!(!body(response).await.to_string().contains("private-payload"));
        assert!(store.recent_operations().unwrap().is_empty());
    }
}
#[tokio::test]
async fn deletion_api_requires_one_safe_request_key_and_auth_before_journaling() {
    for variant in ["missing", "unsafe", "duplicate", "unauthorized"] {
        let (store, provider, approval) = setup();
        let app = router_with_deletions(
            Some(provider),
            store.clone(),
            None,
            false,
            false,
            false,
            false,
            true,
        );
        let mut req = request(&approval, "owned-key");
        match variant {
            "missing" => {
                req.headers_mut().remove("idempotency-key");
            }
            "unsafe" => {
                req.headers_mut()
                    .insert("idempotency-key", "unsafe key".parse().unwrap());
            }
            "duplicate" => {
                req.headers_mut()
                    .append("idempotency-key", "owned-key".parse().unwrap());
            }
            _ => {}
        }
        let app = if variant == "unauthorized" {
            protect(
                app,
                Some(AccessToken::parse("fixture-access-token-0123456789abcdef").unwrap()),
            )
        } else {
            app
        };
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            if variant == "unauthorized" {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            }
        );
        assert!(store.recent_operations().unwrap().is_empty());
    }
}
#[tokio::test]
async fn deletion_api_conflicts_with_changed_key_inputs_and_other_mutations() {
    let (store, provider, approval) = setup();
    let app = router_with_deletions(
        Some(provider),
        store.clone(),
        None,
        false,
        false,
        false,
        false,
        true,
    );
    assert_eq!(
        app.clone()
            .oneshot(request(&approval, "bound"))
            .await
            .unwrap()
            .status(),
        StatusCode::ACCEPTED
    );
    for field in [
        "digest",
        "scope-hash",
        "preserved-media",
        "snapshot",
        "effect-references",
    ] {
        let mut changed = approval.clone();
        match field {
            "digest" => changed.config_digest = "d".repeat(40),
            "scope-hash" => changed.plan.scope_hash = "d".repeat(64),
            "preserved-media" => changed
                .plan
                .preserved_media
                .push("local:iso/other.iso".into()),
            "snapshot" => changed.plan.snapshots.push("new-snapshot".into()),
            _ => changed.plan.volumes[0]
                .references
                .push("live:unused0".into()),
        }
        assert!(changed.is_valid());
        assert_eq!(
            app.clone()
                .oneshot(request(&changed, "bound"))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT,
            "{field}"
        );
    }
    assert_eq!(
        app.oneshot(request(&approval, "different-key"))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let action = vmanyware_server::lifecycle::Approval {
        action: vmanyware_server::lifecycle::Action::Start,
        instance_id: approval.instance_id,
        provider_scope_id: approval.provider_scope_id,
        definition_hash: approval.definition_hash,
        node: approval.node,
        vmid: 202,
        config_digest: approval.config_digest,
        confirmed: true,
    };
    assert!(store.enqueue_lifecycle("competing-start", &action).is_err());
    assert_eq!(store.recent_operations().unwrap().len(), 1);
}
