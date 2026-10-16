use pocketlab_server::{
    config::Config, provisioning::Catalog, proxmox::Proxmox, router_with_catalog, store::Store,
};
use std::future::IntoFuture;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = match std::env::var("POCKETLAB_PORT") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err("POCKETLAB_PORT must contain valid Unicode".into());
        }
    };
    let config = Config::from_port(port.as_deref())?;
    let provider = Proxmox::from_environment()?;
    let catalog = Catalog::from_environment()?;
    let database = match std::env::var("POCKETLAB_DATABASE") {
        Ok(value) if !value.is_empty() => value,
        Err(std::env::VarError::NotPresent) => "pocketlab.sqlite".into(),
        _ => return Err("Invalid POCKETLAB_DATABASE configuration".into()),
    };
    let _database_lock =
        pocketlab_server::service_lock::ServiceLock::acquire(std::path::Path::new(&database))?;
    let store = Store::open(database)?;
    let listener = tokio::net::TcpListener::bind(config.address()).await?;
    store.recover_interrupted_operations()?;
    eprintln!("PocketLab listening on http://{}", listener.local_addr()?);
    let mut worker = tokio::spawn(pocketlab_server::worker::run(
        store.clone(),
        provider.clone(),
    ));
    let server = axum::serve(listener, router_with_catalog(provider, store, catalog))
        .with_graceful_shutdown(shutdown())
        .into_future();
    tokio::select! {
        result = server => { worker.abort(); result?; }
        result = &mut worker => {
            result.map_err(|_| "Operation worker stopped unexpectedly")??;
            return Err("Operation worker stopped unexpectedly".into());
        }
    }
    Ok(())
}

async fn shutdown() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("Failed to register shutdown signal: {error}");
    }
}
