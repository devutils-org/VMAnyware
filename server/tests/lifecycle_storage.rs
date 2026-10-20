use serde_json::{json, Value};
use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    lifecycle::{Action, Approval, Outcome, ResultEvidence},
    operation::{Failure, Status},
    proxmox::{PowerState, QmpState, TaskReference, TaskStatus, VmObservation},
    store::{Error, LifecycleError, Store},
};
const SAMPLE: &str = include_str!("../../examples/kernel-lab.yaml");
fn setup(store: &Store) -> Approval {
    let definition = Definition::parse(SAMPLE).unwrap();
    store.create(&definition).unwrap();
    let instance = store
        .register_instance(
            &Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-test".into(),
                vmid: 202,
            },
            &"a".repeat(64),
        )
        .unwrap();
    Approval {
        action: Action::Start,
        instance_id: instance.summary.id,
        provider_scope_id: "a".repeat(64),
        node: "pve-test".into(),
        vmid: 202,
        definition_hash: definition.hash().unwrap(),
        config_digest: "b".repeat(40),
        confirmed: true,
    }
}
fn task(vmid: u32, kind: &str, pid: &str) -> TaskReference {
    TaskReference::new(
        "pve-test",
        &format!("UPID:pve-test:{pid}:00000002:66F00000:{kind}:{vmid}:reader@pve!fixture:"),
    )
    .unwrap()
}
fn success(approval: &Approval) -> Outcome {
    Outcome::Succeeded(ResultEvidence {
        action: approval.action,
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        task_status: TaskStatus::Succeeded,
        observation: VmObservation {
            node: approval.node.clone(),
            vmid: approval.vmid,
            status: PowerState::Running,
            qmp_status: Some(QmpState::Prelaunch),
        },
    })
}

#[test]
fn lifecycle_confirmation_and_all_inputs_are_bound_to_idempotency() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let mut invalid = approval.clone();
    invalid.confirmed = false;
    assert_eq!(
        store.enqueue_lifecycle("first", &invalid).unwrap_err(),
        LifecycleError::InvalidApproval
    );
    let mut invalid = approval.clone();
    invalid.provider_scope_id = "f".repeat(64);
    assert_eq!(
        store.enqueue_lifecycle("first", &invalid).unwrap_err(),
        LifecycleError::TargetMismatch
    );
    assert!(store.recent_operations().unwrap().is_empty());
    let operation = store.enqueue_lifecycle("first", &approval).unwrap();
    assert_eq!(operation.kind, "vm_start");
    assert_eq!(
        store.enqueue_lifecycle("first", &approval).unwrap().id,
        operation.id
    );
    for changed in [
        Approval {
            action: Action::Shutdown,
            ..approval.clone()
        },
        Approval {
            config_digest: "c".repeat(40),
            ..approval.clone()
        },
    ] {
        assert_eq!(
            store.enqueue_lifecycle("first", &changed).unwrap_err(),
            LifecycleError::Conflict
        );
        assert_eq!(
            store.enqueue_lifecycle("different", &changed).unwrap_err(),
            LifecycleError::Conflict
        );
    }
    assert_eq!(
        store.enqueue_inventory("first").unwrap_err(),
        Error::Conflict
    );
    store
        .enqueue_instance_observation("read-only", &approval.instance_id)
        .unwrap();
    assert_eq!(
        store.claim_operation().unwrap().unwrap().kind,
        "instance_observe"
    );
    assert!(store.claim_operation().unwrap().is_none());
    assert_eq!(
        store.claim_lifecycle().unwrap().unwrap().operation.id,
        operation.id
    );
    let mut payload = serde_json::to_value(&approval).unwrap();
    payload["forceStop"] = json!(true);
    assert!(serde_json::from_value::<Approval>(payload).is_err());
    let mut payload = serde_json::to_value(&approval).unwrap();
    payload["action"] = json!("force_stop");
    assert!(serde_json::from_value::<Approval>(payload).is_err());
}

#[test]
fn dispatch_marker_is_committed_once_and_unknown_outcomes_cannot_be_failed_or_requeued() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("dispatch.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = store.enqueue_lifecycle("action", &approval).unwrap();
    store.claim_lifecycle().unwrap().unwrap();
    assert_eq!(
        store
            .finish_lifecycle(
                &operation.id,
                Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed)
            )
            .unwrap_err(),
        LifecycleError::InvalidOperation
    );
    assert_eq!(
        store
            .link_lifecycle_task(&operation.id, &task(202, "qmstart", "00000001"))
            .unwrap_err(),
        LifecycleError::InvalidOperation
    );
    let mut changed = approval.clone();
    changed.config_digest = "c".repeat(40);
    assert_eq!(
        store
            .begin_lifecycle_dispatch(&operation.id, &changed)
            .unwrap_err(),
        LifecycleError::TargetMismatch
    );
    store
        .begin_lifecycle_dispatch(&operation.id, &approval)
        .unwrap();
    assert_eq!(
        store
            .begin_lifecycle_dispatch(&operation.id, &approval)
            .unwrap_err(),
        LifecycleError::InvalidOperation
    );
    assert_eq!(
        store
            .finish_operation(&operation.id, Ok(json!({})))
            .unwrap_err(),
        Error::InvalidOperation
    );
    assert_eq!(
        store
            .finish_lifecycle(&operation.id, Outcome::Failed(Failure::ProviderTimeout))
            .unwrap_err(),
        LifecycleError::InvalidOperation
    );
    let uncertain = store
        .finish_lifecycle(
            &operation.id,
            Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed),
        )
        .unwrap();
    assert_eq!(uncertain.status, Status::Uncertain);
    assert!(!uncertain.provider_task_known);
    assert!(store.claim_lifecycle().unwrap().is_none());
    assert_eq!(
        store
            .enqueue_lifecycle("new-action", &approval)
            .unwrap_err(),
        LifecycleError::Conflict
    );
    assert_eq!(
        store.enqueue_lifecycle("action", &approval).unwrap().id,
        operation.id
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert!(connection
        .execute(
            "UPDATE operations SET status='queued',failure_code=NULL WHERE id=?1",
            [&operation.id]
        )
        .is_err());
}

#[test]
fn interrupted_dispatch_without_task_stays_uncertain_after_reopen_and_never_claims_again() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unknown.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let id = store.enqueue_lifecycle("action", &approval).unwrap().id;
    store.claim_lifecycle().unwrap().unwrap();
    store.begin_lifecycle_dispatch(&id, &approval).unwrap();
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
    let job = reopened.lifecycle_job(&id).unwrap().unwrap();
    assert!(job.dispatch_started);
    assert!(job.provider_task.is_none());
    assert_eq!(job.operation.status, Status::Uncertain);
    assert_eq!(
        job.operation.failure,
        Some(Failure::ProviderSubmissionUnconfirmed)
    );
    assert!(reopened.claim_lifecycle().unwrap().is_none());
    assert_eq!(
        reopened.enqueue_lifecycle("retry", &approval).unwrap_err(),
        LifecycleError::Conflict
    );
    assert_eq!(
        reopened
            .finish_lifecycle(&id, success(&approval))
            .unwrap_err(),
        LifecycleError::InvalidOperation
    );
}

#[test]
fn known_task_is_bound_redacted_reconciled_without_new_dispatch_and_terminal_result_is_immutable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("known.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let id = store.enqueue_lifecycle("action", &approval).unwrap().id;
    store.claim_lifecycle().unwrap().unwrap();
    store.begin_lifecycle_dispatch(&id, &approval).unwrap();
    let expected = task(202, "qmstart", "00000001");
    assert_eq!(
        store
            .link_lifecycle_task(&id, &task(203, "qmstart", "00000001"))
            .unwrap_err(),
        LifecycleError::TargetMismatch
    );
    assert_eq!(
        store
            .link_lifecycle_task(&id, &task(202, "qmshutdown", "00000001"))
            .unwrap_err(),
        LifecycleError::TargetMismatch
    );
    store.link_lifecycle_task(&id, &expected).unwrap();
    store.link_lifecycle_task(&id, &expected).unwrap();
    assert_eq!(
        store
            .link_lifecycle_task(&id, &task(202, "qmstart", "00000003"))
            .unwrap_err(),
        LifecycleError::Conflict
    );
    drop(store);
    let store = Store::open(&path).unwrap();
    store.recover_interrupted_operations().unwrap();
    let uncertain = store.operation(&id).unwrap().unwrap();
    assert_eq!(uncertain.status, Status::Uncertain);
    assert!(uncertain.provider_task_known);
    let json = serde_json::to_string(&uncertain).unwrap();
    assert!(!json.contains("UPID:"));
    assert!(!json.contains("reader@pve"));
    let job = store.claim_lifecycle().unwrap().unwrap();
    assert_eq!(job.provider_task, Some(expected));
    assert!(!format!("{job:?}").contains("UPID:"));
    assert_eq!(
        store.begin_lifecycle_dispatch(&id, &approval).unwrap_err(),
        LifecycleError::InvalidOperation
    );
    let done = store.finish_lifecycle(&id, success(&approval)).unwrap();
    assert_eq!(done.status, Status::Succeeded);
    assert!(done.provider_task_known);
    assert_eq!(
        done.result.unwrap()["observation"]["qmpStatus"],
        "prelaunch"
    );
    assert_eq!(
        store
            .finish_lifecycle(&id, Outcome::Failed(Failure::ProviderTaskFailed))
            .unwrap_err(),
        LifecycleError::InvalidOperation
    );
    assert_eq!(
        store.enqueue_lifecycle("action", &approval).unwrap().status,
        Status::Succeeded
    );
}

#[test]
fn expiry_is_not_refreshed_by_retries_and_clock_rollback_does_not_authorize_dispatch() {
    for offset in [-121, 10] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("expired.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let id = store.enqueue_lifecycle("action", &approval).unwrap().id;
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE operations SET confirmed_at=unixepoch()+?1 WHERE id=?2",
                rusqlite::params![offset, id],
            )
            .unwrap();
        let previous = store.lifecycle_job(&id).unwrap().unwrap().confirmed_at;
        store.enqueue_lifecycle("action", &approval).unwrap();
        assert_eq!(
            store.lifecycle_job(&id).unwrap().unwrap().confirmed_at,
            previous
        );
        store.claim_lifecycle().unwrap().unwrap();
        assert_eq!(
            store.begin_lifecycle_dispatch(&id, &approval).unwrap_err(),
            LifecycleError::ApprovalExpired
        );
        assert!(!store.lifecycle_job(&id).unwrap().unwrap().dispatch_started);
        store
            .finish_lifecycle(&id, Outcome::Failed(Failure::ApprovalExpired))
            .unwrap();
        store
            .enqueue_lifecycle("reviewed-new-action", &approval)
            .unwrap();
    }
}

#[test]
fn predispatch_recovery_frees_slot_and_corrupt_approval_cannot_be_claimed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("before.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let id = store.enqueue_lifecycle("first", &approval).unwrap().id;
    store.claim_lifecycle().unwrap().unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    store.recover_interrupted_operations().unwrap();
    assert_eq!(
        store.operation(&id).unwrap().unwrap().status,
        Status::Interrupted
    );
    let next = store.enqueue_lifecycle("second", &approval).unwrap().id;
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE operations SET input_hash=?1 WHERE id=?2",
            rusqlite::params!["f".repeat(64), next],
        )
        .unwrap();
    assert_eq!(
        store.claim_lifecycle().unwrap_err(),
        LifecycleError::Unavailable
    );
    assert_eq!(
        store.operation(&next).unwrap().unwrap().status,
        Status::Queued
    );
    assert!(connection
        .execute(
            "UPDATE operations SET dispatch_started=1 WHERE id=?1",
            [&next]
        )
        .is_err());
}

#[test]
fn concurrent_action_requests_have_one_identity_and_do_not_extend_confirmation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            let approval = approval.clone();
            std::thread::spawn(move || {
                Store::open(path)
                    .unwrap()
                    .enqueue_lifecycle("shared", &approval)
                    .unwrap()
                    .id
            })
        })
        .collect();
    let ids: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    let mut claims = Vec::new();
    for _ in 0..4 {
        let database = Store::open(&path).unwrap();
        if let Some(job) = database.claim_lifecycle().unwrap() {
            claims.push(job.operation.id);
        }
    }
    assert_eq!(claims, vec![ids[0].clone()]);
}

#[test]
fn schema_four_upgrade_preserves_read_records_and_instance_snapshots() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v4.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    for migration in [
        include_str!("../../migrations/0001_definitions.sql"),
        include_str!("../../migrations/0002_operations.sql"),
        include_str!("../../migrations/0003_instances.sql"),
        include_str!("../../migrations/0004_instance_observations.sql"),
    ] {
        connection.execute_batch(migration).unwrap();
    }
    connection.execute("INSERT INTO definitions (name,definition_yaml,definition_hash,revision) VALUES ('retained','yaml','hash',1)", []).unwrap();
    let id: String = connection.query_row("INSERT INTO operations (request_key,kind,status,result_json) VALUES ('retained','inventory_refresh','succeeded','{\"nodes\":[]}') RETURNING id", [], |row| row.get(0)).unwrap();
    connection.pragma_update(None, "user_version", 4).unwrap();
    drop(connection);
    let store = Store::open(&path).unwrap();
    let operation = store.enqueue_inventory("retained").unwrap();
    assert_eq!(operation.id, id);
    assert_eq!(operation.status, Status::Succeeded);
    let value: Value = serde_json::to_value(&operation).unwrap();
    assert!(value.get("providerTaskKnown").is_none());
    assert_eq!(store.get("retained").unwrap().unwrap().yaml, "yaml");
    let connection = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        14
    );
}

#[tokio::test]
async fn worker_round_services_both_queues_sequentially_with_explicit_capability() {
    for enabled in [false, true] {
        let store = Store::memory().unwrap();
        let approval = setup(&store);
        for index in 0..3 {
            let read = store.enqueue_inventory(&format!("read-{index}")).unwrap();
            let action = store
                .enqueue_lifecycle(&format!("action-{index}"), &approval)
                .unwrap();
            assert!(vmanyware_server::worker::run_round(&store, None, enabled)
                .await
                .unwrap());
            let read = store.operation(&read.id).unwrap().unwrap();
            let action = store.operation(&action.id).unwrap().unwrap();
            assert_eq!(read.status, Status::Failed);
            assert_eq!(read.failure, Some(Failure::ProviderNotConfigured));
            assert_eq!(action.status, Status::Failed);
            assert_eq!(
                action.failure,
                Some(if enabled {
                    Failure::ProviderNotConfigured
                } else {
                    Failure::LifecycleDisabled
                })
            );
            assert!(
                !store
                    .lifecycle_job(&action.id)
                    .unwrap()
                    .unwrap()
                    .dispatch_started
            );
        }
        assert!(!vmanyware_server::worker::run_round(&store, None, enabled)
            .await
            .unwrap());
    }
}
