use pocketlab_server::{config::Config, proxmox::Proxmox, router_with_services, store::Store};

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
    let database = match std::env::var("POCKETLAB_DATABASE") {
        Ok(value) if !value.is_empty() => value,
        Err(std::env::VarError::NotPresent) => "pocketlab.sqlite".into(),
        _ => return Err("Invalid POCKETLAB_DATABASE configuration".into()),
    };
    let store = Store::open(database)?;
    let listener = tokio::net::TcpListener::bind(config.address()).await?;
    eprintln!("PocketLab listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router_with_services(provider, store))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("Failed to register shutdown signal: {error}");
    }
}
