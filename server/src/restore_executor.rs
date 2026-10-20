//! Durable disk-only snapshot restore executor. Real infrastructure execution requires explicit opt-in.
use crate::lifecycle::Status;
use crate::{
    instance::ObservationError,
    operation::Failure,
    proxmox::{Error, Proxmox, TaskPollOptions, TaskStatus, VmReference},
    snapshot_restore::{Job, Outcome, ResultEvidence},
    store::{self, RestoreError, Store},
};
use std::time::Duration;

fn storage_error(_: RestoreError) -> store::Error {
    store::Error::Unavailable
}
fn read_failure(error: ObservationError) -> Failure {
    match error {
        ObservationError::ScopeChanged => Failure::ProviderScopeChanged,
        ObservationError::InvalidReference => Failure::InstanceReferenceInvalid,
        ObservationError::Provider(Error::Unauthorized) => Failure::ProviderUnauthorized,
        ObservationError::Provider(Error::Forbidden) => Failure::ProviderForbidden,
        ObservationError::Provider(Error::Timeout) => Failure::ProviderTimeout,
        ObservationError::Provider(_) => Failure::ProviderUnavailable,
    }
}

/// Caller supplies explicit capability; default runtime does not submit restores. Reconciliation of retained tasks remains
/// read-only and is permitted even after the write capability is disabled.
pub async fn run_one(
    store: &Store,
    provider: Option<&Proxmox>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_restore())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || database.finish_restore(&job.operation.id, outcome))
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
        return Ok(Outcome::Failed(Failure::RestoreDisabled));
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
        // A recovered task never enters the submission path, regardless of capability.
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
            return Ok(Outcome::Failed(Failure::RestoreTargetChanged));
        }
        let report = match instance
            .summary
            .snapshot_restore_preflight(provider, job.approval.request.clone())
            .await
        {
            Ok(report) => report,
            Err(error) => return Ok(Outcome::Failed(read_failure(error))),
        };
        if report.config_digest != job.approval.config_digest
            || report.snapshot_config_hash.as_deref()
                != Some(job.approval.snapshot_config_hash.as_str())
        {
            return Ok(Outcome::Failed(Failure::RestoreTargetChanged));
        }
        if report.restore_preconditions != Status::Passed {
            return Ok(Outcome::Failed(Failure::RestorePreconditionsFailed));
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        match tokio::task::spawn_blocking(move || database.begin_restore_dispatch(&id, &approval))
            .await
            .map_err(|_| store::Error::Unavailable)?
        {
            Ok(()) => {}
            Err(RestoreError::ApprovalExpired) => {
                return Ok(Outcome::Failed(Failure::ApprovalExpired))
            }
            Err(error) => return Err(storage_error(error)),
        }
        // No cancellation point between a completed POST and spawning persistence.
        let task = match provider
            .dispatch_restore(&target, &job.approval.request)
            .await
        {
            Ok(task) => task,
            Err(Error::Unauthorized) => return Ok(Outcome::Failed(Failure::ProviderUnauthorized)),
            Err(Error::Forbidden) => return Ok(Outcome::Failed(Failure::ProviderForbidden)),
            Err(_) => return Ok(Outcome::Uncertain(Failure::ProviderSubmissionUnconfirmed)),
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let retained = task.clone();
        tokio::task::spawn_blocking(move || database.link_restore_task(&id, &retained))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        task
    };
    let options = TaskPollOptions::new(Duration::from_millis(250), Duration::from_secs(20))
        .map_err(|_| store::Error::Unavailable)?;
    match provider.wait_for_task(&task, options).await {
        Ok(TaskStatus::Succeeded) => {}
        Ok(TaskStatus::Failed) => return Ok(Outcome::Failed(Failure::ProviderTaskFailed)),
        _ => return Ok(Outcome::Uncertain(Failure::ProviderTaskUnconfirmed)),
    }
    // Rollback changes the live configuration. Recheck stopped power, ownership,
    // complete disk-only metadata and the reviewed saved configuration, without
    // requiring the old live digest. This is provider evidence, not guest file proof.
    let database = store.clone();
    let id = job.approval.instance_id.clone();
    let instance = tokio::task::spawn_blocking(move || database.instance(&id))
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(|_| store::Error::Unavailable)?;
    let Some(instance) =
        instance.filter(|instance| job.approval.matches_instance(&instance.summary))
    else {
        return Ok(Outcome::Uncertain(Failure::RestoreStateUnconfirmed));
    };
    match instance
        .summary
        .snapshot_restore_preflight(provider, job.approval.request.clone())
        .await
    {
        Ok(report)
            if report.restore_preconditions == Status::Passed
                && report.snapshot_config_hash.as_deref()
                    == Some(job.approval.snapshot_config_hash.as_str()) =>
        {
            Ok(Outcome::Succeeded(ResultEvidence {
                instance_id: job.approval.instance_id.clone(),
                provider_scope_id: job.approval.provider_scope_id.clone(),
                node: job.approval.node.clone(),
                vmid: job.approval.vmid,
                snapshot_name: job.approval.request.snapshot_name.clone(),
                snapshot_config_hash: job.approval.snapshot_config_hash.clone(),
                task_status: TaskStatus::Succeeded,
                observation: report.observation,
            }))
        }
        _ => Ok(Outcome::Uncertain(Failure::RestoreStateUnconfirmed)),
    }
}
