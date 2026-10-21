use super::*;
use crate::{
    operation::{Failure, Status},
    provisioning_executor::run_final_snapshot,
    provisioning_intent::{PackageOutcome, ReadinessOutcome, ReadinessResult},
    snapshot::Outcome,
    store::Store,
};
const UPID: &str = "UPID:pve-test:00000001:00000002:00000003:qmsnapshot:202:fixture@pve:";
fn ready(
    provider: &Proxmox,
    packages: bool,
) -> (tempfile::TempDir, Store, crate::operation::Operation) {
    let mut yaml = include_str!("../../../../examples/kernel-lab.yaml").replace(
        "snapshotAfterProvisioning: false",
        "snapshotAfterProvisioning: true",
    );
    if !packages {
        yaml = yaml.replace(
            "packages:\n      - build-essential\n      - git",
            "packages: []",
        );
    }
    let (directory, store, operation) = completed_start_for_guest_probe_with_yaml(provider, &yaml);
    let job = store.claim_provisioning_readiness().unwrap().unwrap();
    assert!(store.begin_readiness_probe(&operation.id).unwrap());
    store.link_readiness_pid(&operation.id, 41).unwrap();
    store
        .finish_provisioning_readiness(
            &operation.id,
            ReadinessOutcome::Completed(ReadinessResult {
                instance_id: operation.instance_id.clone().unwrap(),
                provider_scope_id: provider.scope_id(),
                plan_hash: job.approval.plan_hash,
                guest: super::super::GuestReadiness {
                    ipv4: "192.0.2.42".parse().unwrap(),
                    interface: "ens18".into(),
                    cloud_init_exit_code: 0,
                    cloud_init_clean: true,
                },
                guest_ready: true,
            }),
        )
        .unwrap();
    (directory, store, operation)
}
fn installed(store: &Store, id: &str) {
    let job = store.claim_provisioning_packages().unwrap().unwrap();
    assert!(store
        .begin_provisioning_packages(id, &job.approval)
        .unwrap());
    store.link_provisioning_packages(id, 42).unwrap();
    store
        .finish_provisioning_packages(id, PackageOutcome::Completed)
        .unwrap();
}
async fn fixture(variant: &str, recovery: bool) -> Fixture {
    // The owned operation ID is normalized before any claims; definition/plan
    // hashes are still generated from the actual requested YAML.
    let base=serde_json::json!({"data":{"digest":"f".repeat(40),"tags":format!("vmanyware-{}","a".repeat(32)),"template":0}}).to_string();
    let mut routes: Vec<(&str, String)> = Vec::new();
    if !recovery {
        for body in [
            &base,
            r#"{"data":[]}"#,
            r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#,
            &base,
        ] {
            routes.push(("*", body.into()));
        }
        routes.push((
            "*",
            serde_json::json!({"data":{"hasFeature":variant!="unsupported"}}).to_string(),
        ));
        routes.extend([
            ("*", r#"{"data":[]}"#.into()),
            ("*", r#"{"data":[]}"#.into()),
            ("*", base.clone()),
        ]);
        routes.push((
            "*",
            if variant == "lost" {
                DROP_REPLY.into()
            } else {
                serde_json::json!({"data":UPID}).to_string()
            },
        ));
    }
    routes.push(("*",serde_json::json!({"data":{"node":"pve-test","upid":UPID,"status":if variant=="running" {"running"} else {"stopped"},"exitstatus":if variant=="failed" {"ERROR: private-detail"} else {"OK"}}}).to_string()));
    routes.push(("*", base));
    let yaml = include_str!("../../../../examples/kernel-lab.yaml")
        .replace(
            "snapshotAfterProvisioning: false",
            "snapshotAfterProvisioning: true",
        )
        .replace(
            "packages:\n      - build-essential\n      - git",
            "packages: []",
        );
    let definition = crate::definition::Definition::parse(&yaml).unwrap();
    let plan =
        crate::provisioning::Catalog::parse(include_str!("../../../../examples/templates.yaml"))
            .unwrap()
            .plan(&definition, 202)
            .unwrap();
    let description = format!(
        "VMAnyware provisioning {}\ndefinition-sha256: {}\nplan-sha256: {}",
        "b".repeat(32),
        plan.definition_hash,
        plan.plan_hash
    );
    routes.push(("*", serde_json::json!({"data":[{"name":format!("ready_{}","b".repeat(32)),"description":description,"vmstate":0,"snaptime":1700000000}]}).to_string()));
    Fixture::responses(
        "200 OK",
        &routes
            .iter()
            .map(|(p, b)| (*p, b.as_str()))
            .collect::<Vec<_>>(),
        "",
        Duration::ZERO,
        true,
    )
    .await
}
fn due(directory: &tempfile::TempDir, id: &str) {
    rusqlite::Connection::open(directory.path().join("clone.sqlite"))
        .unwrap()
        .execute(
            "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
            [id],
        )
        .unwrap();
}
#[tokio::test]
async fn final_snapshot_waits_for_packages_and_honors_both_write_gates() {
    for (creation, snapshots, expected) in [
        (false, true, Failure::ProvisioningDisabled),
        (true, false, Failure::SnapshotDisabled),
    ] {
        let mut fixture = fixture("normal", false).await;
        let provider = fixture.client();
        let (_directory, store, operation) = ready(&provider, true);
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert!(!run_final_snapshot(
            &store,
            Some(&provider),
            Some(&job.catalog),
            creation,
            snapshots
        )
        .await
        .unwrap());
        installed(&store, &operation.id);
        assert_eq!(
            store
                .provisioning_job(&operation.id)
                .unwrap()
                .unwrap()
                .operation
                .status,
            Status::Running
        );
        assert!(run_final_snapshot(
            &store,
            Some(&provider),
            Some(&job.catalog),
            creation,
            snapshots
        )
        .await
        .unwrap());
        let final_job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(final_job.operation.status, Status::Failed);
        assert_eq!(final_job.operation.failure, Some(expected));
        assert!(fixture.requests.try_recv().is_err());
    }
}
#[tokio::test]
async fn final_snapshot_lost_reply_and_crash_without_task_never_redispatch() {
    for crash in [false, true] {
        let mut fixture = fixture("lost", false).await;
        let provider = fixture.client();
        let (directory, store, operation) = ready(&provider, false);
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        if crash {
            store.claim_provisioning_snapshot().unwrap().unwrap();
            store
                .begin_provisioning_snapshot(&operation.id, &job.approval, &"f".repeat(40))
                .unwrap();
        } else {
            assert!(
                run_final_snapshot(&store, Some(&provider), Some(&job.catalog), true, true)
                    .await
                    .unwrap()
            );
        }
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            usize::from(!crash)
        );
        drop(store);
        let reopened = Store::open(directory.path().join("clone.sqlite")).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        due(&directory, &operation.id);
        assert!(
            !run_final_snapshot(&reopened, Some(&provider), Some(&job.catalog), true, true)
                .await
                .unwrap()
        );
        assert_eq!(
            reopened
                .provisioning_job(&operation.id)
                .unwrap()
                .unwrap()
                .operation
                .status,
            Status::Uncertain
        );
        assert!(fixture.requests.try_recv().is_err());
    }
}
#[tokio::test]
async fn final_snapshot_known_task_recovery_uses_get_only_without_write_flags_or_catalog() {
    let mut fixture = fixture("running", true).await;
    let provider = fixture.client();
    let (directory, store, operation) = ready(&provider, false);
    let job = store.claim_provisioning_snapshot().unwrap().unwrap();
    store
        .begin_provisioning_snapshot(&operation.id, &job.approval, &"f".repeat(40))
        .unwrap();
    let task = super::super::TaskReference::new("pve-test", UPID).unwrap();
    store
        .link_provisioning_snapshot(&operation.id, &task)
        .unwrap();
    drop(store);
    let reopened = Store::open(directory.path().join("clone.sqlite")).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    assert!(
        run_final_snapshot(&reopened, Some(&provider), None, false, false)
            .await
            .unwrap()
    );
    let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.operation.status, Status::Uncertain);
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET "));
}
#[tokio::test]
async fn final_snapshot_refuses_unsupported_storage_and_reports_known_task_failure() {
    for variant in ["unsupported", "failed"] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let (_directory, store, operation) = ready(&provider, false);
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert!(
            run_final_snapshot(&store, Some(&provider), Some(&job.catalog), true, true)
                .await
                .unwrap()
        );
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.operation.status, Status::Failed);
        assert_eq!(
            job.operation.failure,
            Some(if variant == "failed" {
                Failure::ProviderTaskFailed
            } else {
                Failure::SnapshotPreconditionsFailed
            })
        );
        let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("POST ")).count(),
            usize::from(variant == "failed")
        );
    }
}
#[tokio::test]
async fn final_snapshot_completion_requires_matching_disk_only_proof_and_bound_task() {
    let fixture = fixture("normal", false).await;
    let provider = fixture.client();
    let (_directory, store, operation) = ready(&provider, true);
    installed(&store, &operation.id);
    let job = store.claim_provisioning_snapshot().unwrap().unwrap();
    let request = job.final_snapshot_request().unwrap();
    let proof = || crate::snapshot::ResultEvidence {
        instance_id: operation.instance_id.clone().unwrap(),
        provider_scope_id: provider.scope_id(),
        node: "pve-test".into(),
        vmid: 202,
        task_status: super::super::TaskStatus::Succeeded,
        snapshot: super::super::Snapshot {
            name: request.name.clone(),
            description: request.description.clone(),
            created_at: Some(1700000000),
            includes_ram: Some(false),
            parent: None,
            provider_state: None,
        },
    };
    assert!(store
        .finish_provisioning_snapshot(&operation.id, Outcome::Succeeded(proof()))
        .is_err());
    store
        .begin_provisioning_snapshot(&operation.id, &job.approval, &"f".repeat(40))
        .unwrap();
    assert!(store
        .begin_provisioning_snapshot(&operation.id, &job.approval, &"f".repeat(40))
        .is_err());
    let task = super::super::TaskReference::new("pve-test", UPID).unwrap();
    let foreign =
        super::super::TaskReference::new("pve-test", &UPID.replace("qmsnapshot", "qmstart"))
            .unwrap();
    assert!(store
        .link_provisioning_snapshot(&operation.id, &foreign)
        .is_err());
    store
        .link_provisioning_snapshot(&operation.id, &task)
        .unwrap();
    let mut wrong = proof();
    wrong.snapshot.includes_ram = Some(true);
    assert!(store
        .finish_provisioning_snapshot(&operation.id, Outcome::Succeeded(wrong))
        .is_err());
    store
        .finish_provisioning_snapshot(&operation.id, Outcome::Succeeded(proof()))
        .unwrap();
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.operation.status, Status::Succeeded);
    assert!(job.operation.result.unwrap()["guestReady"]
        .as_bool()
        .unwrap());
    let db = rusqlite::Connection::open(_directory.path().join("clone.sqlite")).unwrap();
    let original: String = db
        .query_row(
            "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
            [&operation.id],
            |r| r.get(0),
        )
        .unwrap();
    let mut changed: serde_json::Value = serde_json::from_str(&original).unwrap();
    changed["snapshotDigest"] = "e".repeat(40).into();
    db.execute(
        "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
        rusqlite::params![operation.id, changed.to_string()],
    )
    .unwrap();
    assert!(store.provisioning_job(&operation.id).is_err());
    db.execute(
        "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
        rusqlite::params![operation.id, original],
    )
    .unwrap();
    db.execute("UPDATE provisioning_stages SET result_json='{}' WHERE operation_id=?1 AND kind='snapshot_after_provisioning'",[&operation.id]).unwrap();
    assert!(store.provisioning_job(&operation.id).is_err());
}

#[tokio::test]
async fn final_snapshot_worker_posts_once_and_completes_only_with_matching_provider_metadata() {
    let mut fixture = fixture("normal", false).await;
    let provider = fixture.client();
    let (_directory, store, operation) = ready(&provider, false);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    let request = job.final_snapshot_request().unwrap();
    assert!(
        run_final_snapshot(&store, Some(&provider), Some(&job.catalog), true, true)
            .await
            .unwrap()
    );
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.operation.status, Status::Succeeded);
    let final_stage = job
        .stages
        .iter()
        .find(|s| s.kind == "snapshot_after_provisioning")
        .unwrap();
    assert_eq!(final_stage.status, Status::Succeeded);
    assert_eq!(
        final_stage.result.as_ref().unwrap()["snapshot"]["name"],
        request.name
    );
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 12);
    let posts: Vec<_> = requests.iter().filter(|r| r.starts_with("POST ")).collect();
    assert_eq!(posts.len(), 1);
    assert!(posts[0].starts_with("POST /api2/json/nodes/pve-test/qemu/202/snapshot "));
    let body = posts[0].split_once("\r\n\r\n").unwrap().1;
    let form: std::collections::BTreeMap<_, _> = form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(form.get("snapname"), Some(&request.name));
    assert_eq!(form.get("description"), Some(&request.description));
    assert_eq!(form.get("vmstate").map(String::as_str), Some("0"));
}
