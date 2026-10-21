//! Fixed durable provisioning stages used by the explicitly enabled production worker.
mod cloud_init;
mod disk;
mod packages;
mod readiness;
mod seed;
mod snapshot;
mod start;
use crate::{
    operation::Failure,
    preflight,
    provisioning::Catalog,
    provisioning_intent::{CloneOutcome, CloneResult, Job},
    proxmox::{Error, PowerState, Proxmox, QmpState, TaskPollOptions, TaskStatus, VmReference},
    store::{self, ProvisioningError, Store},
};
pub use cloud_init::run_cloud_init;
pub use disk::run_disk;
pub use packages::run_packages;
pub use readiness::run_readiness;
pub use seed::run_seed;
pub use snapshot::run_final_snapshot;
pub use start::run_start;
use std::time::Duration;
fn storage_error(_: ProvisioningError) -> store::Error {
    store::Error::Unavailable
}

pub async fn run_clone(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_clone())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, catalog, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_clone(&job.operation.id, outcome)
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
) -> Result<CloneOutcome, store::Error> {
    let stage = &job.stages[0];
    if !stage.dispatch_started && !enabled {
        return Ok(CloneOutcome::Failed(Failure::ProvisioningDisabled));
    }
    let Some(provider) = provider else {
        return Ok(if stage.dispatch_started {
            CloneOutcome::Uncertain(Failure::ProviderNotConfigured)
        } else {
            CloneOutcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if stage.dispatch_started {
            CloneOutcome::Uncertain(Failure::ProviderScopeChanged)
        } else {
            CloneOutcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let instance_id = job
        .operation
        .instance_id
        .as_deref()
        .ok_or(store::Error::InvalidOperation)?;
    let request = job
        .plan
        .clone_request(instance_id)
        .map_err(|_| store::Error::InvalidOperation)?;
    let task = if let Some(task) = &stage.task {
        task.clone()
    } else {
        if stage.dispatch_started {
            return Ok(CloneOutcome::Uncertain(
                Failure::ProviderSubmissionUnconfirmed,
            ));
        }
        let current = catalog.and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok());
        let Some(plan) = current.filter(|p| job.approval.matches(p)) else {
            return Ok(CloneOutcome::Failed(Failure::ProvisioningPlanChanged));
        };
        let report = preflight::run(provider, plan).await;
        let Some(revision) = report.clone_revision(&job.approval.plan_hash) else {
            return Ok(CloneOutcome::Failed(
                Failure::ProvisioningPreconditionsFailed,
            ));
        };
        let source = VmReference::new(&job.approval.target_node, job.approval.source_vmid)
            .map_err(|_| store::Error::InvalidOperation)?;
        let stable = tokio::time::timeout(Duration::from_secs(20), async {
            let config = provider
                .template_config(&job.approval.target_node, job.approval.source_vmid)
                .await?;
            let config = config.as_object().ok_or(Error::InvalidResponse)?;
            if config.get("digest").and_then(|v| v.as_str()) != Some(revision)
                || config.contains_key("lock")
                || !(config.get("template").and_then(|v| v.as_bool()) == Some(true)
                    || config.get("template").and_then(|v| v.as_u64()) == Some(1))
            {
                return Ok(false);
            }
            if provider.vm_has_pending_changes(&source).await? {
                return Ok(false);
            }
            let power = provider.vm_status(&source).await?;
            Ok::<_, Error>(
                power.status == PowerState::Stopped
                    && matches!(power.qmp_status, None | Some(QmpState::Stopped)),
            )
        })
        .await;
        if !matches!(stable, Ok(Ok(true))) {
            return Ok(CloneOutcome::Failed(
                Failure::ProvisioningPreconditionsFailed,
            ));
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        match tokio::task::spawn_blocking(move || database.begin_provisioning_clone(&id, &approval))
            .await
            .map_err(|_| store::Error::Unavailable)?
        {
            Ok(()) => {}
            Err(ProvisioningError::ApprovalExpired) => {
                return Ok(CloneOutcome::Failed(Failure::ApprovalExpired))
            }
            Err(error) => return Err(storage_error(error)),
        }
        let task = match provider.dispatch_full_clone(&request).await {
            Ok(task) => task,
            Err(Error::Unauthorized) => {
                return Ok(CloneOutcome::Failed(Failure::ProviderUnauthorized))
            }
            Err(Error::Forbidden) => return Ok(CloneOutcome::Failed(Failure::ProviderForbidden)),
            Err(_) => {
                return Ok(CloneOutcome::Uncertain(
                    Failure::ProviderSubmissionUnconfirmed,
                ))
            }
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let retained = task.clone();
        tokio::task::spawn_blocking(move || database.link_provisioning_clone(&id, &retained))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        task
    };
    let options = TaskPollOptions::new(Duration::from_millis(250), Duration::from_secs(20))
        .map_err(|_| store::Error::Unavailable)?;
    match provider.wait_for_task(&task, options).await {
        Ok(TaskStatus::Succeeded) => {}
        Ok(TaskStatus::Failed) => return Ok(CloneOutcome::Failed(Failure::ProviderTaskFailed)),
        _ => return Ok(CloneOutcome::Uncertain(Failure::ProviderTaskUnconfirmed)),
    }
    match provider.observe_full_clone(&request).await {
        Ok(target) => Ok(CloneOutcome::Completed(CloneResult {
            instance_id: instance_id.into(),
            provider_scope_id: job.approval.provider_scope_id.clone(),
            plan_hash: job.approval.plan_hash.clone(),
            task_status: TaskStatus::Succeeded,
            target,
        })),
        Err(_) => Ok(CloneOutcome::Uncertain(Failure::PowerStateUnconfirmed)),
    }
}

/// Resource-stage executor, still outside the production worker/API.
pub async fn run_resources(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_resources())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute_resources(store, provider, catalog, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_resources(&job.operation.id, outcome)
    })
    .await
    .map_err(|_| store::Error::Unavailable)?
    .map_err(storage_error)?;
    Ok(true)
}

async fn execute_resources(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
    job: &Job,
) -> Result<crate::provisioning_intent::ResourceOutcome, store::Error> {
    use crate::provisioning_intent::{ResourceOutcome, ResourceResult};
    let stage = &job.stages[1];
    if !stage.dispatch_started && !enabled {
        return Ok(ResourceOutcome::Failed(Failure::ProvisioningDisabled));
    }
    let Some(provider) = provider else {
        return Ok(if stage.dispatch_started {
            ResourceOutcome::Uncertain(Failure::ProviderNotConfigured)
        } else {
            ResourceOutcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if stage.dispatch_started {
            ResourceOutcome::Uncertain(Failure::ProviderScopeChanged)
        } else {
            ResourceOutcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let instance_id = job
        .operation
        .instance_id
        .as_deref()
        .ok_or(store::Error::InvalidOperation)?;
    let clone_digest = job
        .cloned_config_digest()
        .ok_or(store::Error::InvalidOperation)?;
    let digest = stage.config_digest.as_deref().unwrap_or(&clone_digest);
    let request = job
        .plan
        .resource_request(instance_id, digest)
        .map_err(|_| store::Error::InvalidOperation)?;
    let mut submission_unconfirmed = false;
    if !stage.dispatch_started {
        let current = catalog.and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok());
        if !current.is_some_and(|p| job.approval.matches(&p)) {
            return Ok(ResourceOutcome::Failed(Failure::ProvisioningPlanChanged));
        }
        let clone = job
            .plan
            .clone_request(instance_id)
            .map_err(|_| store::Error::InvalidOperation)?;
        let target = VmReference::new(&job.approval.target_node, job.approval.target_vmid)
            .map_err(|_| store::Error::InvalidOperation)?;
        let valid = tokio::time::timeout(Duration::from_secs(20), async {
            let observation = provider.observe_full_clone(&clone).await?;
            Ok::<_, Error>(
                observation.config_digest == digest
                    && !provider.vm_has_pending_changes(&target).await?,
            )
        })
        .await;
        if !matches!(valid, Ok(Ok(true))) {
            return Ok(ResourceOutcome::Failed(
                Failure::ProvisioningPreconditionsFailed,
            ));
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        let revision = digest.to_owned();
        tokio::task::spawn_blocking(move || {
            database.begin_provisioning_resources(&id, &approval, &revision)
        })
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
        submission_unconfirmed = provider.dispatch_resources(&request).await.is_err();
    }
    match provider.observe_resources(&request).await {
        Ok(resources) => Ok(ResourceOutcome::Completed(ResourceResult {
            instance_id: instance_id.into(),
            provider_scope_id: job.approval.provider_scope_id.clone(),
            plan_hash: job.approval.plan_hash.clone(),
            resources,
        })),
        Err(_) => Ok(ResourceOutcome::Uncertain(if submission_unconfirmed {
            Failure::ProviderSubmissionUnconfirmed
        } else {
            Failure::ResourceStateUnconfirmed
        })),
    }
}
