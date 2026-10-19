use rusqlite::params;
use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    lifecycle,
    operation::Status,
    provisioning::Catalog,
    provisioning_intent::Approval,
    proxmox::TaskReference,
    store::{LifecycleError, ProvisioningError, Store},
};
fn catalog() -> Catalog {
    Catalog::parse(include_str!("../../examples/templates.yaml")).unwrap()
}
fn setup(store: &Store) -> Approval {
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    setup_definition(store, definition)
}
fn setup_definition(store: &Store, definition: Definition) -> Approval {
    store.create(&definition).unwrap();
    let plan = catalog().plan(&definition, 202).unwrap();
    Approval {
        definition_name: definition.metadata.name,
        definition_hash: plan.definition_hash,
        provider_scope_id: "a".repeat(64),
        plan_hash: plan.plan_hash,
        target_node: "pve-test".into(),
        target_vmid: 202,
        source_vmid: 9000,
        storage: "local-lvm".into(),
        bridge: "vmbr0".into(),
        confirmed: true,
    }
}
fn task(vmid: u32) -> TaskReference {
    TaskReference::new(
        "pve-test",
        &format!("UPID:pve-test:00000001:00000002:00000003:qmclone:{vmid}:fixture@pve:"),
    )
    .unwrap()
}

#[test]
fn provisioning_intent_atomically_snapshots_definition_catalog_plan_and_ordered_stages() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("intent.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = store
        .enqueue_provisioning("confirmed", &approval, &catalog())
        .unwrap();
    assert_eq!(operation.kind, "vm_provision");
    assert_eq!(operation.status, Status::Queued);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.definition.hash().unwrap(), approval.definition_hash);
    assert_eq!(job.plan.plan_hash, approval.plan_hash);
    assert_eq!(
        job.stages
            .iter()
            .map(|s| s.kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "full_clone",
            "configure_resources",
            "expand_disk",
            "configure_cloud_init",
            "start_vm",
            "wait_for_guest_readiness",
            "install_declared_packages"
        ]
    );
    assert!(job
        .stages
        .iter()
        .all(|s| s.status == Status::Queued && !s.dispatch_started && s.task.is_none()));
    let instance = store
        .instance(operation.instance_id.as_ref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        instance.definition_yaml,
        job.definition.canonical_yaml().unwrap()
    );
    let original = job.confirmed_at;
    drop(store);
    let store = Store::open(&path).unwrap();
    let changed_catalog = Catalog::parse(
        &include_str!("../../examples/templates.yaml").replace("local-lvm", "other-storage"),
    )
    .unwrap();
    assert_eq!(
        store
            .enqueue_provisioning("confirmed", &approval, &changed_catalog)
            .unwrap()
            .id,
        operation.id
    );
    assert_eq!(
        store
            .provisioning_job(&operation.id)
            .unwrap()
            .unwrap()
            .confirmed_at,
        original
    );
    assert!(store.claim_operation().unwrap().is_none());
    assert!(store.claim_lifecycle().unwrap().is_none());
    assert!(store
        .finish_operation(
            &operation.id,
            Err(vmanyware_server::operation::Failure::ProviderUnavailable)
        )
        .is_err());
}

#[test]
fn provisioning_confirmation_is_input_bound_and_target_exclusion_covers_registration_and_lifecycle()
{
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let mut bad = approval.clone();
    bad.confirmed = false;
    assert_eq!(
        store
            .enqueue_provisioning("key", &bad, &catalog())
            .unwrap_err(),
        ProvisioningError::InvalidApproval
    );
    bad = approval.clone();
    bad.plan_hash = "f".repeat(64);
    assert_eq!(
        store
            .enqueue_provisioning("key", &bad, &catalog())
            .unwrap_err(),
        ProvisioningError::TargetChanged
    );
    assert!(store.recent_instances().unwrap().is_empty());
    let operation = store
        .enqueue_provisioning("key", &approval, &catalog())
        .unwrap();
    assert_eq!(
        store
            .enqueue_provisioning("new-key", &approval, &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
    assert!(store.enqueue_inventory("key").is_err());
    let mut changed = approval.clone();
    changed.provider_scope_id = "b".repeat(64);
    assert_eq!(
        store
            .enqueue_provisioning("key", &changed, &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
    let registration = Registration {
        definition_name: approval.definition_name.clone(),
        definition_hash: approval.definition_hash.clone(),
        node: "other-node".into(),
        vmid: 202,
    };
    assert!(store
        .register_instance(&registration, &approval.provider_scope_id)
        .is_err());
    let lifecycle = lifecycle::Approval {
        action: lifecycle::Action::Start,
        instance_id: operation.instance_id.unwrap(),
        provider_scope_id: approval.provider_scope_id,
        node: approval.target_node,
        vmid: 202,
        definition_hash: approval.definition_hash,
        config_digest: "b".repeat(40),
        confirmed: true,
    };
    assert_eq!(
        store.enqueue_lifecycle("start", &lifecycle).unwrap_err(),
        LifecycleError::Conflict
    );
    assert!(store
        .enqueue_instance_observation("diagnosis", &lifecycle.instance_id)
        .is_ok());
    let other = Store::memory().unwrap();
    let approval = setup(&other);
    other
        .register_instance(
            &Registration {
                definition_name: approval.definition_name.clone(),
                definition_hash: approval.definition_hash.clone(),
                node: "pve-test".into(),
                vmid: 202,
            },
            &approval.provider_scope_id,
        )
        .unwrap();
    assert_eq!(
        other
            .enqueue_provisioning("key", &approval, &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
}

#[test]
fn provisioning_dispatch_and_task_linkage_are_once_only_and_recover_without_replaying_unknown_submission(
) {
    for known in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recovery.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let operation = store
            .enqueue_provisioning("key", &approval, &catalog())
            .unwrap();
        let job = store.claim_provisioning_clone().unwrap().unwrap();
        assert_eq!(job.operation.id, operation.id);
        assert!(store.claim_provisioning_clone().unwrap().is_none());
        assert_eq!(
            store.link_provisioning_clone(&operation.id, &task(9000)),
            Err(ProvisioningError::InvalidOperation)
        );
        let mut changed = approval.clone();
        changed.storage = "foreign".into();
        assert_eq!(
            store.begin_provisioning_clone(&operation.id, &changed),
            Err(ProvisioningError::TargetChanged)
        );
        store
            .begin_provisioning_clone(&operation.id, &approval)
            .unwrap();
        assert_eq!(
            store.begin_provisioning_clone(&operation.id, &approval),
            Err(ProvisioningError::InvalidOperation)
        );
        assert_eq!(
            store.link_provisioning_clone(&operation.id, &task(202)),
            Err(ProvisioningError::TargetChanged)
        );
        if known {
            store
                .link_provisioning_clone(&operation.id, &task(9000))
                .unwrap();
            store
                .link_provisioning_clone(&operation.id, &task(9000))
                .unwrap();
        }
        drop(store);
        let store = Store::open(&path).unwrap();
        assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.operation.status, Status::Uncertain);
        assert_eq!(job.stages[0].status, Status::Uncertain);
        assert_eq!(job.stages[0].task.is_some(), known);
        assert!(job.stages[0].dispatch_started);
        assert!(!serde_json::to_string(&job.operation)
            .unwrap()
            .contains("fixture@pve"));
        assert!(!format!("{:?}", job.stages[0]).contains("fixture@pve"));
        assert_eq!(
            store
                .enqueue_provisioning("key", &approval, &catalog())
                .unwrap()
                .id,
            operation.id
        );
        assert!(store
            .enqueue_provisioning("new-key", &approval, &catalog())
            .is_err());
        let claim = store.claim_provisioning_clone().unwrap();
        assert_eq!(claim.is_some(), known);
        if known {
            assert_eq!(
                store.begin_provisioning_clone(&operation.id, &approval),
                Err(ProvisioningError::InvalidOperation)
            );
        }
    }
}

#[test]
fn provisioning_expiry_is_not_renewed_and_corrupt_journal_claim_rolls_back() {
    for offset in [-121, 10] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("expiry.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let operation = store
            .enqueue_provisioning("key", &approval, &catalog())
            .unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE operations SET confirmed_at=unixepoch()+?1",
                [offset],
            )
            .unwrap();
        store
            .enqueue_provisioning("key", &approval, &catalog())
            .unwrap();
        store.claim_provisioning_clone().unwrap();
        assert_eq!(
            store.begin_provisioning_clone(&operation.id, &approval),
            Err(ProvisioningError::ApprovalExpired)
        );
        assert!(
            !store
                .provisioning_job(&operation.id)
                .unwrap()
                .unwrap()
                .stages[0]
                .dispatch_started
        );
        assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
        assert_eq!(
            store
                .provisioning_job(&operation.id)
                .unwrap()
                .unwrap()
                .operation
                .status,
            Status::Interrupted
        );
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("corrupt.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = store
        .enqueue_provisioning("key", &approval, &catalog())
        .unwrap();
    rusqlite::Connection::open(path)
        .unwrap()
        .execute("UPDATE provisioning_intents SET plan_json='{}'", [])
        .unwrap();
    assert_eq!(
        store.claim_provisioning_clone().unwrap_err(),
        ProvisioningError::Unavailable
    );
    assert_eq!(
        store.operation(&operation.id).unwrap().unwrap().status,
        Status::Queued
    );
}

#[test]
fn concurrent_provisioning_connections_share_one_intent_and_instance() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let handles: Vec<_> = (0..6)
        .map(|_| {
            let path = path.clone();
            let approval = approval.clone();
            std::thread::spawn(move || {
                Store::open(path)
                    .unwrap()
                    .enqueue_provisioning("same-key", &approval, &catalog())
                    .unwrap()
                    .id
            })
        })
        .collect();
    let ids: Vec<_> = handles.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_eq!(store.recent_instances().unwrap().len(), 1);
    assert_eq!(store.recent_operations().unwrap().len(), 1);
}

#[test]
fn schema_five_upgrade_preserves_dispatched_lifecycle_identity_and_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("upgrade.sqlite");
    let connection = rusqlite::Connection::open(&path).unwrap();
    for sql in [
        include_str!("../../migrations/0001_definitions.sql"),
        include_str!("../../migrations/0002_operations.sql"),
        include_str!("../../migrations/0003_instances.sql"),
        include_str!("../../migrations/0004_instance_observations.sql"),
        include_str!("../../migrations/0005_lifecycle_operations.sql"),
    ] {
        connection.execute_batch(sql).unwrap();
    }
    let definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    let hash = definition.hash().unwrap();
    connection
        .execute(
            "INSERT INTO definitions(name,definition_yaml,definition_hash,revision) VALUES ('kernel-lab',?1,?2,1)",
            params![definition.canonical_yaml().unwrap(), hash],
        )
        .unwrap();
    connection.execute("INSERT INTO instances(id,definition_name,definition_hash,definition_revision,definition_yaml,provider_scope,node,vmid) VALUES (?1,'kernel-lab',?2,1,?3,?4,'pve-test',202)",params!["a".repeat(32),hash,definition.canonical_yaml().unwrap(),"b".repeat(64)]).unwrap();
    let approval = lifecycle::Approval {
        action: lifecycle::Action::Start,
        instance_id: "a".repeat(32),
        provider_scope_id: "b".repeat(64),
        node: "pve-test".into(),
        vmid: 202,
        definition_hash: hash,
        config_digest: "c".repeat(40),
        confirmed: true,
    };
    let upid = "UPID:pve-test:00000001:00000002:00000003:qmstart:202:fixture@pve:";
    connection.execute("INSERT INTO operations(id,request_key,kind,instance_id,status,approval_json,input_hash,confirmed_at,dispatch_started,provider_upid) VALUES (?1,'retained','vm_start',?2,'running',?3,?4,unixepoch(),1,?5)",params!["d".repeat(32),approval.instance_id,serde_json::to_string(&approval).unwrap(),approval.input_hash().unwrap(),upid]).unwrap();
    connection
        .execute_batch("DROP TABLE IF EXISTS provisioning_seed_regeneration;")
        .unwrap();
    connection.pragma_update(None, "user_version", 5).unwrap();
    drop(connection);
    let store = Store::open(&path).unwrap();
    let retained = store.lifecycle_job(&"d".repeat(32)).unwrap().unwrap();
    assert_eq!(retained.operation.status, Status::Running);
    assert!(retained.provider_task.is_some());
    assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
    assert_eq!(
        store
            .lifecycle_job(&"d".repeat(32))
            .unwrap()
            .unwrap()
            .operation
            .status,
        Status::Uncertain
    );
    let connection = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        10
    );
    assert!(connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
}

#[test]
fn clone_completion_requires_bound_task_target_and_keeps_provisioning_incomplete() {
    use vmanyware_server::{
        provisioning_intent::{CloneOutcome, CloneResult},
        proxmox::{CloneObservation, PowerState, TaskStatus, VmObservation},
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("completed-clone.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = store
        .enqueue_provisioning("complete", &approval, &catalog())
        .unwrap();
    store.claim_provisioning_clone().unwrap();
    let result = || CloneResult {
        instance_id: operation.instance_id.clone().unwrap(),
        provider_scope_id: approval.provider_scope_id.clone(),
        plan_hash: approval.plan_hash.clone(),
        task_status: TaskStatus::Succeeded,
        target: CloneObservation {
            config_digest: "d".repeat(40),
            observation: VmObservation {
                node: approval.target_node.clone(),
                vmid: 202,
                status: PowerState::Stopped,
                qmp_status: None,
            },
        },
    };
    assert_eq!(
        store
            .finish_provisioning_clone(&operation.id, CloneOutcome::Completed(result()))
            .unwrap_err(),
        ProvisioningError::InvalidOperation
    );
    store
        .begin_provisioning_clone(&operation.id, &approval)
        .unwrap();
    store
        .link_provisioning_clone(&operation.id, &task(9000))
        .unwrap();
    let mut wrong = result();
    wrong.target.observation.vmid = 203;
    assert!(store
        .finish_provisioning_clone(&operation.id, CloneOutcome::Completed(wrong))
        .is_err());
    let finished = store
        .finish_provisioning_clone(&operation.id, CloneOutcome::Completed(result()))
        .unwrap();
    assert_eq!(finished.status, Status::Running);
    assert!(finished.result.is_none());
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.stages[0].status, Status::Succeeded);
    assert!(job.stages[0].result.is_some());
    assert!(job.stages[1..].iter().all(|s| s.status == Status::Queued));
    assert!(store.claim_provisioning_clone().unwrap().is_none());
    assert!(store
        .finish_provisioning_clone(&operation.id, CloneOutcome::Completed(result()))
        .is_err());
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
    let retained = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(retained.stages[0].status, Status::Succeeded);
    assert!(retained.stages[0].result.is_some());
    assert!(reopened.claim_provisioning_clone().unwrap().is_none());
}

#[test]
fn clone_outcome_never_turns_ambiguous_submission_into_failure_or_releases_target() {
    use vmanyware_server::{operation::Failure, provisioning_intent::CloneOutcome};
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let operation = store
        .enqueue_provisioning("ambiguous", &approval, &catalog())
        .unwrap();
    store.claim_provisioning_clone().unwrap();
    assert!(store
        .finish_provisioning_clone(
            &operation.id,
            CloneOutcome::Uncertain(Failure::ProviderSubmissionUnconfirmed)
        )
        .is_err());
    store
        .begin_provisioning_clone(&operation.id, &approval)
        .unwrap();
    assert!(store
        .finish_provisioning_clone(
            &operation.id,
            CloneOutcome::Failed(Failure::ProviderTimeout)
        )
        .is_err());
    let result = store
        .finish_provisioning_clone(
            &operation.id,
            CloneOutcome::Uncertain(Failure::ProviderSubmissionUnconfirmed),
        )
        .unwrap();
    assert_eq!(result.status, Status::Uncertain);
    assert!(store.claim_provisioning_clone().unwrap().is_none());
    assert!(store
        .enqueue_provisioning("different-key", &approval, &catalog())
        .is_err());
}

fn complete_fixture_clone(
    store: &Store,
    approval: &Approval,
) -> vmanyware_server::operation::Operation {
    use vmanyware_server::{
        provisioning_intent::{CloneOutcome, CloneResult},
        proxmox::{CloneObservation, PowerState, TaskStatus, VmObservation},
    };
    let operation = store
        .enqueue_provisioning("resource-fixture", approval, &catalog())
        .unwrap();
    store.claim_provisioning_clone().unwrap().unwrap();
    store
        .begin_provisioning_clone(&operation.id, approval)
        .unwrap();
    store
        .link_provisioning_clone(&operation.id, &task(9000))
        .unwrap();
    store
        .finish_provisioning_clone(
            &operation.id,
            CloneOutcome::Completed(CloneResult {
                instance_id: operation.instance_id.clone().unwrap(),
                provider_scope_id: approval.provider_scope_id.clone(),
                plan_hash: approval.plan_hash.clone(),
                task_status: TaskStatus::Succeeded,
                target: CloneObservation {
                    config_digest: "d".repeat(40),
                    observation: VmObservation {
                        node: approval.target_node.clone(),
                        vmid: 202,
                        status: PowerState::Stopped,
                        qmp_status: None,
                    },
                },
            }),
        )
        .unwrap()
}
fn fixture_resource_result(
    operation: &vmanyware_server::operation::Operation,
    approval: &Approval,
) -> vmanyware_server::provisioning_intent::ResourceResult {
    use vmanyware_server::{
        provisioning_intent::ResourceResult,
        proxmox::{CloneObservation, PowerState, ResourceObservation, VmObservation},
    };
    ResourceResult {
        instance_id: operation.instance_id.clone().unwrap(),
        provider_scope_id: approval.provider_scope_id.clone(),
        plan_hash: approval.plan_hash.clone(),
        resources: ResourceObservation {
            cpu: 4,
            memory_mib: 8192,
            target: CloneObservation {
                config_digest: "e".repeat(40),
                observation: VmObservation {
                    node: approval.target_node.clone(),
                    vmid: 202,
                    status: PowerState::Stopped,
                    qmp_status: None,
                },
            },
        },
    }
}

#[test]
fn resource_stage_binds_dispatch_and_completion_without_completing_workflow() {
    use vmanyware_server::provisioning_intent::ResourceOutcome;
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    assert!(store.claim_provisioning_resources().unwrap().is_none());
    let operation = complete_fixture_clone(&store, &approval);
    let claimed = store.claim_provisioning_resources().unwrap().unwrap();
    assert_eq!(claimed.operation.id, operation.id);
    assert_eq!(claimed.stages[1].status, Status::Running);
    assert!(store.claim_provisioning_resources().unwrap().is_none());
    assert!(store
        .finish_provisioning_resources(
            &operation.id,
            ResourceOutcome::Completed(fixture_resource_result(&operation, &approval))
        )
        .is_err());
    assert_eq!(
        store
            .begin_provisioning_resources(&operation.id, &approval, &"e".repeat(40))
            .unwrap_err(),
        ProvisioningError::TargetChanged
    );
    let mut wrong_approval = approval.clone();
    wrong_approval.bridge = "other-bridge".into();
    assert_eq!(
        store
            .begin_provisioning_resources(&operation.id, &wrong_approval, &"d".repeat(40))
            .unwrap_err(),
        ProvisioningError::TargetChanged
    );
    store
        .begin_provisioning_resources(&operation.id, &approval, &"d".repeat(40))
        .unwrap();
    assert!(store
        .begin_provisioning_resources(&operation.id, &approval, &"d".repeat(40))
        .is_err());
    for variant in [
        "cpu", "memory", "scope", "plan", "instance", "target", "running", "digest",
    ] {
        let mut result = fixture_resource_result(&operation, &approval);
        match variant {
            "cpu" => result.resources.cpu = 2,
            "memory" => result.resources.memory_mib = 1024,
            "scope" => result.provider_scope_id = "f".repeat(64),
            "plan" => result.plan_hash = "f".repeat(64),
            "instance" => result.instance_id = "f".repeat(32),
            "target" => result.resources.target.observation.vmid = 203,
            "running" => {
                result.resources.target.observation.status =
                    vmanyware_server::proxmox::PowerState::Running
            }
            _ => result.resources.target.config_digest = "bad".into(),
        }
        assert!(
            store
                .finish_provisioning_resources(&operation.id, ResourceOutcome::Completed(result))
                .is_err(),
            "{variant}"
        );
    }
    let finished = store
        .finish_provisioning_resources(
            &operation.id,
            ResourceOutcome::Completed(fixture_resource_result(&operation, &approval)),
        )
        .unwrap();
    assert_eq!(finished.status, Status::Running);
    assert!(finished.result.is_none());
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(job.stages[..2]
        .iter()
        .all(|s| s.status == Status::Succeeded));
    assert_eq!(
        job.stages[1].config_digest.as_deref(),
        Some("d".repeat(40).as_str())
    );
    assert_eq!(
        job.stages[1].result.as_ref().unwrap()["resources"]["target"]["configDigest"],
        "e".repeat(40)
    );
    assert!(job.stages[2..].iter().all(|s| s.status == Status::Queued));
    assert!(store.claim_provisioning_resources().unwrap().is_none());
    assert!(store.claim_provisioning_clone().unwrap().is_none());
}

#[test]
fn resource_restart_reconciles_only_dispatched_stage_and_never_renews_clone_approval() {
    use vmanyware_server::{operation::Failure, provisioning_intent::ResourceOutcome};
    for dispatched in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("resource.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let operation = complete_fixture_clone(&store, &approval);
        let connection = rusqlite::Connection::open(&path).unwrap();
        // A complete plan can outlast clone's initial dispatch window.
        connection
            .execute(
                "UPDATE operations SET confirmed_at=unixepoch()-3600 WHERE id=?1",
                [&operation.id],
            )
            .unwrap();
        store.claim_provisioning_resources().unwrap().unwrap();
        if dispatched {
            store
                .begin_provisioning_resources(&operation.id, &approval, &"d".repeat(40))
                .unwrap();
        }
        drop(store);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
        let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.stages[0].status, Status::Succeeded);
        assert_eq!(
            job.stages[1].status,
            if dispatched {
                Status::Uncertain
            } else {
                Status::Interrupted
            }
        );
        assert!(reopened.claim_provisioning_clone().unwrap().is_none());
        let claimed = reopened.claim_provisioning_resources().unwrap();
        assert!(claimed.is_some());
        if !dispatched {
            reopened
                .begin_provisioning_resources(&operation.id, &approval, &"d".repeat(40))
                .unwrap();
            assert!(
                reopened
                    .provisioning_job(&operation.id)
                    .unwrap()
                    .unwrap()
                    .stages[1]
                    .dispatch_started
            );
        }
        if dispatched {
            assert!(reopened
                .finish_provisioning_resources(
                    &operation.id,
                    ResourceOutcome::Failed(Failure::ProvisioningPreconditionsFailed)
                )
                .is_err());
            reopened
                .finish_provisioning_resources(
                    &operation.id,
                    ResourceOutcome::Uncertain(Failure::ResourceStateUnconfirmed),
                )
                .unwrap();
            assert!(reopened.claim_provisioning_resources().unwrap().is_none());
            connection
                .execute(
                    "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
                    [&operation.id],
                )
                .unwrap();
            reopened.claim_provisioning_resources().unwrap().unwrap();
            reopened
                .finish_provisioning_resources(
                    &operation.id,
                    ResourceOutcome::Completed(fixture_resource_result(&operation, &approval)),
                )
                .unwrap();
            assert_eq!(
                reopened
                    .provisioning_job(&operation.id)
                    .unwrap()
                    .unwrap()
                    .confirmed_at,
                job.confirmed_at
            );
        }
    }
}

#[test]
fn resource_claim_rejects_corrupt_clone_evidence_and_rolls_back() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("corrupt.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = complete_fixture_clone(&store, &approval);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE provisioning_stages SET result_json='{}' WHERE operation_id=?1 AND position=0",
            [&operation.id],
        )
        .unwrap();
    assert_eq!(
        store.claim_provisioning_resources().unwrap_err(),
        ProvisioningError::Unavailable
    );
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.stages[1].status, Status::Queued);
    assert!(!job.stages[1].dispatch_started);
}

#[test]
fn schema_six_upgrade_preserves_completed_clone_and_resource_queue() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("schema-six.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = complete_fixture_clone(&store, &approval);
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE IF EXISTS provisioning_seed_regeneration; DROP TABLE provisioning_cloud_init_dispatch; DROP TABLE provisioning_disk_dispatch; ALTER TABLE provisioning_stages DROP COLUMN config_digest; PRAGMA user_version=6;",
        )
        .unwrap();
    drop(connection);
    let migrated = Store::open(&path).unwrap();
    let job = migrated.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(
        job.cloned_config_digest().as_deref(),
        Some("d".repeat(40).as_str())
    );
    assert_eq!(job.stages[1].status, Status::Queued);
    assert!(job.stages[1].config_digest.is_none());
    migrated.claim_provisioning_resources().unwrap().unwrap();
    migrated
        .begin_provisioning_resources(&operation.id, &approval, &"d".repeat(40))
        .unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        10
    );
    assert!(connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
}

#[test]
fn resource_dispatch_revision_cannot_be_replaced_during_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("revision.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = complete_fixture_clone(&store, &approval);
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert!(connection
        .execute(
            "UPDATE provisioning_stages SET config_digest=?2 WHERE operation_id=?1 AND position=1",
            params![operation.id, "d".repeat(40)]
        )
        .is_err());
    store.claim_provisioning_resources().unwrap().unwrap();
    store
        .begin_provisioning_resources(&operation.id, &approval, &"d".repeat(40))
        .unwrap();
    assert!(connection.execute("UPDATE provisioning_stages SET config_digest='bad' WHERE operation_id=?1 AND position=1",[&operation.id]).is_err());
    connection
        .execute(
            "UPDATE provisioning_stages SET config_digest=?2 WHERE operation_id=?1 AND position=1",
            params![operation.id, "f".repeat(40)],
        )
        .unwrap();
    drop(store);
    let reopened = Store::open(&path).unwrap();
    reopened.recover_interrupted_operations().unwrap();
    assert_eq!(
        reopened.claim_provisioning_resources().unwrap_err(),
        ProvisioningError::Unavailable
    );
    assert_eq!(
        reopened
            .provisioning_job(&operation.id)
            .unwrap()
            .unwrap()
            .stages[1]
            .status,
        Status::Uncertain
    );
}

fn complete_fixture_resources(
    store: &Store,
    approval: &Approval,
) -> vmanyware_server::operation::Operation {
    let operation = complete_fixture_clone(store, approval);
    store.claim_provisioning_resources().unwrap().unwrap();
    store
        .begin_provisioning_resources(&operation.id, approval, &"d".repeat(40))
        .unwrap();
    store
        .finish_provisioning_resources(
            &operation.id,
            vmanyware_server::provisioning_intent::ResourceOutcome::Completed(
                fixture_resource_result(&operation, approval),
            ),
        )
        .unwrap()
}
fn resize_task(vmid: &str) -> TaskReference {
    TaskReference::new(
        "pve-test",
        &format!("UPID:pve-test:00000001:00000002:00000003:resize:{vmid}:fixture@pve:"),
    )
    .unwrap()
}
fn fixture_disk_result(
    operation: &vmanyware_server::operation::Operation,
    approval: &Approval,
) -> vmanyware_server::provisioning_intent::DiskResult {
    let mut resources = fixture_resource_result(operation, approval).resources;
    resources.target.config_digest = "f".repeat(40);
    vmanyware_server::provisioning_intent::DiskResult {
        instance_id: operation.instance_id.clone().unwrap(),
        provider_scope_id: approval.provider_scope_id.clone(),
        plan_hash: approval.plan_hash.clone(),
        task_status: vmanyware_server::proxmox::TaskStatus::Succeeded,
        disk: vmanyware_server::proxmox::DiskObservation {
            resources,
            device: "scsi0".into(),
            volume: "local-lvm:vm-202-disk-0".into(),
            size_bytes: 40 * 1024_u64.pow(3),
        },
    }
}

#[test]
fn disk_journal_binds_revision_volume_task_and_completion_to_the_resource_result() {
    use vmanyware_server::provisioning_intent::DiskOutcome;
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    assert!(store.claim_provisioning_disk().unwrap().is_none());
    let operation = complete_fixture_resources(&store, &approval);
    let job = store.claim_provisioning_disk().unwrap().unwrap();
    assert_eq!(job.stages[2].status, Status::Running);
    assert!(store.claim_provisioning_disk().unwrap().is_none());
    assert!(store
        .finish_provisioning_disk(
            &operation.id,
            DiskOutcome::Completed(fixture_disk_result(&operation, &approval))
        )
        .is_err());
    for (digest, volume) in [
        (&"d".repeat(40), "local-lvm:vm-202-disk-0"),
        (&"e".repeat(40), "other:vm-202-disk-0"),
    ] {
        assert_eq!(
            store
                .begin_provisioning_disk(&operation.id, &approval, digest, volume)
                .unwrap_err(),
            ProvisioningError::TargetChanged
        );
    }
    assert!(store
        .provisioning_job(&operation.id)
        .unwrap()
        .unwrap()
        .disk_dispatch
        .is_none());
    store
        .begin_provisioning_disk(
            &operation.id,
            &approval,
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
        )
        .unwrap();
    let marked = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(!marked.operation.provider_task_known);
    assert!(marked.stages[0].task.is_some());
    assert!(store
        .begin_provisioning_disk(
            &operation.id,
            &approval,
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0"
        )
        .is_err());
    for task in [task(9000), resize_task("9000"), resize_task("0202")] {
        assert_eq!(
            store
                .link_provisioning_disk(&operation.id, &task)
                .unwrap_err(),
            ProvisioningError::TargetChanged
        );
    }
    assert!(store
        .finish_provisioning_disk(
            &operation.id,
            DiskOutcome::Completed(fixture_disk_result(&operation, &approval))
        )
        .is_err());
    store
        .link_provisioning_disk(&operation.id, &resize_task("202"))
        .unwrap();
    store
        .link_provisioning_disk(&operation.id, &resize_task("202"))
        .unwrap();
    let replacement = TaskReference::new(
        "pve-test",
        "UPID:pve-test:00000004:00000002:00000003:resize:202:fixture@pve:",
    )
    .unwrap();
    assert_eq!(
        store
            .link_provisioning_disk(&operation.id, &replacement)
            .unwrap_err(),
        ProvisioningError::Conflict
    );
    for variant in [
        "instance", "scope", "plan", "task", "volume", "device", "size", "cpu", "memory", "target",
        "power", "digest",
    ] {
        let mut result = fixture_disk_result(&operation, &approval);
        match variant {
            "instance" => result.instance_id = "b".repeat(32),
            "scope" => result.provider_scope_id = "b".repeat(64),
            "plan" => result.plan_hash = "b".repeat(64),
            "task" => result.task_status = vmanyware_server::proxmox::TaskStatus::Failed,
            "volume" => result.disk.volume = "local-lvm:vm-203-disk-0".into(),
            "device" => result.disk.device = "sata0".into(),
            "size" => result.disk.size_bytes = 41 * 1024_u64.pow(3),
            "cpu" => result.disk.resources.cpu = 1,
            "memory" => result.disk.resources.memory_mib = 1024,
            "target" => result.disk.resources.target.observation.vmid = 203,
            "power" => {
                result.disk.resources.target.observation.status =
                    vmanyware_server::proxmox::PowerState::Running
            }
            _ => result.disk.resources.target.config_digest = "invalid".into(),
        }
        assert!(
            store
                .finish_provisioning_disk(&operation.id, DiskOutcome::Completed(result))
                .is_err(),
            "{variant}"
        );
    }
    let finished = store
        .finish_provisioning_disk(
            &operation.id,
            DiskOutcome::Completed(fixture_disk_result(&operation, &approval)),
        )
        .unwrap();
    assert_eq!(finished.status, Status::Running);
    assert!(finished.result.is_none());
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(job.stages[..3]
        .iter()
        .all(|s| s.status == Status::Succeeded));
    assert!(job.stages[3..].iter().all(|s| s.status == Status::Queued));
    assert!(job.stages[2].result.is_some());
    assert!(store.claim_provisioning_disk().unwrap().is_none());
    assert!(store.claim_provisioning_clone().unwrap().is_none());
    assert!(store.claim_provisioning_resources().unwrap().is_none());
}

#[test]
fn disk_journal_restart_never_replays_unknown_submission_and_only_reclaims_known_tasks() {
    use vmanyware_server::{operation::Failure, provisioning_intent::DiskOutcome};
    for state in ["before_dispatch", "unknown", "known", "completed"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("disk.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let operation = complete_fixture_resources(&store, &approval);
        store.claim_provisioning_disk().unwrap().unwrap();
        if state != "before_dispatch" {
            store
                .begin_provisioning_disk(
                    &operation.id,
                    &approval,
                    &"e".repeat(40),
                    "local-lvm:vm-202-disk-0",
                )
                .unwrap();
        }
        if ["known", "completed"].contains(&state) {
            store
                .link_provisioning_disk(&operation.id, &resize_task("202"))
                .unwrap();
        }
        if state == "completed" {
            store
                .finish_provisioning_disk(
                    &operation.id,
                    DiskOutcome::Completed(fixture_disk_result(&operation, &approval)),
                )
                .unwrap();
        }
        drop(store);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
        let claimed = reopened.claim_provisioning_disk().unwrap();
        assert_eq!(
            claimed.is_some(),
            ["before_dispatch", "known"].contains(&state),
            "{state}"
        );
        assert!(reopened.claim_provisioning_clone().unwrap().is_none());
        assert!(reopened.claim_provisioning_resources().unwrap().is_none());
        if state == "known" {
            reopened
                .finish_provisioning_disk(
                    &operation.id,
                    DiskOutcome::Uncertain(Failure::DiskStateUnconfirmed),
                )
                .unwrap();
            assert!(reopened.claim_provisioning_disk().unwrap().is_none());
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute(
                    "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
                    [&operation.id],
                )
                .unwrap();
            reopened.claim_provisioning_disk().unwrap().unwrap();
            reopened
                .finish_provisioning_disk(
                    &operation.id,
                    DiskOutcome::Completed(fixture_disk_result(&operation, &approval)),
                )
                .unwrap();
        }
        if state == "unknown" {
            assert_eq!(
                reopened.operation(&operation.id).unwrap().unwrap().failure,
                Some(Failure::ProviderSubmissionUnconfirmed)
            );
            assert_eq!(
                reopened
                    .enqueue_provisioning("another-key", &approval, &catalog())
                    .unwrap_err(),
                ProvisioningError::Conflict
            );
        }
    }
}

#[test]
fn disk_journal_rejects_definite_failure_for_ambiguous_dispatch_and_holds_target() {
    use vmanyware_server::{operation::Failure, provisioning_intent::DiskOutcome};
    let store = Store::memory().unwrap();
    let approval = setup(&store);
    let operation = complete_fixture_resources(&store, &approval);
    store.claim_provisioning_disk().unwrap().unwrap();
    assert!(store
        .finish_provisioning_disk(
            &operation.id,
            DiskOutcome::Uncertain(Failure::DiskStateUnconfirmed)
        )
        .is_err());
    store
        .begin_provisioning_disk(
            &operation.id,
            &approval,
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
        )
        .unwrap();
    for failure in [
        Failure::ProviderTaskFailed,
        Failure::ProviderTimeout,
        Failure::ProvisioningPreconditionsFailed,
    ] {
        assert!(store
            .finish_provisioning_disk(&operation.id, DiskOutcome::Failed(failure))
            .is_err());
    }
    store
        .finish_provisioning_disk(
            &operation.id,
            DiskOutcome::Uncertain(Failure::ProviderSubmissionUnconfirmed),
        )
        .unwrap();
    assert!(store.claim_provisioning_disk().unwrap().is_none());
    assert_eq!(
        store
            .enqueue_provisioning("new-target-intent", &approval, &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
}

#[test]
fn disk_journal_schema_seven_upgrade_preserves_resource_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("schema-seven.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = complete_fixture_resources(&store, &approval);
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TABLE IF EXISTS provisioning_seed_regeneration; DROP TABLE provisioning_cloud_init_dispatch; DROP TABLE provisioning_disk_dispatch; PRAGMA user_version=7;")
        .unwrap();
    drop(connection);
    let migrated = Store::open(&path).unwrap();
    assert_eq!(
        migrated
            .provisioning_job(&operation.id)
            .unwrap()
            .unwrap()
            .resource_config_digest()
            .as_deref(),
        Some("e".repeat(40).as_str())
    );
    migrated.claim_provisioning_disk().unwrap().unwrap();
    migrated
        .begin_provisioning_disk(
            &operation.id,
            &approval,
            &"e".repeat(40),
            "local-lvm:vm-202-disk-0",
        )
        .unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        10
    );
    assert!(connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
}

#[test]
fn disk_journal_corrupt_resource_dispatch_or_task_fails_closed_and_claim_rolls_back() {
    for variant in ["resource", "dispatch", "task"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("corrupt-disk.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup(&store);
        let operation = complete_fixture_resources(&store, &approval);
        let connection = rusqlite::Connection::open(&path).unwrap();
        if variant == "resource" {
            connection.execute("UPDATE provisioning_stages SET result_json='{}' WHERE operation_id=?1 AND position=1",[&operation.id]).unwrap();
        } else {
            store.claim_provisioning_disk().unwrap().unwrap();
            store
                .begin_provisioning_disk(
                    &operation.id,
                    &approval,
                    &"e".repeat(40),
                    "local-lvm:vm-202-disk-0",
                )
                .unwrap();
            if variant == "dispatch" {
                connection.execute("UPDATE provisioning_disk_dispatch SET config_digest=?2 WHERE operation_id=?1",params![operation.id,"b".repeat(40)]).unwrap();
            } else {
                store
                    .link_provisioning_disk(&operation.id, &resize_task("202"))
                    .unwrap();
                connection.execute("UPDATE provisioning_stages SET provider_upid=?2 WHERE operation_id=?1 AND position=2",params![operation.id,"UPID:pve-test:00000001:00000002:00000003:resize:9000:fixture@pve:"]).unwrap();
            }
            store.recover_interrupted_operations().unwrap();
            if variant == "dispatch" {
                // Supply the existing canonical task so this row reaches validation.
                connection.execute("UPDATE provisioning_stages SET provider_upid=?2 WHERE operation_id=?1 AND position=2",params![operation.id,"UPID:pve-test:00000001:00000002:00000003:resize:202:fixture@pve:"]).unwrap();
            }
        }
        assert_eq!(
            store.claim_provisioning_disk().unwrap_err(),
            ProvisioningError::Unavailable,
            "{variant}"
        );
        let status: String = connection
            .query_row(
                "SELECT status FROM provisioning_stages WHERE operation_id=?1 AND position=2",
                [&operation.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            status,
            if variant == "resource" {
                "queued"
            } else {
                "uncertain"
            }
        );
    }
}

#[test]
fn disk_journal_retains_definite_failure_and_never_advances_later_stages() {
    use vmanyware_server::{operation::Failure, provisioning_intent::DiskOutcome};
    for failure in [
        Failure::ProviderUnauthorized,
        Failure::ProviderForbidden,
        Failure::ProviderTaskFailed,
    ] {
        let store = Store::memory().unwrap();
        let approval = setup(&store);
        let operation = complete_fixture_resources(&store, &approval);
        store.claim_provisioning_disk().unwrap().unwrap();
        store
            .begin_provisioning_disk(
                &operation.id,
                &approval,
                &"e".repeat(40),
                "local-lvm:vm-202-disk-0",
            )
            .unwrap();
        if failure == Failure::ProviderTaskFailed {
            store
                .link_provisioning_disk(&operation.id, &resize_task("202"))
                .unwrap();
        }
        let finished = store
            .finish_provisioning_disk(&operation.id, DiskOutcome::Failed(failure))
            .unwrap();
        assert_eq!(finished.status, Status::Failed);
        assert_eq!(finished.failure, Some(failure));
        let retained = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(retained.stages[2].status, Status::Failed);
        assert!(retained.disk_dispatch.is_some());
        assert_eq!(
            retained.stages[2].task.is_some(),
            failure == Failure::ProviderTaskFailed
        );
        assert!(retained.stages[3..]
            .iter()
            .all(|s| s.status == Status::Queued));
        assert!(store.claim_provisioning_disk().unwrap().is_none());
        assert_eq!(
            store
                .enqueue_provisioning("new-intent", &approval, &catalog())
                .unwrap_err(),
            ProvisioningError::Conflict
        );
    }
}

const CI_VOLUME: &str = "local-lvm:vm-202-disk-0";
const CI_NETWORK: &str = "virtio=BC:24:11:12:34:56,bridge=old-br,firewall=1";
fn setup_cloud_init(store: &Store, expanded: bool) -> Approval {
    if expanded {
        setup(store)
    } else {
        setup_definition(
            store,
            Definition::parse(
                &include_str!("../../examples/kernel-lab.yaml")
                    .replace("diskGiB: 40", "diskGiB: 8"),
            )
            .unwrap(),
        )
    }
}
fn complete_before_cloud_init(
    store: &Store,
    approval: &Approval,
    expanded: bool,
) -> vmanyware_server::operation::Operation {
    let operation = complete_fixture_resources(store, approval);
    if expanded {
        store.claim_provisioning_disk().unwrap().unwrap();
        store
            .begin_provisioning_disk(&operation.id, approval, &"e".repeat(40), CI_VOLUME)
            .unwrap();
        store
            .link_provisioning_disk(&operation.id, &resize_task("202"))
            .unwrap();
        store
            .finish_provisioning_disk(
                &operation.id,
                vmanyware_server::provisioning_intent::DiskOutcome::Completed(fixture_disk_result(
                    &operation, approval,
                )),
            )
            .unwrap();
    }
    operation
}
fn cloud_init_result(
    operation: &vmanyware_server::operation::Operation,
    approval: &Approval,
    expanded: bool,
) -> Box<vmanyware_server::provisioning_intent::CloudInitResult> {
    let mut resources = fixture_resource_result(operation, approval).resources;
    resources.target.config_digest = "a".repeat(40);
    Box::new(vmanyware_server::provisioning_intent::CloudInitResult {
        instance_id: operation.instance_id.clone().unwrap(),
        provider_scope_id: approval.provider_scope_id.clone(),
        plan_hash: approval.plan_hash.clone(),
        settings: vmanyware_server::proxmox::CloudInitObservation {
            resources,
            primary_volume: CI_VOLUME.into(),
            disk_size_bytes: if expanded { 40 } else { 8 } * 1024_u64.pow(3),
            network: "virtio=BC:24:11:12:34:56,bridge=vmbr0,firewall=1".into(),
            guest_user: "debian".into(),
            ssh_key_count: 1,
            guest_ready: false,
        },
    })
}
#[test]
fn cloud_init_journal_requires_prior_steps_and_binds_settings_without_readiness_or_workflow_success(
) {
    use vmanyware_server::{operation::Failure, provisioning_intent::CloudInitOutcome};
    for expanded in [false, true] {
        let store = Store::memory().unwrap();
        let approval = setup_cloud_init(&store, expanded);
        store
            .enqueue_provisioning("resource-fixture", &approval, &catalog())
            .unwrap();
        assert!(store.claim_provisioning_cloud_init().unwrap().is_none());
        let operation = complete_before_cloud_init(&store, &approval, expanded);
        let claimed = store.claim_provisioning_cloud_init().unwrap().unwrap();
        let index = if expanded { 3 } else { 2 };
        assert_eq!(claimed.stages[index].kind, "configure_cloud_init");
        assert!(store.claim_provisioning_cloud_init().unwrap().is_none());
        assert!(store
            .finish_provisioning_cloud_init(
                &operation.id,
                CloudInitOutcome::Completed(cloud_init_result(&operation, &approval, expanded))
            )
            .is_err());
        assert!(store
            .finish_provisioning_cloud_init(
                &operation.id,
                CloudInitOutcome::Uncertain(Failure::CloudInitStateUnconfirmed)
            )
            .is_err());
        let digest = if expanded {
            "f".repeat(40)
        } else {
            "e".repeat(40)
        };
        for (revision, volume, network) in [
            (&"d".repeat(40), CI_VOLUME, CI_NETWORK),
            (&digest, "other:vm-202-disk-0", CI_NETWORK),
            (&digest, CI_VOLUME, "virtio=00:00:00:00:00:00,bridge=old-br"),
        ] {
            assert_eq!(
                store
                    .begin_provisioning_cloud_init(
                        &operation.id,
                        &approval,
                        revision,
                        volume,
                        network
                    )
                    .unwrap_err(),
                ProvisioningError::TargetChanged
            );
        }
        store
            .begin_provisioning_cloud_init(&operation.id, &approval, &digest, CI_VOLUME, CI_NETWORK)
            .unwrap();
        assert!(store
            .begin_provisioning_cloud_init(&operation.id, &approval, &digest, CI_VOLUME, CI_NETWORK)
            .is_err());
        assert!(
            !store
                .operation(&operation.id)
                .unwrap()
                .unwrap()
                .provider_task_known
        );
        assert!(store
            .finish_provisioning_cloud_init(
                &operation.id,
                CloudInitOutcome::Failed(Failure::ProvisioningPreconditionsFailed)
            )
            .is_err());
        for variant in [
            "instance", "scope", "plan", "volume", "size", "network", "user", "keys", "ready",
            "cpu", "memory", "target", "power", "digest",
        ] {
            let mut result = cloud_init_result(&operation, &approval, expanded);
            match variant {
                "instance" => result.instance_id = "b".repeat(32),
                "scope" => result.provider_scope_id = "b".repeat(64),
                "plan" => result.plan_hash = "b".repeat(64),
                "volume" => result.settings.primary_volume = "local-lvm:vm-203-disk-0".into(),
                "size" => result.settings.disk_size_bytes = 1,
                "network" => {
                    result.settings.network =
                        "virtio=BC:24:11:12:34:57,bridge=vmbr0,firewall=1".into()
                }
                "user" => result.settings.guest_user = "root".into(),
                "keys" => result.settings.ssh_key_count = 2,
                "ready" => result.settings.guest_ready = true,
                "cpu" => result.settings.resources.cpu = 1,
                "memory" => result.settings.resources.memory_mib = 1024,
                "target" => result.settings.resources.target.observation.vmid = 203,
                "power" => {
                    result.settings.resources.target.observation.status =
                        vmanyware_server::proxmox::PowerState::Running
                }
                _ => result.settings.resources.target.config_digest = "bad".into(),
            }
            assert!(
                store
                    .finish_provisioning_cloud_init(
                        &operation.id,
                        CloudInitOutcome::Completed(result)
                    )
                    .is_err(),
                "{variant}"
            );
        }
        let finished = store
            .finish_provisioning_cloud_init(
                &operation.id,
                CloudInitOutcome::Completed(cloud_init_result(&operation, &approval, expanded)),
            )
            .unwrap();
        assert_eq!(finished.status, Status::Running);
        assert!(finished.result.is_none());
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert!(job.stages[..=index]
            .iter()
            .all(|s| s.status == Status::Succeeded));
        assert!(job.stages[index + 1..]
            .iter()
            .all(|s| s.status == Status::Queued));
        assert!(
            !job.stages[index].result.as_ref().unwrap()["settings"]["guestReady"]
                .as_bool()
                .unwrap()
        );
        assert!(store.claim_provisioning_cloud_init().unwrap().is_none());
        assert!(store.claim_provisioning_clone().unwrap().is_none());
        assert!(store.claim_provisioning_disk().unwrap().is_none());
    }
}
#[test]
fn cloud_init_journal_restart_preserves_inputs_and_only_reconciles_after_dispatch() {
    use vmanyware_server::{operation::Failure, provisioning_intent::CloudInitOutcome};
    for state in ["before", "dispatched", "completed"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ci-restart.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup_cloud_init(&store, true);
        let operation = complete_before_cloud_init(&store, &approval, true);
        store.claim_provisioning_cloud_init().unwrap().unwrap();
        if state != "before" {
            store
                .begin_provisioning_cloud_init(
                    &operation.id,
                    &approval,
                    &"f".repeat(40),
                    CI_VOLUME,
                    CI_NETWORK,
                )
                .unwrap();
        }
        if state == "completed" {
            store
                .finish_provisioning_cloud_init(
                    &operation.id,
                    CloudInitOutcome::Completed(cloud_init_result(&operation, &approval, true)),
                )
                .unwrap();
        }
        drop(store);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.recover_interrupted_operations().unwrap(), 1);
        let claimed = reopened.claim_provisioning_cloud_init().unwrap();
        assert_eq!(claimed.is_some(), state != "completed");
        if state == "before" {
            reopened
                .begin_provisioning_cloud_init(
                    &operation.id,
                    &approval,
                    &"f".repeat(40),
                    CI_VOLUME,
                    CI_NETWORK,
                )
                .unwrap();
        }
        if state != "completed" {
            let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
            let retained = job.cloud_init_dispatch.unwrap();
            assert_eq!(retained.config_digest, "f".repeat(40));
            assert_eq!(retained.volume, CI_VOLUME);
            assert_eq!(retained.network, CI_NETWORK);
            assert!(reopened
                .begin_provisioning_cloud_init(
                    &operation.id,
                    &approval,
                    &"f".repeat(40),
                    CI_VOLUME,
                    CI_NETWORK
                )
                .is_err());
            reopened
                .finish_provisioning_cloud_init(
                    &operation.id,
                    CloudInitOutcome::Uncertain(Failure::CloudInitStateUnconfirmed),
                )
                .unwrap();
            assert!(reopened.claim_provisioning_cloud_init().unwrap().is_none());
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute(
                    "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
                    [&operation.id],
                )
                .unwrap();
            reopened.claim_provisioning_cloud_init().unwrap().unwrap();
            reopened
                .finish_provisioning_cloud_init(
                    &operation.id,
                    CloudInitOutcome::Completed(cloud_init_result(&operation, &approval, true)),
                )
                .unwrap();
        }
        assert!(reopened.claim_provisioning_clone().unwrap().is_none());
        assert!(reopened.claim_provisioning_resources().unwrap().is_none());
        assert!(reopened.claim_provisioning_disk().unwrap().is_none());
    }
}
#[test]
fn cloud_init_journal_schema_eight_upgrade_preserves_completed_disk_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("schema-eight.sqlite");
    let store = Store::open(&path).unwrap();
    let approval = setup(&store);
    let operation = complete_before_cloud_init(&store, &approval, true);
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TABLE IF EXISTS provisioning_seed_regeneration; DROP TABLE provisioning_cloud_init_dispatch; PRAGMA user_version=8;")
        .unwrap();
    drop(connection);
    let migrated = Store::open(&path).unwrap();
    let job = migrated.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(
        job.cloud_init_source().unwrap(),
        ("f".repeat(40), Some(CI_VOLUME.into()))
    );
    assert!(job.cloud_init_dispatch.is_none());
    migrated.claim_provisioning_cloud_init().unwrap().unwrap();
    migrated
        .begin_provisioning_cloud_init(
            &operation.id,
            &approval,
            &"f".repeat(40),
            CI_VOLUME,
            CI_NETWORK,
        )
        .unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        10
    );
    assert!(connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
}

#[test]
fn cloud_init_journal_rejects_corrupt_prior_evidence_or_dispatch_and_rolls_back_claim() {
    for variant in ["resource", "disk", "digest", "network", "volume"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ci-corrupt.sqlite");
        let store = Store::open(&path).unwrap();
        let approval = setup_cloud_init(&store, true);
        let operation = complete_before_cloud_init(&store, &approval, true);
        let connection = rusqlite::Connection::open(&path).unwrap();
        if ["resource", "disk"].contains(&variant) {
            connection.execute("UPDATE provisioning_stages SET result_json='{}' WHERE operation_id=?1 AND position=?2",params![operation.id,if variant=="resource" {1}else{2}]).unwrap();
        } else {
            store.claim_provisioning_cloud_init().unwrap().unwrap();
            store
                .begin_provisioning_cloud_init(
                    &operation.id,
                    &approval,
                    &"f".repeat(40),
                    CI_VOLUME,
                    CI_NETWORK,
                )
                .unwrap();
            match variant {
                "digest"=>connection.execute("UPDATE provisioning_cloud_init_dispatch SET config_digest=?2 WHERE operation_id=?1",params![operation.id,"b".repeat(40)]).unwrap(),
                "network"=>connection.execute("UPDATE provisioning_cloud_init_dispatch SET network='invalid' WHERE operation_id=?1",[&operation.id]).unwrap(),
                _=>connection.execute("UPDATE provisioning_cloud_init_dispatch SET volume='local-lvm:vm-203-disk-0' WHERE operation_id=?1",[&operation.id]).unwrap(),
            };
            store.recover_interrupted_operations().unwrap();
        }
        assert_eq!(
            store.claim_provisioning_cloud_init().unwrap_err(),
            ProvisioningError::Unavailable,
            "{variant}"
        );
        let status:String=connection.query_row("SELECT status FROM provisioning_stages WHERE operation_id=?1 AND kind='configure_cloud_init'",[&operation.id],|r|r.get(0)).unwrap();
        assert_eq!(
            status,
            if ["resource", "disk"].contains(&variant) {
                "queued"
            } else {
                "uncertain"
            }
        );
    }
}
#[test]
fn cloud_init_journal_failed_preconditions_keep_target_and_prevent_later_stages() {
    use vmanyware_server::{operation::Failure, provisioning_intent::CloudInitOutcome};
    let store = Store::memory().unwrap();
    let approval = setup_cloud_init(&store, false);
    let operation = complete_before_cloud_init(&store, &approval, false);
    store.claim_provisioning_cloud_init().unwrap().unwrap();
    let finished = store
        .finish_provisioning_cloud_init(
            &operation.id,
            CloudInitOutcome::Failed(Failure::ProvisioningPreconditionsFailed),
        )
        .unwrap();
    assert_eq!(finished.status, Status::Failed);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.stages[2].status, Status::Failed);
    assert!(job.stages[3..].iter().all(|s| s.status == Status::Queued));
    assert!(store.claim_provisioning_cloud_init().unwrap().is_none());
    assert_eq!(
        store
            .enqueue_provisioning("replacement", &approval, &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
}

fn completed_cloud_init_for_seed(
    store: &Store,
    expanded: bool,
) -> (Approval, vmanyware_server::operation::Operation) {
    let approval = setup_cloud_init(store, expanded);
    let operation = complete_before_cloud_init(store, &approval, expanded);
    assert!(store.claim_provisioning_seed().unwrap().is_none());
    store.claim_provisioning_cloud_init().unwrap().unwrap();
    store
        .begin_provisioning_cloud_init(
            &operation.id,
            &approval,
            &if expanded { "f" } else { "e" }.repeat(40),
            CI_VOLUME,
            CI_NETWORK,
        )
        .unwrap();
    store
        .finish_provisioning_cloud_init(
            &operation.id,
            vmanyware_server::provisioning_intent::CloudInitOutcome::Completed(cloud_init_result(
                &operation, &approval, expanded,
            )),
        )
        .unwrap();
    (approval, operation)
}
fn fixture_seed_proof() -> vmanyware_server::proxmox::SeedPreflight {
    vmanyware_server::proxmox::SeedPreflight {
        config_digest: "a".repeat(40),
        user_data_hash: "b".repeat(64),
        network_data_hash: "c".repeat(64),
        user_format: vmanyware_server::proxmox::SeedUserFormat::Named,
        compatible: true,
        guest_ready: false,
    }
}
#[test]
fn seed_journal_binds_approval_prior_settings_and_proof_without_finishing_start() {
    use vmanyware_server::provisioning_intent::SeedOutcome;
    for expanded in [false, true] {
        let store = Store::memory().unwrap();
        let (approval, operation) = completed_cloud_init_for_seed(&store, expanded);
        let job = store.claim_provisioning_seed().unwrap().unwrap();
        assert!(job.regenerated_seed().is_none());
        assert!(store.claim_provisioning_seed().unwrap().is_none());
        assert!(store
            .finish_provisioning_seed(&operation.id, SeedOutcome::Completed(fixture_seed_proof()))
            .is_err());
        for variant in [
            "revision",
            "hash",
            "ready",
            "scalar",
            "default",
            "incompatible",
            "approval",
        ] {
            let mut proof = fixture_seed_proof();
            let mut wrong = approval.clone();
            match variant {
                "revision" => proof.config_digest = "f".repeat(40),
                "hash" => proof.user_data_hash = "bad".into(),
                "ready" => proof.guest_ready = true,
                "scalar" => {
                    proof.user_format = vmanyware_server::proxmox::SeedUserFormat::ScalarUnsupported
                }
                "default" => {
                    proof.user_format = vmanyware_server::proxmox::SeedUserFormat::ImageDefault
                }
                "incompatible" => proof.compatible = false,
                _ => wrong.target_vmid = 203,
            }
            assert!(
                store
                    .begin_provisioning_seed(&operation.id, &wrong, proof)
                    .is_err(),
                "{variant}"
            );
        }
        store
            .begin_provisioning_seed(&operation.id, &approval, fixture_seed_proof())
            .unwrap();
        assert!(store
            .begin_provisioning_seed(&operation.id, &approval, fixture_seed_proof())
            .is_err());
        let mut wrong = fixture_seed_proof();
        wrong.network_data_hash = "d".repeat(64);
        assert!(store
            .finish_provisioning_seed(&operation.id, SeedOutcome::Completed(wrong))
            .is_err());
        let mut completed = fixture_seed_proof();
        completed.config_digest = "f".repeat(40);
        store
            .finish_provisioning_seed(&operation.id, SeedOutcome::Completed(completed))
            .unwrap();
        assert!(store
            .finish_provisioning_seed(
                &operation.id,
                SeedOutcome::Uncertain(
                    vmanyware_server::operation::Failure::SeedRegenerationUnconfirmed
                )
            )
            .is_err());
        let retained = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(
            retained.regenerated_seed().unwrap().config_digest,
            "f".repeat(40)
        );
        let start = retained
            .stages
            .iter()
            .find(|s| s.kind == "start_vm")
            .unwrap();
        assert_eq!(start.status, Status::Running);
        assert!(start.task.is_none());
        assert_eq!(retained.operation.status, Status::Running);
        assert!(store.claim_provisioning_seed().unwrap().is_none());
    }
}
#[test]
fn seed_journal_restart_retains_success_and_never_replays_unknown_submission() {
    use vmanyware_server::{operation::Failure, provisioning_intent::SeedOutcome};
    for variant in ["before_marker", "after_marker", "uncertain", "success"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("seed.sqlite3");
        let store = Store::open(&path).unwrap();
        let (approval, operation) = completed_cloud_init_for_seed(&store, true);
        store.claim_provisioning_seed().unwrap().unwrap();
        if variant != "before_marker" {
            store
                .begin_provisioning_seed(&operation.id, &approval, fixture_seed_proof())
                .unwrap();
        }
        if variant == "success" {
            store
                .finish_provisioning_seed(
                    &operation.id,
                    SeedOutcome::Completed(fixture_seed_proof()),
                )
                .unwrap();
        }
        if variant == "uncertain" {
            store
                .finish_provisioning_seed(
                    &operation.id,
                    SeedOutcome::Uncertain(Failure::SeedRegenerationUnconfirmed),
                )
                .unwrap();
        }
        drop(store);
        let reopened = Store::open(&path).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(job.regenerated_seed().is_some(), variant == "success");
        assert_eq!(
            reopened.claim_provisioning_seed().unwrap().is_some(),
            variant == "before_marker"
        );
        if variant == "after_marker" {
            assert_eq!(
                job.stages
                    .iter()
                    .find(|s| s.kind == "start_vm")
                    .unwrap()
                    .status,
                Status::Uncertain
            );
        }
    }
}

#[test]
fn seed_journal_schema_nine_upgrade_preserves_settings_without_inventing_regeneration() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v9.sqlite");
    let store = Store::open(&path).unwrap();
    let (_, operation) = completed_cloud_init_for_seed(&store, true);
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TABLE provisioning_seed_regeneration; PRAGMA user_version=9;")
        .unwrap();
    drop(connection);
    let reopened = Store::open(&path).unwrap();
    let job = reopened.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(job.cloud_init_settings().is_some());
    assert!(job.regenerated_seed().is_none());
    assert!(reopened.claim_provisioning_seed().unwrap().is_some());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        10
    );
    assert!(!connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_some());
}

fn completed_seed_for_start(
    store: &Store,
    expanded: bool,
) -> (Approval, vmanyware_server::operation::Operation) {
    let (approval, operation) = completed_cloud_init_for_seed(store, expanded);
    assert!(store.claim_provisioning_start().unwrap().is_none());
    store.claim_provisioning_seed().unwrap().unwrap();
    store
        .begin_provisioning_seed(&operation.id, &approval, fixture_seed_proof())
        .unwrap();
    store
        .finish_provisioning_seed(
            &operation.id,
            vmanyware_server::provisioning_intent::SeedOutcome::Completed(fixture_seed_proof()),
        )
        .unwrap();
    (approval, operation)
}
fn start_task(vmid: u32) -> TaskReference {
    TaskReference::new(
        "pve-test",
        &format!("UPID:pve-test:00000001:00000002:00000003:qmstart:{vmid}:fixture@pve:"),
    )
    .unwrap()
}
fn fixture_start_result(
    operation: &vmanyware_server::operation::Operation,
    approval: &Approval,
) -> vmanyware_server::provisioning_intent::StartResult {
    vmanyware_server::provisioning_intent::StartResult {
        instance_id: operation.instance_id.clone().unwrap(),
        provider_scope_id: approval.provider_scope_id.clone(),
        plan_hash: approval.plan_hash.clone(),
        task_status: vmanyware_server::proxmox::TaskStatus::Succeeded,
        observation: vmanyware_server::proxmox::VmObservation {
            node: approval.target_node.clone(),
            vmid: approval.target_vmid,
            status: vmanyware_server::proxmox::PowerState::Running,
            qmp_status: Some(vmanyware_server::proxmox::QmpState::Running),
        },
        guest_ready: false,
    }
}
#[test]
fn provisioning_start_journal_requires_seed_and_separate_power_marker_and_target_task() {
    use vmanyware_server::{operation::Failure, provisioning_intent::StartOutcome};
    for expanded in [false, true] {
        let store = Store::memory().unwrap();
        let (approval, operation) = completed_seed_for_start(&store, expanded);
        assert!(!store
            .provisioning_job(&operation.id)
            .unwrap()
            .unwrap()
            .power_dispatch_started());
        let job = store.claim_provisioning_start().unwrap().unwrap();
        assert!(job.regenerated_seed().is_some());
        assert!(store.claim_provisioning_start().unwrap().is_none());
        assert!(store
            .link_provisioning_start(&operation.id, &start_task(202))
            .is_err());
        assert!(store
            .finish_provisioning_start(
                &operation.id,
                StartOutcome::Completed(fixture_start_result(&operation, &approval))
            )
            .is_err());
        let mut wrong = approval.clone();
        wrong.target_vmid = 203;
        assert!(store
            .begin_provisioning_start(&operation.id, &wrong, &"a".repeat(40))
            .is_err());
        assert!(store
            .begin_provisioning_start(&operation.id, &approval, &"b".repeat(40))
            .is_err());
        store
            .begin_provisioning_start(&operation.id, &approval, &"a".repeat(40))
            .unwrap();
        assert!(store
            .begin_provisioning_start(&operation.id, &approval, &"a".repeat(40))
            .is_err());
        assert!(store
            .link_provisioning_start(&operation.id, &start_task(9000))
            .is_err());
        assert!(store
            .link_provisioning_start(&operation.id, &task(202))
            .is_err());
        store
            .link_provisioning_start(&operation.id, &start_task(202))
            .unwrap();
        store
            .link_provisioning_start(&operation.id, &start_task(202))
            .unwrap();
        assert!(store
            .finish_provisioning_start(
                &operation.id,
                StartOutcome::Failed(Failure::ProvisioningDisabled)
            )
            .is_err());
        for variant in ["ready", "vmid", "scope", "paused", "task", "plan"] {
            let mut proof = fixture_start_result(&operation, &approval);
            match variant {
                "ready" => proof.guest_ready = true,
                "vmid" => proof.observation.vmid = 203,
                "scope" => proof.provider_scope_id = "f".repeat(64),
                "paused" => {
                    proof.observation.qmp_status = Some(vmanyware_server::proxmox::QmpState::Paused)
                }
                "task" => proof.task_status = vmanyware_server::proxmox::TaskStatus::Failed,
                _ => proof.plan_hash = "e".repeat(64),
            }
            assert!(
                store
                    .finish_provisioning_start(&operation.id, StartOutcome::Completed(proof))
                    .is_err(),
                "{variant}"
            );
        }
        store
            .finish_provisioning_start(
                &operation.id,
                StartOutcome::Completed(fixture_start_result(&operation, &approval)),
            )
            .unwrap();
        let job = store.provisioning_job(&operation.id).unwrap().unwrap();
        assert_eq!(
            job.stages
                .iter()
                .find(|s| s.kind == "start_vm")
                .unwrap()
                .status,
            Status::Succeeded
        );
        assert_eq!(
            job.stages
                .iter()
                .find(|s| s.kind == "wait_for_guest_readiness")
                .unwrap()
                .status,
            Status::Queued
        );
        assert_eq!(job.operation.status, Status::Running);
    }
}
#[test]
fn provisioning_start_restart_resumes_before_marker_and_only_reconciles_known_tasks() {
    for variant in ["seed_only", "claimed", "marked", "known", "complete"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("start.sqlite");
        let store = Store::open(&path).unwrap();
        let (approval, operation) = completed_seed_for_start(&store, true);
        if variant != "seed_only" {
            store.claim_provisioning_start().unwrap().unwrap();
        }
        if ["marked", "known", "complete"].contains(&variant) {
            store
                .begin_provisioning_start(&operation.id, &approval, &"a".repeat(40))
                .unwrap();
        }
        if ["known", "complete"].contains(&variant) {
            store
                .link_provisioning_start(&operation.id, &start_task(202))
                .unwrap();
        }
        if variant == "complete" {
            store
                .finish_provisioning_start(
                    &operation.id,
                    vmanyware_server::provisioning_intent::StartOutcome::Completed(
                        fixture_start_result(&operation, &approval),
                    ),
                )
                .unwrap();
        }
        drop(store);
        let reopened = Store::open(&path).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        let job = reopened.claim_provisioning_start().unwrap();
        assert_eq!(
            job.is_some(),
            ["seed_only", "claimed", "known"].contains(&variant),
            "{variant}"
        );
        if variant == "known" {
            let job = job.unwrap();
            assert!(job.power_dispatch_started());
            assert_eq!(
                job.stages
                    .iter()
                    .find(|s| s.kind == "start_vm")
                    .unwrap()
                    .task,
                Some(start_task(202))
            );
        }
    }
}

#[test]
fn provisioning_start_rejects_corrupt_seed_proof_and_rolls_back_claim() {
    for variant in ["seed_hash", "power_revision", "task"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("corrupt-start.sqlite");
        let store = Store::open(&path).unwrap();
        let (_, operation) = completed_seed_for_start(&store, true);
        let connection = rusqlite::Connection::open(&path).unwrap();
        match variant {
            "seed_hash" => {
                connection.execute("UPDATE provisioning_seed_regeneration SET journal_json=json_set(journal_json,'$.completed.userDataHash',?2) WHERE operation_id=?1",params![operation.id,"e".repeat(64)]).unwrap();
            }
            "power_revision" => {
                connection.execute("UPDATE provisioning_seed_regeneration SET journal_json=json_set(journal_json,'$.power',json_object('configDigest',?2)) WHERE operation_id=?1",params![operation.id,"e".repeat(40)]).unwrap();
            }
            _ => {
                connection.execute("UPDATE provisioning_stages SET provider_upid=?2 WHERE operation_id=?1 AND kind='start_vm'",params![operation.id,"UPID:pve-test:00000001:00000002:00000003:qmstart:9000:fixture@pve:"]).unwrap();
            }
        }
        assert_eq!(
            store.provisioning_job(&operation.id).unwrap_err(),
            ProvisioningError::Unavailable
        );
        if variant == "seed_hash" {
            assert_eq!(
                store.claim_provisioning_start().unwrap_err(),
                ProvisioningError::Unavailable
            );
            let power:Option<String>=connection.query_row("SELECT json_extract(journal_json,'$.power') FROM provisioning_seed_regeneration WHERE operation_id=?1",[&operation.id],|r|r.get(0)).unwrap();
            assert!(power.is_none());
        }
    }
}

fn completed_start_for_readiness(
    store: &Store,
) -> (Approval, vmanyware_server::operation::Operation) {
    let (approval, operation) = completed_seed_for_start(store, true);
    assert!(store.claim_provisioning_readiness().unwrap().is_none());
    store.claim_provisioning_start().unwrap().unwrap();
    store
        .begin_provisioning_start(&operation.id, &approval, &"a".repeat(40))
        .unwrap();
    store
        .link_provisioning_start(&operation.id, &start_task(202))
        .unwrap();
    store
        .finish_provisioning_start(
            &operation.id,
            vmanyware_server::provisioning_intent::StartOutcome::Completed(fixture_start_result(
                &operation, &approval,
            )),
        )
        .unwrap();
    (approval, operation)
}
fn fixture_readiness_result(
    operation: &vmanyware_server::operation::Operation,
    approval: &Approval,
) -> vmanyware_server::provisioning_intent::ReadinessResult {
    vmanyware_server::provisioning_intent::ReadinessResult {
        instance_id: operation.instance_id.clone().unwrap(),
        provider_scope_id: approval.provider_scope_id.clone(),
        plan_hash: approval.plan_hash.clone(),
        guest: vmanyware_server::proxmox::GuestReadiness {
            ipv4: "192.0.2.42".parse().unwrap(),
            interface: "ens18".into(),
            cloud_init_exit_code: 0,
            cloud_init_clean: true,
        },
        guest_ready: true,
    }
}
#[test]
fn durable_readiness_requires_start_pid_and_strict_proof_without_skipping_later_steps() {
    use vmanyware_server::provisioning_intent::ReadinessOutcome;
    let store = Store::memory().unwrap();
    let (approval, operation) = completed_start_for_readiness(&store);
    assert!(store.claim_provisioning_readiness().unwrap().is_some());
    assert!(store.claim_provisioning_readiness().unwrap().is_none());
    assert!(store
        .finish_provisioning_readiness(
            &operation.id,
            ReadinessOutcome::Completed(fixture_readiness_result(&operation, &approval))
        )
        .is_err());
    assert!(store.link_readiness_pid(&operation.id, 42).is_err());
    assert!(store.begin_readiness_probe(&operation.id).unwrap());
    assert!(store.begin_readiness_probe(&operation.id).is_err());
    assert!(store.link_readiness_pid(&operation.id, 0).is_err());
    store.link_readiness_pid(&operation.id, 42).unwrap();
    assert!(store.link_readiness_pid(&operation.id, 43).is_err());
    assert!(store
        .finish_provisioning_readiness(&operation.id, ReadinessOutcome::TimedOut)
        .is_err());
    for variant in [
        "scope",
        "ready",
        "exit2",
        "unclean",
        "loopback",
        "interface",
    ] {
        let mut proof = fixture_readiness_result(&operation, &approval);
        match variant {
            "scope" => proof.provider_scope_id = "e".repeat(64),
            "ready" => proof.guest_ready = false,
            "exit2" => proof.guest.cloud_init_exit_code = 2,
            "unclean" => proof.guest.cloud_init_clean = false,
            "loopback" => proof.guest.ipv4 = "127.0.0.1".parse().unwrap(),
            _ => proof.guest.interface = "bad iface".into(),
        }
        assert!(
            store
                .finish_provisioning_readiness(&operation.id, ReadinessOutcome::Completed(proof))
                .is_err(),
            "{variant}"
        );
    }
    let result = store
        .finish_provisioning_readiness(
            &operation.id,
            ReadinessOutcome::Completed(fixture_readiness_result(&operation, &approval)),
        )
        .unwrap();
    assert_eq!(result.status, Status::Running);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(
        job.stages
            .iter()
            .find(|s| s.kind == "wait_for_guest_readiness")
            .unwrap()
            .status,
        Status::Succeeded
    );
    assert_eq!(
        job.stages
            .iter()
            .find(|s| s.kind == "install_declared_packages")
            .unwrap()
            .status,
        Status::Queued
    );
    assert!(store.claim_provisioning_readiness().unwrap().is_none());
}
#[test]
fn durable_readiness_restart_preserves_pid_deadline_and_completed_pending_allows_next_probe() {
    use vmanyware_server::provisioning_intent::ReadinessOutcome;
    for variant in [
        "before_marker",
        "unknown_pid",
        "known_pid",
        "completed_pending",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ready.sqlite");
        let store = Store::open(&path).unwrap();
        let (_, operation) = completed_start_for_readiness(&store);
        store.claim_provisioning_readiness().unwrap().unwrap();
        if variant != "before_marker" {
            assert!(store.begin_readiness_probe(&operation.id).unwrap());
        }
        if ["known_pid", "completed_pending"].contains(&variant) {
            store.link_readiness_pid(&operation.id, 42).unwrap();
        }
        if variant == "completed_pending" {
            store
                .finish_provisioning_readiness(
                    &operation.id,
                    ReadinessOutcome::Pending {
                        process_exited: true,
                    },
                )
                .unwrap();
        }
        let connection = rusqlite::Connection::open(&path).unwrap();
        let original: String = connection
            .query_row(
                "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
                [&operation.id],
                |r| r.get(0),
            )
            .unwrap();
        let original: serde_json::Value = serde_json::from_str(&original).unwrap();
        drop(store);
        let reopened = Store::open(&path).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        connection
            .execute(
                "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
                [&operation.id],
            )
            .unwrap();
        assert!(reopened.claim_provisioning_readiness().unwrap().is_some());
        let retained: String = connection
            .query_row(
                "SELECT journal_json FROM provisioning_seed_regeneration WHERE operation_id=?1",
                [&operation.id],
                |r| r.get(0),
            )
            .unwrap();
        let retained: serde_json::Value = serde_json::from_str(&retained).unwrap();
        assert_eq!(
            retained["readiness"]["deadline"],
            original["readiness"]["deadline"]
        );
        assert_eq!(
            retained["readiness"]["pid"],
            if variant == "known_pid" {
                serde_json::json!(42)
            } else {
                serde_json::Value::Null
            }
        );
        assert_eq!(
            reopened.begin_readiness_probe(&operation.id).is_ok(),
            ["before_marker", "completed_pending"].contains(&variant)
        );
    }
}
#[test]
fn durable_readiness_expired_deadline_cannot_be_extended_or_completed() {
    use vmanyware_server::{operation::Failure, provisioning_intent::ReadinessOutcome};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("expired.sqlite");
    let store = Store::open(&path).unwrap();
    let (approval, operation) = completed_start_for_readiness(&store);
    store.claim_provisioning_readiness().unwrap().unwrap();
    store.begin_readiness_probe(&operation.id).unwrap();
    store.link_readiness_pid(&operation.id, 42).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute("UPDATE provisioning_seed_regeneration SET journal_json=json_set(journal_json,'$.readiness.startedAt',unixepoch()-301,'$.readiness.deadline',unixepoch()-1) WHERE operation_id=?1",[&operation.id]).unwrap();
    let result = store
        .finish_provisioning_readiness(
            &operation.id,
            ReadinessOutcome::Completed(fixture_readiness_result(&operation, &approval)),
        )
        .unwrap();
    assert_eq!(result.status, Status::Failed);
    assert_eq!(result.failure, Some(Failure::GuestReadinessTimedOut));
    assert!(result.result.is_none());
    assert!(store.claim_provisioning_readiness().unwrap().is_none());
}

#[test]
fn durable_readiness_finishes_minimal_workflow_only_after_clean_guest_proof() {
    use vmanyware_server::provisioning_intent::{
        CloudInitOutcome, ReadinessOutcome, SeedOutcome, StartOutcome,
    };
    let store = Store::memory().unwrap();
    let mut definition = Definition::parse(include_str!("../../examples/kernel-lab.yaml")).unwrap();
    definition.spec.provisioning.packages.clear();
    let approval = setup_definition(&store, definition);
    let operation = complete_before_cloud_init(&store, &approval, true);
    store.claim_provisioning_cloud_init().unwrap().unwrap();
    store
        .begin_provisioning_cloud_init(
            &operation.id,
            &approval,
            &"f".repeat(40),
            CI_VOLUME,
            CI_NETWORK,
        )
        .unwrap();
    store
        .finish_provisioning_cloud_init(
            &operation.id,
            CloudInitOutcome::Completed(cloud_init_result(&operation, &approval, true)),
        )
        .unwrap();
    store.claim_provisioning_seed().unwrap().unwrap();
    store
        .begin_provisioning_seed(&operation.id, &approval, fixture_seed_proof())
        .unwrap();
    store
        .finish_provisioning_seed(&operation.id, SeedOutcome::Completed(fixture_seed_proof()))
        .unwrap();
    store.claim_provisioning_start().unwrap().unwrap();
    store
        .begin_provisioning_start(&operation.id, &approval, &"a".repeat(40))
        .unwrap();
    store
        .link_provisioning_start(&operation.id, &start_task(202))
        .unwrap();
    store
        .finish_provisioning_start(
            &operation.id,
            StartOutcome::Completed(fixture_start_result(&operation, &approval)),
        )
        .unwrap();
    assert_ne!(
        store
            .provisioning_job(&operation.id)
            .unwrap()
            .unwrap()
            .operation
            .status,
        Status::Succeeded
    );
    store.claim_provisioning_readiness().unwrap().unwrap();
    store.begin_readiness_probe(&operation.id).unwrap();
    store.link_readiness_pid(&operation.id, 42).unwrap();
    let result = store
        .finish_provisioning_readiness(
            &operation.id,
            ReadinessOutcome::Completed(fixture_readiness_result(&operation, &approval)),
        )
        .unwrap();
    assert_eq!(result.status, Status::Succeeded);
    assert_eq!(result.result.unwrap()["guestReady"], true);
    assert!(store
        .provisioning_job(&operation.id)
        .unwrap()
        .unwrap()
        .stages
        .iter()
        .all(|s| s.status == Status::Succeeded));
}
