use super::{lifecycle_config, lifecycle_summary, registered_fixture_instance, Fixture};
use crate::{lifecycle::Status, proxmox::VmReference, snapshot_restore::Request};
use serde_json::{json, Value};
use std::time::Duration;
fn request() -> Request {
    Request {
        snapshot_name: "before-work".into(),
    }
}
async fn fixture(variant: &str) -> Fixture {
    let live = lifecycle_config();
    let mut final_live = live.clone();
    let mut snapshot = json!({"tags":format!("vmanyware-{}","a".repeat(32)),"template":0,"scsi0":"storage:vm-202-disk-0,size=8G","snaptime":1700000000,"description":"Before","privateExtra":"private-field"});
    let mut metadata =
        json!({"name":"before-work","snaptime":1700000000,"description":"Before","vmstate":0});
    match variant {
        "ram-list" => metadata["vmstate"] = json!(1),
        "ram-config" => snapshot["vmstate"] = json!("storage:vm-202-state"),
        "incomplete-list" => metadata["snapstate"] = json!("prepare"),
        "incomplete-config" => snapshot["snapstate"] = json!("prepare"),
        "foreign" => snapshot["tags"] = json!("foreign"),
        "template" => snapshot["template"] = json!(1),
        "locked" => snapshot["lock"] = json!("backup"),
        "malformed" => snapshot["vmstate"] = json!(0),
        "live-change" => final_live["data"]["digest"] = json!("c".repeat(40)),
        _ => {}
    }
    let mut final_snapshot = snapshot.clone();
    if variant == "snapshot-change" {
        final_snapshot["scsi0"] = json!("other:vm-202-disk-0,size=8G");
    }
    let mut final_metadata = metadata.clone();
    if variant == "metadata-change" {
        final_metadata["description"] = json!("After");
    }
    let list = if variant == "missing" {
        json!({"data":[{"name":"current"}]})
    } else {
        json!({"data":[metadata,{"name":"current"}]})
    };
    let final_list = if ["missing", "gone"].contains(&variant) {
        json!({"data":[{"name":"current"}]})
    } else {
        json!({"data":[final_metadata,{"name":"current"}]})
    };
    let power = |running| {
        json!({"data":{"vmid":202,"status":if running {"running"}else{"stopped"},"qmpstatus":if running{Some("running")}else{None}}}).to_string()
    };
    let mut routes = vec![
        (
            "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
            live.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/pending",
            json!({"data":[]}).to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/status/current",
            power(variant == "running"),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
            live.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/snapshot",
            list.to_string(),
        ),
    ];
    if variant != "missing" {
        routes.extend([
            (
                "/api2/json/nodes/pve-lab/qemu/202/snapshot/before-work/config",
                json!({"data":snapshot}).to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/feature?feature=snapshot&snapname=before-work",
                json!({"data":{"hasFeature":variant!="unsupported"}}).to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/snapshot/before-work/config",
                json!({"data":final_snapshot}).to_string(),
            ),
        ]);
    }
    routes.extend([
        (
            "/api2/json/nodes/pve-lab/qemu/202/snapshot",
            final_list.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/pending",
            if variant == "pending" {
                json!({"data":[{"key":"memory","pending":"4096"}]})
            } else {
                json!({"data":[]})
            }
            .to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/config?current=1",
            final_live.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/status/current",
            power(["running", "power-change"].contains(&variant)),
        ),
    ]);
    let refs: Vec<_> = routes
        .iter()
        .map(|(path, body)| (*path, body.as_str()))
        .collect();
    Fixture::responses("200 OK", &refs, "", Duration::ZERO, true).await
}
#[test]
fn snapshot_restore_preflight_request_rejects_reserved_names_and_execution_options() {
    for name in ["", "current", "x", "../bad", "1bad", &"a".repeat(41)] {
        assert!(!Request {
            snapshot_name: name.into()
        }
        .is_valid());
    }
    for option in ["start", "skiplock", "confirmed"] {
        let mut value = json!({"snapshotName":"before-work"});
        value[option] = json!(true);
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    assert!(request().is_valid());
}
#[tokio::test]
async fn snapshot_restore_preflight_reads_exact_snapshot_and_never_enables_execution() {
    let mut fixture = fixture("valid").await;
    let provider = fixture.client();
    let report = lifecycle_summary(&provider)
        .snapshot_restore_preflight(&provider, request())
        .await
        .unwrap();
    assert_eq!(report.restore_preconditions, Status::Passed);
    assert!(report.destructive && !report.starts_vm && !report.execution_enabled);
    assert_eq!(report.checks.len(), 13);
    assert_eq!(
        report.selected_snapshot.as_ref().unwrap().name,
        "before-work"
    );
    assert_eq!(report.snapshot_config_hash.as_ref().unwrap().len(), 64);
    assert_eq!(report.config_digest, "b".repeat(40));
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("private-field") && !serialized.contains("storage:vm-202-disk"));
    for _ in 0..12 {
        let request = fixture.requests.recv().await.unwrap();
        assert!(request.starts_with("GET "));
    }
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn snapshot_restore_preflight_blocks_unsafe_missing_or_changed_selection() {
    for (variant, code) in [
        ("ram-list", "restore_snapshot_disk_only"),
        ("ram-config", "restore_snapshot_disk_only"),
        ("incomplete-list", "restore_snapshot_complete"),
        ("incomplete-config", "restore_snapshot_complete"),
        ("foreign", "restore_snapshot_identity"),
        ("template", "restore_snapshot_identity"),
        ("locked", "restore_snapshot_identity"),
        ("snapshot-change", "restore_snapshot_stable"),
        ("metadata-change", "restore_snapshot_stable"),
        ("live-change", "restore_target_unchanged"),
        ("pending", "restore_target_unchanged"),
        ("power-change", "restore_vm_stopped"),
        ("running", "restore_vm_stopped"),
        ("unsupported", "restore_snapshot_storage"),
        ("missing", "restore_snapshot_exists"),
        ("gone", "restore_snapshot_exists"),
    ] {
        let fixture = fixture(variant).await;
        let provider = fixture.client();
        let report = lifecycle_summary(&provider)
            .snapshot_restore_preflight(&provider, request())
            .await
            .unwrap();
        assert_eq!(report.restore_preconditions, Status::Failed, "{variant}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == code && check.status == Status::Failed),
            "{variant}"
        );
        assert!(!report.execution_enabled);
    }
    let fixture = fixture("malformed").await;
    let provider = fixture.client();
    assert!(lifecycle_summary(&provider)
        .snapshot_restore_preflight(&provider, request())
        .await
        .is_err());
}
#[tokio::test]
async fn snapshot_restore_preflight_canonical_hash_ignores_object_key_order() {
    let first = format!(
        r#"{{"data":{{"tags":"vmanyware-{}","nested":{{"b":2,"a":1}},"template":0}}}}"#,
        "a".repeat(32)
    );
    let second = format!(
        r#"{{"data":{{"template":0,"nested":{{"a":1,"b":2}},"tags":"vmanyware-{}"}}}}"#,
        "a".repeat(32)
    );
    let fixture = Fixture::responses(
        "200 OK",
        &[("*", &first), ("*", &second)],
        "",
        Duration::ZERO,
        true,
    )
    .await;
    let provider = fixture.client();
    let target = VmReference::new("pve-lab", 202).unwrap();
    let marker = format!("vmanyware-{}", "a".repeat(32));
    let first = provider
        .restore_snapshot_identity(&target, "before-work", &marker)
        .await
        .unwrap();
    let second = provider
        .restore_snapshot_identity(&target, "before-work", &marker)
        .await
        .unwrap();
    assert_eq!(first.identity.digest, second.identity.digest);
}
#[tokio::test]
async fn snapshot_restore_preflight_api_binds_target_and_stays_off_with_creation_enabled() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut fixture = fixture("valid").await;
    let provider = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("restore.sqlite");
    let store = crate::store::Store::open(&path).unwrap();
    let random = registered_fixture_instance(&store, &provider.scope_id());
    let id = "a".repeat(32);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE instances SET id=?1 WHERE id=?2",
            rusqlite::params![id, random],
        )
        .unwrap();
    let app = crate::router_with_snapshots(Some(provider), store.clone(), None, true, true, true);
    let response = app
        .oneshot(
            Request::post(format!(
                "/api/v1/environments/{id}/snapshot-restore-preflight?node=other&vmid=999"
            ))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"snapshotName":"before-work"}).to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(report["instanceId"], id);
    assert_eq!(report["node"], "pve-lab");
    assert_eq!(report["vmid"], 202);
    assert_eq!(report["restorePreconditions"], "passed");
    assert_eq!(report["executionEnabled"], false);
    assert_eq!(report["startsVm"], false);
    assert_eq!(report["destructive"], true);
    assert!(store.recent_operations().unwrap().is_empty());
    for _ in 0..12 {
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
    }
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn snapshot_restore_preflight_api_refuses_bad_input_scope_and_provider_errors() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    for variant in ["invalid", "scope", "provider"] {
        let mut fixture = Fixture::new(
            "403 Forbidden",
            "private-provider-detail",
            "",
            Duration::ZERO,
        )
        .await;
        let provider = fixture.client();
        let store = crate::store::Store::memory().unwrap();
        let scope = if variant == "scope" {
            "a".repeat(64)
        } else {
            provider.scope_id()
        };
        let id = registered_fixture_instance(&store, &scope);
        let app = crate::router_with_services(Some(provider), store.clone());
        let response = app
            .oneshot(
                Request::post(format!(
                    "/api/v1/environments/{id}/snapshot-restore-preflight"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"snapshotName":if variant=="invalid"{"current"}else{"before-work"}})
                        .to_string(),
                ))
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            match variant {
                "invalid" => StatusCode::BAD_REQUEST,
                "scope" => StatusCode::CONFLICT,
                _ => StatusCode::BAD_GATEWAY,
            }
        );
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("private-provider-detail"));
        assert!(store.recent_operations().unwrap().is_empty());
        if variant != "provider" {
            assert!(fixture.requests.try_recv().is_err());
        }
    }
}
