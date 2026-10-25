//! Explicitly opt-in, read-only live probe. Never used by default tests.
use vmanyware_server::proxmox::Proxmox;

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("Missing or invalid {name}."))
}

async fn probe() -> Result<(), String> {
    if std::env::var("VMANYWARE_PROXMOX_PROBE").as_deref() != Ok("read-only") {
        return Err(
            "Set VMANYWARE_PROXMOX_PROBE=read-only to explicitly enable the live probe.".into(),
        );
    }
    let permission_vmid = match std::env::var("VMANYWARE_PROXMOX_PROBE_VMID") {
        Ok(value) => Some(
            value
                .parse::<u32>()
                .ok()
                .filter(|id| (100..=999999999).contains(id) && id.to_string() == value)
                .ok_or("Invalid VMANYWARE_PROXMOX_PROBE_VMID; use a canonical VMID.")?,
        ),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("Invalid VMANYWARE_PROXMOX_PROBE_VMID.".into()),
    };
    let ca = match std::env::var("VMANYWARE_PROXMOX_CA") {
        Ok(path) => Some(std::fs::read(path).map_err(|_| "Cannot read the configured CA file.")?),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err("Invalid CA file configuration.".into()),
    };
    let client = Proxmox::new(
        &required("VMANYWARE_PROXMOX_URL")?,
        &required("VMANYWARE_PROXMOX_TOKEN_ID")?,
        &required("VMANYWARE_PROXMOX_TOKEN_SECRET")?,
        ca.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    if let Some(vmid) = permission_vmid {
        let privileges = client
            .vm_privileges(vmid)
            .await
            .map_err(|e| e.to_string())?;
        println!(
            "Effective privileges at /vms/{vmid}: {}",
            if privileges.is_empty() {
                "none".to_owned()
            } else {
                privileges.join(", ")
            }
        );
    }
    let version = client.version().await.map_err(|e| e.to_string())?;
    println!(
        "Proxmox version {} (release {})",
        version.version, version.release
    );
    let nodes = client.nodes().await.map_err(|e| e.to_string())?;
    for node in nodes {
        println!("Node {}: {}", node.node, node.status);
        let vms = client
            .virtual_machines(&node.node)
            .await
            .map_err(|e| e.to_string())?;
        for vm in vms.into_iter().filter(|vm| vm.template == 1) {
            println!(
                "Template {}: {}",
                vm.vmid,
                vm.name.as_deref().unwrap_or("unnamed")
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match probe().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            std::process::ExitCode::FAILURE
        }
    }
}
