//! Durable, explicitly enabled deletion; retained tasks reconcile through reads only.
use crate::{
    deletion::{Job, Outcome, Plan},
    instance::ObservationError,
    lifecycle::Status,
    operation::Failure,
    proxmox::{Error, Proxmox, TaskPollOptions, TaskStatus, VmReference},
    store::{self, DeletionError, Store},
};
use std::time::Duration;
fn storage_error(_: DeletionError) -> store::Error {
    store::Error::Unavailable
}
fn provider_failure(error: Error) -> Failure {
    match error {
        Error::Unauthorized => Failure::ProviderUnauthorized,
        Error::Forbidden => Failure::ProviderForbidden,
        Error::Timeout => Failure::ProviderTimeout,
        _ => Failure::ProviderUnavailable,
    }
}
fn read_failure(error: ObservationError) -> Failure {
    match error {
        ObservationError::ScopeChanged => Failure::ProviderScopeChanged,
        ObservationError::InvalidReference => Failure::InstanceReferenceInvalid,
        ObservationError::Provider(error) => provider_failure(error),
    }
}
/// Runtime integration is separate. Known tasks may reconcile even when dispatch is disabled.
pub async fn run_one(
    store: &Store,
    provider: Option<&Proxmox>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_deletion())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || database.finish_deletion(&job.operation.id, outcome))
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    Ok(true)
}
async fn execute(
    store: &Store,
    provider: Option<&Proxmox>,
    enabled: bool,
    job: &Job,
) -> Result<Outcome, store::Error> {
    if !job.dispatch_started && !enabled {
        return Ok(Outcome::Failed(Failure::DeletionDisabled));
    }
    let Some(provider) = provider else {
        return Ok(if job.dispatch_started {
            Outcome::Uncertain(Failure::ProviderNotConfigured)
        } else {
            Outcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if job.dispatch_started {
            Outcome::Uncertain(Failure::ProviderScopeChanged)
        } else {
            Outcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let target = VmReference::new(&job.approval.node, job.approval.vmid)
        .map_err(|_| store::Error::InvalidOperation)?;
    let task = if let Some(task) = &job.provider_task {
        task.clone()
    } else {
        if job.dispatch_started {
            return Ok(Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed));
        }
        let database = store.clone();
        let id = job.approval.instance_id.clone();
        let instance = tokio::task::spawn_blocking(move || database.instance(&id))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(|_| store::Error::Unavailable)?;
        let Some(instance) = instance else {
            return Ok(Outcome::Failed(Failure::InstanceReferenceInvalid));
        };
        if !job.approval.matches_instance(&instance.summary) {
            return Ok(Outcome::Failed(Failure::DeletionTargetChanged));
        }
        // VM-specific ACLs disappear on delete. Prove storage visibility independently first.
        if let Err(error) = provider.deletion_absence_visibility(&job.approval).await {
            return Ok(Outcome::Failed(provider_failure(error)));
        }
        let report = match instance.summary.deletion_preflight(provider).await {
            Ok(report) => report,
            Err(error) => return Ok(Outcome::Failed(read_failure(error))),
        };
        let plan = Plan {
            scope_hash: report.scope_hash,
            volumes: report.volumes,
            snapshots: report.snapshots,
            preserved_media: report.preserved_media,
            preserved_owned_volumes: report.preserved_owned_volumes,
            effects: report.effects,
        };
        if report.config_digest != job.approval.config_digest || plan != job.approval.plan {
            return Ok(Outcome::Failed(Failure::DeletionTargetChanged));
        }
        if report.deletion_preconditions != Status::Passed {
            return Ok(Outcome::Failed(Failure::DeletionPreconditionsFailed));
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        match tokio::task::spawn_blocking(move || database.begin_deletion_dispatch(&id, &approval))
            .await
            .map_err(|_| store::Error::Unavailable)?
        {
            Ok(()) => {}
            Err(DeletionError::ApprovalExpired) => {
                return Ok(Outcome::Failed(Failure::ApprovalExpired))
            }
            Err(error) => return Err(storage_error(error)),
        }
        // Persist a returned task immediately; a lost/invalid response never permits replay.
        let task = match provider.dispatch_deletion(&target).await {
            Ok(task) => task,
            Err(Error::Unauthorized) => return Ok(Outcome::Failed(Failure::ProviderUnauthorized)),
            Err(Error::Forbidden) => return Ok(Outcome::Failed(Failure::ProviderForbidden)),
            Err(_) => return Ok(Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed)),
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let retained = task.clone();
        tokio::task::spawn_blocking(move || database.link_deletion_task(&id, &retained))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        task
    };
    let options = TaskPollOptions::new(Duration::from_millis(250), Duration::from_secs(20))
        .map_err(|_| store::Error::Unavailable)?;
    match provider.wait_for_task(&task, options).await {
        Ok(TaskStatus::Succeeded) => {}
        // Destroy may already have removed part of the scope even when the task fails.
        Ok(TaskStatus::Failed) => return Ok(Outcome::Uncertain(Failure::ProviderTaskFailed)),
        _ => return Ok(Outcome::Uncertain(Failure::ProviderTaskUnconfirmed)),
    }
    match provider
        .verify_deletion_completion(&job.approval, &task)
        .await
    {
        Ok(proof) => Ok(Outcome::Succeeded(proof)),
        Err(_) => Ok(Outcome::Uncertain(Failure::DeletionStateUnconfirmed)),
    }
}
