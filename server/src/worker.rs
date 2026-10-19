//! Sequential provider worker; lifecycle writes are disabled in the default runner.
use crate::{
    instance::ObservationError,
    inventory,
    operation::Failure,
    proxmox::{Error, Proxmox},
    store::{self, Store},
};
use std::time::Duration;

pub async fn run(store: Store, provider: Option<Proxmox>) -> Result<(), store::Error> {
    run_with_lifecycle(store, provider, false).await
}

pub async fn run_with_lifecycle(
    store: Store,
    provider: Option<Proxmox>,
    lifecycle_enabled: bool,
) -> Result<(), store::Error> {
    loop {
        if !run_round(&store, provider.as_ref(), lifecycle_enabled).await? {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

/// At most one read, then one lifecycle claim. Neither queue can starve the other.
/// Work remains sequential; a long provider request can delay the next claim.
pub async fn run_round(
    store: &Store,
    provider: Option<&Proxmox>,
    lifecycle_enabled: bool,
) -> Result<bool, store::Error> {
    let read = run_read(store, provider).await?;
    let lifecycle = crate::lifecycle_executor::run_one(store, provider, lifecycle_enabled).await?;
    Ok(read || lifecycle)
}

fn provider_failure(error: Error) -> Failure {
    match error {
        Error::Unauthorized => Failure::ProviderUnauthorized,
        Error::Forbidden => Failure::ProviderForbidden,
        Error::Timeout => Failure::ProviderTimeout,
        _ => Failure::ProviderUnavailable,
    }
}

/// Returns false when idle. A successful read is not guest readiness or ownership.
pub async fn run_one(store: &Store, provider: Option<&Proxmox>) -> Result<bool, store::Error> {
    if run_read(store, provider).await? {
        Ok(true)
    } else {
        crate::lifecycle_executor::run_one(store, provider, false).await
    }
}

async fn run_read(store: &Store, provider: Option<&Proxmox>) -> Result<bool, store::Error> {
    let database = store.clone();
    let operation = tokio::task::spawn_blocking(move || database.claim_operation())
        .await
        .map_err(|_| store::Error::Unavailable)??;
    let Some(operation) = operation else {
        return Ok(false);
    };
    let result = match (operation.kind.as_str(), provider) {
        (_, None) => Err(Failure::ProviderNotConfigured),
        ("inventory_refresh", Some(provider)) => inventory::refresh(provider)
            .await
            .map_err(provider_failure)
            .and_then(|value| {
                serde_json::to_value(value).map_err(|_| Failure::ProviderUnavailable)
            }),
        ("instance_observe", Some(provider)) => {
            let id = operation
                .instance_id
                .clone()
                .ok_or(store::Error::InvalidOperation)?;
            let database = store.clone();
            let instance = tokio::task::spawn_blocking(move || database.instance(&id))
                .await
                .map_err(|_| store::Error::Unavailable)?
                .map_err(|_| store::Error::Unavailable)?;
            match instance {
                None => Err(Failure::InstanceReferenceInvalid),
                Some(instance) => instance
                    .summary
                    .observe(provider)
                    .await
                    .map_err(|error| match error {
                        ObservationError::ScopeChanged => Failure::ProviderScopeChanged,
                        ObservationError::InvalidReference => Failure::InstanceReferenceInvalid,
                        ObservationError::Provider(error) => provider_failure(error),
                    })
                    .and_then(|value| {
                        serde_json::to_value(value).map_err(|_| Failure::ProviderUnavailable)
                    }),
            }
        }
        _ => return Err(store::Error::InvalidOperation),
    };
    let database = store.clone();
    tokio::task::spawn_blocking(move || database.finish_operation(&operation.id, result))
        .await
        .map_err(|_| store::Error::Unavailable)??;
    Ok(true)
}

/// Production runner: all queues remain sequential and writes require explicit flags.
pub async fn run_with_execution(
    store: Store,
    provider: Option<Proxmox>,
    catalog: Option<crate::provisioning::Catalog>,
    lifecycle_enabled: bool,
    provisioning_enabled: bool,
) -> Result<(), store::Error> {
    loop {
        if !run_execution_round(
            &store,
            provider.as_ref(),
            catalog.as_ref(),
            lifecycle_enabled,
            provisioning_enabled,
        )
        .await?
        {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}
/// One read, one lifecycle claim, then at most one provisioning stage per round.
pub async fn run_execution_round(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&crate::provisioning::Catalog>,
    lifecycle_enabled: bool,
    provisioning_enabled: bool,
) -> Result<bool, store::Error> {
    let existing = run_round(store, provider, lifecycle_enabled).await?;
    let provisioning = run_provisioning_one(store, provider, catalog, provisioning_enabled).await?;
    Ok(existing || provisioning)
}
async fn run_provisioning_one(
    store: &Store,
    provider: Option<&Proxmox>,
    catalog: Option<&crate::provisioning::Catalog>,
    enabled: bool,
) -> Result<bool, store::Error> {
    use crate::provisioning_executor as execution;
    // Finish existing work before cloning more VMs; do not chain new stages in one round.
    if execution::run_readiness(store, provider).await? {
        return Ok(true);
    }
    if execution::run_start(store, provider, catalog, enabled).await? {
        return Ok(true);
    }
    if execution::run_seed(store, provider, catalog, enabled).await? {
        return Ok(true);
    }
    if execution::run_cloud_init(store, provider, catalog, enabled).await? {
        return Ok(true);
    }
    if execution::run_disk(store, provider, catalog, enabled).await? {
        return Ok(true);
    }
    if execution::run_resources(store, provider, catalog, enabled).await? {
        return Ok(true);
    }
    execution::run_clone(store, provider, catalog, enabled).await
}
