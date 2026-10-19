//! Fixed expansion of the retained primary volume; no guest filesystem claim.
use super::{CloneRequest, Error, Proxmox, ResourceObservation, ResourceRequest, TaskReference};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DiskRequest {
    clone: CloneRequest,
    resources: ResourceRequest,
    digest: String,
    device: &'static str,
    volume: String,
    source_gib: u32,
    target_gib: u32,
}
impl DiskRequest {
    pub(crate) fn new(
        clone: CloneRequest,
        resources: ResourceRequest,
        digest: &str,
        device: &'static str,
        volume: &str,
        source_gib: u32,
        target_gib: u32,
    ) -> Result<Self, Error> {
        let (storage, name) = volume.split_once(':').ok_or(Error::Configuration)?;
        if storage != clone.storage
            || name.is_empty()
            || volume.len() > 256
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
            || name
                .split('/')
                .any(|s| s.is_empty() || matches!(s, "." | ".."))
            || !matches!(device, "scsi0" | "virtio0" | "sata0")
            || digest.len() != 40
            || !digest.bytes().all(|b| b.is_ascii_hexdigit())
            || !(4..=4096).contains(&source_gib)
            || !(4..=4096).contains(&target_gib)
            || target_gib <= source_gib
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            clone,
            resources,
            digest: digest.into(),
            device,
            volume: volume.into(),
            source_gib,
            target_gib,
        })
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskObservation {
    pub resources: ResourceObservation,
    pub device: String,
    pub volume: String,
    pub size_bytes: u64,
}

// Deliberately accept only unambiguous integer sizes in the provider configuration.
pub(super) fn volume_size(value: &str, expected_volume: &str) -> Option<u64> {
    let mut fields = value.split(',');
    if fields.next()? != expected_volume {
        return None;
    }
    let mut size = None;
    let mut keys = std::collections::HashSet::new();
    for field in fields {
        let (key, value) = field.split_once('=')?;
        if key.is_empty() || value.is_empty() || !keys.insert(key) {
            return None;
        }
        if key == "media" && value != "disk" {
            return None;
        }
        if key == "size" {
            let (digits, multiplier) = match value.as_bytes().last()? {
                b'K' => (&value[..value.len() - 1], 1024_u64),
                b'M' => (&value[..value.len() - 1], 1024_u64.pow(2)),
                b'G' => (&value[..value.len() - 1], 1024_u64.pow(3)),
                b'T' => (&value[..value.len() - 1], 1024_u64.pow(4)),
                _ => (value, 1),
            };
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            size = Some(digits.parse::<u64>().ok()?.checked_mul(multiplier)?);
        }
    }
    size
}
impl Proxmox {
    /// Call after retaining volume/revision and a stage dispatch marker. Never replay
    /// an uncertain PUT, even though the requested size is absolute rather than relative.
    pub async fn dispatch_disk_expansion(
        &self,
        request: &DiskRequest,
    ) -> Result<TaskReference, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            self.preflight_disk_expansion(request).await?;
            let body = form_urlencoded::Serializer::new(String::new())
                .append_pair("disk", request.device)
                .append_pair("size", &format!("{}G", request.target_gib))
                .append_pair("digest", &request.digest)
                .finish();
            let mut url = self.endpoint.clone();
            url.path_segments_mut()
                .map_err(|_| Error::Configuration)?
                .extend([
                    "api2",
                    "json",
                    "nodes",
                    request.clone.target.node(),
                    "qemu",
                    &request.clone.target.vmid().to_string(),
                    "resize",
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
            let upid: String = Self::decode_response(response).await?;
            let task = TaskReference::new(request.clone.target.node(), &upid)
                .map_err(|_| Error::InvalidResponse)?;
            if !task.matches_resize(&request.clone.target) {
                return Err(Error::InvalidResponse);
            }
            Ok(task)
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    /// Read-only guard shared by preparation and the post-marker transport check.
    pub(crate) async fn preflight_disk_expansion(
        &self,
        request: &DiskRequest,
    ) -> Result<DiskObservation, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let observed = self.disk_observation(request).await?;
            if observed.resources.target.config_digest != request.digest
                || observed.size_bytes != u64::from(request.source_gib) * 1024_u64.pow(3)
            {
                return Err(Error::InvalidResponse);
            }
            Ok(observed)
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    /// Exact configured volume size after task success; no partition/filesystem evidence.
    pub async fn observe_disk_expansion(
        &self,
        request: &DiskRequest,
    ) -> Result<DiskObservation, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let observed = self.disk_observation(request).await?;
            if observed.size_bytes != u64::from(request.target_gib) * 1024_u64.pow(3) {
                return Err(Error::InvalidResponse);
            }
            Ok(observed)
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    async fn disk_observation(&self, request: &DiskRequest) -> Result<DiskObservation, Error> {
        let config = self
            .template_config(request.clone.target.node(), request.clone.target.vmid())
            .await?;
        let resources = self.observe_resources(&request.resources).await?;
        if config.get("digest").and_then(|v| v.as_str())
            != Some(resources.target.config_digest.as_str())
        {
            return Err(Error::InvalidResponse);
        }
        let size_bytes = config
            .get(request.device)
            .and_then(|v| v.as_str())
            .and_then(|v| volume_size(v, &request.volume))
            .ok_or(Error::InvalidResponse)?;
        Ok(DiskObservation {
            resources,
            device: request.device.into(),
            volume: request.volume.clone(),
            size_bytes,
        })
    }
}
