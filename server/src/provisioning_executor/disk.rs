//! Standalone disk-stage execution; the production worker does not call it yet.
use super::{
    storage_error, Catalog, Error, Failure, Job, Proxmox, Store, TaskPollOptions, TaskStatus,
};
use crate::{
    provisioning::DiskDevice,
    provisioning_intent::{DiskOutcome, DiskResult},
    store,
};
use std::time::Duration;

pub async fn run_disk(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_disk())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, catalog, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_disk(&job.operation.id, outcome)
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
    enabled: bool,
    job: &Job,
) -> Result<DiskOutcome, store::Error> {
    let stage = &job.stages[2];
    if !stage.dispatch_started && !enabled {
        return Ok(DiskOutcome::Failed(Failure::ProvisioningDisabled));
    }
    let Some(provider) = provider else {
        return Ok(if stage.dispatch_started {
            DiskOutcome::Uncertain(Failure::ProviderNotConfigured)
        } else {
            DiskOutcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if stage.dispatch_started {
            DiskOutcome::Uncertain(Failure::ProviderScopeChanged)
        } else {
            DiskOutcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let instance = job
        .operation
        .instance_id
        .as_deref()
        .ok_or(store::Error::InvalidOperation)?;
    let (request, task) = if stage.dispatch_started {
        let retained = job
            .disk_dispatch
            .as_ref()
            .ok_or(store::Error::InvalidOperation)?;
        let request = job
            .plan
            .disk_request(instance, &retained.config_digest, &retained.volume)
            .map_err(|_| store::Error::InvalidOperation)?;
        let Some(task) = &stage.task else {
            return Ok(DiskOutcome::Uncertain(
                Failure::ProviderSubmissionUnconfirmed,
            ));
        };
        (request, task.clone())
    } else {
        let current = catalog.and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok());
        if !current.is_some_and(|p| job.approval.matches(&p)) {
            return Ok(DiskOutcome::Failed(Failure::ProvisioningPlanChanged));
        }
        let digest = job
            .resource_config_digest()
            .ok_or(store::Error::InvalidOperation)?;
        let prepared = tokio::time::timeout(Duration::from_secs(20), async {
            let config = provider
                .template_config(&job.approval.target_node, job.approval.target_vmid)
                .await?;
            let device = match job.plan.source.disk_device {
                DiskDevice::Scsi0 => "scsi0",
                DiskDevice::Virtio0 => "virtio0",
                DiskDevice::Sata0 => "sata0",
            };
            let volume = config
                .get(device)
                .and_then(|v| v.as_str())
                .and_then(|v| v.split(',').next())
                .ok_or(Error::InvalidResponse)?;
            let request = job.plan.disk_request(instance, &digest, volume)?;
            provider.preflight_disk_expansion(&request).await?;
            Ok::<_, Error>((request, volume.to_owned()))
        })
        .await;
        let Ok(Ok((request, volume))) = prepared else {
            return Ok(DiskOutcome::Failed(
                Failure::ProvisioningPreconditionsFailed,
            ));
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        tokio::task::spawn_blocking(move || {
            database.begin_provisioning_disk(&id, &approval, &digest, &volume)
        })
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
        let task = match provider.dispatch_disk_expansion(&request).await {
            Ok(task) => task,
            Err(Error::Unauthorized) => {
                return Ok(DiskOutcome::Failed(Failure::ProviderUnauthorized))
            }
            Err(Error::Forbidden) => return Ok(DiskOutcome::Failed(Failure::ProviderForbidden)),
            Err(_) => {
                return Ok(DiskOutcome::Uncertain(
                    Failure::ProviderSubmissionUnconfirmed,
                ))
            }
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let retained = task.clone();
        tokio::task::spawn_blocking(move || database.link_provisioning_disk(&id, &retained))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        (request, task)
    };
    let options = TaskPollOptions::new(Duration::from_millis(250), Duration::from_secs(20))
        .map_err(|_| store::Error::Unavailable)?;
    match provider.wait_for_task(&task, options).await {
        Ok(TaskStatus::Succeeded) => {}
        Ok(TaskStatus::Failed) => return Ok(DiskOutcome::Failed(Failure::ProviderTaskFailed)),
        _ => return Ok(DiskOutcome::Uncertain(Failure::ProviderTaskUnconfirmed)),
    }
    match provider.observe_disk_expansion(&request).await {
        Ok(disk) => Ok(DiskOutcome::Completed(DiskResult {
            instance_id: instance.into(),
            provider_scope_id: job.approval.provider_scope_id.clone(),
            plan_hash: job.approval.plan_hash.clone(),
            task_status: TaskStatus::Succeeded,
            disk,
        })),
        Err(_) => Ok(DiskOutcome::Uncertain(Failure::DiskStateUnconfirmed)),
    }
}
