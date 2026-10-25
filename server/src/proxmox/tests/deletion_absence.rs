use super::Fixture;
use crate::{deletion::Approval, proxmox::Error};
use serde_json::{json, Value};
use std::time::Duration;
fn approval(scope: String) -> Approval {
    serde_json::from_value(json!({"instanceId":"a".repeat(32),"providerScopeId":scope,"definitionHash":"b".repeat(64),"node":"pve-lab","vmid":202,"configDigest":"c".repeat(40),"confirmed":true,"acknowledgeDataLoss":true,"plan":{"scopeHash":"d".repeat(64),"volumes":[{"volume":"storage:vm-202-disk-0","references":["live:scsi0"],"ownerVerified":true,"effect":"delete_owned_volume"}],"snapshots":[],"preservedMedia":[],"preservedOwnedVolumes":[],"effects":{"version":1,"removeVmConfiguration":true,"removeSnapshotMetadata":true,"removeVmPermissions":true,"removeVmFirewall":true,"cleanupIpam":true,"cleanupRecordedFleecingImages":true,"purgeJobs":false,"destroyUnreferencedDisks":false,"retainDefinitionAndHistory":true}}})).unwrap()
}
fn grants() -> Value {
    json!({"data":{"/storage/storage":{"Datastore.Allocate":false}}})
}
async fn fixture(values: Vec<Value>) -> Fixture {
    let bodies = values.iter().map(Value::to_string).collect::<Vec<_>>();
    let responses = bodies
        .iter()
        .map(|body| ("*", body.as_str()))
        .collect::<Vec<_>>();
    Fixture::responses("200 OK", &responses, "", Duration::ZERO, true).await
}
#[tokio::test]
async fn deletion_absence_covers_exact_plan_with_gets_and_retains_partial_cleanup() {
    for remaining in [false, true] {
        let mut fixture = fixture(vec![json!({"data":202}),grants(),json!({"data":if remaining {vec![json!({"volid":"storage:vm-202-disk-0","vmid":203,"private":"hidden"})]}else{vec![]}}),grants(),json!({"data":"202"})]).await;
        let client = fixture.client();
        let approval = approval(client.scope_id());
        assert!(approval.is_valid());
        let result = client.deletion_absence(&approval).await.unwrap();
        assert_eq!(result.approval_hash, approval.input_hash().unwrap());
        assert_eq!(result.remaining_volumes.len(), usize::from(remaining));
        for path in [
            "/api2/json/cluster/nextid?vmid=202",
            "/api2/json/access/permissions?path=%2Fstorage%2Fstorage",
            "/api2/json/nodes/pve-lab/storage/storage/content?content=images",
            "/api2/json/access/permissions?path=%2Fstorage%2Fstorage",
            "/api2/json/cluster/nextid?vmid=202",
        ] {
            assert!(fixture
                .requests
                .recv()
                .await
                .unwrap()
                .starts_with(&format!("GET {path} HTTP/1.1")));
        }
        assert!(fixture.requests.try_recv().is_err());
    }
}
#[tokio::test]
async fn deletion_absence_refuses_filtered_malformed_or_changed_observations() {
    for variant in [
        "missing",
        "audit-only",
        "revoked",
        "malformed",
        "duplicate",
        "wrong-storage",
        "unsafe",
        "reused",
        "wrong-first",
        "malformed-vmid",
        "oversize",
    ] {
        let mut values = vec![
            json!({"data":202}),
            grants(),
            json!({"data":[]}),
            grants(),
            json!({"data":202}),
        ];
        match variant {
            "missing" => values[1] = json!({"data":{}}),
            "audit-only" => values[1] = json!({"data":{"/storage/storage":{"Datastore.Audit":1}}}),
            "revoked" => values[3] = json!({"data":{}}),
            "malformed" => values[1]["data"]["/storage/storage"]["Datastore.Allocate"] = json!(2),
            "duplicate" => {
                values[2] = json!({"data":[{"volid":"storage:x"},{"volid":"storage:x"}]})
            }
            "wrong-storage" => values[2] = json!({"data":[{"volid":"other:x"}]}),
            "unsafe" => values[2] = json!({"data":[{"volid":"storage:../x"}]}),
            "reused" => values[4] = json!({"data":203}),
            "wrong-first" => values[0] = json!({"data":"203"}),
            "malformed-vmid" => values[0] = json!({"data":"0202"}),
            "oversize" => values[2] = json!({"data":vec![json!({"volid":"storage:x"});4097]}),
            _ => unreachable!(),
        }
        let fixture = fixture(values).await;
        let client = fixture.client();
        assert!(
            client
                .deletion_absence(&approval(client.scope_id()))
                .await
                .is_err(),
            "{variant}"
        );
    }
}
#[tokio::test]
async fn deletion_absence_rejects_provider_errors_and_invalid_scope() {
    for status in [
        "401 Unauthorized",
        "403 Forbidden",
        "404 Not Found",
        "500 Internal Server Error",
    ] {
        let fixture = Fixture::responses(
            status,
            &[("*", r#"{"data":null,"message":"private"}"#)],
            "",
            Duration::ZERO,
            true,
        )
        .await;
        let client = fixture.client();
        assert!(client
            .deletion_absence(&approval(client.scope_id()))
            .await
            .is_err());
    }
    let mut fixture = fixture(vec![]).await;
    let client = fixture.client();
    assert!(matches!(
        client.deletion_absence(&approval("a".repeat(64))).await,
        Err(Error::Configuration)
    ));
    let mut invalid = approval(client.scope_id());
    invalid.confirmed = false;
    assert!(matches!(
        client.deletion_absence(&invalid).await,
        Err(Error::Configuration)
    ));
    assert!(fixture.requests.try_recv().is_err());
}

fn destroy_task(index: u32) -> crate::proxmox::TaskReference {
    crate::proxmox::TaskReference::new(
        "pve-lab",
        &format!("UPID:pve-lab:{index:08X}:00000002:00000003:qmdestroy:202:fixture@pve:"),
    )
    .unwrap()
}
fn completion_responses(
    task: &crate::proxmox::TaskReference,
    status: &str,
    remaining: bool,
) -> Vec<Value> {
    let mut values = vec![
        json!({"data":{"node":"pve-lab","upid":task.upid(),"status":if status=="running"{"running"}else{"stopped"},"exitstatus":if status=="running"{None}else{Some(status)}}}),
    ];
    values.extend([
        json!({"data":202}),
        grants(),
        json!({"data":if remaining {vec![json!({"volid":"storage:vm-202-disk-0"})]}else{vec![]}}),
        grants(),
        json!({"data":202}),
    ]);
    values
}
fn registered_store(scope: String) -> (crate::store::Store, Approval) {
    use crate::store::Store;
    register_in(Store::memory().unwrap(), scope)
}
fn register_in(store: crate::store::Store, scope: String) -> (crate::store::Store, Approval) {
    use crate::{definition::Definition, instance::Registration};
    let definition =
        Definition::parse(include_str!("../../../../examples/kernel-lab.yaml")).unwrap();
    store.create(&definition).unwrap();
    let instance = store
        .register_instance(
            &Registration {
                definition_name: definition.metadata.name.clone(),
                definition_hash: definition.hash().unwrap(),
                node: "pve-lab".into(),
                vmid: 202,
            },
            &scope,
        )
        .unwrap();
    let mut approval = approval(scope);
    approval.instance_id = instance.summary.id;
    approval.definition_hash = instance.summary.definition_hash;
    (store, approval)
}
#[tokio::test]
async fn deletion_completion_atomically_retires_identity_and_retains_definition_history() {
    use crate::{
        deletion::Outcome,
        instance::{Registration, Status as InstanceStatus},
        operation::Status,
    };
    let task = destroy_task(1);
    let mut fixture = fixture(completion_responses(&task, "OK", false)).await;
    let client = fixture.client();
    let (store, approval) = registered_store(client.scope_id());
    let before = store.instance(&approval.instance_id).unwrap().unwrap();
    let operation = store.enqueue_deletion("delete-retire", &approval).unwrap();
    store.claim_deletion().unwrap().unwrap();
    store
        .begin_deletion_dispatch(&operation.id, &approval)
        .unwrap();
    store.link_deletion_task(&operation.id, &task).unwrap();
    let proof = client
        .verify_deletion_completion(&approval, &task)
        .await
        .unwrap();
    let completed = store
        .finish_deletion(&operation.id, Outcome::Succeeded(proof))
        .unwrap();
    assert_eq!(completed.status, Status::Succeeded);
    let result: crate::deletion::ResultEvidence =
        serde_json::from_value(completed.result.clone().unwrap()).unwrap();
    assert!(
        result.vm_absence_verified
            && result.volumes_absence_verified
            && result.definition_retained
            && result.association_retired
    );
    assert_eq!(result.approval_hash, approval.input_hash().unwrap());
    let retired = store.instance(&approval.instance_id).unwrap().unwrap();
    assert_eq!(retired.summary.status, InstanceStatus::Deleted);
    assert_eq!(retired.definition_yaml, before.definition_yaml);
    assert_eq!(retired.summary.created_at, before.summary.created_at);
    assert!(store
        .get(&retired.summary.definition_name)
        .unwrap()
        .is_some());
    assert_eq!(
        store
            .deletion_job(&operation.id)
            .unwrap()
            .unwrap()
            .operation
            .result,
        completed.result
    );
    assert_eq!(
        store
            .enqueue_deletion("delete-retire", &approval)
            .unwrap()
            .id,
        operation.id
    );
    assert!(store
        .enqueue_deletion("new-delete-old-identity", &approval)
        .is_err());
    assert!(store
        .enqueue_instance_observation("observe-retired", &approval.instance_id)
        .is_err());
    let replacement = store
        .register_instance(
            &Registration {
                definition_name: retired.summary.definition_name.clone(),
                definition_hash: approval.definition_hash.clone(),
                node: approval.node.clone(),
                vmid: 202,
            },
            &approval.provider_scope_id,
        )
        .unwrap();
    assert_ne!(replacement.summary.id, approval.instance_id);
    assert_eq!(replacement.summary.status, InstanceStatus::Registered);
    assert_eq!(store.recent_instances().unwrap().len(), 2);
    // A stale identity must never read or mutate a VM that has reused its VMID.
    assert!(retired.summary.observe(&client).await.is_err());
    let lifecycle = crate::lifecycle::Approval {
        action: crate::lifecycle::Action::Start,
        instance_id: approval.instance_id.clone(),
        provider_scope_id: approval.provider_scope_id.clone(),
        definition_hash: approval.definition_hash.clone(),
        node: approval.node.clone(),
        vmid: 202,
        config_digest: approval.config_digest.clone(),
        confirmed: true,
    };
    assert!(store
        .enqueue_lifecycle("start-retired", &lifecycle)
        .is_err());
    for _ in 0..6 {
        assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
    }
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn deletion_completion_requires_successful_exact_task_and_full_absence() {
    for (status, remaining) in [
        ("running", false),
        ("disk cleanup failed", false),
        ("OK", true),
    ] {
        let task = destroy_task(1);
        let mut fixture = fixture(completion_responses(&task, status, remaining)).await;
        let client = fixture.client();
        let approval = approval(client.scope_id());
        assert!(client
            .verify_deletion_completion(&approval, &task)
            .await
            .is_err());
        let expected_reads = if status == "OK" { 6 } else { 1 };
        for _ in 0..expected_reads {
            assert!(fixture.requests.recv().await.unwrap().starts_with("GET "));
        }
        assert!(fixture.requests.try_recv().is_err());
    }
    let mut fixture = fixture(vec![]).await;
    let client = fixture.client();
    let wrong = crate::proxmox::TaskReference::new(
        "pve-lab",
        "UPID:pve-lab:00000001:00000002:00000003:qmstart:202:fixture@pve:",
    )
    .unwrap();
    assert!(client
        .verify_deletion_completion(&approval(client.scope_id()), &wrong)
        .await
        .is_err());
    assert!(fixture.requests.try_recv().is_err());
}
#[tokio::test]
async fn deletion_completion_cannot_cross_approval_or_linked_task() {
    use crate::deletion::Outcome;
    for mismatch in ["approval", "task", "unmarked"] {
        let task = destroy_task(1);
        let fixture = fixture(completion_responses(&task, "OK", false)).await;
        let client = fixture.client();
        let (store, approval) = registered_store(client.scope_id());
        let operation = store
            .enqueue_deletion("guard-completion", &approval)
            .unwrap();
        store.claim_deletion().unwrap().unwrap();
        if mismatch != "unmarked" {
            store
                .begin_deletion_dispatch(&operation.id, &approval)
                .unwrap();
            store
                .link_deletion_task(
                    &operation.id,
                    &if mismatch == "task" {
                        destroy_task(2)
                    } else {
                        task.clone()
                    },
                )
                .unwrap();
        }
        let mut observed = approval.clone();
        if mismatch == "approval" {
            observed.plan.scope_hash = "e".repeat(64);
        }
        let proof = client
            .verify_deletion_completion(&observed, &task)
            .await
            .unwrap();
        assert!(store
            .finish_deletion(&operation.id, Outcome::Succeeded(proof))
            .is_err());
        assert_eq!(
            store
                .instance(&approval.instance_id)
                .unwrap()
                .unwrap()
                .summary
                .status,
            crate::instance::Status::Registered
        );
        assert_eq!(
            store
                .deletion_job(&operation.id)
                .unwrap()
                .unwrap()
                .operation
                .status,
            crate::operation::Status::Running
        );
    }
}

#[tokio::test]
async fn deletion_completion_rolls_back_retirement_on_failure_and_survives_reopen() {
    use crate::{deletion::Outcome, instance::Status, store::Store};
    let task = destroy_task(1);
    let mut values = completion_responses(&task, "OK", false);
    values.extend(completion_responses(&task, "OK", false));
    let fixture = fixture(values).await;
    let client = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("retirement.sqlite");
    let (store, approval) = register_in(Store::open(&path).unwrap(), client.scope_id());
    let operation = store
        .enqueue_deletion("atomic-retirement", &approval)
        .unwrap();
    store.claim_deletion().unwrap().unwrap();
    store
        .begin_deletion_dispatch(&operation.id, &approval)
        .unwrap();
    store.link_deletion_task(&operation.id, &task).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_completion BEFORE UPDATE ON operations WHEN NEW.status='succeeded' BEGIN SELECT RAISE(ABORT, 'injected_failure'); END;").unwrap();
    let proof = client
        .verify_deletion_completion(&approval, &task)
        .await
        .unwrap();
    assert!(store
        .finish_deletion(&operation.id, Outcome::Succeeded(proof))
        .is_err());
    assert_eq!(
        store
            .instance(&approval.instance_id)
            .unwrap()
            .unwrap()
            .summary
            .status,
        Status::Registered
    );
    assert_eq!(
        store
            .deletion_job(&operation.id)
            .unwrap()
            .unwrap()
            .operation
            .status,
        crate::operation::Status::Running
    );
    connection
        .execute_batch("DROP TRIGGER fail_completion")
        .unwrap();
    let proof = client
        .verify_deletion_completion(&approval, &task)
        .await
        .unwrap();
    let completed = store
        .finish_deletion(&operation.id, Outcome::Succeeded(proof))
        .unwrap();
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        reopened
            .instance(&approval.instance_id)
            .unwrap()
            .unwrap()
            .summary
            .status,
        Status::Deleted
    );
    assert_eq!(
        reopened
            .deletion_job(&operation.id)
            .unwrap()
            .unwrap()
            .operation
            .result,
        completed.result
    );
    assert_eq!(
        reopened
            .enqueue_deletion("atomic-retirement", &approval)
            .unwrap()
            .id,
        operation.id
    );
    assert!(!connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    assert!(connection
        .execute(
            "UPDATE instances SET status='registered' WHERE id=?1",
            [approval.instance_id]
        )
        .is_err());
}
