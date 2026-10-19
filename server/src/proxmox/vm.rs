//! Point-in-time provider observations, independent of guest readiness and ownership.
use super::{Error, Proxmox};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmReference {
    node: String,
    vmid: u32,
}

impl VmReference {
    pub(crate) fn node(&self) -> &str {
        &self.node
    }
    pub(crate) fn vmid(&self) -> u32 {
        self.vmid
    }

    pub fn new(node: &str, vmid: u32) -> Result<Self, Error> {
        if node.is_empty()
            || node.len() > 63
            || !node.as_bytes()[0].is_ascii_alphanumeric()
            || !node
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || !(100..=999_999_999).contains(&vmid)
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            node: node.into(),
            vmid,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerState {
    Running,
    Stopped,
}

/// Unrecognized QEMU states remain unknown; never infer execution from power alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QmpState {
    Running,
    Stopped,
    Paused,
    Prelaunch,
    Shutdown,
    Suspended,
    GuestPanicked,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VmObservation {
    pub node: String,
    pub vmid: u32,
    pub status: PowerState,
    pub qmp_status: Option<QmpState>,
}

#[derive(Deserialize)]
struct ProviderStatus {
    vmid: u32,
    status: PowerState,
    qmpstatus: Option<QmpState>,
}

impl Proxmox {
    pub async fn vm_status(&self, target: &VmReference) -> Result<VmObservation, Error> {
        let response: ProviderStatus = self
            .get(&[
                "nodes",
                &target.node,
                "qemu",
                &target.vmid.to_string(),
                "status",
                "current",
            ])
            .await?;
        if response.vmid != target.vmid {
            return Err(Error::InvalidResponse);
        }
        Ok(VmObservation {
            node: target.node.clone(),
            vmid: target.vmid,
            status: response.status,
            qmp_status: response.qmpstatus,
        })
    }
}
