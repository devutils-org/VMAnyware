//! One bounded readiness round; PID and overall deadline survive process restarts.
use super::{storage_error, Proxmox, Store};
use crate::{
    provisioning_intent::{Job, ReadinessOutcome, ReadinessResult},
    proxmox::{Error, GuestProbe},
    store,
};
use std::time::{SystemTime, UNIX_EPOCH};
pub async fn run_readiness(
    store: &Store,
    provider: Option<&Proxmox>,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_readiness())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_readiness(&job.operation.id, outcome)
    })
    .await
    .map_err(|_| store::Error::Unavailable)?
    .map_err(storage_error)?;
    Ok(true)
}
async fn execute(
    store: &Store,
    provider: Option<&Proxmox>,
    job: &Job,
) -> Result<ReadinessOutcome, store::Error> {
    let probe = job
        .seed_journal
        .as_ref()
        .and_then(|j| j.readiness.as_ref())
        .ok_or(store::Error::InvalidOperation)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| store::Error::Unavailable)?
        .as_secs() as i64;
    if now < probe.started_at
        || now >= probe.deadline
        || (probe.attempts >= 60 && !probe.probe_started)
    {
        return Ok(ReadinessOutcome::TimedOut);
    }
    let Some(provider) = provider else {
        return Ok(ReadinessOutcome::Uncertain);
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(ReadinessOutcome::Uncertain);
    }
    let seed = job
        .regenerated_seed()
        .ok_or(store::Error::InvalidOperation)?;
    let settings = job
        .cloud_init_settings()
        .ok_or(store::Error::InvalidOperation)?;
    let request = job
        .plan
        .readiness_request(
            job.operation
                .instance_id
                .as_deref()
                .ok_or(store::Error::InvalidOperation)?,
            &settings.settings.network,
            &seed.config_digest,
        )
        .map_err(|_| store::Error::InvalidOperation)?;
    let pid = if let Some(pid) = probe.pid {
        pid
    } else {
        if probe.probe_started {
            return Ok(ReadinessOutcome::Uncertain);
        }
        match provider.preflight_guest_probe(&request).await {
            Ok(()) => {}
            Err(Error::InvalidResponse) => return Ok(ReadinessOutcome::Rejected),
            Err(_) => {
                return Ok(ReadinessOutcome::Pending {
                    process_exited: false,
                })
            }
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        if !tokio::task::spawn_blocking(move || database.begin_readiness_probe(&id))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?
        {
            return Ok(ReadinessOutcome::TimedOut);
        }
        let pid = match provider.dispatch_guest_probe(&request).await {
            Ok(pid) => pid,
            Err(_) => return Ok(ReadinessOutcome::Uncertain),
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        tokio::task::spawn_blocking(move || database.link_readiness_pid(&id, pid))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        pid
    };
    Ok(match provider.observe_guest_probe(&request, pid).await {
        Ok(GuestProbe::Pending { process_exited }) => ReadinessOutcome::Pending { process_exited },
        Ok(GuestProbe::Ready(guest)) => ReadinessOutcome::Completed(ReadinessResult {
            instance_id: job
                .operation
                .instance_id
                .clone()
                .ok_or(store::Error::InvalidOperation)?,
            provider_scope_id: job.approval.provider_scope_id.clone(),
            plan_hash: job.approval.plan_hash.clone(),
            guest,
            guest_ready: true,
        }),
        Err(Error::InvalidResponse) => ReadinessOutcome::Rejected,
        Err(_) => ReadinessOutcome::Uncertain,
    })
}
