//! Fixed clone and stopped-target CPU/memory transport; no public creation route.
use super::{header, Error, PowerState, Proxmox, TaskReference, VmObservation, VmReference};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct CloneRequest {
    source: VmReference,
    pub(super) target: VmReference,
    pub(super) storage: String,
    name: String,
    pub(super) marker: String,
    tag: String,
}
impl CloneRequest {
    pub(crate) fn new(
        source: VmReference,
        target: VmReference,
        storage: &str,
        name: &str,
        instance_id: &str,
        definition_hash: &str,
        plan_hash: &str,
    ) -> Result<Self, Error> {
        let hex = |value: &str, length| {
            value.len() == length
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if source.node() != target.node()
            || source.vmid() == target.vmid()
            || storage.is_empty()
            || storage.len() > 64
            || !storage.as_bytes()[0].is_ascii_alphanumeric()
            || !storage
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || name.is_empty()
            || name.len() > 63
            || name.starts_with('-')
            || name.ends_with('-')
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !hex(instance_id, 32)
            || !hex(definition_hash, 64)
            || !hex(plan_hash, 64)
        {
            return Err(Error::Configuration);
        }
        Ok(Self { source, target, storage: storage.into(), name: name.into(), tag: format!("vmanyware-{instance_id}"),
            marker: format!("VMAnyware instance {instance_id}\ndefinition-sha256: {definition_hash}\nplan-sha256: {plan_hash}") })
    }
}
#[derive(Debug, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneObservation {
    pub config_digest: String,
    pub observation: VmObservation,
}
impl Proxmox {
    /// Call only after confirmed intent, target exclusion and dispatch marker commit.
    /// Ambiguous replies never authorize another POST. UPID binds the source VMID.
    pub async fn dispatch_full_clone(
        &self,
        request: &CloneRequest,
    ) -> Result<TaskReference, Error> {
        let body = form_urlencoded::Serializer::new(String::new())
            .append_pair("newid", &request.target.vmid().to_string())
            .append_pair("name", &request.name)
            .append_pair("storage", &request.storage)
            .append_pair("full", "1")
            .append_pair("description", &request.marker)
            .finish();
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .map_err(|_| Error::Configuration)?
            .extend([
                "api2",
                "json",
                "nodes",
                request.source.node(),
                "qemu",
                &request.source.vmid().to_string(),
                "clone",
            ]);
        let response = self
            .client
            .post(url)
            .header(header::AUTHORIZATION, self.authorization.clone())
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(super::transport_error)?;
        let upid: String = Self::decode_response(response).await?;
        let task =
            TaskReference::new(request.source.node(), &upid).map_err(|_| Error::InvalidResponse)?;
        if !task.matches_clone(&request.source) {
            return Err(Error::InvalidResponse);
        }
        Ok(task)
    }

    /// Post-task target evidence; never reports resources, Cloud-Init or readiness complete.
    pub async fn observe_full_clone(
        &self,
        request: &CloneRequest,
    ) -> Result<CloneObservation, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let config = self
                .template_config(request.target.node(), request.target.vmid())
                .await?;
            let config = config.as_object().ok_or(Error::InvalidResponse)?;
            let digest = config
                .get("digest")
                .and_then(|v| v.as_str())
                .filter(|v| v.len() == 40 && v.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or(Error::InvalidResponse)?;
            let template = config.get("template");
            if config.get("description").and_then(|v| v.as_str()) != Some(request.marker.as_str())
                || config.get("name").and_then(|v| v.as_str()) != Some(request.name.as_str())
                || config.contains_key("lock")
                || !matches!(template, None | Some(serde_json::Value::Bool(false)))
                    && template.and_then(|v| v.as_u64()) != Some(0)
            {
                return Err(Error::InvalidResponse);
            }
            let observation = self.vm_status(&request.target).await?;
            if observation.status != PowerState::Stopped
                || !matches!(
                    observation.qmp_status,
                    None | Some(super::QmpState::Stopped)
                )
            {
                return Err(Error::InvalidResponse);
            }
            Ok(CloneObservation {
                config_digest: digest.to_owned(),
                observation,
            })
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}

/// Fixed CPU/memory settings for an exact clone; disk/network/Cloud-Init are separate.
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    clone: CloneRequest,
    digest: String,
    cpu: u16,
    memory_mib: u32,
}

impl ResourceRequest {
    pub(crate) fn new(
        clone: CloneRequest,
        digest: &str,
        cpu: u16,
        memory_mib: u32,
    ) -> Result<Self, Error> {
        if digest.len() != 40
            || !digest.bytes().all(|b| b.is_ascii_hexdigit())
            || !(1..=64).contains(&cpu)
            || !(256..=262_144).contains(&memory_mib)
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            clone,
            digest: digest.into(),
            cpu,
            memory_mib,
        })
    }
}

#[derive(Debug, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceObservation {
    pub target: CloneObservation,
    pub cpu: u16,
    pub memory_mib: u32,
}

// Config fields can retain their decimal form from the provider's text config.
// Accept only exact unsigned integers, never expressions, signs or whitespace.
fn config_integer(value: &serde_json::Value) -> Option<u64> {
    match value {
        serde_json::Value::String(text) => text
            .parse::<u64>()
            .ok()
            .filter(|number| number.to_string() == *text),
        _ => value.as_u64(),
    }
}

impl Proxmox {
    /// Call only after the resource stage's durable dispatch marker. No retry is safe
    /// from an ambiguous reply; reconcile exact applied values by GET instead.
    pub async fn dispatch_resources(&self, request: &ResourceRequest) -> Result<(), Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let target = self.observe_full_clone(&request.clone).await?;
            if target.config_digest != request.digest
                || self.vm_has_pending_changes(&request.clone.target).await?
            {
                return Err(Error::InvalidResponse);
            }
            let body = form_urlencoded::Serializer::new(String::new())
                .append_pair("digest", &request.digest)
                .append_pair("sockets", "1")
                .append_pair("cores", &request.cpu.to_string())
                .append_pair("vcpus", &request.cpu.to_string())
                .append_pair("cpulimit", "0")
                .append_pair("memory", &request.memory_mib.to_string())
                .append_pair("balloon", "0")
                .append_pair("tags", &request.clone.tag)
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
                    "config",
                ]);
            let response = self
                .client
                .put(url)
                .header(header::AUTHORIZATION, self.authorization.clone())
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(body)
                .send()
                .await
                .map_err(super::transport_error)?;
            let result: serde_json::Value = Self::decode_response(response).await?;
            if !result.is_null() {
                return Err(Error::InvalidResponse);
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::Timeout)?
    }

    /// Applied CPU/memory evidence only; does not imply complete provisioning.
    pub async fn observe_resources(
        &self,
        request: &ResourceRequest,
    ) -> Result<ResourceObservation, Error> {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            // Bind the separately read settings to the exact observed config revision.
            let config = self
                .template_config(request.clone.target.node(), request.clone.target.vmid())
                .await?;
            let target = self.observe_full_clone(&request.clone).await?;
            if config.get("digest").and_then(|v| v.as_str()) != Some(target.config_digest.as_str())
                || config.get("tags").and_then(|v| v.as_str()) != Some(request.clone.tag.as_str())
                || config.get("sockets").and_then(config_integer) != Some(1)
                || config.get("cores").and_then(config_integer) != Some(u64::from(request.cpu))
                || config.get("vcpus").and_then(config_integer) != Some(u64::from(request.cpu))
                || !config
                    .get("cpulimit")
                    .is_some_and(|value| value.as_f64() == Some(0.0) || value.as_str() == Some("0"))
                || config.get("memory").and_then(config_integer)
                    != Some(u64::from(request.memory_mib))
                || config.get("balloon").and_then(config_integer) != Some(0)
                || self.vm_has_pending_changes(&request.clone.target).await?
            {
                return Err(Error::InvalidResponse);
            }
            Ok(ResourceObservation {
                target,
                cpu: request.cpu,
                memory_mib: request.memory_mib,
            })
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
