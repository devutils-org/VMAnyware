//! Fixed read-only guest command and strict clean Cloud-Init/network observation.
use super::{CloneRequest, Error, PowerState, Proxmox, QmpState};
use serde::{Deserialize, Serialize};
use std::{net::Ipv4Addr, time::Duration};

#[derive(Debug)]
pub struct ReadinessRequest {
    clone: CloneRequest,
    mac: String,
    digest: String,
}
impl ReadinessRequest {
    pub(crate) fn new(clone: CloneRequest, network: &str, digest: &str) -> Result<Self, Error> {
        let mac = super::cloud_init::network_parts(network)
            .ok_or(Error::Configuration)?
            .0;
        if digest.len() != 40 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::Configuration);
        }
        Ok(Self {
            clone,
            mac,
            digest: digest.into(),
        })
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GuestReadiness {
    pub ipv4: Ipv4Addr,
    pub interface: String,
    pub cloud_init_exit_code: u8,
    pub cloud_init_clean: bool,
}
#[derive(Debug)]
pub enum GuestProbe {
    Pending { process_exited: bool },
    Ready(GuestReadiness),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Process {
    pid: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ProcessStatus {
    #[serde(deserialize_with = "super::deserialize_flag")]
    exited: u8,
    exitcode: Option<u32>,
    signal: Option<u32>,
    out_data: Option<String>,
    err_data: Option<String>,
    #[serde(default, deserialize_with = "super::deserialize_flag")]
    out_truncated: u8,
    #[serde(default, deserialize_with = "super::deserialize_flag")]
    err_truncated: u8,
}
#[derive(Deserialize)]
struct CloudStatus {
    status: String,
    extended_status: String,
    errors: Vec<serde_json::Value>,
    recoverable_errors: serde_json::Map<String, serde_json::Value>,
    #[serde(rename = "init-local")]
    init_local: Option<CloudStage>,
    init: Option<CloudStage>,
    #[serde(rename = "modules-config")]
    modules_config: Option<CloudStage>,
    #[serde(rename = "modules-final")]
    modules_final: Option<CloudStage>,
}
#[derive(Deserialize)]
struct CloudStage {
    errors: Vec<serde_json::Value>,
    recoverable_errors: serde_json::Map<String, serde_json::Value>,
    start: Option<f64>,
    finished: Option<f64>,
}
#[derive(Deserialize)]
struct Network {
    result: Vec<Interface>,
}
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Interface {
    name: String,
    hardware_address: Option<String>,
    #[serde(default)]
    ip_addresses: Vec<Address>,
}
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Address {
    ip_address: String,
    ip_address_type: String,
    prefix: u8,
}
fn usable(ip: Ipv4Addr) -> bool {
    let b = ip.octets();
    b[0] != 0
        && b[0] < 224
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_unspecified()
        && !ip.is_broadcast()
}
fn cloud_clean(status: &ProcessStatus) -> Result<bool, Error> {
    if status.signal.is_some()
        || status.out_truncated != 0
        || status.err_truncated != 0
        || status.err_data.as_ref().is_some_and(|s| !s.is_empty())
    {
        return Err(Error::InvalidResponse);
    }
    if status.exited == 0 {
        if status.exitcode.is_some() || status.out_data.is_some() {
            return Err(Error::InvalidResponse);
        }
        return Ok(false);
    }
    let output = status
        .out_data
        .as_ref()
        .filter(|s| s.len() <= 65536)
        .ok_or(Error::InvalidResponse)?;
    let data: CloudStatus = serde_json::from_str(output).map_err(|_| Error::InvalidResponse)?;
    if !data.errors.is_empty() || !data.recoverable_errors.is_empty() {
        return Err(Error::InvalidResponse);
    }
    let stages = [
        data.init_local.as_ref(),
        data.init.as_ref(),
        data.modules_config.as_ref(),
        data.modules_final.as_ref(),
    ];
    for stage in stages.into_iter().flatten() {
        if !stage.errors.is_empty() || !stage.recoverable_errors.is_empty() {
            return Err(Error::InvalidResponse);
        }
    }
    if ["running", "not started"].contains(&data.status.as_str())
        && data.extended_status == data.status
        && status.exitcode == Some(0)
    {
        return Ok(false);
    }
    if status.exitcode != Some(0) || data.status != "done" || data.extended_status != "done" {
        return Err(Error::InvalidResponse);
    }
    for stage in stages {
        let stage = stage.ok_or(Error::InvalidResponse)?;
        let start = stage.start.ok_or(Error::InvalidResponse)?;
        let finished = stage.finished.ok_or(Error::InvalidResponse)?;
        if !start.is_finite() || !finished.is_finite() || start <= 0.0 || finished < start {
            return Err(Error::InvalidResponse);
        }
    }
    Ok(true)
}
impl Proxmox {
    async fn readiness_guard(&self, request: &ReadinessRequest) -> Result<String, Error> {
        let target = &request.clone.target;
        let config = self.template_config(target.node(), target.vmid()).await?;
        let digest = config
            .get("digest")
            .and_then(|v| v.as_str())
            .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or(Error::InvalidResponse)?;
        if digest != request.digest
            || config.get("description").and_then(|v| v.as_str())
                != Some(request.clone.marker.as_str())
            || config.get("lock").is_some()
            || !matches!(
                config.get("template"),
                None | Some(serde_json::Value::Bool(false))
            ) && config.get("template").and_then(|v| v.as_u64()) != Some(0)
        {
            return Err(Error::InvalidResponse);
        }
        let power = self.vm_status(target).await?;
        if power.status != PowerState::Running
            || !matches!(power.qmp_status, None | Some(QmpState::Running))
            || self.vm_has_pending_changes(target).await?
        {
            return Err(Error::InvalidResponse);
        }
        Ok(digest.into())
    }
    /// A read-only agent availability check before committing an exec marker.
    pub async fn preflight_guest_probe(&self, request: &ReadinessRequest) -> Result<(), Error> {
        tokio::time::timeout(Duration::from_secs(20), async {
            self.readiness_guard(request).await?;
            let target = &request.clone.target;
            let network: Network = self
                .get(&[
                    "nodes",
                    target.node(),
                    "qemu",
                    &target.vmid().to_string(),
                    "agent",
                    "network-get-interfaces",
                ])
                .await?;
            if network.result.len() > 128 {
                return Err(Error::ResponseTooLarge);
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    /// One fixed non-waiting read-only command; caller must retain returned PID.
    /// No shell, input data, caller-supplied executable or mutable guest command.
    pub async fn dispatch_guest_probe(&self, request: &ReadinessRequest) -> Result<u32, Error> {
        tokio::time::timeout(Duration::from_secs(20),async {
            self.readiness_guard(request).await?;
            let target=&request.clone.target;
            let mut url=self.endpoint.clone();
            url.path_segments_mut().map_err(|_|Error::Configuration)?.extend(["api2","json","nodes",target.node(),"qemu",&target.vmid().to_string(),"agent","exec"]);
            let response=self.client.post(url).header(super::header::AUTHORIZATION,self.authorization.clone())
                .json(&serde_json::json!({"command":["/usr/bin/cloud-init","status","--format","json"]})).send().await.map_err(super::transport_error)?;
            let process:Process=Self::decode_response(response).await?;
            if process.pid==0 || process.pid>i32::MAX as u32 {return Err(Error::InvalidResponse);}
            Ok(process.pid)
        }).await.map_err(|_|Error::Timeout)?
    }
    /// One bounded observation. Pending is never ready; no raw guest output escapes.
    pub async fn observe_guest_probe(
        &self,
        request: &ReadinessRequest,
        pid: u32,
    ) -> Result<GuestProbe, Error> {
        if pid == 0 || pid > i32::MAX as u32 {
            return Err(Error::Configuration);
        }
        tokio::time::timeout(Duration::from_secs(20), async {
            let before = self.readiness_guard(request).await?;
            let target = &request.clone.target;
            let vmid = target.vmid().to_string();
            let status: ProcessStatus = self
                .get_query(
                    &[
                        "nodes",
                        target.node(),
                        "qemu",
                        &vmid,
                        "agent",
                        "exec-status",
                    ],
                    &[("pid", pid.to_string())],
                )
                .await?;
            if !cloud_clean(&status)? {
                return Ok(GuestProbe::Pending {
                    process_exited: status.exited != 0,
                });
            }
            let network: Network = self
                .get(&[
                    "nodes",
                    target.node(),
                    "qemu",
                    &vmid,
                    "agent",
                    "network-get-interfaces",
                ])
                .await?;
            if network.result.len() > 128 {
                return Err(Error::ResponseTooLarge);
            }
            let mut selected = None;
            let mut matches = 0;
            for interface in network.result {
                if interface
                    .hardware_address
                    .as_ref()
                    .is_none_or(|mac| !mac.eq_ignore_ascii_case(&request.mac))
                {
                    continue;
                }
                matches += 1;
                if matches > 1
                    || interface.name.is_empty()
                    || interface.name.len() > 15
                    || !interface
                        .name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                    || interface.ip_addresses.len() > 128
                {
                    return Err(Error::InvalidResponse);
                }
                for address in interface.ip_addresses {
                    if address.ip_address_type != "ipv4" {
                        continue;
                    }
                    let ip = address
                        .ip_address
                        .parse::<Ipv4Addr>()
                        .map_err(|_| Error::InvalidResponse)?;
                    if address.prefix == 0 || address.prefix > 32 {
                        return Err(Error::InvalidResponse);
                    }
                    let host_mask = if address.prefix == 32 {
                        0
                    } else {
                        u32::MAX >> address.prefix
                    };
                    let host = u32::from(ip) & host_mask;
                    let subnet_address = address.prefix < 31 && (host == 0 || host == host_mask);
                    if usable(ip) && !subnet_address {
                        if selected.is_some() {
                            return Err(Error::InvalidResponse);
                        }
                        selected = Some(GuestReadiness {
                            ipv4: ip,
                            interface: interface.name.clone(),
                            cloud_init_exit_code: 0,
                            cloud_init_clean: true,
                        });
                    }
                }
            }
            let Some(proof) = selected else {
                return Ok(GuestProbe::Pending {
                    process_exited: true,
                });
            };
            if self.readiness_guard(request).await? != before {
                return Err(Error::InvalidResponse);
            }
            Ok(GuestProbe::Ready(proof))
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
