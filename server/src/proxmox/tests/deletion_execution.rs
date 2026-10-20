use super::{deletion::routes, registered_fixture_instance, Fixture, DROP_REPLY};
use crate::{
    deletion::{Approval, Plan},
    operation::{Failure, Status},
    proxmox::{Error, Proxmox, TaskReference, VmReference},
    store::Store,
};
use serde_json::{json, Value};
use std::time::Duration;
const UPID: &str = "UPID:pve-lab:00000001:00000002:00000003:qmdestroy:202:fixture@pve:";
const DELETE_PATH: &str = "/api2/json/nodes/pve-lab/qemu/202?purge=0&destroy-unreferenced-disks=0";
fn permissions() -> Value {
    json!({"data":{"/storage/storage":{"Datastore.Allocate":0}}})
}
fn task_status(status: &str) -> Value {
    json!({"data":{"node":"pve-lab","upid":UPID,"status":"stopped","exitstatus":status}})
}
async fn fixture(variant: &str, recovery: bool) -> Fixture {
    // The first review derives real approval from the public provider report.
    let mut responses = routes("valid");
    if !recovery {
        responses.push((
            "/api2/json/access/permissions?path=%2Fstorage%2Fstorage".into(),
            if variant == "visibility" {
                json!({"data":{"/storage/storage":{"Datastore.Audit":1}}})
            } else {
                permissions()
            }
            .to_string(),
        ));
        responses.extend(routes(variant.strip_prefix("before-").unwrap_or("valid")));
        let body = match variant {
            "lost" => DROP_REPLY.into(),
            "wrong-task" => json!({"data":UPID.replace("qmdestroy","qmstart")}).to_string(),
            "wrong-vmid" => json!({"data":UPID.replace(":202:",":203:")}).to_string(),
            "wrong-node" => json!({"data":UPID.replace("pve-lab","other-node")}).to_string(),
            _ => json!({"data":UPID}).to_string(),
        };
        responses.push((DELETE_PATH.into(), body));
    }
    let task_path = format!("/api2/json/nodes/pve-lab/tasks/{UPID}/status");
    responses.push((
        task_path.clone(),
        if variant == "poll-lost" {
            DROP_REPLY.into()
        } else {
            task_status(if variant == "task-failed" {
                "ERROR: private detail"
            } else {
                "OK"
            })
            .to_string()
        },
    ));
    responses.push((task_path, task_status("OK").to_string()));
    responses.push((
        "/api2/json/cluster/nextid?vmid=202".into(),
        json!({"data":if variant=="vm-remains"{203}else{202}}).to_string(),
    ));
    responses.push((
        "/api2/json/access/permissions?path=%2Fstorage%2Fstorage".into(),
        permissions().to_string(),
    ));
    responses.push(("/api2/json/nodes/pve-lab/storage/storage/content?content=images".into(),if variant=="inventory-lost"{DROP_REPLY.into()}else{json!({"data":if variant.starts_with("volume-remains"){vec![json!({"volid":"storage:vm-202-disk-0","vmid":203})]}else{vec![]}}).to_string()}));
    responses.push((
        "/api2/json/access/permissions?path=%2Fstorage%2Fstorage".into(),
        if variant == "visibility-revoked" {
            json!({"data":{}})
        } else {
            permissions()
        }
        .to_string(),
    ));
    responses.push((
        "/api2/json/cluster/nextid?vmid=202".into(),
        json!({"data":if variant=="vm-reused"{203}else{202}}).to_string(),
    ));
    if variant == "volume-remains-then-clean" {
        responses.extend([
            (
                format!("/api2/json/nodes/pve-lab/tasks/{UPID}/status"),
                task_status("OK").to_string(),
            ),
            (
                format!("/api2/json/nodes/pve-lab/tasks/{UPID}/status"),
                task_status("OK").to_string(),
            ),
            (
                "/api2/json/cluster/nextid?vmid=202".into(),
                json!({"data":202}).to_string(),
            ),
            (
                "/api2/json/access/permissions?path=%2Fstorage%2Fstorage".into(),
                permissions().to_string(),
            ),
            (
                "/api2/json/nodes/pve-lab/storage/storage/content?content=images".into(),
                json!({"data":[]}).to_string(),
            ),
            (
                "/api2/json/access/permissions?path=%2Fstorage%2Fstorage".into(),
                permissions().to_string(),
            ),
            (
                "/api2/json/cluster/nextid?vmid=202".into(),
                json!({"data":202}).to_string(),
            ),
        ]);
    }
    let refs = responses
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect::<Vec<_>>();
    Fixture::responses("200 OK", &refs, "", Duration::ZERO, true).await
}
async fn prepare(path: &std::path::Path, provider: &Proxmox) -> (Store, Approval, String) {
    let store = Store::open(path).unwrap();
    let old = registered_fixture_instance(&store, &provider.scope_id());
    let id = "a".repeat(32);
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "UPDATE instances SET id=?1 WHERE id=?2",
            rusqlite::params![id, old],
        )
        .unwrap();
    let report = store
        .instance(&id)
        .unwrap()
        .unwrap()
        .summary
        .deletion_preflight(provider)
        .await
        .unwrap();
    assert_eq!(
        report.deletion_preconditions,
        crate::lifecycle::Status::Passed
    );
    let approval = Approval {
        instance_id: id,
        provider_scope_id: provider.scope_id(),
        definition_hash: report.definition_hash,
        node: report.node,
        vmid: report.vmid,
        config_digest: report.config_digest,
        plan: Plan {
            scope_hash: report.scope_hash,
            volumes: report.volumes,
            snapshots: report.snapshots,
            preserved_media: report.preserved_media,
            preserved_owned_volumes: report.preserved_owned_volumes,
            effects: report.effects,
        },
        confirmed: true,
        acknowledge_data_loss: true,
    };
    let op = store.enqueue_deletion("owned-deletion", &approval).unwrap();
    (store, approval, op.id)
}
fn requests(fixture: &mut Fixture) -> Vec<String> {
    let mut all = vec![];
    while let Ok(request) = fixture.requests.try_recv() {
        all.push(request);
    }
    all
}
#[tokio::test]
async fn deletion_execution_one_fixed_delete_then_independent_completion_and_retirement() {
    let mut fixture = fixture("valid", false).await;
    let provider = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let (store, approval, id) = prepare(&directory.path().join("success.sqlite"), &provider).await;
    let review = requests(&mut fixture);
    assert_eq!(review.len(), 23);
    assert!(
        crate::deletion_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap()
    );
    let job = store.deletion_job(&id).unwrap().unwrap();
    assert_eq!(job.operation.status, Status::Succeeded);
    assert!(job.dispatch_started && job.provider_task.is_some());
    assert_eq!(
        job.operation.result.as_ref().unwrap()["approvalHash"],
        approval.input_hash().unwrap()
    );
    assert_eq!(
        store
            .instance(&approval.instance_id)
            .unwrap()
            .unwrap()
            .summary
            .status,
        crate::instance::Status::Deleted
    );
    assert!(store.get("kernel-lab").unwrap().is_some());
    assert!(
        !crate::deletion_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap()
    );
    let all = requests(&mut fixture);
    assert_eq!(all.len(), 32);
    let deletes = all
        .iter()
        .filter(|r| r.starts_with("DELETE "))
        .collect::<Vec<_>>();
    assert_eq!(deletes.len(), 1);
    assert!(deletes[0].starts_with(&format!("DELETE {DELETE_PATH} HTTP/1.1\r\n")));
    assert_eq!(deletes[0].split("\r\n\r\n").nth(1), Some(""));
    assert!(all
        .iter()
        .all(|r| r.starts_with("GET ") || r.starts_with("DELETE ")));
    assert!(all.iter().take(24).all(|r| r.starts_with("GET ")));
}
#[tokio::test]
async fn deletion_execution_revalidates_full_plan_and_guards_before_dispatch() {
    for variant in [
        "disabled",
        "provider-missing",
        "scope",
        "expired",
        "digest",
        "plan",
        "visibility",
        "before-protected",
        "before-foreign",
        "before-locked",
        "before-pending",
        "before-running",
        "before-paused",
        "before-ha",
        "before-replication",
        "before-fleecing-foreign",
        "before-snapshot-only",
        "before-scope-change",
    ] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("guards.sqlite");
        let (store, mut approval, id) = prepare(&path, &provider).await;
        requests(&mut fixture);
        let connection = rusqlite::Connection::open(&path).unwrap();
        match variant {
            "scope" => approval.provider_scope_id = "c".repeat(64),
            "digest" => approval.config_digest = "c".repeat(40),
            "plan" => approval
                .plan
                .preserved_owned_volumes
                .push("storage:vm-202-other".into()),
            _ => {}
        }
        if ["scope", "digest", "plan"].contains(&variant) {
            connection
                .execute(
                    "UPDATE operations SET approval_json=?1,input_hash=?2 WHERE id=?3",
                    rusqlite::params![
                        serde_json::to_string(&approval).unwrap(),
                        approval.input_hash().unwrap(),
                        id
                    ],
                )
                .unwrap();
        }
        if variant == "expired" {
            connection
                .execute(
                    "UPDATE operations SET confirmed_at=unixepoch()-121 WHERE id=?1",
                    [&id],
                )
                .unwrap();
        }
        assert!(
            crate::deletion_executor::run_one(
                &store,
                if variant == "provider-missing" {
                    None
                } else {
                    Some(&provider)
                },
                variant != "disabled"
            )
            .await
            .unwrap(),
            "{variant}"
        );
        let job = store.deletion_job(&id).unwrap().unwrap();
        assert_eq!(job.operation.status, Status::Failed, "{variant}");
        let expected = match variant {
            "disabled" => Failure::DeletionDisabled,
            "provider-missing" => Failure::ProviderNotConfigured,
            "scope" => Failure::ProviderScopeChanged,
            "expired" => Failure::ApprovalExpired,
            "visibility" => Failure::ProviderForbidden,
            "before-pending" | "before-running" | "before-paused" => {
                Failure::DeletionPreconditionsFailed
            }
            _ => Failure::DeletionTargetChanged,
        };
        assert_eq!(job.operation.failure, Some(expected), "{variant}");
        assert!(
            !job.dispatch_started && job.provider_task.is_none(),
            "{variant}"
        );
        assert!(
            requests(&mut fixture).iter().all(|r| r.starts_with("GET ")),
            "{variant}"
        );
        assert_eq!(
            store
                .instance(&approval.instance_id)
                .unwrap()
                .unwrap()
                .summary
                .status,
            crate::instance::Status::Registered
        );
    }
}
#[tokio::test]
async fn deletion_execution_unknown_submission_never_replays_after_reopen() {
    for variant in ["lost", "wrong-task", "wrong-node", "wrong-vmid"] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("unknown.sqlite");
        let (store, _, id) = prepare(&path, &provider).await;
        requests(&mut fixture);
        crate::deletion_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap();
        let job = store.deletion_job(&id).unwrap().unwrap();
        assert_eq!(
            job.operation.failure,
            Some(Failure::ProviderSubmissionUnconfirmed),
            "{variant}"
        );
        assert_eq!(job.operation.status, Status::Uncertain);
        assert!(job.dispatch_started && job.provider_task.is_none());
        drop(store);
        let reopened = Store::open(&path).unwrap();
        reopened.recover_interrupted_operations().unwrap();
        assert!(
            !crate::deletion_executor::run_one(&reopened, Some(&provider), true)
                .await
                .unwrap()
        );
        let all = requests(&mut fixture);
        assert_eq!(all.iter().filter(|r| r.starts_with("DELETE ")).count(), 1);
        assert_eq!(all.len(), 25);
    }
}
#[tokio::test]
async fn deletion_execution_retained_task_restart_is_get_only_with_dispatch_disabled() {
    let mut fixture = fixture("valid", true).await;
    let provider = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("restart.sqlite");
    let (store, approval, id) = prepare(&path, &provider).await;
    requests(&mut fixture);
    store.claim_deletion().unwrap().unwrap();
    store.begin_deletion_dispatch(&id, &approval).unwrap();
    store
        .link_deletion_task(&id, &TaskReference::new("pve-lab", UPID).unwrap())
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(store.recover_interrupted_operations().unwrap(), 1);
    assert!(
        crate::deletion_executor::run_one(&store, Some(&provider), false)
            .await
            .unwrap()
    );
    assert_eq!(
        store.operation(&id).unwrap().unwrap().status,
        Status::Succeeded
    );
    let all = requests(&mut fixture);
    assert_eq!(all.len(), 7);
    assert!(all.iter().all(|r| r.starts_with("GET ")));
}
#[tokio::test]
async fn deletion_execution_partial_cleanup_or_missing_evidence_holds_identity() {
    for variant in [
        "task-failed",
        "poll-lost",
        "volume-remains",
        "vm-remains",
        "vm-reused",
        "inventory-lost",
        "visibility-revoked",
    ] {
        let mut fixture = fixture(variant, false).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let (store, approval, id) =
            prepare(&directory.path().join("partial.sqlite"), &provider).await;
        requests(&mut fixture);
        crate::deletion_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap();
        let job = store.deletion_job(&id).unwrap().unwrap();
        assert_eq!(job.operation.status, Status::Uncertain, "{variant}");
        assert_eq!(
            job.operation.failure,
            Some(match variant {
                "task-failed" => Failure::ProviderTaskFailed,
                "poll-lost" => Failure::ProviderTaskUnconfirmed,
                _ => Failure::DeletionStateUnconfirmed,
            }),
            "{variant}"
        );
        assert!(job.dispatch_started && job.provider_task.is_some());
        assert!(job.operation.result.is_none());
        assert!(!serde_json::to_string(&job.operation)
            .unwrap()
            .contains("private"));
        assert_eq!(
            store
                .instance(&approval.instance_id)
                .unwrap()
                .unwrap()
                .summary
                .status,
            crate::instance::Status::Registered
        );
        assert!(store.enqueue_deletion("overlap", &approval).is_err());
        assert!(
            !crate::deletion_executor::run_one(&store, Some(&provider), true)
                .await
                .unwrap()
        );
        assert_eq!(
            requests(&mut fixture)
                .iter()
                .filter(|r| r.starts_with("DELETE "))
                .count(),
            1
        );
    }
}
#[tokio::test]
async fn deletion_transport_refuses_unverified_tasks_errors_and_redirects_without_retry() {
    for (status, body, headers) in [
        ("401 Unauthorized", r#"{"data":null}"#, ""),
        ("403 Forbidden", r#"{"data":null}"#, ""),
        ("500 Internal Server Error", "private failure", ""),
        (
            "302 Found",
            r#"{"data":null}"#,
            "Location: /must-not-follow\r\n",
        ),
        (
            "200 OK",
            r#"{"data":"UPID:pve-lab:00000001:00000002:00000003:qmdestroy:203:fixture@pve:"}"#,
            "",
        ),
    ] {
        let mut fixture = Fixture::responses(
            status,
            &[(DELETE_PATH, body)],
            headers,
            Duration::ZERO,
            true,
        )
        .await;
        let result = fixture
            .client()
            .dispatch_deletion(&VmReference::new("pve-lab", 202).unwrap())
            .await;
        assert!(result.is_err());
        if status.starts_with("401") {
            assert!(matches!(result, Err(Error::Unauthorized)));
        }
        if status.starts_with("403") {
            assert!(matches!(result, Err(Error::Forbidden)));
        }
        let all = requests(&mut fixture);
        assert_eq!(all.len(), 1);
        assert!(all[0].starts_with(&format!("DELETE {DELETE_PATH} HTTP/1.1")));
    }
}

#[tokio::test]
async fn deletion_reconciliation_resolves_partial_absence_without_another_delete() {
    let mut fixture = fixture("volume-remains-then-clean", false).await;
    let provider = fixture.client();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reconcile.sqlite");
    let (store, approval, id) = prepare(&path, &provider).await;
    requests(&mut fixture);
    crate::deletion_executor::run_one(&store, Some(&provider), true)
        .await
        .unwrap();
    let partial = store.deletion_job(&id).unwrap().unwrap();
    assert_eq!(partial.operation.status, Status::Uncertain);
    assert_eq!(
        partial.operation.failure,
        Some(Failure::DeletionStateUnconfirmed)
    );
    let initial = requests(&mut fixture);
    assert_eq!(
        initial.iter().filter(|r| r.starts_with("DELETE ")).count(),
        1
    );
    assert!(
        !crate::deletion_executor::run_one(&store, Some(&provider), true)
            .await
            .unwrap()
    );
    drop(store);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
            [&id],
        )
        .unwrap();
    let store = Store::open(&path).unwrap();
    assert!(
        crate::deletion_executor::run_one(&store, Some(&provider), false)
            .await
            .unwrap()
    );
    let completed = store.deletion_job(&id).unwrap().unwrap();
    assert_eq!(completed.operation.status, Status::Succeeded);
    assert_eq!(completed.provider_task, partial.provider_task);
    assert_eq!(
        store
            .instance(&approval.instance_id)
            .unwrap()
            .unwrap()
            .summary
            .status,
        crate::instance::Status::Deleted
    );
    let recovery = requests(&mut fixture);
    assert_eq!(recovery.len(), 7);
    assert!(recovery.iter().all(|r| r.starts_with("GET ")));
}
#[tokio::test]
async fn deletion_retained_task_keeps_exclusion_without_provider_or_matching_scope() {
    for missing in [true, false] {
        let mut fixture = fixture("valid", true).await;
        let provider = fixture.client();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("unavailable.sqlite");
        let (store, approval, id) = prepare(&path, &provider).await;
        requests(&mut fixture);
        store.claim_deletion().unwrap().unwrap();
        store.begin_deletion_dispatch(&id, &approval).unwrap();
        store
            .link_deletion_task(&id, &TaskReference::new("pve-lab", UPID).unwrap())
            .unwrap();
        store.recover_interrupted_operations().unwrap();
        let other = Proxmox::new(
            "https://unrelated.invalid",
            "reader@pve!read",
            "fixture-secret",
            Some(&fixture.ca),
        )
        .unwrap();
        crate::deletion_executor::run_one(&store, if missing { None } else { Some(&other) }, false)
            .await
            .unwrap();
        let uncertain = store.deletion_job(&id).unwrap().unwrap();
        assert_eq!(uncertain.operation.status, Status::Uncertain);
        assert_eq!(
            uncertain.operation.failure,
            Some(if missing {
                Failure::ProviderNotConfigured
            } else {
                Failure::ProviderScopeChanged
            })
        );
        assert!(uncertain.dispatch_started && uncertain.provider_task.is_some());
        assert!(store
            .enqueue_deletion("must-retain-exclusion", &approval)
            .is_err());
        assert!(requests(&mut fixture).is_empty());
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE operations SET next_reconcile_at=0 WHERE id=?1",
                [&id],
            )
            .unwrap();
        crate::deletion_executor::run_one(&store, Some(&provider), false)
            .await
            .unwrap();
        assert_eq!(
            store.operation(&id).unwrap().unwrap().status,
            Status::Succeeded
        );
        assert!(requests(&mut fixture).iter().all(|r| r.starts_with("GET ")));
    }
}
