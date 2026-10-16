//! Single inventory worker; no VM mutations or generic executable jobs.
use crate::{
    inventory,
    operation::Failure,
    proxmox::{Error, Proxmox},
    store::{self, Store},
};
use std::time::Duration;

pub async fn run(store: Store, provider: Option<Proxmox>) -> Result<(), store::Error> {
    loop {
        if !run_one(&store, provider.as_ref()).await? {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

/// Returns false when idle. A completed inventory read is not guest readiness.
pub async fn run_one(store: &Store, provider: Option<&Proxmox>) -> Result<bool, store::Error> {
    let database = store.clone();
    let operation = tokio::task::spawn_blocking(move || database.claim_inventory())
        .await
        .map_err(|_| store::Error::Unavailable)??;
    let Some(operation) = operation else {
        return Ok(false);
    };
    let result = match provider {
        None => Err(Failure::ProviderNotConfigured),
        Some(provider) => inventory::refresh(provider)
            .await
            .map_err(|e| match e {
                Error::Unauthorized => Failure::ProviderUnauthorized,
                Error::Forbidden => Failure::ProviderForbidden,
                Error::Timeout => Failure::ProviderTimeout,
                _ => Failure::ProviderUnavailable,
            })
            .and_then(|value| {
                serde_json::to_value(value).map_err(|_| Failure::ProviderUnavailable)
            }),
    };
    let database = store.clone();
    tokio::task::spawn_blocking(move || database.finish_inventory(&operation.id, result))
        .await
        .map_err(|_| store::Error::Unavailable)??;
    Ok(true)
}
