use super::{registered_fixture_instance, Fixture};
use crate::{proxmox::VmReference, router_with_services, store::Store};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn snapshot_list_uses_registered_target_and_excludes_live_vm() {
    let mut fixture = Fixture::new("200 OK", r#"{"data":[
        {"name":"z-after","snaptime":1700000001,"vmstate":true,"description":"After work","parent":"a-before","snapstate":"prepare","unexpected":"private-detail"},
        {"name":"current","description":"You are here!","digest":"private-digest","running":1},
        {"name":"a-before","snaptime":0,"vmstate":0,"description":"Before work"}
    ]}"#, "", Duration::ZERO).await;
    let provider = fixture.client();
    let scope = provider.scope_id();
    let store = Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &scope);
    let app = router_with_services(Some(provider), store.clone());
    let response = app
        .oneshot(
            Request::get(format!(
                "/api/v1/environments/{id}/snapshots?node=other&vmid=999"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(
        result,
        json!({"instanceId":id,"providerScopeId":scope,"node":"pve-lab","vmid":202,"snapshots":[
            {"name":"a-before","description":"Before work","createdAt":null,"includesRam":false,"parent":null,"providerState":null},
            {"name":"z-after","description":"After work","createdAt":1700000001_u64,"includesRam":true,"parent":"a-before","providerState":"prepare"}
        ]})
    );
    assert!(fixture
        .requests
        .recv()
        .await
        .unwrap()
        .starts_with("GET /api2/json/nodes/pve-lab/qemu/202/snapshot HTTP/1.1\r\n"));
    assert!(fixture.requests.try_recv().is_err());
    assert!(store.recent_operations().unwrap().is_empty());
    assert_eq!(store.recent_instances().unwrap().len(), 1);
}

#[tokio::test]
async fn snapshot_list_distinguishes_empty_unknown_and_invalid_metadata() {
    let target = VmReference::new("pve-lab", 202).unwrap();
    for body in [r#"{"data":[]}"#, r#"{"data":[{"name":"current"}]}"#] {
        let fixture = Fixture::new("200 OK", body, "", Duration::ZERO).await;
        assert!(fixture
            .client()
            .snapshots(&target)
            .await
            .unwrap()
            .is_empty());
    }
    let fixture = Fixture::new(
        "200 OK",
        r#"{"data":[{"name":"before"}]}"#,
        "",
        Duration::ZERO,
    )
    .await;
    let value = serde_json::to_value(fixture.client().snapshots(&target).await.unwrap()).unwrap();
    assert_eq!(value[0]["includesRam"], Value::Null);
    assert_eq!(value[0]["createdAt"], Value::Null);
    for body in [
        r#"{"data":[{"name":"../bad"}]}"#,
        r#"{"data":[{"name":"a"},{"name":"a"}]}"#,
        r#"{"data":[{"name":"current"},{"name":"current"}]}"#,
        r#"{"data":[{"name":"a","vmstate":2}]}"#,
        r#"{"data":[{"name":"a","snaptime":-1}]}"#,
        r#"{"data":[{"name":"a","parent":"../bad"}]}"#,
        r#"{"data":[{"name":"a","snapstate":""}]}"#,
        r#"{"data":null}"#,
    ] {
        let fixture = Fixture::new("200 OK", body, "", Duration::ZERO).await;
        assert_eq!(
            fixture.client().snapshots(&target).await.unwrap_err(),
            crate::proxmox::Error::InvalidResponse,
            "{body}"
        );
    }
}

#[tokio::test]
async fn snapshot_list_refuses_missing_instance_or_changed_scope_before_network() {
    let mut fixture = Fixture::new("200 OK", r#"{"data":[]}"#, "", Duration::ZERO).await;
    let store = Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &"a".repeat(64));
    let app = router_with_services(Some(fixture.client()), store.clone());
    for (id, expected) in [
        (&id[..], StatusCode::CONFLICT),
        ("invalid", StatusCode::NOT_FOUND),
        (&"0".repeat(32), StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/api/v1/environments/{id}/snapshots"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert!(fixture.requests.try_recv().is_err());
    let response = router_with_services(None, store)
        .oneshot(
            Request::get(format!("/api/v1/environments/{id}/snapshots"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn snapshot_list_provider_failures_are_safe_and_never_an_empty_success() {
    for (status, body, delay, expected, code) in [
        (
            "403 Forbidden",
            "private-provider-detail",
            Duration::ZERO,
            StatusCode::BAD_GATEWAY,
            "provider_forbidden",
        ),
        (
            "401 Unauthorized",
            "private-provider-detail",
            Duration::ZERO,
            StatusCode::BAD_GATEWAY,
            "provider_unauthorized",
        ),
        (
            "200 OK",
            r#"{"data":{}}"#,
            Duration::ZERO,
            StatusCode::BAD_GATEWAY,
            "provider_unavailable",
        ),
        (
            "200 OK",
            r#"{"data":[]}"#,
            Duration::from_secs(2),
            StatusCode::GATEWAY_TIMEOUT,
            "provider_timeout",
        ),
    ] {
        let mut fixture = Fixture::new(status, body, "", delay).await;
        let provider = crate::proxmox::Proxmox::build(
            &fixture.endpoint,
            "reader@pve!fixture",
            "fake-test-secret",
            Some(&fixture.ca),
            Duration::from_millis(200),
        )
        .unwrap();
        let store = Store::memory().unwrap();
        let id = registered_fixture_instance(&store, &provider.scope_id());
        let response = router_with_services(Some(provider), store)
            .oneshot(
                Request::get(format!("/api/v1/environments/{id}/snapshots"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["error"]["code"], code);
        assert!(!String::from_utf8_lossy(&bytes).contains("private-provider-detail"));
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        assert!(fixture.requests.try_recv().is_err());
    }
}

fn snapshot_request() -> crate::snapshot::CreateRequest {
    crate::snapshot::CreateRequest {
        name: "before-work".into(),
        description: "Before work\nDisk only".into(),
    }
}

#[test]
fn snapshot_preflight_request_validates_exact_bounded_inputs() {
    for name in [
        "",
        "a",
        "current",
        "../bad",
        " leading",
        "1start",
        "has.dot",
        &"a".repeat(41),
    ] {
        let mut request = snapshot_request();
        request.name = name.into();
        assert!(!request.is_valid(), "{name}");
    }
    for name in ["ab", "Before_work-1", &"a".repeat(40)] {
        let mut request = snapshot_request();
        request.name = name.into();
        assert!(request.is_valid(), "{name}");
    }
    for description in ["secret\0bad".to_string(), "x".repeat(4097)] {
        let mut request = snapshot_request();
        request.description = description;
        assert!(!request.is_valid());
    }
    assert!(serde_json::from_value::<crate::snapshot::CreateRequest>(
        json!({"name":"valid","includesRam":true})
    )
    .is_err());
}

async fn snapshot_preflight_fixture(variant: &str) -> Fixture {
    let config = super::lifecycle_config().to_string();
    let mut after = super::lifecycle_config();
    if variant == "changed" {
        after["data"]["digest"] = json!("c".repeat(40));
    }
    let snapshots = match variant {
        "duplicate" => json!({"data":[{"name":"before-work"}]}),
        "incomplete" => json!({"data":[{"name":"old-work","snapstate":"prepare"}]}),
        _ => json!({"data":[{"name":"current"}]}),
    };
    let feature = match variant {
        "malformed" => json!({"data":{"hasFeature":2}}),
        "unsupported" => json!({"data":{"hasFeature":false}}),
        _ => json!({"data":{"hasFeature":1}}),
    };
    Fixture::responses(
        "200 OK",
        &[
            (
                "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
                &config,
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/pending",
                r#"{"data":[]}"#,
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/status/current",
                r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#,
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
                &config,
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/feature?feature=snapshot",
                &feature.to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/snapshot",
                &snapshots.to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/pending",
                r#"{"data":[]}"#,
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
                &after.to_string(),
            ),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await
}

#[tokio::test]
async fn snapshot_preflight_reads_capability_name_and_target_without_enabling_writes() {
    let mut fixture = snapshot_preflight_fixture("valid").await;
    let provider = fixture.client();
    let summary = super::lifecycle_summary(&provider);
    let result = summary
        .snapshot_preflight(&provider, snapshot_request())
        .await
        .unwrap();
    assert_eq!(
        result.creation_preconditions,
        crate::lifecycle::Status::Passed
    );
    assert!(!result.execution_enabled && !result.includes_ram);
    let value = serde_json::to_value(result).unwrap();
    assert_eq!(value["request"]["name"], "before-work");
    assert_eq!(value["instanceId"], summary.id);
    assert_eq!(value["configDigest"], "b".repeat(40));
    assert!(!value.to_string().contains("sensitive-marker"));
    for _ in 0..8 {
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
    }
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn snapshot_preflight_blocks_unsupported_duplicate_incomplete_and_changed_targets() {
    for (variant, code) in [
        ("unsupported", "snapshot_supported"),
        ("duplicate", "snapshot_name_available"),
        ("incomplete", "snapshot_tree_complete"),
        ("changed", "snapshot_target_unchanged"),
    ] {
        let fixture = snapshot_preflight_fixture(variant).await;
        let provider = fixture.client();
        let result = super::lifecycle_summary(&provider)
            .snapshot_preflight(&provider, snapshot_request())
            .await
            .unwrap();
        assert_eq!(
            result.creation_preconditions,
            crate::lifecycle::Status::Failed,
            "{variant}"
        );
        assert!(result
            .checks
            .iter()
            .any(|check| check.code == code && check.status == crate::lifecycle::Status::Failed));
    }
    let fixture = snapshot_preflight_fixture("malformed").await;
    let provider = fixture.client();
    assert!(matches!(
        super::lifecycle_summary(&provider)
            .snapshot_preflight(&provider, snapshot_request())
            .await,
        Err(crate::instance::ObservationError::Provider(
            crate::proxmox::Error::InvalidResponse
        ))
    ));
}

#[tokio::test]
async fn snapshot_preflight_api_rejects_invalid_inputs_and_changed_scope_without_reads() {
    let mut fixture = snapshot_preflight_fixture("valid").await;
    let store = Store::memory().unwrap();
    let id = registered_fixture_instance(&store, &"a".repeat(64));
    let app = router_with_services(Some(fixture.client()), store.clone());
    for (body, expected) in [
        (json!({"name":"current"}), StatusCode::BAD_REQUEST),
        (
            json!({"name":"valid","includesRam":true}),
            StatusCode::BAD_REQUEST,
        ),
        (json!({"name":"before-work"}), StatusCode::CONFLICT),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(format!("/api/v1/environments/{id}/snapshot-preflight"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert!(fixture.requests.try_recv().is_err());
    assert!(store.recent_operations().unwrap().is_empty());
}

#[tokio::test]
async fn snapshot_preflight_api_returns_bound_disk_only_report_without_operation_or_write() {
    let mut fixture = snapshot_preflight_fixture("valid").await;
    let provider = fixture.client();
    let scope = provider.scope_id();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("snapshot-preflight.sqlite");
    let store = Store::open(&path).unwrap();
    let random_id = registered_fixture_instance(&store, &scope);
    // Deterministic owned database identity matches the generated provider tag.
    let id = "a".repeat(32);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE instances SET id=?1 WHERE id=?2",
            rusqlite::params![id, random_id],
        )
        .unwrap();
    let app = router_with_services(Some(provider), store.clone());
    let mut request = snapshot_request();
    request.description = "\n".repeat(4096); // Valid decoded size with an escaped JSON body above 8 KiB.
    let response = app
        .oneshot(
            Request::post(format!(
                "/api/v1/environments/{id}/snapshot-preflight?node=other&vmid=999"
            ))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&request).unwrap()))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(value["request"]["description"], request.description);
    assert_eq!(value["instanceId"], id);
    assert_eq!(value["providerScopeId"], scope);
    assert_eq!(value["node"], "pve-lab");
    assert_eq!(value["vmid"], 202);
    assert_eq!(value["creationPreconditions"], "passed");
    assert_eq!(value["executionEnabled"], false);
    assert_eq!(value["includesRam"], false);
    assert!(store.recent_operations().unwrap().is_empty());
    for _ in 0..8 {
        let request = fixture.requests.recv().await.unwrap();
        assert!(request.starts_with("GET /api2/json/nodes/pve-lab/qemu/202/"));
    }
    assert!(fixture.requests.try_recv().is_err());
}

#[tokio::test]
async fn snapshot_preflight_api_capability_requires_the_independent_runtime_flag() {
    for enabled in [false, true] {
        let fixture = snapshot_preflight_fixture("valid").await;
        let provider = fixture.client();
        let store = Store::memory().unwrap();
        let id = registered_fixture_instance(&store, &provider.scope_id());
        let app = crate::router_with_snapshots(
            Some(provider),
            store.clone(),
            None,
            !enabled,
            !enabled,
            enabled,
        );
        let response = app
            .oneshot(
                Request::post(format!("/api/v1/environments/{id}/snapshot-preflight"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&snapshot_request()).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
        assert_eq!(value["executionEnabled"], enabled);
        // A capability report is independent from eligibility. The fixture tag
        // intentionally differs from this randomly registered instance ID.
        assert_eq!(value["creationPreconditions"], "failed");
        assert!(store.recent_operations().unwrap().is_empty());
    }
}
