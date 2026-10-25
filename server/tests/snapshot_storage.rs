use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    lifecycle,
    operation::{Failure, Status},
    proxmox::{Snapshot, TaskReference, TaskStatus},
    snapshot::{Approval, CreateRequest, Outcome, ResultEvidence},
    store::{SnapshotError, Store},
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
        instance_id: instance.summary.id,
        provider_scope_id: "a".repeat(64),
        definition_hash: definition.hash().unwrap(),
        node: "pve-test".into(),
        vmid: 202,
        config_digest: "b".repeat(40),
        request: CreateRequest {
            name: "before-work".into(),
            description: "Before work".into(),
        },
        confirmed: true,
    }
}
fn lifecycle(approval: &Approval) -> lifecycle::Approval {
    lifecycle::Approval {
        action: lifecycle::Action::Start,
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        definition_hash: approval.definition_hash.clone(),
        node: approval.node.clone(),
        vmid: approval.vmid,
        config_digest: approval.config_digest.clone(),
        confirmed: true,
    }
}
fn task(kind: &str, vmid: u32) -> TaskReference {
    TaskReference::new(
        "pve-test",
        &format!("UPID:pve-test:00000001:00000002:66F00000:{kind}:{vmid}:reader@pve!fixture:"),
    )
    .unwrap()
}
fn evidence(approval: &Approval) -> ResultEvidence {
    ResultEvidence {
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        node: approval.node.clone(),
        vmid: approval.vmid,
        task_status: TaskStatus::Succeeded,
        snapshot: Snapshot {
            name: approval.request.name.clone(),
            description: approval.request.description.clone(),
            created_at: Some(1700000000),
            includes_ram: Some(false),
            parent: None,
            provider_state: None,
        },
    }
}
#[test]
fn snapshot_journal_binds_confirmation_target_request_and_idempotency() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let mut invalid = approval.clone();
    invalid.confirmed = false;
    assert_eq!(
        store.enqueue_snapshot("key", &invalid).unwrap_err(),
        SnapshotError::InvalidApproval
    );
    invalid = approval.clone();
    invalid.vmid += 1;
    assert_eq!(
        store.enqueue_snapshot("key", &invalid).unwrap_err(),
        SnapshotError::TargetMismatch
    );
    let operation = store.enqueue_snapshot("key", &approval).unwrap();
    assert_eq!(operation.kind, "vm_snapshot_create");
    assert_eq!(
        store.enqueue_snapshot("key", &approval).unwrap().id,
        operation.id
    );
    for field in ["name", "description", "digest"] {
        let mut changed = approval.clone();
        match field {
            "name" => changed.request.name = "after-work".into(),
            "description" => changed.request.description.push('!'),
            _ => changed.config_digest = "c".repeat(40),
        }
        assert_eq!(
            store.enqueue_snapshot("key", &changed).unwrap_err(),
            SnapshotError::Conflict
        );
    }
    assert_eq!(
        store.enqueue_snapshot("new-key", &approval).unwrap_err(),
        SnapshotError::Conflict
    );
    assert_eq!(
        store
            .enqueue_lifecycle("start", &lifecycle(&approval))
            .unwrap_err(),
        SnapshotError::Conflict
    );
    assert!(store.claim_operation().unwrap().is_none());
}
#[test]
fn snapshot_journal_cannot_overlap_an_existing_lifecycle_action() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    store
        .enqueue_lifecycle("start", &lifecycle(&approval))
        .unwrap();
    assert_eq!(
        store.enqueue_snapshot("snapshot", &approval).unwrap_err(),
        SnapshotError::Conflict
    );
}
#[test]
fn snapshot_journal_dispatch_requires_claim_exact_approval_and_fresh_age() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("expiry.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let op = store.enqueue_snapshot("expiry", &approval).unwrap();
    assert_eq!(
        store
            .begin_snapshot_dispatch(&op.id, &approval)
            .unwrap_err(),
        SnapshotError::InvalidOperation
    );
    store.claim_snapshot().unwrap().unwrap();
    let mut changed = approval.clone();
    changed.request.description.push('!');
    assert_eq!(
        store.begin_snapshot_dispatch(&op.id, &changed).unwrap_err(),
        SnapshotError::TargetMismatch
    );
    for age in [-121, 10] {
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE operations SET confirmed_at=unixepoch()+?1 WHERE id=?2",
                rusqlite::params![age, op.id],
            )
            .unwrap();
        let original_age = store.snapshot_job(&op.id).unwrap().unwrap().confirmed_at;
        store.enqueue_snapshot("expiry", &approval).unwrap();
        assert_eq!(
            store.snapshot_job(&op.id).unwrap().unwrap().confirmed_at,
            original_age
        );
        assert_eq!(
            store
                .begin_snapshot_dispatch(&op.id, &approval)
                .unwrap_err(),
            SnapshotError::ApprovalExpired
        );
    }
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE operations SET confirmed_at=unixepoch() WHERE id=?1",
            [&op.id],
        )
        .unwrap();
    store.begin_snapshot_dispatch(&op.id, &approval).unwrap();
    assert_eq!(
        store
            .begin_snapshot_dispatch(&op.id, &approval)
            .unwrap_err(),
        SnapshotError::InvalidOperation
    );
}
#[test]
fn snapshot_journal_restart_never_reclaims_unknown_submission() {
    for dispatched in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("restart.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let op = store.enqueue_snapshot("restart", &approval).unwrap();
        store.claim_snapshot().unwrap().unwrap();
        if dispatched {
            store.begin_snapshot_dispatch(&op.id, &approval).unwrap();
        }
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(
            store.operation(&op.id).unwrap().unwrap().status,
            Status::Running
        );
        assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
        let result = store.snapshot_job(&op.id).unwrap().unwrap();
        assert_eq!(
            result.operation.status,
            if dispatched {
                Status::Uncertain
            } else {
                Status::Interrupted
            }
        );
        assert!(store.claim_snapshot().unwrap().is_none());
        assert_eq!(
            store.enqueue_snapshot("restart", &approval).unwrap().id,
            op.id
        );
        if dispatched {
            assert_eq!(
                store.enqueue_snapshot("another", &approval).unwrap_err(),
                SnapshotError::Conflict
            );
        }
    }
}
#[test]
fn snapshot_journal_retained_task_only_reconciles_and_needs_exact_completed_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("task.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let op = store.enqueue_snapshot("task", &approval).unwrap();
    store.claim_snapshot().unwrap().unwrap();
    assert!(store
        .link_snapshot_task(&op.id, &task("qmsnapshot", 202))
        .is_err());
    store.begin_snapshot_dispatch(&op.id, &approval).unwrap();
    for wrong in [task("qmstart", 202), task("qmsnapshot", 203)] {
        assert_eq!(
            store.link_snapshot_task(&op.id, &wrong).unwrap_err(),
            SnapshotError::TargetMismatch
        );
    }
    store
        .link_snapshot_task(&op.id, &task("qmsnapshot", 202))
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    store.recover_interrupted_operations().unwrap();
    let recovered = store.claim_snapshot().unwrap().unwrap();
    assert!(recovered.dispatch_started && recovered.provider_task.is_some());
    assert!(store.begin_snapshot_dispatch(&op.id, &approval).is_err());
    for field in [
        "name",
        "description",
        "ram",
        "incomplete",
        "status",
        "target",
    ] {
        let mut wrong = evidence(&approval);
        match field {
            "name" => wrong.snapshot.name = "after-work".into(),
            "description" => wrong.snapshot.description.clear(),
            "ram" => wrong.snapshot.includes_ram = None,
            "incomplete" => wrong.snapshot.provider_state = Some("prepare".into()),
            "status" => wrong.task_status = TaskStatus::Running,
            _ => wrong.vmid += 1,
        }
        assert_eq!(
            store
                .finish_snapshot(&op.id, Outcome::Succeeded(wrong))
                .unwrap_err(),
            SnapshotError::InvalidOperation
        );
    }
    let done = store
        .finish_snapshot(&op.id, Outcome::Succeeded(evidence(&approval)))
        .unwrap();
    assert_eq!(done.status, Status::Succeeded);
    assert!(store.claim_snapshot().unwrap().is_none());
    assert_eq!(store.enqueue_snapshot("task", &approval).unwrap().id, op.id);
}
#[test]
fn snapshot_journal_requires_uncertainty_after_dispatch_and_enforces_retry_backoff() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let op = store.enqueue_snapshot("uncertain", &approval).unwrap();
    store.claim_snapshot().unwrap().unwrap();
    assert!(store
        .finish_snapshot(
            &op.id,
            Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed)
        )
        .is_err());
    store.begin_snapshot_dispatch(&op.id, &approval).unwrap();
    assert!(store
        .finish_snapshot(&op.id, Outcome::Failed(Failure::ProviderUnavailable))
        .is_err());
    store
        .link_snapshot_task(&op.id, &task("qmsnapshot", 202))
        .unwrap();
    assert!(store
        .finish_snapshot(&op.id, Outcome::Failed(Failure::SnapshotStateUnconfirmed))
        .is_err());
    let result = store
        .finish_snapshot(
            &op.id,
            Outcome::Uncertain(Failure::SnapshotStateUnconfirmed),
        )
        .unwrap();
    assert_eq!(result.status, Status::Uncertain);
    assert!(store.claim_snapshot().unwrap().is_none());
}

#[test]
fn snapshot_schema_migration_preserves_provisioning_graph_and_restores_foreign_keys() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v10.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    // Construct an actual schema-10 operation table with retained child journals.
    let old = include_str!("../../migrations/0006_provisioning_intent.sql")
        .split("CREATE TABLE provisioning_intents")
        .next()
        .unwrap();
    connection
        .execute_batch("DROP TABLE recreation_sources;")
        .unwrap();
    connection.execute_batch(old).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let provision = "d".repeat(32);
    connection.execute("INSERT INTO operations (id,request_key,kind,instance_id,status,approval_json,input_hash,confirmed_at,failure_code) VALUES (?1,'retained','vm_provision',?2,'failed','{}',?3,1,'provider_unavailable')",rusqlite::params![provision,approval.instance_id,"e".repeat(64)]).unwrap();
    connection.execute("INSERT INTO provisioning_intents (operation_id,catalog_yaml,plan_json) VALUES (?1,'retained-catalog','{}')",[&provision]).unwrap();
    connection.execute("INSERT INTO provisioning_stages (operation_id,position,kind,status) VALUES (?1,0,'full_clone','queued')",[&provision]).unwrap();
    connection.execute("INSERT INTO provisioning_seed_regeneration (operation_id,journal_json) VALUES (?1,'{}')",[&provision]).unwrap();
    connection
        .execute_batch("DROP TABLE IF EXISTS recreation_sources;")
        .unwrap();
    connection.pragma_update(None, "user_version", 10).unwrap();
    drop(connection);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        store.operation(&provision).unwrap().unwrap().status,
        Status::Failed
    );
    // Failed provisioning is retained as a blocker, even though it is not active.
    assert_eq!(
        store.enqueue_snapshot("blocked", &approval).unwrap_err(),
        SnapshotError::Conflict
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    for table in [
        "provisioning_intents",
        "provisioning_stages",
        "provisioning_seed_regeneration",
    ] {
        assert_eq!(
            connection
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE operation_id=?1"),
                    [&provision],
                    |r| r.get::<_, u32>(0)
                )
                .unwrap(),
            1
        );
    }
    assert!(!connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        17
    );
    // Use the store's connection through an operation, not an unrelated handle's pragma.
    connection
        .execute("DELETE FROM provisioning_seed_regeneration", [])
        .unwrap();
    connection
        .execute("DELETE FROM provisioning_stages", [])
        .unwrap();
    connection
        .execute("DELETE FROM provisioning_intents", [])
        .unwrap();
    connection.execute("DELETE FROM operations", []).unwrap();
    let operation = store
        .enqueue_snapshot("after-migration", &approval)
        .unwrap();
    store.claim_snapshot().unwrap().unwrap();
    store
        .finish_snapshot(
            &operation.id,
            Outcome::Failed(Failure::ProviderNotConfigured),
        )
        .unwrap();
    let mut orphan = approval;
    orphan.instance_id = "f".repeat(32);
    assert_eq!(
        store.enqueue_snapshot("orphan", &orphan).unwrap_err(),
        SnapshotError::InstanceNotFound
    );
}

#[test]
fn snapshot_schema_migration_rejects_orphaned_journals_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("broken.sqlite");
    let store = Store::open(&path).unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    // Deliberately inject corruption into this owned fixture only.
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute("INSERT INTO provisioning_intents (operation_id,catalog_yaml,plan_json) VALUES (?1,'retained','{}')",["a".repeat(32)]).unwrap();
    connection
        .execute_batch("DROP TABLE IF EXISTS recreation_sources;")
        .unwrap();
    connection.pragma_update(None, "user_version", 10).unwrap();
    drop(connection);
    assert!(Store::open(&path).is_err());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        10
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM provisioning_intents", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert!(!connection
        .prepare("SELECT name FROM sqlite_master WHERE name='operations_v11'")
        .unwrap()
        .exists([])
        .unwrap());
}

#[test]
fn snapshot_journal_concurrent_connections_share_one_confirmed_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let path = path.clone();
            let approval = approval.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let store = Store::open(path).unwrap();
                barrier.wait();
                store.enqueue_snapshot("same-key", &approval).unwrap().id
            })
        })
        .collect();
    barrier.wait();
    let ids: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(ids[0], ids[1]);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
}

#[test]
fn snapshot_journal_corruption_cannot_be_claimed_or_advance_status() {
    for dispatched in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("corrupt.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let op = store.enqueue_snapshot("corrupt", &approval).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        if dispatched {
            store.claim_snapshot().unwrap().unwrap();
            store.begin_snapshot_dispatch(&op.id, &approval).unwrap();
            store
                .link_snapshot_task(&op.id, &task("qmsnapshot", 202))
                .unwrap();
            store.recover_interrupted_operations().unwrap();
            connection
                .execute(
                    "UPDATE operations SET provider_upid=?1 WHERE id=?2",
                    rusqlite::params![
                        "UPID:pve-test:00000001:00000002:66F00000:qmstart:202:reader@pve!fixture:",
                        op.id
                    ],
                )
                .unwrap();
        } else {
            connection.execute("UPDATE operations SET approval_json=json_set(approval_json,'$.request.name','after-work') WHERE id=?1",[&op.id]).unwrap();
        }
        assert_eq!(
            store.claim_snapshot().unwrap_err(),
            SnapshotError::Unavailable
        );
        assert_eq!(
            store.operation(&op.id).unwrap().unwrap().status,
            if dispatched {
                Status::Uncertain
            } else {
                Status::Queued
            }
        );
    }
}
