//! Sanitized configuration metadata for read-only lifecycle preconditions.
use super::{Error, Proxmox, VmReference};
use serde_json::Value;
use std::collections::HashSet;

pub(crate) struct Identity {
    pub digest: String,
    pub instance_marker: bool,
    pub template: bool,
    pub locked: bool,
}

impl Proxmox {
    pub(crate) async fn vm_identity(
        &self,
        target: &VmReference,
        expected_marker: &str,
    ) -> Result<Identity, Error> {
        let value = self.template_config(target.node(), target.vmid()).await?;
        let config = value.as_object().ok_or(Error::InvalidResponse)?;
        let digest = config
            .get("digest")
            .and_then(Value::as_str)
            .filter(|value| value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or(Error::InvalidResponse)?
            .to_string();
        decode_identity(&value, expected_marker, digest)
    }

    pub(crate) async fn vm_has_pending_changes(&self, target: &VmReference) -> Result<bool, Error> {
        let entries: Vec<Value> = self
            .get(&[
                "nodes",
                target.node(),
                "qemu",
                &target.vmid().to_string(),
                "pending",
            ])
            .await?;
        if entries.len() > 512 {
            return Err(Error::ResponseTooLarge);
        }
        let mut keys = HashSet::new();
        let mut pending = false;
        for entry in entries {
            let entry = entry.as_object().ok_or(Error::InvalidResponse)?;
            let key = entry
                .get("key")
                .and_then(Value::as_str)
                .filter(|key| !key.is_empty() && key.len() <= 128)
                .ok_or(Error::InvalidResponse)?;
            if !keys.insert(key.to_string()) {
                return Err(Error::InvalidResponse);
            }
            if let Some(value) = entry.get("pending") {
                if !value.is_string() {
                    return Err(Error::InvalidResponse);
                }
                pending = true;
            }
            if let Some(value) = entry.get("delete") {
                let value = value
                    .as_u64()
                    .filter(|value| *value <= 2)
                    .ok_or(Error::InvalidResponse)?;
                pending |= value != 0;
            }
        }
        Ok(pending)
    }
}

pub(super) fn decode_identity(
    value: &Value,
    expected_marker: &str,
    digest: String,
) -> Result<Identity, Error> {
    let config = value.as_object().ok_or(Error::InvalidResponse)?;
    let template = match config.get("template") {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(value) if value.as_u64() == Some(0) => false,
        Some(value) if value.as_u64() == Some(1) => true,
        _ => return Err(Error::InvalidResponse),
    };
    let locked = match config.get("lock") {
        None => false,
        Some(Value::String(value)) if !value.is_empty() => true,
        _ => return Err(Error::InvalidResponse),
    };
    let tags = match config.get("tags") {
        None => "",
        Some(Value::String(value)) if value.len() <= 4096 => value,
        _ => return Err(Error::InvalidResponse),
    };
    if !tags.is_empty()
        && tags
            .split(';')
            .any(|tag| tag.is_empty() || tag.trim() != tag)
    {
        return Err(Error::InvalidResponse);
    }
    let markers: Vec<_> = tags
        .split(';')
        .filter(|tag| tag.to_ascii_lowercase().starts_with("vmanyware-"))
        .collect();
    let instance_marker = markers.len() == 1 && markers[0] == expected_marker;
    Ok(Identity {
        digest,
        instance_marker,
        template,
        locked,
    })
}
