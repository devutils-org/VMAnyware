//! Fixed passwordless NoCloud settings; never first-boot or guest readiness proof.
mod seed;
pub use seed::{SeedPreflight, SeedUserFormat};

use super::{Error, Proxmox, ResourceObservation, ResourceRequest, VmReference};
use crate::{
    definition::Definition,
    provisioning::{DiskDevice, Plan},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt::Write};

pub struct CloudInitRequest {
    target: VmReference,
    resources: ResourceRequest,
    digest: String,
    volume: String,
    device: &'static str,
    disk_gib: u32,
    source_network: String,
    network: String,
    user: String,
    image_default_user: bool,
    name: String,
    key_count: usize,
    encoded_keys: String,
    storage: String,
}
impl std::fmt::Debug for CloudInitRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudInitRequest")
            .field("target", &self.target)
            .field("keys", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}
impl CloudInitRequest {
    pub(crate) fn expected_network(&self) -> &str {
        &self.network
    }

    pub(crate) fn new(
        plan: &Plan,
        definition: &Definition,
        instance: &str,
        digest: &str,
        volume: &str,
        source_network: &str,
    ) -> Result<Self, Error> {
        if definition.hash().map_err(|_| Error::Configuration)? != plan.definition_hash
            || definition.metadata.name != plan.definition_name
        {
            return Err(Error::Configuration);
        }
        let (storage, name) = volume.split_once(':').ok_or(Error::Configuration)?;
        if storage != plan.target.storage
            || name.is_empty()
            || volume.len() > 256
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
            || name
                .split('/')
                .any(|p| p.is_empty() || matches!(p, "." | ".."))
        {
            return Err(Error::Configuration);
        }
        let (mac, firewall, _) = network_parts(source_network).ok_or(Error::Configuration)?;
        let network = format!(
            "virtio={mac},bridge={}{}",
            plan.target.bridge,
            firewall
                .map(|f| format!(",firewall={}", u8::from(f)))
                .unwrap_or_default()
        );
        let mut keys = Vec::new();
        for key in &definition.spec.provisioning.ssh_authorized_keys {
            let mut key =
                ssh_key::PublicKey::from_openssh(key).map_err(|_| Error::Configuration)?;
            key.set_comment("");
            keys.push(key.to_openssh().map_err(|_| Error::Configuration)?);
        }
        let key_count = keys.len();
        let keys = keys.join("\n");
        let encoded_keys = uri_encode(&keys);
        Ok(Self {
            target: VmReference::new(&plan.target.node, plan.target.vmid)?,
            resources: plan.resource_request(instance, digest)?,
            digest: digest.into(),
            volume: volume.into(),
            device: match plan.source.disk_device {
                DiskDevice::Scsi0 => "scsi0",
                DiskDevice::Virtio0 => "virtio0",
                DiskDevice::Sata0 => "sata0",
            },
            disk_gib: plan.target.resources.disk_gib,
            source_network: source_network.into(),
            network,
            user: plan.target.guest_user.clone(),
            image_default_user: plan.source.image_default_user.is_some(),
            name: plan.target.name.clone(),
            key_count,
            encoded_keys,
            storage: storage.into(),
        })
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitObservation {
    pub resources: ResourceObservation,
    pub primary_volume: String,
    pub disk_size_bytes: u64,
    pub network: String,
    pub guest_user: String,
    pub ssh_key_count: usize,
    pub guest_ready: bool,
}
fn uri_encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else {
            write!(&mut out, "%{b:02X}").expect("writing to a string cannot fail");
        }
    }
    out
}
pub(super) fn network_parts(value: &str) -> Option<(String, Option<bool>, String)> {
    let mut fields = value.split(',');
    let mac = fields.next()?.strip_prefix("virtio=")?;
    let bytes: Vec<_> = mac
        .split(':')
        .map(|b| {
            if b.len() == 2 {
                u8::from_str_radix(b, 16).ok()
            } else {
                None
            }
        })
        .collect::<Option<_>>()?;
    if bytes.len() != 6 || bytes[0] & 1 != 0 || bytes.iter().all(|b| *b == 0) {
        return None;
    }
    let mut keys = HashSet::new();
    let mut bridge = None;
    let mut firewall = None;
    for field in fields {
        let (key, value) = field.split_once('=')?;
        if !keys.insert(key) {
            return None;
        }
        match key {
            "bridge"
                if !value.is_empty()
                    && value.len() <= 15
                    && value.as_bytes()[0].is_ascii_alphanumeric()
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) =>
            {
                bridge = Some(value.to_owned())
            }
            "firewall" => {
                firewall = Some(match value {
                    "0" => false,
                    "1" => true,
                    _ => return None,
                })
            }
            _ => return None,
        }
    }
    Some((mac.to_ascii_uppercase(), firewall, bridge?))
}
fn cloud_init_drive(config: &serde_json::Value, request: &CloudInitRequest) -> bool {
    let Some(drive) = config.get("ide2").and_then(|v| v.as_str()) else {
        return false;
    };
    let mut fields = drive.split(',');
    let volume = fields.next().unwrap_or_default();
    let base = format!("{}:vm-{}-cloudinit", request.storage, request.target.vmid());
    let directory = format!(
        "{}:{}/vm-{}-cloudinit",
        request.storage,
        request.target.vmid(),
        request.target.vmid()
    );
    if volume != base
        && !["raw", "qcow2"]
            .iter()
            .any(|extension| volume == format!("{directory}.{extension}"))
    {
        return false;
    }
    let mut keys = HashSet::new();
    let mut media = false;
    for field in fields {
        let Some((key, value)) = field.split_once('=') else {
            return false;
        };
        if !keys.insert(key) {
            return false;
        }
        match (key, value) {
            ("media", "cdrom") => media = true,
            ("size", "4M") => {}
            _ => return false,
        }
    }
    media
}
fn extra_interfaces(config: &serde_json::Value) -> bool {
    config.as_object().is_none_or(|c| {
        c.keys().any(|key| {
            ["net", "ipconfig"].iter().any(|prefix| {
                key.strip_prefix(prefix).is_some_and(|suffix| {
                    !suffix.is_empty()
                        && suffix.bytes().all(|b| b.is_ascii_digit())
                        && suffix != "0"
                })
            })
        })
    })
}
impl Proxmox {
    async fn cloud_init_base(
        &self,
        request: &CloudInitRequest,
    ) -> Result<(serde_json::Value, ResourceObservation), Error> {
        let config = self
            .template_config(request.target.node(), request.target.vmid())
            .await?;
        let resources = self.observe_resources(&request.resources).await?;
        if config.get("digest").and_then(|v| v.as_str())
            != Some(resources.target.config_digest.as_str())
            || super::disk::volume_size(
                config
                    .get(request.device)
                    .and_then(|v| v.as_str())
                    .ok_or(Error::InvalidResponse)?,
                &request.volume,
            ) != Some(u64::from(request.disk_gib) * 1024_u64.pow(3))
            || !cloud_init_drive(&config, request)
            || extra_interfaces(&config)
        {
            return Err(Error::InvalidResponse);
        }
        Ok((config, resources))
    }
    pub(crate) async fn preflight_cloud_init(
        &self,
        request: &CloudInitRequest,
    ) -> Result<(), Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let (config, resources) = self.cloud_init_base(request).await?;
            if resources.target.config_digest != request.digest
                || config.get("net0").and_then(|v| v.as_str())
                    != Some(request.source_network.as_str())
            {
                return Err(Error::InvalidResponse);
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    /// Call after a dedicated durable stage marker; ambiguous replies must reconcile by GET.
    pub async fn dispatch_cloud_init(&self, request: &CloudInitRequest) -> Result<(), Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            self.preflight_cloud_init(request).await?;
            let body = {
                let mut form = form_urlencoded::Serializer::new(String::new());
                form.append_pair("digest", &request.digest)
                    .append_pair("net0", &request.network)
                    .append_pair("ipconfig0", "ip=dhcp")
                    .append_pair("citype", "nocloud")
                    .append_pair("ciupgrade", "0")
                    .append_pair("sshkeys", &request.encoded_keys)
                    .append_pair(
                        "delete",
                        if request.image_default_user {
                            "cicustom,cipassword,ciuser"
                        } else {
                            "cicustom,cipassword"
                        },
                    );
                if !request.image_default_user {
                    form.append_pair("ciuser", &request.user);
                }
                form.finish()
            };
            let mut url = self.endpoint.clone();
            url.path_segments_mut()
                .map_err(|_| Error::Configuration)?
                .extend([
                    "api2",
                    "json",
                    "nodes",
                    request.target.node(),
                    "qemu",
                    &request.target.vmid().to_string(),
                    "config",
                ]);
            let response = self
                .client
                .put(url)
                .header(super::header::AUTHORIZATION, self.authorization.clone())
                .header(
                    super::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(body)
                .send()
                .await
                .map_err(super::transport_error)?;
            let data: serde_json::Value = Self::decode_response(response).await?;
            if !data.is_null() {
                return Err(Error::InvalidResponse);
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    /// API settings only: does not inspect generated seed contents or prove guest boot compatibility.
    pub async fn observe_cloud_init(
        &self,
        request: &CloudInitRequest,
    ) -> Result<CloudInitObservation, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let (config, resources) = self.cloud_init_base(request).await?;
            if config
                .get("net0")
                .and_then(|v| v.as_str())
                .and_then(network_parts)
                != network_parts(&request.network)
                || config.get("ipconfig0").and_then(|v| v.as_str()) != Some("ip=dhcp")
                || config.get("citype").and_then(|v| v.as_str()) != Some("nocloud")
                || if request.image_default_user {
                    config.get("ciuser").is_some()
                } else {
                    config.get("ciuser").and_then(|v| v.as_str()) != Some(request.user.as_str())
                }
                || config.get("ciupgrade").and_then(|v| v.as_u64()) != Some(0)
                || config.get("sshkeys").and_then(|v| v.as_str())
                    != Some(request.encoded_keys.as_str())
                || config.get("cicustom").is_some()
                || config.get("cipassword").is_some()
            {
                return Err(Error::InvalidResponse);
            }
            Ok(CloudInitObservation {
                resources,
                primary_volume: request.volume.clone(),
                disk_size_bytes: u64::from(request.disk_gib) * 1024_u64.pow(3),
                network: request.network.clone(),
                guest_user: request.user.clone(),
                ssh_key_count: request.key_count,
                guest_ready: false,
            })
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
