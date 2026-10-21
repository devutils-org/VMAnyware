use std::{future::IntoFuture, time::Duration};
use vmanyware_server::{
    config::Config, diagnostics, provisioning::Catalog, proxmox::Proxmox, router_with_deletions,
    store::Store,
};

const DRAIN_LIMIT: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() {
    if let Err(reason) = run().await {
        diagnostics::failed(reason);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), &'static str> {
    let port = match std::env::var("VMANYWARE_PORT") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("invalid_port_configuration"),
    };
    let config = Config::from_port(port.as_deref()).map_err(|_| "invalid_port_configuration")?;
    let lifecycle = match std::env::var("VMANYWARE_LIFECYCLE_ENABLED") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("invalid_lifecycle_configuration"),
    };
    let config = config
        .with_lifecycle(lifecycle.as_deref())
        .map_err(|_| "invalid_lifecycle_configuration")?;
    let provisioning = match std::env::var("VMANYWARE_PROVISIONING_ENABLED") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("invalid_provisioning_configuration"),
    };
    let config = config
        .with_provisioning(provisioning.as_deref())
        .map_err(|_| "invalid_provisioning_configuration")?;
    let snapshots = match std::env::var("VMANYWARE_SNAPSHOTS_ENABLED") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("invalid_snapshot_configuration"),
    };
    let config = config
        .with_snapshots(snapshots.as_deref())
        .map_err(|_| "invalid_snapshot_configuration")?;
    let restores = match std::env::var("VMANYWARE_RESTORES_ENABLED") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("invalid_restore_configuration"),
    };
    let config = config
        .with_restores(restores.as_deref())
        .map_err(|_| "invalid_restore_configuration")?;
    let deletions = match std::env::var("VMANYWARE_DELETIONS_ENABLED") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("invalid_deletion_configuration"),
    };
    let config = config
        .with_deletions(deletions.as_deref())
        .map_err(|_| "invalid_deletion_configuration")?;
    let access = vmanyware_server::auth::AccessToken::from_environment()
        .map_err(|_| "invalid_access_configuration")?;
    let provider = Proxmox::from_environment().map_err(|_| "invalid_provider_configuration")?;
    let catalog = Catalog::from_environment().map_err(|_| "invalid_template_configuration")?;
    let database = match std::env::var("VMANYWARE_DATABASE") {
        Ok(value) if !value.is_empty() => value,
        Err(std::env::VarError::NotPresent) => "vmanyware.sqlite".into(),
        _ => return Err("invalid_database_configuration"),
    };
    let _database_lock =
        vmanyware_server::service_lock::ServiceLock::acquire(std::path::Path::new(&database))
            .map_err(|_| "database_lock_unavailable")?;
    let store = Store::open(database).map_err(|_| "database_unavailable")?;
    let shutdown = Shutdown::register()?;
    let listener = tokio::net::TcpListener::bind(config.address())
        .await
        .map_err(|_| "listener_unavailable")?;
    store
        .recover_interrupted_operations()
        .map_err(|_| "operation_recovery_failed")?;
    let mut worker = tokio::spawn(vmanyware_server::worker::run_with_deletions(
        store.clone(),
        provider.clone(),
        catalog.clone(),
        config.lifecycle_enabled(),
        config.provisioning_enabled(),
        config.snapshots_enabled(),
        config.restores_enabled(),
        config.deletions_enabled(),
    ));
    let (stopping, stopped_signal) = tokio::sync::oneshot::channel();
    let server = axum::serve(
        listener,
        vmanyware_server::request_limits::protect(vmanyware_server::auth::protect(
            router_with_deletions(
                provider,
                store,
                catalog,
                config.lifecycle_enabled(),
                config.provisioning_enabled(),
                config.snapshots_enabled(),
                config.restores_enabled(),
                config.deletions_enabled(),
            ),
            access,
        )),
    )
    .with_graceful_shutdown(async move {
        shutdown.wait().await;
        let _ = stopping.send(());
    })
    .into_future();
    tokio::pin!(server);
    diagnostics::listening(config.address().port());
    tokio::select! {
        result = &mut server => {
            worker.abort();
            result.map_err(|_| "server_stopped_unexpectedly")?;
        }
        _ = stopped_signal => {
            match tokio::time::timeout(DRAIN_LIMIT, &mut server).await {
                Ok(result) => result.map_err(|_| "server_stopped_unexpectedly")?,
                Err(_) => diagnostics::drain_timed_out(),
            }
            worker.abort();
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
