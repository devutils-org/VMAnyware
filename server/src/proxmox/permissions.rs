//! Explicit VM-scoped effective privilege observation; never grants access.
use super::{Error, Proxmox};
use serde_json::Value;

impl Proxmox {
    /// Names of effective privileges at one VM path, not proof an operation will succeed.
    /// Proxmox privilege values indicate propagation: zero/false is still a grant.
    pub async fn vm_privileges(&self, vmid: u32) -> Result<Vec<String>, Error> {
        if !(100..=999999999).contains(&vmid) {
            return Err(Error::Configuration);
        }
        let path = format!("/vms/{vmid}");
        let response: Value = self
            .get_query(&["access", "permissions"], &[("path", path.clone())])
            .await?;
        let paths = response.as_object().ok_or(Error::InvalidResponse)?;
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        if paths.len() != 1 {
            return Err(Error::InvalidResponse);
        }
        let grants = paths
            .get(&path)
            .and_then(Value::as_object)
            .ok_or(Error::InvalidResponse)?;
        if grants.len() > 256 {
            return Err(Error::ResponseTooLarge);
        }
        let mut names = Vec::with_capacity(grants.len());
        for (name, propagation) in grants {
            if name.is_empty()
                || name.len() > 128
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
                || !(propagation.is_boolean() || matches!(propagation.as_u64(), Some(0 | 1)))
            {
                return Err(Error::InvalidResponse);
            }
            names.push(name.clone());
        }
        names.sort();
        Ok(names)
    }
}
