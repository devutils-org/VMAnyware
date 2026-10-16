use pocketlab_server::{
    config::Config, diagnostics, provisioning::Catalog, proxmox::Proxmox, router_with_catalog,
    store::Store,
};
use std::future::IntoFuture;

#[tokio::main]
async fn main() {
    if let Err(reason) = run().await {
        diagnostics::failed(reason);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), &'static str> {
    let port = match std::env::var("POCKETLAB_PORT") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("invalid_port_configuration"),
    };
    let config = Config::from_port(port.as_deref()).map_err(|_| "invalid_port_configuration")?;
    let provider = Proxmox::from_environment().map_err(|_| "invalid_provider_configuration")?;
    let catalog = Catalog::from_environment().map_err(|_| "invalid_template_configuration")?;
    let database = match std::env::var("POCKETLAB_DATABASE") {
        Ok(value) if !value.is_empty() => value,
        Err(std::env::VarError::NotPresent) => "pocketlab.sqlite".into(),
        _ => return Err("invalid_database_configuration"),
    };
    let _database_lock =
        pocketlab_server::service_lock::ServiceLock::acquire(std::path::Path::new(&database))
            .map_err(|_| "database_lock_unavailable")?;
    let store = Store::open(database).map_err(|_| "database_unavailable")?;
    let shutdown = Shutdown::register()?;
    let listener = tokio::net::TcpListener::bind(config.address())
        .await
        .map_err(|_| "listener_unavailable")?;
    store
        .recover_interrupted_operations()
        .map_err(|_| "operation_recovery_failed")?;
    let mut worker = tokio::spawn(pocketlab_server::worker::run(
        store.clone(),
        provider.clone(),
    ));
    let server = axum::serve(listener, router_with_catalog(provider, store, catalog))
        .with_graceful_shutdown(shutdown.wait())
        .into_future();
    diagnostics::listening(config.address().port());
    tokio::select! {
        result = server => {
            worker.abort();
            result.map_err(|_| "server_stopped_unexpectedly")?;
        }
        _ = &mut worker => return Err("operation_worker_stopped_unexpectedly"),
    }
    diagnostics::stopped();
    Ok(())
}

#[cfg(unix)]
struct Shutdown {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl Shutdown {
    fn register() -> Result<Self, &'static str> {
        use tokio::signal::unix::{signal, SignalKind};
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())
                .map_err(|_| "shutdown_signal_unavailable")?,
            terminate: signal(SignalKind::terminate())
                .map_err(|_| "shutdown_signal_unavailable")?,
        })
    }

    async fn wait(mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {},
            _ = self.terminate.recv() => {},
        }
    }
}

#[cfg(windows)]
struct Shutdown(tokio::signal::windows::CtrlC);

#[cfg(windows)]
impl Shutdown {
    fn register() -> Result<Self, &'static str> {
        tokio::signal::windows::ctrl_c()
            .map(Self)
            .map_err(|_| "shutdown_signal_unavailable")
    }

    async fn wait(mut self) {
        self.0.recv().await;
    }
}
