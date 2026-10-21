//! One package dispatch or retained-PID read; uncertain submissions are never replayed.
use super::{storage_error, Catalog, Failure, Job, Proxmox, Store};
use crate::{provisioning_intent::PackageOutcome, proxmox::PackageProgress, store};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn run_packages(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_packages())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, catalog, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_packages(&job.operation.id, outcome)
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
) -> Result<PackageOutcome, store::Error> {
    let stage = job
        .stages
        .iter()
        .find(|s| s.kind == "install_declared_packages")
        .ok_or(store::Error::InvalidOperation)?;
    let journal = job
        .seed_journal
        .as_ref()
        .and_then(|j| j.packages.as_ref())
        .ok_or(store::Error::InvalidOperation)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| store::Error::Unavailable)?
        .as_secs() as i64;
    if now < journal.started_at || now >= journal.deadline {
        return Ok(PackageOutcome::TimedOut);
    }
    if stage.dispatch_started && journal.pid.is_none() {
        return Ok(PackageOutcome::Uncertain);
    }
    if !stage.dispatch_started && !enabled {
        return Ok(PackageOutcome::Failed(Failure::ProvisioningDisabled));
    }
    let Some(provider) = provider else {
        return Ok(if stage.dispatch_started {
            PackageOutcome::Uncertain
        } else {
            PackageOutcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if stage.dispatch_started {
            PackageOutcome::Uncertain
        } else {
            PackageOutcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let seed = job
        .regenerated_seed()
        .ok_or(store::Error::InvalidOperation)?;
    let settings = job
        .cloud_init_settings()
        .ok_or(store::Error::InvalidOperation)?;
    let request = job
        .plan
        .package_request(
            job.operation
                .instance_id
                .as_deref()
                .ok_or(store::Error::InvalidOperation)?,
            &settings.settings.network,
            &seed.config_digest,
        )
        .map_err(|_| store::Error::InvalidOperation)?;
    let pid = if let Some(pid) = journal.pid {
        pid
    } else {
        let current = catalog.and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok());
        if !current.is_some_and(|p| job.approval.matches(&p)) {
            return Ok(PackageOutcome::Failed(Failure::ProvisioningPlanChanged));
        }
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        if !tokio::task::spawn_blocking(move || {
            database.begin_provisioning_packages(&id, &approval)
        })
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?
        {
            return Ok(PackageOutcome::TimedOut);
        }
        let pid = match provider.dispatch_packages(&request).await {
            Ok(pid) => pid,
            Err(_) => return Ok(PackageOutcome::Uncertain),
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        tokio::task::spawn_blocking(move || database.link_provisioning_packages(&id, pid))
            .await
            .map_err(|_| store::Error::Unavailable)?
            .map_err(storage_error)?;
        pid
    };
    Ok(match provider.observe_packages(&request, pid).await {
        Ok(PackageProgress::Pending) => PackageOutcome::Pending,
        Ok(PackageProgress::Completed) => PackageOutcome::Completed,
        Ok(PackageProgress::Failed) => PackageOutcome::Failed(Failure::PackagesFailed),
        Err(_) => PackageOutcome::Uncertain,
    })
}
