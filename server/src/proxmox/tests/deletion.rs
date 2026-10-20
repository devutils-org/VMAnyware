use super::{lifecycle_config, lifecycle_summary, registered_fixture_instance, Fixture};
use crate::{lifecycle::Status, store::Store};
use serde_json::{json, Value};
use std::time::Duration;
pub(super) fn routes(variant: &str) -> Vec<(String, String)> {
    let mut live = lifecycle_config();
    live["data"]["scsi0"] = json!("storage:vm-202-disk-0,size=8G");
    live["data"]["unused0"] = json!("storage:vm-202-disk-1");
    live["data"]["ide2"] = json!("storage:vm-202-cloudinit,media=cdrom");
    live["data"]["sata0"] = json!("local:iso/installer.iso,media=cdrom");
    live["data"]["privateExtra"] = json!("private-config-value");
    if variant.starts_with("fleecing") {
        live["data"]["special-sections"] = json!({"fleecing":{"fleecing-images":if variant=="fleecing-foreign"{"storage:vm-203-fleecing"}else if variant=="fleecing-malformed"{"/private-fleecing-path"}else{"storage:vm-202-fleecing"}},"cloudinit":{"privateExtra":"private-cloudinit"}});
    }
    if variant == "unknown-section" {
        live["data"]["special-sections"] =
            json!({"future-cleanup":{"privateExtra":"private-special-section"}});
    }
    if variant == "unsupported-slot" {
        live["data"]["scsi999"] = json!("storage:vm-202-disk-0");
    }
    if variant == "raw-cdrom" {
        live["data"]["sata0"] = json!("storage:vm-202-disk-1,media=cdrom");
        live["data"].as_object_mut().unwrap().remove("unused0");
    }
    match variant {
        "protected" => live["data"]["protection"] = json!(1),
        "foreign" => live["data"]["tags"] = json!("foreign"),
        "locked" => live["data"]["lock"] = json!("backup"),
        "host-path" => live["data"]["scsi0"] = json!("/dev/private-host-disk,size=8G"),
        "ambiguous-file" => {
            live["data"]["scsi0"] = json!("storage:vm-202-disk-0,file=storage:vm-203-disk-0")
        }
        _ => {}
    }
    let mut saved = json!({"tags":format!("vmanyware-{}","a".repeat(32)),"template":0,"scsi0":"storage:vm-202-disk-0,size=8G","vmstate":"storage:vm-202-state","privateExtra":"private-saved-value"});
    if variant == "snapshot-only" {
        saved["scsi0"] = json!("storage:vm-202-old-disk,size=8G");
    }
    if variant == "saved-foreign" {
        saved["tags"] = json!("foreign");
    }
    let list = json!({"data":[{"name":"before-work","description":"Before","vmstate":1,"snaptime":1700000000,"snapstate":if variant=="snapshot-incomplete"{Some("prepare")}else{None}},{"name":"current"}]});
    let mut inventory=vec!["vm-202-disk-0","vm-202-disk-1","vm-202-cloudinit","vm-202-old-disk","vm-202-state"].into_iter().map(|name|json!({"volid":format!("storage:{name}"),"vmid":202,"format":"raw","privateExtra":"private-inventory-value"})).collect::<Vec<_>>();
    if variant == "owned-count" {
        inventory.extend((0..257).map(|index|json!({"volid":format!("storage:vm-202-orphan-{index}"),"vmid":202,"format":"raw"})));
    }
    if variant.starts_with("fleecing") {
        inventory.push(json!({"volid":if variant=="fleecing-foreign"{"storage:vm-203-fleecing"}else{"storage:vm-202-fleecing"},"vmid":if variant=="fleecing-foreign"{203}else{202},"format":"raw"}));
        if variant == "fleecing-changed" {
            inventory
                .push(json!({"volid":"storage:vm-202-fleecing-next","vmid":202,"format":"raw"}));
        }
    }
    if variant == "wrong-owner" {
        inventory[0]["vmid"] = json!(203);
    }
    if variant == "missing-volume" {
        inventory.pop();
    }
    if variant == "duplicate" {
        inventory.push(inventory[0].clone());
    }
    let pending = json!({"data":[]});
    let power = json!({"data":{"vmid":202,"status":if variant=="running"{"running"}else{"stopped"},"qmpstatus":if variant=="paused"{Some("paused")}else{None}}});
    let mut routes = vec![
        (
            "/api2/json/nodes/pve-lab/qemu/202/config?current=1".to_string(),
            live.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/pending".into(),
            pending.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/status/current".into(),
            power.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/config?current=1".into(),
            live.to_string(),
        ),
    ];
    fn management(routes: &mut Vec<(String, String)>, variant: &str, after: bool) {
        let permissions =
            json!({"data":{"/vms/202":{"VM.Audit":0,"privateExtra":"private-permission"}}});
        let ha = if variant == "ha" || (variant == "ha-added" && after) {
            json!({"data":[{"sid":"vm:202","state":"disabled","comment":"private-ha"}]})
        } else {
            json!({"data":[{"sid":"ct:203"}]})
        };
        let replication = if variant == "replication" || (variant == "replication-added" && after) {
            json!({"data":[{"id":"202-0","guest":202,"disable":1,"remove_job":"full","privateExtra":"private-job"}]})
        } else {
            json!({"data":[{"id":"203-0","guest":"203"}]})
        };
        routes.extend([
            (
                "/api2/json/access/permissions?path=%2Fvms%2F202".into(),
                permissions.to_string(),
            ),
            ("/api2/json/cluster/ha/resources".into(), ha.to_string()),
            (
                "/api2/json/cluster/replication".into(),
                replication.to_string(),
            ),
            (
                "/api2/json/access/permissions?path=%2Fvms%2F202".into(),
                permissions.to_string(),
            ),
        ]);
    }
    management(&mut routes, variant, false);
    for round in 0..2 {
        let mut snapshot = saved.clone();
        if round == 1 && variant == "scope-change" {
            snapshot["scsi0"] = json!("storage:vm-202-old-disk,size=8G");
        }
        let mut scope_live = live.clone();
        if round == 1 && variant == "fleecing-changed" {
            scope_live["data"]["special-sections"]["fleecing"]["fleecing-images"] =
                json!("storage:vm-202-fleecing-next");
        }
        routes.extend([
            (
                "/api2/json/nodes/pve-lab/qemu/202/config?current=1".into(),
                scope_live.to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/snapshot".into(),
                list.to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/qemu/202/snapshot/before-work/config".into(),
                json!({"data":snapshot}).to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/storage/storage/content?content=images&vmid=202".into(),
                json!({"data":inventory}).to_string(),
            ),
        ]);
    }
    management(&mut routes, variant, true);
    let mut final_live = live.clone();
    if variant == "live-change" {
        final_live["data"]["digest"] = json!("c".repeat(40));
    }
    routes.extend([
        (
            "/api2/json/nodes/pve-lab/qemu/202/pending".into(),
            if variant == "pending" {
                json!({"data":[{"key":"memory","pending":"4096"}]})
            } else {
                pending
            }
            .to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/config?current=1".into(),
            final_live.to_string(),
        ),
        (
            "/api2/json/nodes/pve-lab/qemu/202/status/current".into(),
            power.to_string(),
        ),
    ]);
    routes
}
async fn fixture(variant: &str) -> Fixture {
    let routes = routes(variant);
    let refs = routes
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect::<Vec<_>>();
    Fixture::responses("200 OK", &refs, "", Duration::ZERO, true).await
}
#[tokio::test]
async fn deletion_preflight_binds_live_unused_cloudinit_and_snapshot_ram_volumes_without_writes() {
    let mut fixture = fixture("valid").await;
    let provider = fixture.client();
    let report = lifecycle_summary(&provider)
        .deletion_preflight(&provider)
        .await
        .unwrap();
    assert_eq!(report.deletion_preconditions, Status::Passed);
    assert!(
        !report.execution_enabled
            && report.definition_retained
            && report.destructive
            && !report.purge
            && !report.destroy_unreferenced_disks
    );
    assert_eq!(report.volumes.len(), 4);
    assert_eq!(report.preserved_owned_volumes, ["storage:vm-202-old-disk"]);
    assert_eq!(report.effects.version, 1);
    assert!(
        report.effects.remove_vm_configuration
            && report.effects.remove_vm_firewall
            && report.effects.cleanup_recorded_fleecing_images
            && report.effects.retain_definition_and_history
    );
    assert!(!report.effects.purge_jobs && !report.effects.destroy_unreferenced_disks);
    assert!(report.volumes.iter().all(|volume| volume.owner_verified));
    assert!(report
        .volumes
        .iter()
        .any(|volume| volume.volume == "storage:vm-202-state"
            && volume.references == ["snapshot/before-work:vmstate"]));
    assert_eq!(report.preserved_media, ["local:iso/installer.iso"]);
    assert_eq!(report.snapshots, ["before-work"]);
    assert_eq!(report.scope_hash.len(), 64);
    assert!(report
        .checks
        .iter()
        .filter(|check| check.code != "delete_provider_effect_scope")
        .all(|check| check.status == Status::Passed));
    let approval = crate::deletion::Approval {
        instance_id: report.instance_id.clone(),
        provider_scope_id: report.provider_scope_id.clone(),
        definition_hash: report.definition_hash.clone(),
        node: report.node.clone(),
        vmid: report.vmid,
        config_digest: report.config_digest.clone(),
        confirmed: true,
        acknowledge_data_loss: true,
        plan: crate::deletion::Plan {
            scope_hash: report.scope_hash.clone(),
            volumes: report.volumes.clone(),
            snapshots: report.snapshots.clone(),
            preserved_media: report.preserved_media.clone(),
            preserved_owned_volumes: report.preserved_owned_volumes.clone(),
            effects: report.effects.clone(),
        },
    };
    assert!(
        approval.is_valid(),
        "A real reviewed public plan must be retainable by the deletion journal"
    );
    let public = serde_json::to_string(&report).unwrap();
    assert!(!public.contains("private-") && !public.contains("privateExtra"));
    for _ in 0..23 {
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
    }
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn deletion_preflight_blocks_unsafe_identity_disk_ownership_power_or_scope() {
    for (variant, code) in [
        ("protected", "delete_not_protected"),
        ("foreign", "instance_marker"),
        ("locked", "unlocked"),
        ("running", "delete_vm_stopped"),
        ("paused", "delete_vm_stopped"),
        ("pending", "delete_target_unchanged"),
        ("host-path", "delete_scope_complete"),
        ("ambiguous-file", "delete_scope_complete"),
        ("wrong-owner", "delete_volumes_owned"),
        ("missing-volume", "delete_volumes_owned"),
        ("saved-foreign", "delete_snapshots_complete"),
        ("snapshot-incomplete", "delete_snapshots_complete"),
        ("scope-change", "delete_scope_stable"),
        ("live-change", "delete_target_unchanged"),
    ] {
        let fixture = fixture(variant).await;
        let provider = fixture.client();
        let report = lifecycle_summary(&provider)
            .deletion_preflight(&provider)
            .await
            .unwrap();
        assert_eq!(report.deletion_preconditions, Status::Failed, "{variant}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == code && check.status == Status::Failed),
            "{variant}"
        );
        assert!(!report.execution_enabled);
        assert!(!serde_json::to_string(&report)
            .unwrap()
            .contains("/dev/private-host-disk"));
    }
}
#[tokio::test]
async fn deletion_preflight_rejects_duplicate_inventory_and_cross_scope_before_access() {
    let fixture = fixture("duplicate").await;
    let provider = fixture.client();
    assert!(lifecycle_summary(&provider)
        .deletion_preflight(&provider)
        .await
        .is_err());
    let mut summary = lifecycle_summary(&provider);
    summary.provider_reference.scope_id = "f".repeat(64);
    assert!(matches!(
        summary.deletion_preflight(&provider).await,
        Err(crate::instance::ObservationError::ScopeChanged)
    ));
}
#[tokio::test]
async fn deletion_preflight_api_retains_target_definition_and_disabled_execution() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut fixture = fixture("valid").await;
    let provider = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("delete.sqlite");
    let store = Store::open(&path).unwrap();
    let previous = registered_fixture_instance(&store, &provider.scope_id());
    let id = "a".repeat(32);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE instances SET id=?1 WHERE id=?2",
            rusqlite::params![id, previous],
        )
        .unwrap();
    let before = serde_json::to_value(store.instance(&id).unwrap().unwrap()).unwrap();
    let app =
        crate::router_with_restores(Some(provider), store.clone(), None, true, true, true, true);
    let response = app
        .oneshot(
            Request::get(format!(
                "/api/v1/environments/{id}/delete-preflight?node=other&vmid=999&purge=1"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 32768).await.unwrap()).unwrap();
    assert_eq!(report["instanceId"], id);
    assert_eq!(report["node"], "pve-lab");
    assert_eq!(report["vmid"], 202);
    assert_eq!(report["executionEnabled"], false);
    assert_eq!(report["purge"], false);
    assert_eq!(
        serde_json::to_value(store.instance(&id).unwrap().unwrap()).unwrap(),
        before
    );
    assert!(store.recent_operations().unwrap().is_empty());
    for _ in 0..23 {
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
    }
}

#[tokio::test]
async fn deletion_preflight_api_errors_stay_private_without_preparing_work() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    for variant in ["missing", "no-provider", "scope", "forbidden", "malformed"] {
        let mut fixture = Fixture::new(
            if variant == "forbidden" {
                "403 Forbidden"
            } else {
                "200 OK"
            },
            "private-provider-detail",
            "",
            Duration::ZERO,
        )
        .await;
        let provider = fixture.client();
        let store = Store::memory().unwrap();
        let scope = if variant == "scope" {
            "f".repeat(64)
        } else {
            provider.scope_id()
        };
        let id = registered_fixture_instance(&store, &scope);
        let selected = if variant == "missing" {
            "e".repeat(32)
        } else {
            id
        };
        let app = crate::router_with_services(
            if variant == "no-provider" {
                None
            } else {
                Some(provider)
            },
            store.clone(),
        );
        let response = app
            .oneshot(
                Request::get(format!("/api/v1/environments/{selected}/delete-preflight"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            match variant {
                "missing" => StatusCode::NOT_FOUND,
                "no-provider" => StatusCode::SERVICE_UNAVAILABLE,
                "scope" => StatusCode::CONFLICT,
                _ => StatusCode::BAD_GATEWAY,
            },
            "{variant}"
        );
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("private-provider-detail"));
        assert!(store.recent_operations().unwrap().is_empty());
        if ["forbidden", "malformed"].contains(&variant) {
            assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        }
        assert!(fixture.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn deletion_preflight_blocks_managed_disabled_or_newly_added_dependencies() {
    for (variant, code) in [
        ("ha", "delete_not_ha_managed"),
        ("replication", "delete_no_replication_jobs"),
        ("ha-added", "delete_management_stable"),
        ("replication-added", "delete_management_stable"),
    ] {
        let mut fixture = fixture(variant).await;
        let provider = fixture.client();
        let report = lifecycle_summary(&provider)
            .deletion_preflight(&provider)
            .await
            .unwrap();
        assert_eq!(report.deletion_preconditions, Status::Failed, "{variant}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == code && check.status == Status::Failed),
            "{variant}"
        );
        assert!(!report.execution_enabled && !report.purge);
        assert!(!serde_json::to_string(&report).unwrap().contains("private-"));
        for _ in 0..23 {
            assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        }
    }
}

#[tokio::test]
async fn deletion_effect_scope_reviews_fleecing_and_non_cloudinit_media_explicitly() {
    use crate::proxmox::DeletionVolumeEffect;
    for variant in ["fleecing-valid", "raw-cdrom"] {
        let fixture = fixture(variant).await;
        let provider = fixture.client();
        let report = lifecycle_summary(&provider)
            .deletion_preflight(&provider)
            .await
            .unwrap();
        assert_eq!(report.deletion_preconditions, Status::Passed, "{variant}");
        assert!(!report.execution_enabled && !report.purge && !report.destroy_unreferenced_disks);
        if variant == "fleecing-valid" {
            assert!(report
                .volumes
                .iter()
                .any(|volume| volume.volume == "storage:vm-202-fleecing"
                    && volume.owner_verified
                    && volume.effect == DeletionVolumeEffect::FleecingCleanup
                    && volume.references == ["live:special/fleecing"]));
            assert_eq!(report.preserved_owned_volumes, ["storage:vm-202-old-disk"]);
        } else {
            assert!(report
                .preserved_media
                .contains(&"storage:vm-202-disk-1".into()));
            assert!(!report
                .volumes
                .iter()
                .any(|volume| volume.volume == "storage:vm-202-disk-1"));
        }
        assert!(!serde_json::to_string(&report).unwrap().contains("private-"));
    }
}
#[tokio::test]
async fn deletion_effect_scope_refuses_unknown_snapshot_only_foreign_or_unsafe_cleanup() {
    for (variant, code) in [
        ("snapshot-only", "delete_provider_effect_scope"),
        ("unknown-section", "delete_provider_effect_scope"),
        ("fleecing-foreign", "delete_volumes_owned"),
        ("fleecing-malformed", "delete_provider_effect_scope"),
        ("unsupported-slot", "delete_scope_complete"),
    ] {
        let fixture = fixture(variant).await;
        let provider = fixture.client();
        let report = lifecycle_summary(&provider)
            .deletion_preflight(&provider)
            .await
            .unwrap();
        assert_eq!(report.deletion_preconditions, Status::Failed, "{variant}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.code == code && check.status == Status::Failed),
            "{variant}"
        );
        assert!(!report.execution_enabled);
        assert!(!serde_json::to_string(&report)
            .unwrap()
            .contains("/private-fleecing-path"));
    }
}
#[tokio::test]
async fn deletion_effect_scope_fingerprint_tracks_cleanup_and_retained_inventory_changes() {
    let fixture = fixture("fleecing-changed").await;
    let provider = fixture.client();
    let report = lifecycle_summary(&provider)
        .deletion_preflight(&provider)
        .await
        .unwrap();
    assert_eq!(report.deletion_preconditions, Status::Failed);
    assert!(report
        .checks
        .iter()
        .any(|check| check.code == "delete_scope_stable" && check.status == Status::Failed));
    assert!(!report.execution_enabled);
}

#[tokio::test]
async fn deletion_effect_scope_bounds_combined_deleted_and_preserved_images() {
    let fixture = fixture("owned-count").await;
    let provider = fixture.client();
    assert!(matches!(
        lifecycle_summary(&provider)
            .deletion_preflight(&provider)
            .await,
        Err(crate::instance::ObservationError::Provider(
            crate::proxmox::Error::ResponseTooLarge
        ))
    ));
}
