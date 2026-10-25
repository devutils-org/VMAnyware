use vmanyware_server::{
    definition::Definition,
    deletion::{Approval, Effects, Outcome, Plan},
    instance::Registration,
    lifecycle,
    operation::{Failure, Status},
    proxmox::{DeletionVolume, DeletionVolumeEffect, TaskReference},
    store::{DeletionError, Store},
};
fn effects() -> Effects {
    serde_json::from_value(serde_json::json!({"version":1,"removeVmConfiguration":true,"removeSnapshotMetadata":true,"removeVmPermissions":true,"removeVmFirewall":true,"cleanupIpam":true,"cleanupRecordedFleecingImages":true,"purgeJobs":false,"destroyUnreferencedDisks":false,"retainDefinitionAndHistory":true})).unwrap()
}
fn setup(store: &Store) -> Approval {
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
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
        confirmed: true,
        acknowledge_data_loss: true,
        plan: Plan {
            scope_hash: "c".repeat(64),
            volumes: vec![DeletionVolume {
                volume: "storage:vm-202-disk-0".into(),
                references: vec!["live:scsi0".into(), "snapshot/before-work:scsi0".into()],
                owner_verified: true,
                effect: DeletionVolumeEffect::DeleteOwnedVolume,
            }],
            snapshots: vec!["before-work".into()],
            preserved_media: vec!["local:iso/image.iso".into()],
            preserved_owned_volumes: vec!["storage:vm-202-orphan".into()],
            effects: effects(),
        },
    }
}
fn action(approval: &Approval) -> lifecycle::Approval {
    lifecycle::Approval {
        action: lifecycle::Action::Start,
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        definition_hash: approval.definition_hash.clone(),
        node: approval.node.clone(),
        vmid: 202,
        config_digest: approval.config_digest.clone(),
        confirmed: true,
    }
}
fn task(kind: &str, vmid: u32) -> TaskReference {
    TaskReference::new(
        "pve-test",
        &format!("UPID:pve-test:00000001:00000002:66F00000:{kind}:{vmid}:fixture@pve:"),
    )
    .unwrap()
}
#[test]
fn deletion_approval_binds_full_scope_and_both_confirmations() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    for field in [
        "confirmation",
        "loss",
        "purge",
        "owner",
        "effect",
        "host-path",
        "reference",
        "retention",
    ] {
        let mut invalid = approval.clone();
        match field {
            "confirmation" => invalid.confirmed = false,
            "loss" => invalid.acknowledge_data_loss = false,
            "purge" => invalid.plan.effects.purge_jobs = true,
            "owner" => invalid.plan.volumes[0].owner_verified = false,
            "effect" => invalid.plan.volumes[0].effect = DeletionVolumeEffect::SnapshotReference,
            "host-path" => invalid.plan.volumes[0].volume = "/private/path".into(),
            "reference" => {
                invalid.plan.volumes[0].references = vec!["snapshot/unreviewed:scsi0".into()]
            }
            _ => invalid.plan.effects.retain_definition_and_history = false,
        }
        assert_eq!(
            store.enqueue_deletion(field, &invalid).unwrap_err(),
            DeletionError::InvalidApproval,
            "{field}"
        );
    }
    let operation = store.enqueue_deletion("same-key", &approval).unwrap();
    assert_eq!(operation.kind, "vm_delete");
    assert_eq!(
        store.enqueue_deletion("same-key", &approval).unwrap().id,
        operation.id
    );
    for field in ["digest", "scope", "preserved", "snapshot"] {
        let mut changed = approval.clone();
        match field {
            "digest" => changed.config_digest = "d".repeat(40),
            "scope" => changed.plan.scope_hash = "e".repeat(64),
            "preserved" => changed.plan.preserved_owned_volumes.clear(),
            _ => changed.plan.snapshots.push("other-work".into()),
        }
        assert_eq!(
            store.enqueue_deletion("same-key", &changed).unwrap_err(),
            DeletionError::Conflict,
            "{field}"
        );
    }
    let mut unknown = serde_json::to_value(&approval).unwrap();
    unknown["force"] = true.into();
    assert!(serde_json::from_value::<Approval>(unknown).is_err());
    assert!(store.claim_operation().unwrap().is_none());
}
#[test]
fn deletion_and_existing_mutations_exclude_each_other() {
    for deletion_first in [false, true] {
        let store = Store::memory().unwrap();
        let approval = setup(&store);
        if deletion_first {
            store.enqueue_deletion("delete", &approval).unwrap();
            assert_eq!(
                store
                    .enqueue_lifecycle("start", &action(&approval))
                    .unwrap_err(),
                DeletionError::Conflict
            );
        } else {
            store
                .enqueue_lifecycle("start", &action(&approval))
                .unwrap();
            assert_eq!(
                store.enqueue_deletion("delete", &approval).unwrap_err(),
                DeletionError::Conflict
            );
        }
    }
}
#[test]
fn deletion_dispatch_is_once_only_fresh_and_does_not_renew_on_retry() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("expiry.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = store.enqueue_deletion("expiry", &approval).unwrap();
    assert!(store
        .begin_deletion_dispatch(&operation.id, &approval)
        .is_err());
    store.claim_deletion().unwrap().unwrap();
    let mut changed = approval.clone();
    changed.plan.scope_hash = "d".repeat(64);
    assert_eq!(
        store
            .begin_deletion_dispatch(&operation.id, &changed)
            .unwrap_err(),
        DeletionError::TargetMismatch
    );
    for age in [-121, 10] {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE operations SET confirmed_at=unixepoch()+?1 WHERE id=?2",
                rusqlite::params![age, operation.id],
            )
            .unwrap();
        let confirmed = store
            .deletion_job(&operation.id)
            .unwrap()
            .unwrap()
            .confirmed_at;
        store.enqueue_deletion("expiry", &approval).unwrap();
        assert_eq!(
            store
                .deletion_job(&operation.id)
                .unwrap()
                .unwrap()
                .confirmed_at,
            confirmed
        );
        assert_eq!(
            store
                .begin_deletion_dispatch(&operation.id, &approval)
                .unwrap_err(),
            DeletionError::ApprovalExpired
        );
    }
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE operations SET confirmed_at=unixepoch() WHERE id=?1",
            [&operation.id],
        )
        .unwrap();
    store
        .begin_deletion_dispatch(&operation.id, &approval)
        .unwrap();
    assert!(store
        .begin_deletion_dispatch(&operation.id, &approval)
        .is_err());
}
#[test]
fn deletion_unknown_submission_stays_uncertain_across_reopen_and_never_reclaims() {
    for marked in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("restart.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let original = store.instance(&approval.instance_id).unwrap().unwrap();
        let operation = store.enqueue_deletion("restart", &approval).unwrap();
        store.claim_deletion().unwrap().unwrap();
        if marked {
            store
                .begin_deletion_dispatch(&operation.id, &approval)
                .unwrap();
        }
        drop(store);
        let store = Store::open(&path).unwrap();
        store.recover_interrupted_operations().unwrap();
        let job = store.deletion_job(&operation.id).unwrap().unwrap();
        assert_eq!(
            job.operation.status,
            if marked {
                Status::Uncertain
            } else {
                Status::Interrupted
            }
        );
        assert!(store.claim_deletion().unwrap().is_none());
        assert_eq!(
            store.enqueue_deletion("restart", &approval).unwrap().id,
            operation.id
        );
        if marked {
            assert_eq!(
                store.enqueue_deletion("other", &approval).unwrap_err(),
                DeletionError::Conflict
            );
        }
        assert_eq!(
            serde_json::to_value(store.instance(&approval.instance_id).unwrap().unwrap()).unwrap(),
            serde_json::to_value(original).unwrap()
        );
        assert!(store.get("kernel-lab").unwrap().is_some());
    }
}
#[test]
fn deletion_known_task_only_reconciles_and_partial_task_failure_holds_target() {
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let operation = store.enqueue_deletion("task", &approval).unwrap();
    store.claim_deletion().unwrap().unwrap();
    assert!(store
        .link_deletion_task(&operation.id, &task("qmdestroy", 202))
        .is_err());
    store
        .begin_deletion_dispatch(&operation.id, &approval)
        .unwrap();
    assert!(store
        .finish_deletion(
            &operation.id,
            Outcome::Uncertain(Failure::DeletionStateUnconfirmed)
        )
        .is_err());
    for wrong in [task("qmsnapshot", 202), task("qmdestroy", 203)] {
        assert_eq!(
            store.link_deletion_task(&operation.id, &wrong).unwrap_err(),
            DeletionError::TargetMismatch
        );
    }
    store
        .link_deletion_task(&operation.id, &task("qmdestroy", 202))
        .unwrap();
    store.recover_interrupted_operations().unwrap();
    let recovered = store.claim_deletion().unwrap().unwrap();
    assert!(recovered.dispatch_started && recovered.provider_task.is_some());
    assert!(store
        .begin_deletion_dispatch(&operation.id, &approval)
        .is_err());
    assert!(store
        .finish_deletion(&operation.id, Outcome::Failed(Failure::ProviderTaskFailed))
        .is_err());
    store
        .finish_deletion(
            &operation.id,
            Outcome::Uncertain(Failure::ProviderTaskFailed),
        )
        .unwrap();
    assert!(store.claim_deletion().unwrap().is_none());
    assert_eq!(
        store
            .enqueue_lifecycle("start", &action(&approval))
            .unwrap_err(),
        DeletionError::Conflict
    );
    assert!(store
        .finish_operation(&operation.id, Ok(serde_json::json!({"deleted":true})))
        .is_err());
}

#[test]
fn deletion_schema_twelve_upgrade_preserves_restore_and_failed_provisioning_graph() {
    use vmanyware_server::snapshot_restore::{Approval as RestoreApproval, Request};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v12.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    // Remove schema-15+ objects before constructing this historical schema fixture.
    connection
        .execute_batch("DROP TABLE recreation_sources;")
        .unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0012_snapshot_restore.sql"))
        .unwrap();
    connection.pragma_update(None, "user_version", 12).unwrap();
    let restore = RestoreApproval {
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        definition_hash: approval.definition_hash.clone(),
        node: approval.node.clone(),
        vmid: 202,
        config_digest: approval.config_digest.clone(),
        request: Request {
            snapshot_name: "before-work".into(),
        },
        snapshot_config_hash: "d".repeat(64),
        confirmed: true,
        acknowledge_data_loss: true,
    };
    let operation = store.enqueue_restore("retained-restore", &restore).unwrap();
    store.claim_restore().unwrap().unwrap();
    store
        .begin_restore_dispatch(&operation.id, &restore)
        .unwrap();
    store
        .link_restore_task(&operation.id, &task("qmrollback", 202))
        .unwrap();
    store
        .finish_restore(
            &operation.id,
            vmanyware_server::snapshot_restore::Outcome::Uncertain(
                Failure::RestoreStateUnconfirmed,
            ),
        )
        .unwrap();
    let before = serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap();
    let provision = "d".repeat(32);
    connection.execute("INSERT INTO operations(id,request_key,kind,instance_id,status,approval_json,input_hash,confirmed_at,failure_code) VALUES (?1,'failed-provision','vm_provision',?2,'failed','{}',?3,1,'provider_unavailable')",rusqlite::params![provision,approval.instance_id,"e".repeat(64)]).unwrap();
    for sql in ["INSERT INTO provisioning_intents(operation_id,catalog_yaml,plan_json) VALUES (?1,'retained','{}')","INSERT INTO provisioning_stages(operation_id,position,kind,status) VALUES (?1,0,'full_clone','queued')","INSERT INTO provisioning_seed_regeneration(operation_id,journal_json) VALUES (?1,'{}')"] {connection.execute(sql,[&provision]).unwrap();}
    // Schema-17 columns did not exist at schema 12; drop them before the upgrade.
    connection
        .execute_batch(
            "ALTER TABLE instances DROP COLUMN guest_address;
             ALTER TABLE instances DROP COLUMN guest_user;
             ALTER TABLE instances DROP COLUMN guest_observed_at;",
        )
        .unwrap();
    drop(connection);
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        serde_json::to_value(store.operation(&operation.id).unwrap().unwrap()).unwrap(),
        before
    );
    assert!(store
        .restore_job(&operation.id)
        .unwrap()
        .unwrap()
        .provider_task
        .is_some());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        17
    );
    assert!(!connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
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
                    |row| row.get::<_, u32>(0)
                )
                .unwrap(),
            1
        );
    }
    assert_eq!(
        store
            .enqueue_deletion("blocked-active", &approval)
            .unwrap_err(),
        DeletionError::Conflict
    );
    connection
        .execute("DELETE FROM operations WHERE id=?1", [&operation.id])
        .unwrap();
    // Failed provisioning remains recorded but must not prevent explicit cleanup.
    assert_eq!(
        store
            .enqueue_deletion("cleanup-failed", &approval)
            .unwrap()
            .status,
        Status::Queued
    );
    assert_eq!(
        store.operation(&provision).unwrap().unwrap().status,
        Status::Failed
    );
    assert!(store.get("kernel-lab").unwrap().is_some());
}
#[test]
fn deletion_schema_twelve_orphan_refuses_upgrade_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("broken.sqlite");
    drop(Store::open(&path).unwrap());
    let connection = rusqlite::Connection::open(&path).unwrap();
    // Remove schema-15 objects before constructing this historical schema fixture.
    connection
        .execute_batch("DROP TABLE recreation_sources;")
        .unwrap();
    connection
        .execute_batch(include_str!("../../migrations/0012_snapshot_restore.sql"))
        .unwrap();
    connection.pragma_update(None, "user_version", 12).unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute("INSERT INTO provisioning_intents(operation_id,catalog_yaml,plan_json) VALUES ('orphan','retained','{}')",[]).unwrap();
    drop(connection);
    assert!(Store::open(&path).is_err());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        12
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM provisioning_intents", [], |row| row
                .get::<_, u32>(
                0
            ))
            .unwrap(),
        1
    );
    assert!(!connection
        .prepare("SELECT name FROM sqlite_master WHERE name='operations_v13'")
        .unwrap()
        .exists([])
        .unwrap());
}
#[test]
fn deletion_corruption_rolls_back_claim_and_concurrent_requests_keep_one_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let handles = (0..2)
        .map(|_| {
            let path = path.clone();
            let approval = approval.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let store = Store::open(path).unwrap();
                barrier.wait();
                store.enqueue_deletion("same", &approval).unwrap().id
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let ids = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids[0], ids[1]);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    rusqlite::Connection::open(&path).unwrap().execute("UPDATE operations SET approval_json=json_set(approval_json,'$.plan.preservedOwnedVolumes',json('[]')) WHERE id=?1",[&ids[0]]).unwrap();
    assert_eq!(
        store.claim_deletion().unwrap_err(),
        DeletionError::Unavailable
    );
    assert_eq!(
        store.operation(&ids[0]).unwrap().unwrap().status,
        Status::Queued
    );
}

fn legacy_thirteen(path: &std::path::Path) -> rusqlite::Connection {
    let connection = rusqlite::Connection::open(path).unwrap();
    for migration in [
        include_str!("../../migrations/0001_definitions.sql"),
        include_str!("../../migrations/0002_operations.sql"),
        include_str!("../../migrations/0003_instances.sql"),
        include_str!("../../migrations/0004_instance_observations.sql"),
        include_str!("../../migrations/0005_lifecycle_operations.sql"),
        include_str!("../../migrations/0006_provisioning_intent.sql"),
        include_str!("../../migrations/0007_resource_dispatch.sql"),
        include_str!("../../migrations/0008_disk_dispatch.sql"),
        include_str!("../../migrations/0009_cloud_init_dispatch.sql"),
        include_str!("../../migrations/0010_seed_regeneration.sql"),
        include_str!("../../migrations/0011_snapshot_creation.sql"),
        include_str!("../../migrations/0012_snapshot_restore.sql"),
        include_str!("../../migrations/0013_deletion_intent.sql"),
    ] {
        connection.execute_batch(migration).unwrap();
    }
    connection.pragma_update(None, "user_version", 13).unwrap();
    connection
}
#[test]
fn retirement_schema_thirteen_preserves_marked_delete_and_rejects_orphan_atomically() {
    for orphan in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("actual-v13.sqlite");
        let connection = legacy_thirteen(&path);
        let approval = setup(&Store::memory().unwrap());
        let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
        connection.execute("INSERT INTO definitions(name,definition_yaml,definition_hash,revision) VALUES ('kernel-lab',?1,?2,1)",rusqlite::params![definition.canonical_yaml().unwrap(),approval.definition_hash]).unwrap();
        connection.execute("INSERT INTO instances(id,definition_name,definition_hash,definition_revision,definition_yaml,provider_scope,node,vmid) VALUES (?1,'kernel-lab',?2,1,?3,?4,?5,202)",rusqlite::params![approval.instance_id,approval.definition_hash,definition.canonical_yaml().unwrap(),approval.provider_scope_id,approval.node]).unwrap();
        let task = task("qmdestroy", 202);
        let operation_id = "f".repeat(32);
        if orphan {
            connection
                .pragma_update(None, "foreign_keys", false)
                .unwrap();
        }
        connection.execute("INSERT INTO operations(id,request_key,kind,instance_id,status,approval_json,input_hash,confirmed_at,dispatch_started,provider_upid) VALUES (?1,'v13-delete','vm_delete',?2,'running',?3,?4,unixepoch(),1,?5)",rusqlite::params![operation_id,if orphan{"0".repeat(32)}else{approval.instance_id.clone()},serde_json::to_string(&approval).unwrap(),approval.input_hash().unwrap(),"UPID:pve-test:00000001:00000002:66F00000:qmdestroy:202:fixture@pve:"]).unwrap();
        if orphan {
            assert!(Store::open(&path).is_err());
            assert_eq!(
                connection
                    .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                13
            );
            assert!(!connection.prepare("SELECT 1 FROM sqlite_master WHERE name='instances_v14' OR name='one_registered_provider_vmid'").unwrap().exists([]).unwrap());
        } else {
            let store = Store::open(&path).unwrap();
            let retained = store.deletion_job(&operation_id).unwrap().unwrap();
            assert_eq!(retained.provider_task, Some(task));
            assert!(retained.dispatch_started);
            assert_eq!(retained.operation.status, Status::Running);
            assert_eq!(
                retained.approval.input_hash().unwrap(),
                approval.input_hash().unwrap()
            );
            assert_eq!(
                store
                    .instance(&approval.instance_id)
                    .unwrap()
                    .unwrap()
                    .summary
                    .status,
                vmanyware_server::instance::Status::Registered
            );
            assert_eq!(
                connection
                    .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                17
            );
            assert!(!connection
                .prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap());
            assert!(store
                .register_instance(
                    &Registration {
                        definition_name: "kernel-lab".into(),
                        definition_hash: approval.definition_hash.clone(),
                        node: "different-node".into(),
                        vmid: 202
                    },
                    &approval.provider_scope_id
                )
                .is_err());
        }
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM instances", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM operations", [], |r| r
                    .get::<_, u32>(0))
                .unwrap(),
            1
        );
    }
}

#[test]
fn deletion_approval_matches_native_golden_contract() {
    let approval: Approval =
        serde_json::from_str(include_str!("../../fixtures/deletion-approval.json")).unwrap();
    assert!(approval.is_valid());
    assert_eq!(
        approval.input_hash().unwrap(),
        include_str!("../../fixtures/deletion-approval.sha256").trim()
    );
}
