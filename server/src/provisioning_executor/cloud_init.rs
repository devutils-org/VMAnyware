//! API settings only; no seed compatibility, start or guest readiness claim.
use super::{storage_error, Catalog, Error, Failure, Job, Proxmox, Store};
use crate::{
    provisioning::DiskDevice,
    provisioning_intent::{CloudInitOutcome, CloudInitResult},
    store,
};
use std::time::Duration;
pub async fn run_cloud_init(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    let database = store.clone();
    let job = tokio::task::spawn_blocking(move || database.claim_provisioning_cloud_init())
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
    let Some(job) = job else {
        return Ok(false);
    };
    let outcome = execute(store, provider, catalog, enabled, &job).await?;
    let database = store.clone();
    tokio::task::spawn_blocking(move || {
        database.finish_provisioning_cloud_init(&job.operation.id, outcome)
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
) -> Result<CloudInitOutcome, store::Error> {
    let stage = job
        .stages
        .iter()
        .find(|s| s.kind == "configure_cloud_init")
        .ok_or(store::Error::InvalidOperation)?;
    if !stage.dispatch_started && !enabled {
        return Ok(CloudInitOutcome::Failed(Failure::ProvisioningDisabled));
    }
    let Some(provider) = provider else {
        return Ok(if stage.dispatch_started {
            CloudInitOutcome::Uncertain(Failure::ProviderNotConfigured)
        } else {
            CloudInitOutcome::Failed(Failure::ProviderNotConfigured)
        });
    };
    if provider.scope_id() != job.approval.provider_scope_id {
        return Ok(if stage.dispatch_started {
            CloudInitOutcome::Uncertain(Failure::ProviderScopeChanged)
        } else {
            CloudInitOutcome::Failed(Failure::ProviderScopeChanged)
        });
    }
    let instance = job
        .operation
        .instance_id
        .as_deref()
        .ok_or(store::Error::InvalidOperation)?;
    let mut submission_unconfirmed = false;
    let request = if let Some(retained) = &job.cloud_init_dispatch {
        job.plan
            .cloud_init_request(
                &job.definition,
                instance,
                &retained.config_digest,
                &retained.volume,
                &retained.network,
            )
            .map_err(|_| store::Error::InvalidOperation)?
    } else {
        let current = catalog.and_then(|c| c.plan(&job.definition, job.approval.target_vmid).ok());
        if !current.is_some_and(|p| job.approval.matches(&p)) {
            return Ok(CloudInitOutcome::Failed(Failure::ProvisioningPlanChanged));
        }
        let (digest, expected_volume) = job
            .cloud_init_source()
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
            if expected_volume.as_ref().is_some_and(|v| v != volume) {
                return Err(Error::InvalidResponse);
            }
            let network = config
                .get("net0")
                .and_then(|v| v.as_str())
                .ok_or(Error::InvalidResponse)?;
            let request =
                job.plan
                    .cloud_init_request(&job.definition, instance, &digest, volume, network)?;
            provider.preflight_cloud_init(&request).await?;
            Ok::<_, Error>((request, volume.to_owned(), network.to_owned()))
        })
        .await;
        let Ok(Ok((request, volume, network))) = prepared else {
            return Ok(CloudInitOutcome::Failed(
                Failure::ProvisioningPreconditionsFailed,
            ));
        };
        let database = store.clone();
        let id = job.operation.id.clone();
        let approval = job.approval.clone();
        tokio::task::spawn_blocking(move || {
            database.begin_provisioning_cloud_init(&id, &approval, &digest, &volume, &network)
        })
        .await
        .map_err(|_| store::Error::Unavailable)?
        .map_err(storage_error)?;
        submission_unconfirmed = provider.dispatch_cloud_init(&request).await.is_err();
        request
    };
    match provider.observe_cloud_init(&request).await {
        Ok(settings) => Ok(CloudInitOutcome::Completed(Box::new(CloudInitResult {
            instance_id: instance.into(),
            provider_scope_id: job.approval.provider_scope_id.clone(),
            plan_hash: job.approval.plan_hash.clone(),
            settings,
        }))),
        Err(_) => Ok(CloudInitOutcome::Uncertain(if submission_unconfirmed {
            Failure::ProviderSubmissionUnconfirmed
        } else {
            Failure::CloudInitStateUnconfirmed
        })),
    }
}
