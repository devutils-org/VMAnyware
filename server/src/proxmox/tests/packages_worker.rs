use super::*;
use crate::{
    operation::{Failure, Status},
    provisioning_executor::run_packages,
    provisioning_intent::{PackageOutcome, ReadinessOutcome, ReadinessResult},
    store::Store,
};
fn ready(provider: &Proxmox) -> (tempfile::TempDir, Store, crate::operation::Operation) {
    let (directory, store, operation) = completed_start_for_guest_probe(provider);
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
async fn flow_fixture(reply: &str, statuses: &[&str]) -> Fixture {
    let config = guest_guard_config();
    let power = r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#;
    let pending = r#"{"data":[]}"#;
    let mut routes = vec![
        ("*", config.as_str()),
        ("*", power),
        ("*", pending),
        ("*", reply),
    ];
    for status in statuses {
        routes.extend_from_slice(&[
            ("*", config.as_str()),
            ("*", power),
            ("*", pending),
            ("*", status),
            ("*", config.as_str()),
            ("*", power),
            ("*", pending),
        ]);
    }
    Fixture::responses("200 OK", &routes, "", Duration::ZERO, true).await
}
fn connection(directory: &tempfile::TempDir) -> rusqlite::Connection {
    rusqlite::Connection::open(directory.path().join("clone.sqlite")).unwrap()
}
fn due(directory: &tempfile::TempDir, id: &str) {
    connection(directory)
        .execute(
            "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
            [id],
        )
        .unwrap();
}
#[tokio::test]
async fn package_worker_persists_pid_and_deadline_then_resumes_get_only_with_writes_disabled() {
    let mut fixture = flow_fixture(
        r#"{"data":{"pid":42}}"#,
        &[
            r#"{"data":{"exited":0}}"#,
            r#"{"data":{"exited":1,"exitcode":0}}"#,
        ],
    )
    .await;
    let provider = fixture.client();
    let (directory, store, operation) = ready(&provider);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(
        run_packages(&store, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
    let retained = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(retained.operation.status, Status::Uncertain);
    let journal = retained.seed_journal.unwrap().packages.unwrap();
    assert_eq!(journal.pid, Some(42));
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 11);
    assert_eq!(
        requests.iter().filter(|r| r.starts_with("POST ")).count(),
        1
    );
    drop(store);
    let reopened = Store::open(directory.path().join("clone.sqlite")).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    due(&directory, &operation.id);
    assert!(run_packages(&reopened, Some(&provider), None, false)
        .await
        .unwrap());
    let finished = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(finished.operation.status, Status::Succeeded);
    assert_eq!(finished.installed_packages().unwrap().package_count, 2);
    assert!(finished.operation.result.unwrap()["guestReady"]
        .as_bool()
        .unwrap());
    let after = finished.seed_journal.unwrap().packages.unwrap();
    assert_eq!(journal.deadline, after.deadline);
    assert_eq!(after.pid, Some(42));
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 7);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
    assert!(requests.iter().any(|r| r.contains("exec-status?pid=42")));
    assert!(!run_packages(&reopened, Some(&provider), None, false)
        .await
        .unwrap());
}
#[tokio::test]
async fn package_worker_lost_reply_is_never_replayed_after_restart_or_deadline() {
    let mut fixture = flow_fixture(DROP_REPLY, &[]).await;
    let provider = fixture.client();
    let (directory, store, operation) = ready(&provider);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(
        run_packages(&store, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
    let retained = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(retained
        .stages
        .iter()
        .any(|s| s.kind == "install_declared_packages" && s.dispatch_started));
    assert_eq!(retained.seed_journal.unwrap().packages.unwrap().pid, None);
    assert_eq!(retained.operation.status, Status::Uncertain);
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 4);
    assert_eq!(
        requests.iter().filter(|r| r.starts_with("POST ")).count(),
        1
    );
    drop(store);
    let reopened = Store::open(directory.path().join("clone.sqlite")).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    due(&directory, &operation.id);
    assert!(
        run_packages(&reopened, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
    assert!(fixture.requests.try_recv().is_err());
    let db = connection(&directory);
    let json: String = db
        .query_row(
            "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
            [&operation.id],
            |r| r.get(0),
        )
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&json).unwrap();
    let time: i64 = db
        .query_row("SELECT unixepoch()", [], |r| r.get(0))
        .unwrap();
    json["packages"]["startedAt"] = (time - 1000).into();
    json["packages"]["deadline"] = (time - 40).into();
    db.execute(
        "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
        rusqlite::params![operation.id, json.to_string()],
    )
    .unwrap();
    due(&directory, &operation.id);
    assert!(
        run_packages(&reopened, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
    let finished = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(finished.operation.status, Status::Uncertain);
    assert_eq!(
        finished.operation.failure.unwrap(),
        Failure::PackagesTimedOut
    );
    assert!(
        !run_packages(&reopened, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn package_worker_requires_creation_gate_and_reports_nonzero_as_failed() {
    let mut fixture = flow_fixture(
        r#"{"data":{"pid":42}}"#,
        &[r#"{"data":{"exited":1,"exitcode":100}}"#],
    )
    .await;
    let provider = fixture.client();
    let (_directory, store, operation) = ready(&provider);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(
        run_packages(&store, Some(&provider), Some(&job.catalog), false)
            .await
            .unwrap()
    );
    let stopped = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(stopped.operation.status, Status::Failed);
    assert_eq!(
        stopped.operation.failure.unwrap(),
        Failure::ProvisioningDisabled
    );
    assert!(fixture.requests.try_recv().is_err());
    let (_directory, store, operation) = ready(&provider);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(
        run_packages(&store, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
    let failed = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(failed.operation.status, Status::Failed);
    assert_eq!(failed.operation.failure.unwrap(), Failure::PackagesFailed);
    assert!(failed.installed_packages().is_none());
    assert!(failed.operation.result.is_none());
}
#[tokio::test]
async fn package_journal_refuses_forged_completion_and_pid_rebinding() {
    let fixture = flow_fixture(DROP_REPLY, &[]).await;
    let provider = fixture.client();
    let (_directory, store, operation) = ready(&provider);
    let job = store.claim_provisioning_packages().unwrap().unwrap();
    assert!(store
        .finish_provisioning_packages(&operation.id, PackageOutcome::Completed)
        .is_err());
    assert!(store
        .finish_provisioning_packages(
            &operation.id,
            PackageOutcome::Failed(Failure::PackagesFailed)
        )
        .is_err());
    assert!(store.link_provisioning_packages(&operation.id, 42).is_err());
    assert!(store
        .begin_provisioning_packages(&operation.id, &job.approval)
        .unwrap());
    assert!(store
        .begin_provisioning_packages(&operation.id, &job.approval)
        .is_err());
    assert!(store.link_provisioning_packages(&operation.id, 0).is_err());
    store.link_provisioning_packages(&operation.id, 42).unwrap();
    assert!(store.link_provisioning_packages(&operation.id, 43).is_err());
    assert!(store
        .finish_provisioning_packages(
            &operation.id,
            PackageOutcome::Failed(Failure::ProvisioningDisabled)
        )
        .is_err());
}

#[tokio::test]
async fn package_worker_crash_after_marker_without_pid_freezes_dispatch() {
    let mut fixture = flow_fixture(DROP_REPLY, &[]).await;
    let provider = fixture.client();
    let (directory, store, operation) = ready(&provider);
    let job = store.claim_provisioning_packages().unwrap().unwrap();
    assert!(store
        .begin_provisioning_packages(&operation.id, &job.approval)
        .unwrap());
    drop(store); // Simulate process death before dispatch or receipt of its PID.
    let reopened = Store::open(directory.path().join("clone.sqlite")).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    assert!(
        run_packages(&reopened, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
    let recovered = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(recovered.operation.status, Status::Uncertain);
    assert_eq!(
        recovered.operation.failure,
        Some(Failure::PackagesUnconfirmed)
    );
    assert!(fixture.requests.try_recv().is_err());
    assert!(
        !run_packages(&reopened, Some(&provider), Some(&job.catalog), true)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn package_worker_crash_after_linked_pid_recovers_read_only_and_corruption_is_refused() {
    let config = guest_guard_config();
    let power = r#"{"data":{"vmid":202,"status":"running","qmpstatus":"running"}}"#;
    let pending = r#"{"data":[]}"#;
    let mut fixture = Fixture::responses(
        "200 OK",
        &[
            ("*", &config),
            ("*", power),
            ("*", pending),
            ("*", r#"{"data":{"exited":1,"exitcode":0}}"#),
            ("*", &config),
            ("*", power),
            ("*", pending),
        ],
        "",
        Duration::ZERO,
        true,
    )
    .await;
    let provider = fixture.client();
    let (directory, store, operation) = ready(&provider);
    let job = store.claim_provisioning_packages().unwrap().unwrap();
    assert!(store
        .begin_provisioning_packages(&operation.id, &job.approval)
        .unwrap());
    store.link_provisioning_packages(&operation.id, 42).unwrap();
    drop(store);
    let reopened = Store::open(directory.path().join("clone.sqlite")).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    assert!(run_packages(&reopened, Some(&provider), None, false)
        .await
        .unwrap());
    assert_eq!(
        reopened
            .provisioning_job(&operation.id)
            .unwrap()
            .unwrap()
            .operation
            .status,
        Status::Succeeded
    );
    let requests: Vec<_> = std::iter::from_fn(|| fixture.requests.try_recv().ok()).collect();
    assert_eq!(requests.len(), 7);
    assert!(requests.iter().all(|r| r.starts_with("GET ")));
    let db = connection(&directory);
    let original: String = db
        .query_row(
            "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
            [&operation.id],
            |r| r.get(0),
        )
        .unwrap();
    for (field, value) in [
        ("pid", serde_json::json!(0)),
        ("deadline", serde_json::json!(1)),
    ] {
        let mut journal: serde_json::Value = serde_json::from_str(&original).unwrap();
        journal["packages"][field] = value;
        db.execute(
            "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
            rusqlite::params![operation.id, journal.to_string()],
        )
        .unwrap();
        assert!(reopened.provisioning_job(&operation.id).is_err());
    }
    db.execute(
        "UPDATE provisioning_seed_regeneration SET journal_json=?2 WHERE operation_id=?1",
        rusqlite::params![operation.id, original],
    )
    .unwrap();
    db.execute("UPDATE provisioning_stages SET result_json='{}' WHERE operation_id=?1 AND kind='install_declared_packages'",[&operation.id]).unwrap();
    assert!(reopened.provisioning_job(&operation.id).is_err());
}
