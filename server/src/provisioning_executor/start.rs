//! Power start after regenerated seed; known tasks reconcile through GET only.
use super::{storage_error, Catalog, Failure, Proxmox, Store};
use crate::{
    lifecycle::Action,
    provisioning_intent::{Job, StartOutcome, StartResult},
    proxmox::{PowerState, QmpState, TaskPollOptions, TaskStatus, VmReference},
    store,
};
use std::time::Duration;
pub async fn run_start(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_start())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, catalog, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_start(&job.operation.id, outcome)
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
) -> Result<StartOutcome, store::Error> {
    let marked = job.power_dispatch_started();
    if !marked && !enabled {
        return Ok(StartOutcome::Failed(Failure::ProvisioningDisabled));
    }
    let Some(provider) = provider else {
        return Ok(if marked {
            StartOutcome::Uncertain(Failure::ProviderNotConfigured)
        } else {
            StartOutcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if marked {
            StartOutcome::Uncertain(Failure::ProviderScopeChanged)
        } else {
            StartOutcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let stage = job
        .stages
        .iter()
        .find(|s| s.kind == "start_vm")
        .ok_or(store::Error::InvalidOperation)?;
    let target = VmReference::new(&job.approval.target_node, job.approval.target_vmid)
        .map_err(|_| store::Error::InvalidOperation)?;
    let task = if let Some(task) = &stage.task {
        task.clone()
    } else {
        if marked {
            return Ok(StartOutcome::Uncertain(
                Failure::ProviderSubmissionUnconfirmed,
            ));
        }
        if !catalog
            .and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok())
            .is_some_and(|p| job.approval.matches(&p))
        {
            return Ok(StartOutcome::Failed(Failure::ProvisioningPlanChanged));
        }
        let seed = job
            .regenerated_seed()
            .ok_or(store::Error::InvalidOperation)?;
        let retained = job
            .cloud_init_dispatch
            .as_ref()
            .ok_or(store::Error::InvalidOperation)?;
        let request = job
            .plan
            .cloud_init_request(
                &job.definition,
                job.operation
                    .instance_id
                    .as_deref()
                    .ok_or(store::Error::InvalidOperation)?,
                &retained.config_digest,
                &retained.volume,
                &retained.network,
            )
            .map_err(|_| store::Error::InvalidOperation)?;
        match provider.preflight_generated_seed(&request).await {
            Ok(current)
                if current.compatible
                    && current.config_digest == seed.config_digest
                    && current.user_data_hash == seed.user_data_hash
                    && current.network_data_hash == seed.network_data_hash
                    && current.user_format == seed.user_format => {}
            _ => {
                return Ok(StartOutcome::Failed(
                    Failure::ProvisioningPreconditionsFailed,
                ))
            }
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        tokio::task::spawn_blocking(move || {
            database.begin_provisioning_start(&id, &approval, &seed.config_digest)
        })
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
        let task = match provider.dispatch_lifecycle(&target, Action::Start).await {
            Ok(task) => task,
            Err(_) => {
                return Ok(StartOutcome::Uncertain(
                    Failure::ProviderSubmissionUnconfirmed,
                ))
            }
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let retained = task.clone();
        tokio::task::spawn_blocking(move || database.link_provisioning_start(&id, &retained))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        task
    };
    let options = TaskPollOptions::new(Duration::from_millis(250), Duration::from_secs(20))
        .map_err(|_| store::Error::Unavailable)?;
    match provider.wait_for_task(&task, options).await {
        Ok(TaskStatus::Succeeded) => {}
        Ok(TaskStatus::Failed) => return Ok(StartOutcome::Failed(Failure::ProviderTaskFailed)),
        _ => return Ok(StartOutcome::Uncertain(Failure::ProviderTaskUnconfirmed)),
    }
    match provider.vm_status(&target).await {
        Ok(observation)
            if observation.status == PowerState::Running
                && matches!(observation.qmp_status, None | Some(QmpState::Running)) =>
        {
            Ok(StartOutcome::Completed(StartResult {
                instance_id: job
                    .operation
                    .instance_id
                    .clone()
                    .ok_or(store::Error::InvalidOperation)?,
                provider_scope_id: job.approval.provider_scope_id.clone(),
                plan_hash: job.approval.plan_hash.clone(),
                task_status: TaskStatus::Succeeded,
                observation,
                guest_ready: false,
            }))
        }
        _ => Ok(StartOutcome::Uncertain(Failure::PowerStateUnconfirmed)),
    }
}
