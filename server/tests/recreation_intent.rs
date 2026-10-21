use vmanyware_server::{
    definition::Definition,
    instance::Registration,
    provisioning::Catalog,
    provisioning_intent::Approval,
    store::{ProvisioningError, Store},
};
const YAML: &str = include_str!("../../examples/kernel-lab.yaml");
fn catalog() -> Catalog {
    Catalog::parse(include_str!("../../examples/templates.yaml")).unwrap()
}
fn source(store: &Store, path: &std::path::Path, vmid: u32) -> String {
    let definition = Definition::parse(YAML).unwrap();
    if store.get("kernel-lab").unwrap().is_none() {
        store.create(&definition).unwrap();
    }
    let instance = store
        .register_instance(
            &Registration {
                definition_name: "kernel-lab".into(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-test".into(),
                vmid,
            },
            &"a".repeat(64),
        )
        .unwrap();
    // Owned fixture only: production retirement requires task and absence proof.
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "UPDATE instances SET status='deleted' WHERE id=?1",
            [&instance.summary.id],
        )
        .unwrap();
    instance.summary.id
}
fn approval(vmid: u32) -> Approval {
    let definition = Definition::parse(YAML).unwrap();
    let plan = catalog().plan(&definition, vmid).unwrap();
    Approval {
        definition_name: "kernel-lab".into(),
        definition_hash: plan.definition_hash,
        provider_scope_id: "a".repeat(64),
        plan_hash: plan.plan_hash,
        target_node: "pve-test".into(),
        target_vmid: vmid,
        source_vmid: 9000,
        storage: "local-lvm".into(),
        bridge: "vmbr0".into(),
        confirmed: true,
    }
}
#[test]
fn recreation_intent_is_atomic_distinct_retained_and_idempotent_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite");
    let store = Store::open(&path).unwrap();
    let source_id = source(&store, &path, 202);
    let before = serde_json::to_value(store.instance(&source_id).unwrap()).unwrap();
    let operation = store
        .enqueue_recreation("recreate", &source_id, &approval(202), &catalog())
        .unwrap();
    assert_ne!(operation.instance_id.as_deref(), Some(source_id.as_str()));
    let target = store
        .instance(operation.instance_id.as_ref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(target.definition_yaml, before["definitionYaml"]);
    assert_eq!(target.summary.definition_hash, before["definitionHash"]);
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert_eq!(job.recreated_from.as_deref(), Some(source_id.as_str()));
    assert!(job.stages.iter().all(|s| !s.dispatch_started));
    drop(store);
    let store = Store::open(&path).unwrap();
    let retry = store
        .enqueue_recreation("recreate", &source_id, &approval(202), &catalog())
        .unwrap();
    assert_eq!(retry.id, operation.id);
    assert_eq!(
        serde_json::to_value(store.instance(&source_id).unwrap()).unwrap(),
        before
    );
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    assert_eq!(store.recent_instances().unwrap().len(), 2);
    assert_eq!(
        store
            .enqueue_provisioning("recreate", &approval(202), &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
    let other = source(&store, &path, 203);
    assert_eq!(
        store
            .enqueue_recreation("recreate", &other, &approval(202), &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
    assert_eq!(
        store
            .enqueue_recreation("another-key", &source_id, &approval(202), &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
    assert_eq!(store.recent_operations().unwrap().len(), 1);
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert!(connection
        .execute(
            "UPDATE recreation_sources SET source_instance_id=?1",
            [other]
        )
        .is_err());
    assert!(connection
        .execute("DELETE FROM recreation_sources", [])
        .is_err());
    let claimed = store.claim_provisioning_clone().unwrap().unwrap();
    assert_eq!(claimed.operation.id, operation.id);
    assert_eq!(claimed.recreated_from.as_deref(), Some(source_id.as_str()));
}
#[test]
fn recreation_intent_rejects_unconfirmed_mismatch_and_active_source_without_partial_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite");
    let store = Store::open(&path).unwrap();
    let id = source(&store, &path, 202);
    let mut invalid = approval(203);
    invalid.confirmed = false;
    assert_eq!(
        store
            .enqueue_recreation("no-consent", &id, &invalid, &catalog())
            .unwrap_err(),
        ProvisioningError::InvalidApproval
    );
    invalid = approval(203);
    invalid.definition_hash = "b".repeat(64);
    assert_eq!(
        store
            .enqueue_recreation("mismatch", &id, &invalid, &catalog())
            .unwrap_err(),
        ProvisioningError::TargetChanged
    );
    assert!(store
        .enqueue_recreation("missing", &"c".repeat(32), &approval(203), &catalog())
        .is_err());
    let active = store
        .register_instance(
            &Registration {
                definition_name: "kernel-lab".into(),
                definition_hash: approval(203).definition_hash,
                node: "pve-test".into(),
                vmid: 204,
            },
            &"a".repeat(64),
        )
        .unwrap();
    assert_eq!(
        store
            .enqueue_recreation("active", &active.summary.id, &approval(203), &catalog())
            .unwrap_err(),
        ProvisioningError::Conflict
    );
    assert!(store.recent_operations().unwrap().is_empty());
    assert_eq!(store.recent_instances().unwrap().len(), 2);
}

#[test]
fn recreation_schema_fourteen_upgrade_preserves_ordinary_provisioning() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("upgrade.sqlite");
    let store = Store::open(&path).unwrap();
    store.create(&Definition::parse(YAML).unwrap()).unwrap();
    let operation = store
        .enqueue_provisioning("ordinary", &approval(202), &catalog())
        .unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TABLE recreation_sources; PRAGMA user_version=14;")
        .unwrap();
    drop(connection);
    let store = Store::open(&path).unwrap();
    let job = store.provisioning_job(&operation.id).unwrap().unwrap();
    assert!(job.recreated_from.is_none());
    assert_eq!(job.approval.plan_hash, approval(202).plan_hash);
    assert_eq!(
        store
            .enqueue_provisioning("ordinary", &approval(202), &catalog())
            .unwrap()
            .id,
        operation.id
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        15
    );
    assert!(!connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
}
