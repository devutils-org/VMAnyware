//! Read-only cluster dependencies. Empty permission-filtered lists need visibility proof.
use super::{Error, Proxmox, VmReference};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct DeletionManagement {
    pub ha_managed: bool,
    pub replication_jobs: Vec<String>,
}
fn vmid(value: &str) -> Option<u32> {
    let id = value.parse::<u32>().ok()?;
    ((100..=999999999).contains(&id) && id.to_string() == value).then_some(id)
}
#[derive(Deserialize)]
struct Ha {
    sid: String,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Guest {
    Number(u32),
    Text(String),
}
#[derive(Deserialize)]
struct Replication {
    id: String,
    guest: Guest,
}
impl Proxmox {
    async fn deletion_audit_visibility(&self, target: &VmReference) -> Result<(), Error> {
        let path = format!("/vms/{}", target.vmid());
        let permissions: Value = self
            .get_query(&["access", "permissions"], &[("path", path.clone())])
            .await?;
        let permissions = permissions.as_object().ok_or(Error::InvalidResponse)?;
        let Some(privileges) = permissions.get(&path) else {
            return Err(Error::Forbidden);
        };
        let privileges = privileges.as_object().ok_or(Error::InvalidResponse)?;
        let Some(value) = privileges.get("VM.Audit") else {
            return Err(Error::Forbidden);
        };
        // The value denotes propagation, not whether the privilege is granted.
        match value {
            Value::Bool(_) => Ok(()),
            value if matches!(value.as_u64(), Some(0 | 1)) => Ok(()),
            _ => Err(Error::InvalidResponse),
        }
    }
    pub(crate) async fn deletion_management(
        &self,
        target: &VmReference,
    ) -> Result<DeletionManagement, Error> {
        self.deletion_audit_visibility(target).await?;
        let resources: Vec<Ha> = self.get(&["cluster", "ha", "resources"]).await?;
        if resources.len() > 4096 {
            return Err(Error::ResponseTooLarge);
        }
        let mut seen = BTreeSet::new();
        let mut ha_managed = false;
        for resource in resources {
            let (kind, id) = resource.sid.split_once(':').ok_or(Error::InvalidResponse)?;
            if !matches!(kind, "vm" | "ct")
                || vmid(id).is_none()
                || !seen.insert(resource.sid.clone())
            {
                return Err(Error::InvalidResponse);
            }
            ha_managed |= kind == "vm" && vmid(id) == Some(target.vmid());
        }
        let jobs: Vec<Replication> = self.get(&["cluster", "replication"]).await?;
        if jobs.len() > 4096 {
            return Err(Error::ResponseTooLarge);
        }
        let mut seen = BTreeSet::new();
        let mut replication_jobs = BTreeSet::new();
        for job in jobs {
            let guest = match job.guest {
                Guest::Number(id) if (100..=999999999).contains(&id) => id,
                Guest::Text(value) => vmid(&value).ok_or(Error::InvalidResponse)?,
                _ => return Err(Error::InvalidResponse),
            };
            let (id, index) = job.id.split_once('-').ok_or(Error::InvalidResponse)?;
            let number = index.parse::<u32>().map_err(|_| Error::InvalidResponse)?;
            if job.id.len() > 64
                || vmid(id) != Some(guest)
                || number.to_string() != index
                || !seen.insert(job.id.clone())
            {
                return Err(Error::InvalidResponse);
            }
            if guest == target.vmid() {
                replication_jobs.insert(job.id);
            }
        }
        self.deletion_audit_visibility(target).await?;
        Ok(DeletionManagement {
            ha_managed,
            replication_jobs: replication_jobs.into_iter().collect(),
        })
    }
}
