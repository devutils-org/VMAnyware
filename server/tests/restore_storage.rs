use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    lifecycle,
    operation::{Failure, Status},
    proxmox::{PowerState, QmpState, TaskReference, TaskStatus, VmObservation},
    snapshot_restore::{Approval, Outcome, Request, ResultEvidence},
    store::{RestoreError, Store},
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
        snapshot_config_hash: "c".repeat(64),
        acknowledge_data_loss: true,
        request: Request {
            snapshot_name: "before-work".into(),
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
        snapshot_name: approval.request.snapshot_name.clone(),
        snapshot_config_hash: approval.snapshot_config_hash.clone(),
        observation: VmObservation {
            node: approval.node.clone(),
            vmid: approval.vmid,
            status: PowerState::Stopped,
            qmp_status: Some(QmpState::Stopped),
        },
    }
}
#[test]
fn restore_journal_binds_confirmation_target_request_and_idempotency() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let mut invalid = approval.clone();
    invalid.confirmed = false;
    assert_eq!(
        store.enqueue_restore("key", &invalid).unwrap_err(),
        RestoreError::InvalidApproval
    );
    invalid = approval.clone();
    invalid.acknowledge_data_loss = false;
    assert_eq!(
        store.enqueue_restore("key", &invalid).unwrap_err(),
        RestoreError::InvalidApproval
    );
    let mut json = serde_json::to_value(&approval).unwrap();
    json.as_object_mut().unwrap().remove("acknowledgeDataLoss");
    assert!(serde_json::from_value::<Approval>(json).is_err());
    let mut json = serde_json::to_value(&approval).unwrap();
    json["start"] = true.into();
    assert!(serde_json::from_value::<Approval>(json).is_err());
    invalid = approval.clone();
    invalid.vmid += 1;
    assert_eq!(
        store.enqueue_restore("key", &invalid).unwrap_err(),
        RestoreError::TargetMismatch
    );
    let operation = store.enqueue_restore("key", &approval).unwrap();
    assert_eq!(operation.kind, "vm_snapshot_restore");
    assert_eq!(
        store.enqueue_restore("key", &approval).unwrap().id,
        operation.id
    );
    for field in ["name", "saved-config", "digest"] {
        let mut changed = approval.clone();
        match field {
            "name" => changed.request.snapshot_name = "after-work".into(),
            "saved-config" => changed.snapshot_config_hash = "d".repeat(64),
            _ => changed.config_digest = "c".repeat(40),
        }
        assert_eq!(
            store.enqueue_restore("key", &changed).unwrap_err(),
            RestoreError::Conflict
        );
    }
    assert_eq!(
        store.enqueue_restore("new-key", &approval).unwrap_err(),
        RestoreError::Conflict
    );
    assert_eq!(
        store
            .enqueue_lifecycle("start", &lifecycle(&approval))
            .unwrap_err(),
        RestoreError::Conflict
    );
    assert!(store.claim_operation().unwrap().is_none());
}
#[test]
fn restore_journal_cannot_overlap_an_existing_lifecycle_action() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    store
        .enqueue_lifecycle("start", &lifecycle(&approval))
        .unwrap();
    assert_eq!(
        store.enqueue_restore("snapshot", &approval).unwrap_err(),
        RestoreError::Conflict
    );
}
#[test]
fn restore_journal_dispatch_requires_claim_exact_approval_and_fresh_age() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("expiry.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let op = store.enqueue_restore("expiry", &approval).unwrap();
    assert_eq!(
        store.begin_restore_dispatch(&op.id, &approval).unwrap_err(),
        RestoreError::InvalidOperation
    );
    store.claim_restore().unwrap().unwrap();
    let mut changed = approval.clone();
    changed.snapshot_config_hash = "d".repeat(64);
    assert_eq!(
        store.begin_restore_dispatch(&op.id, &changed).unwrap_err(),
        RestoreError::TargetMismatch
    );
    for age in [-121, 10] {
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE operations SET confirmed_at=unixepoch()+?1 WHERE id=?2",
                rusqlite::params![age, op.id],
            )
            .unwrap();
        let original_age = store.restore_job(&op.id).unwrap().unwrap().confirmed_at;
        store.enqueue_restore("expiry", &approval).unwrap();
        assert_eq!(
            store.restore_job(&op.id).unwrap().unwrap().confirmed_at,
            original_age
        );
        assert_eq!(
            store.begin_restore_dispatch(&op.id, &approval).unwrap_err(),
            RestoreError::ApprovalExpired
        );
    }
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE operations SET confirmed_at=unixepoch() WHERE id=?1",
            [&op.id],
        )
        .unwrap();
    store.begin_restore_dispatch(&op.id, &approval).unwrap();
    assert_eq!(
        store.begin_restore_dispatch(&op.id, &approval).unwrap_err(),
        RestoreError::InvalidOperation
    );
}
#[test]
fn restore_journal_restart_never_reclaims_unknown_submission() {
    for dispatched in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("restart.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let op = store.enqueue_restore("restart", &approval).unwrap();
        store.claim_restore().unwrap().unwrap();
        if dispatched {
            store.begin_restore_dispatch(&op.id, &approval).unwrap();
        }
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(
            store.operation(&op.id).unwrap().unwrap().status,
            Status::Running
        );
        assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
        let result = store.restore_job(&op.id).unwrap().unwrap();
        assert_eq!(
            result.operation.status,
            if dispatched {
                Status::Uncertain
            } else {
                Status::Interrupted
            }
        );
        assert!(store.claim_restore().unwrap().is_none());
        assert_eq!(
            store.enqueue_restore("restart", &approval).unwrap().id,
            op.id
        );
        if dispatched {
            assert_eq!(
                store.enqueue_restore("another", &approval).unwrap_err(),
                RestoreError::Conflict
            );
        }
    }
}
#[test]
fn restore_journal_retained_task_only_reconciles_and_needs_exact_snapshot_and_stopped_target() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("task.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let op = store.enqueue_restore("task", &approval).unwrap();
    store.claim_restore().unwrap().unwrap();
    assert!(store
        .link_restore_task(&op.id, &task("qmrollback", 202))
        .is_err());
    store.begin_restore_dispatch(&op.id, &approval).unwrap();
    assert!(store
        .finish_restore(&op.id, Outcome::Succeeded(evidence(&approval)))
        .is_err());
    let wrong_node = TaskReference::new(
        "other-node",
        "UPID:other-node:00000001:00000002:66F00000:qmrollback:202:fixture@pve:",
    )
    .unwrap();
    for wrong in [
        task("qmstart", 202),
        task("qmsnapshot", 202),
        task("qmrollback", 203),
        wrong_node,
    ] {
        assert_eq!(
            store.link_restore_task(&op.id, &wrong).unwrap_err(),
            RestoreError::TargetMismatch
        );
    }
    store
        .link_restore_task(&op.id, &task("qmrollback", 202))
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    store.recover_interrupted_operations().unwrap();
    let recovered = store.claim_restore().unwrap().unwrap();
    assert!(recovered.dispatch_started && recovered.provider_task.is_some());
    assert!(store.begin_restore_dispatch(&op.id, &approval).is_err());
    for field in [
        "name",
        "saved-config",
        "power",
        "qmp",
        "observed-target",
        "status",
        "target",
    ] {
        let mut wrong = evidence(&approval);
        match field {
            "name" => wrong.snapshot_name = "after-work".into(),
            "saved-config" => wrong.snapshot_config_hash = "d".repeat(64),
            "power" => wrong.observation.status = PowerState::Running,
            "qmp" => wrong.observation.qmp_status = Some(QmpState::Paused),
            "observed-target" => wrong.observation.vmid += 1,
            "status" => wrong.task_status = TaskStatus::Running,
            _ => wrong.vmid += 1,
        }
        assert_eq!(
            store
                .finish_restore(&op.id, Outcome::Succeeded(wrong))
                .unwrap_err(),
            RestoreError::InvalidOperation
        );
    }
    let done = store
        .finish_restore(&op.id, Outcome::Succeeded(evidence(&approval)))
        .unwrap();
    assert_eq!(done.status, Status::Succeeded);
    assert!(store
        .finish_restore(&op.id, Outcome::Succeeded(evidence(&approval)))
        .is_err());
    assert!(store.claim_restore().unwrap().is_none());
    assert_eq!(store.enqueue_restore("task", &approval).unwrap().id, op.id);
}
#[test]
fn restore_journal_requires_uncertainty_after_dispatch_and_enforces_retry_backoff() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let op = store.enqueue_restore("uncertain", &approval).unwrap();
    store.claim_restore().unwrap().unwrap();
    assert!(store
        .finish_restore(
            &op.id,
            Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed)
        )
        .is_err());
    store.begin_restore_dispatch(&op.id, &approval).unwrap();
    assert!(store
        .finish_restore(&op.id, Outcome::Failed(Failure::ProviderUnavailable))
        .is_err());
    store
        .link_restore_task(&op.id, &task("qmrollback", 202))
        .unwrap();
    assert!(store
        .finish_restore(&op.id, Outcome::Failed(Failure::RestoreStateUnconfirmed))
        .is_err());
    let result = store
        .finish_restore(&op.id, Outcome::Uncertain(Failure::RestoreStateUnconfirmed))
        .unwrap();
    assert_eq!(result.status, Status::Uncertain);
    assert!(store.claim_restore().unwrap().is_none());
}

#[test]
fn restore_journal_concurrent_connections_share_one_confirmed_identity() {
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
                store.enqueue_restore("same-key", &approval).unwrap().id
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
fn restore_journal_corruption_cannot_be_claimed_or_advance_status() {
    for dispatched in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("corrupt.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let op = store.enqueue_restore("corrupt", &approval).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        if dispatched {
            store.claim_restore().unwrap().unwrap();
            store.begin_restore_dispatch(&op.id, &approval).unwrap();
            store
                .link_restore_task(&op.id, &task("qmrollback", 202))
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
            connection.execute("UPDATE operations SET approval_json=json_set(approval_json,'$.request.snapshotName','after-work') WHERE id=?1",[&op.id]).unwrap();
        }
        assert_eq!(
            store.claim_restore().unwrap_err(),
            RestoreError::Unavailable
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

#[test]
fn restore_schema_eleven_upgrade_preserves_snapshot_and_provisioning_journals() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v11.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    // Rebuild an actual schema-11 fixture, not just its version marker.
    // Remove schema-15 objects before constructing this historical schema fixture.
    connection
        .execute_batch("DROP TABLE recreation_sources;")
        .unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0011_snapshot_creation.sql"))
        .unwrap();
    connection.pragma_update(None, "user_version", 11).unwrap();
    let create = vmanyware_server::snapshot::Approval {
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        definition_hash: approval.definition_hash.clone(),
        node: approval.node.clone(),
        vmid: approval.vmid,
        config_digest: approval.config_digest.clone(),
        confirmed: true,
        request: vmanyware_server::snapshot::CreateRequest {
            name: "before-work".into(),
            description: "Retain café".into(),
        },
    };
    let op = store
        .enqueue_snapshot("retained-snapshot", &create)
        .unwrap();
    store.claim_snapshot().unwrap().unwrap();
    store.begin_snapshot_dispatch(&op.id, &create).unwrap();
    store
        .link_snapshot_task(&op.id, &task("qmsnapshot", 202))
        .unwrap();
    let before =
        serde_json::to_value(store.snapshot_job(&op.id).unwrap().unwrap().operation).unwrap();
    let provision = "d".repeat(32);
    connection.execute("INSERT INTO operations (id,request_key,kind,instance_id,status,approval_json,input_hash,confirmed_at,failure_code) VALUES (?1,'retained-provision','vm_provision',?2,'failed','{}',?3,1,'provider_unavailable')",rusqlite::params![provision,approval.instance_id,"e".repeat(64)]).unwrap();
    for sql in [
        "INSERT INTO provisioning_intents(operation_id,catalog_yaml,plan_json) VALUES (?1,'retained-catalog','{}')",
        "INSERT INTO provisioning_stages(operation_id,position,kind,status) VALUES (?1,0,'full_clone','queued')",
        "INSERT INTO provisioning_seed_regeneration(operation_id,journal_json) VALUES (?1,'{}')",
    ] {connection.execute(sql,[&provision]).unwrap();}
    drop(connection);
    drop(store);
    let store = Store::open(&path).unwrap();
    let retained = store.snapshot_job(&op.id).unwrap().unwrap();
    assert_eq!(serde_json::to_value(&retained.operation).unwrap(), before);
    assert!(retained.dispatch_started && retained.provider_task.is_some());
    assert_eq!(retained.approval.request.description, "Retain café");
    // Opening a database does not recover/claim running work.
    assert_eq!(retained.operation.status, Status::Running);
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        17
    );
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
    connection
        .execute("DELETE FROM operations WHERE id=?1", [&op.id])
        .unwrap();
    // Failed provisioning still blocks restore after active snapshot work is removed.
    assert_eq!(
        store.enqueue_restore("blocked", &approval).unwrap_err(),
        RestoreError::Conflict
    );
}

#[test]
fn restore_schema_eleven_corruption_aborts_migration_without_replacing_tables() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("broken-v11.sqlite");
    drop(Store::open(&path).unwrap());
    let connection = rusqlite::Connection::open(&path).unwrap();
    // Remove schema-15 objects before constructing this historical schema fixture.
    connection
        .execute_batch("DROP TABLE recreation_sources;")
        .unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0011_snapshot_creation.sql"))
        .unwrap();
    connection.pragma_update(None, "user_version", 11).unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute("INSERT INTO provisioning_intents(operation_id,catalog_yaml,plan_json) VALUES ('orphan','fixture','{}')",[]).unwrap();
    drop(connection);
    assert!(Store::open(&path).is_err());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        11
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM provisioning_intents", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert!(!connection
        .prepare("SELECT name FROM sqlite_master WHERE name='operations_v12'")
        .unwrap()
        .exists([])
        .unwrap());
}

#[test]
fn restore_and_snapshot_creation_cannot_overlap_in_either_direction() {
    for restore_first in [false, true] {
        let store = Store::memory().unwrap();
        let approval = setup(&store);
        let create = vmanyware_server::snapshot::Approval {
            instance_id: approval.instance_id.clone(),
            provider_scope_id: approval.provider_scope_id.clone(),
            definition_hash: approval.definition_hash.clone(),
            node: approval.node.clone(),
            vmid: approval.vmid,
            config_digest: approval.config_digest.clone(),
            confirmed: true,
            request: vmanyware_server::snapshot::CreateRequest {
                name: "other-snapshot".into(),
                description: String::new(),
            },
        };
        assert_ne!(approval.input_hash().unwrap(), create.input_hash().unwrap());
        if restore_first {
            store.enqueue_restore("restore", &approval).unwrap();
            assert_eq!(
                store.enqueue_snapshot("create", &create).unwrap_err(),
                RestoreError::Conflict
            );
        } else {
            store.enqueue_snapshot("create", &create).unwrap();
            assert_eq!(
                store.enqueue_restore("restore", &approval).unwrap_err(),
                RestoreError::Conflict
            );
        }
    }
}
