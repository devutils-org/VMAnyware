//! Optional disk-only final snapshot, never a guest-state backup or automatic retry.
use super::{storage_error, Catalog, Failure, Job, Proxmox, Store};
use crate::{
    lifecycle::Status,
    proxmox::{TaskStatus, VmReference},
    snapshot::{Outcome, ResultEvidence},
    store,
};
pub async fn run_final_snapshot(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    creation_enabled: bool,
    snapshots_enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_snapshot())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(
        store,
        provider,
        catalog,
        creation_enabled,
        snapshots_enabled,
        &job,
    )
    .await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_snapshot(&job.operation.id, outcome)
    })
    .await
    .map_err(|_| store::Error::Unavailable)?
    .map_err(storage_error)?;
    Ok(true)
}
async fn execute(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    creation_enabled: bool,
    snapshots_enabled: bool,
    job: &Job,
) -> Result<Outcome, store::Error> {
    let stage = job
        .stages
        .iter()
        .find(|s| s.kind == "snapshot_after_provisioning")
        .ok_or(store::Error::InvalidOperation)?;
    if !stage.dispatch_started {
        if !creation_enabled {
            return Ok(Outcome::Failed(Failure::ProvisioningDisabled));
        }
        if !snapshots_enabled {
            return Ok(Outcome::Failed(Failure::SnapshotDisabled));
        }
    } else if stage.task.is_none() {
        return Ok(Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed));
    }
    let Some(provider) = provider else {
        return Ok(if stage.dispatch_started {
            Outcome::Uncertain(Failure::ProviderNotConfigured)
        } else {
            Outcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if stage.dispatch_started {
            Outcome::Uncertain(Failure::ProviderScopeChanged)
        } else {
            Outcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let target = VmReference::new(&job.approval.target_node, job.approval.target_vmid)
        .map_err(|_| store::Error::InvalidOperation)?;
    let request = job
        .final_snapshot_request()
        .ok_or(store::Error::InvalidOperation)?;
    let task = if let Some(task) = &stage.task {
        task.clone()
    } else {
        let current = catalog.and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok());
        if !current.is_some_and(|p| job.approval.matches(&p)) {
            return Ok(Outcome::Failed(Failure::ProvisioningPlanChanged));
        }
        let database = store.clone();
        let instance_id = job
            .operation
            .instance_id
            .clone()
            .ok_or(store::Error::InvalidOperation)?;
        let instance = tokio::task::spawn_blocking(move || database.instance(&instance_id))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(|_| store::Error::Unavailable)?
            .ok_or(store::Error::InvalidOperation)?;
        let report = match instance
            .summary
            .snapshot_preflight(provider, request.clone())
            .await
        {
            Ok(report) => report,
            Err(_) => return Ok(Outcome::Failed(Failure::SnapshotPreconditionsFailed)),
        };
        if report.creation_preconditions != Status::Passed {
            return Ok(Outcome::Failed(Failure::SnapshotPreconditionsFailed));
        }
        if job
            .regenerated_seed()
            .is_none_or(|seed| seed.config_digest != report.config_digest)
        {
            return Ok(Outcome::Failed(Failure::SnapshotTargetChanged));
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        tokio::task::spawn_blocking(move || {
            database.begin_provisioning_snapshot(&id, &approval, &report.config_digest)
        })
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
        let task = match provider.dispatch_snapshot(&target, &request).await {
            Ok(task) => task,
            Err(_) => return Ok(Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed)),
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let retained = task.clone();
        tokio::task::spawn_blocking(move || database.link_provisioning_snapshot(&id, &retained))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        task
    };
    // One read per round; a busy task does not hold up all provisioning queues.
    match provider.task_status(&task).await {
        Ok(TaskStatus::Succeeded) => {}
        Ok(TaskStatus::Failed) => return Ok(Outcome::Failed(Failure::ProviderTaskFailed)),
        _ => return Ok(Outcome::Uncertain(Failure::ProviderTaskUnconfirmed)),
    }
    let identity = provider
        .vm_identity(
            &target,
            &format!(
                "vmanyware-{}",
                job.operation
                    .instance_id
                    .as_deref()
                    .ok_or(store::Error::InvalidOperation)?
            ),
        )
        .await;
    if !matches!(identity,Ok(ref i) if i.instance_marker && !i.template && !i.locked) {
        return Ok(Outcome::Uncertain(Failure::SnapshotStateUnconfirmed));
    }
    let snapshots = match provider.snapshots(&target).await {
        Ok(s) => s,
        Err(_) => return Ok(Outcome::Uncertain(Failure::SnapshotStateUnconfirmed)),
    };
    let snapshot = snapshots.into_iter().find(|s| {
        s.name == request.name
            && s.description == request.description
            && s.includes_ram == Some(false)
            && s.provider_state.is_none()
    });
    Ok(match snapshot {
        Some(snapshot) => Outcome::Succeeded(ResultEvidence {
            instance_id: job
                .operation
                .instance_id
                .clone()
                .ok_or(store::Error::InvalidOperation)?,
            provider_scope_id: job.approval.provider_scope_id.clone(),
            node: job.approval.target_node.clone(),
            vmid: job.approval.target_vmid,
            task_status: TaskStatus::Succeeded,
            snapshot,
        }),
        None => Outcome::Uncertain(Failure::SnapshotStateUnconfirmed),
    })
}
