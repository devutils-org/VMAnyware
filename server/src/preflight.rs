//! Read-only observations, never reservation or permission to execute a plan.
use crate::{provisioning::Plan, proxmox::Proxmox};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    plan_hash: String,
    checks: Vec<Check>,
    creation_enabled: bool,
    vmid_reserved: bool,
}

#[derive(Serialize)]
pub struct Check {
    code: &'static str,
    status: &'static str,
    message: &'static str,
}

fn check(code: &'static str, result: Option<bool>, message: &'static str) -> Check {
    Check {
        code,
        status: match result {
            Some(true) => "passed",
            Some(false) => "failed",
            None => "unknown",
        },
        message,
    }
}
fn flag(value: &Value) -> Option<bool> {
    value.as_bool().or_else(|| match value.as_u64() {
        Some(0) => Some(false),
        Some(1) => Some(true),
        _ => None,
    })
}

pub async fn run(provider: &Proxmox, plan: Plan) -> Report {
    // One overall deadline bounds all observations, including partial provider failure.
    let observations = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(
            provider.template_config(&plan.source.node, plan.source.template_vmid),
            provider.storage(&plan.target.node),
            provider.networks(&plan.target.node),
            provider.available_vmid(plan.target.vmid),
        )
    })
    .await;
    let mut checks = Vec::new();
    match observations {
        Ok((config, storage, networks, vmid)) => {
            let config = config.ok();
            checks.push(check(
                "template",
                config
                    .as_ref()
                    .and_then(|v| v.get("template"))
                    .and_then(flag),
                "Source configuration must identify a template.",
            ));
            let disk_key = match plan.source.disk_device {
                crate::provisioning::DiskDevice::Scsi0 => "scsi0",
                crate::provisioning::DiskDevice::Virtio0 => "virtio0",
                crate::provisioning::DiskDevice::Sata0 => "sata0",
            };
            let disk = config
                .as_ref()
                .and_then(|v| v.get(disk_key))
                .and_then(Value::as_str);
            let size = disk
                .and_then(|v| v.split(',').find_map(|part| part.strip_prefix("size=")))
                .and_then(disk_bytes);
            checks.push(check("source_disk", size.map(|size| size == u64::from(plan.source.source_disk_gib) * 1024_u64.pow(3)), "Selected disk size must match the catalog; missing or unsupported size syntax is unknown."));
            let cloud_init = config.as_ref().and_then(Value::as_object).map(|v| {
                v.iter().any(|(key, value)| {
                    is_drive(key)
                        && value.as_str().is_some_and(|value| {
                            value.split(',').next().is_some_and(|volume| {
                                volume.split_once(':').is_some_and(|(_, name)| {
                                    cloud_init_volume(name, plan.source.template_vmid)
                                })
                            })
                        })
                })
            });
            checks.push(check("cloud_init_drive", cloud_init, "Template must contain a Cloud-Init drive; this does not verify guest Cloud-Init software."));
            let store = storage.as_ref().ok().and_then(|items| {
                items.iter().find(|item| {
                    item.get("storage").and_then(Value::as_str) == Some(&plan.target.storage)
                })
            });
            let storage_ready = store.and_then(|v| {
                Some(
                    flag(v.get("active")?)?
                        && flag(v.get("enabled")?)?
                        && v.get("content")?
                            .as_str()?
                            .split(',')
                            .any(|c| c == "images"),
                )
            });
            checks.push(check("storage", storage_ready, "Target storage must be visible, enabled, active and support VM images; invisible storage is unknown."));
            checks.push(check("storage_capacity", store.and_then(|v| v.get("avail")).and_then(Value::as_u64).map(|available| available >= u64::from(plan.target.resources.disk_gib) * 1024_u64.pow(3)), "Reported free space must cover the requested primary disk; ancillary disks and overhead are not included."));
            let bridge = networks.as_ref().ok().and_then(|items| {
                items.iter().find(|item| {
                    item.get("iface").and_then(Value::as_str) == Some(&plan.target.bridge)
                })
            });
            checks.push(check("bridge", bridge.and_then(|v| Some(flag(v.get("active")?)? && matches!(v.get("type")?.as_str()?, "bridge" | "OVSBridge"))), "Selected bridge must be visible and active; this does not verify guest connectivity."));
            checks.push(check("target_vmid", vmid.ok().map(|id| id == plan.target.vmid), "Proxmox must assert the requested VMID is free at observation time; API failures are unknown, and no reservation is made."));
        }
        Err(_) => checks.push(check(
            "provider_deadline",
            None,
            "Provider observations exceeded the overall deadline; retry the read-only check.",
        )),
    }
    checks.push(check("template_revision", None, "Catalog revision is an operator label; image identity and guest software have not been verified."));
    checks.push(check("execution_permissions", None, "Read access does not establish clone, configure, start or snapshot permissions or backend support."));
    checks.push(check("guest_readiness", None, "DHCP, SSH access, package installation and guest readiness require a real disposable VM test."));
    Report {
        plan_hash: plan.plan_hash,
        checks,
        creation_enabled: false,
        vmid_reserved: false,
    }
}

fn cloud_init_volume(name: &str, vmid: u32) -> bool {
    let file = name.rsplit('/').next().unwrap_or(name);
    let expected = format!("vm-{vmid}-cloudinit");
    file == expected
        || [".qcow2", ".raw"]
            .iter()
            .any(|extension| file.strip_suffix(extension) == Some(expected.as_str()))
}

fn is_drive(key: &str) -> bool {
    ["ide", "sata", "scsi", "virtio"].iter().any(|prefix| {
        key.strip_prefix(prefix)
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
    })
}
fn disk_bytes(value: &str) -> Option<u64> {
    let (digits, multiplier) = match value.as_bytes().last()? {
        b'K' => (&value[..value.len() - 1], 1024),
        b'M' => (&value[..value.len() - 1], 1024_u64.pow(2)),
        b'G' => (&value[..value.len() - 1], 1024_u64.pow(3)),
        b'T' => (&value[..value.len() - 1], 1024_u64.pow(4)),
        _ => (value, 1),
    };
    digits.parse::<u64>().ok()?.checked_mul(multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_parsers_are_conservative() {
        assert_eq!(disk_bytes("8192M"), Some(8 * 1024_u64.pow(3)));
        assert_eq!(disk_bytes("8G"), Some(8 * 1024_u64.pow(3)));
        assert_eq!(disk_bytes("8.5G"), None);
        assert_eq!(disk_bytes("18446744073709551615T"), None);
        assert!(is_drive("ide2"));
        assert!(!is_drive("description"));
        assert_eq!(flag(&serde_json::json!(2)), None);
    }
}
