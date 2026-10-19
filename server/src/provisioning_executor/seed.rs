//! First start_vm substep: durable seed regeneration, never replayed after a marker.
use super::{storage_error, Catalog, Failure, Proxmox, Store};
use crate::{provisioning_intent::SeedOutcome, store};

pub async fn run_seed(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_seed())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = if !enabled {
        SeedOutcome::Failed(Failure::ProvisioningDisabled)
    } else if let Some(provider) = provider {
        if provider.scope_id() != job.approval.provider_scope_id {
            SeedOutcome::Failed(Failure::ProviderScopeChanged)
        } else if !catalog
            .and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok())
            .is_some_and(|p| job.approval.matches(&p))
        {
            SeedOutcome::Failed(Failure::ProvisioningPlanChanged)
        } else {
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
                Ok(expected)
                    if job.valid_seed(&expected)
                        && job.cloud_init_settings().is_some_and(|s| {
                            s.settings.resources.target.config_digest == expected.config_digest
                        }) =>
                {
                    let retained_expected = expected.clone();
                    let database = store.clone();
                    let id = job.operation.id.clone();
                    let approval = job.approval.clone();
                    tokio::task::spawn_blocking(move || {
                        database.begin_provisioning_seed(&id, &approval, retained_expected)
                    })
                    .await
                    .map_err(|_| store::Error::Unavailable)?
                    .map_err(storage_error)?;
                    match provider
                        .dispatch_seed_regeneration(&request, &expected)
                        .await
                    {
                        Ok(proof) => SeedOutcome::Completed(proof),
                        Err(_) => SeedOutcome::Uncertain(Failure::SeedRegenerationUnconfirmed),
                    }
                }
                _ => SeedOutcome::Failed(Failure::ProvisioningPreconditionsFailed),
            }
        }
    } else {
        SeedOutcome::Failed(Failure::ProviderNotConfigured)
    };
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_seed(&job.operation.id, outcome)
    })
    .await
    .map_err(|_| store::Error::Unavailable)?
    .map_err(storage_error)?;
    Ok(true)
}
