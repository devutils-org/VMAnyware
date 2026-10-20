//! Read-only snapshot metadata; neither listing nor an empty list proves support.
use super::{Error, Proxmox, VmReference};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub name: String,
    pub description: String,
    pub created_at: Option<u64>,
    pub includes_ram: Option<bool>,
    pub parent: Option<String>,
    /// Preserve incomplete provider state; its absence is not restore validation.
    pub provider_state: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Memory {
    Boolean(bool),
    Integer(u8),
}

#[derive(Deserialize)]
struct Entry {
    name: String,
    #[serde(default)]
    description: String,
    snaptime: Option<u64>,
    vmstate: Option<Memory>,
    parent: Option<String>,
    snapstate: Option<String>,
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}

impl Proxmox {
    pub async fn snapshots(&self, target: &VmReference) -> Result<Vec<Snapshot>, Error> {
        let entries: Vec<Entry> = self
            .get(&[
                "nodes",
                target.node(),
                "qemu",
                &target.vmid().to_string(),
                "snapshot",
            ])
            .await?;
        let mut names = HashSet::new();
        let mut snapshots = Vec::new();
        for entry in entries {
            if !valid_name(&entry.name) || !names.insert(entry.name.clone()) {
                return Err(Error::InvalidResponse);
            }
            // This synthetic entry describes the live VM, not a restorable snapshot.
            if entry.name == "current" {
                continue;
            }
            if entry.description.len() > 65536
                || entry
                    .parent
                    .as_deref()
                    .is_some_and(|name| !valid_name(name) || name == "current")
                || entry.snapstate.as_ref().is_some_and(|state| {
                    state.is_empty() || state.len() > 64 || state.chars().any(char::is_control)
                })
            {
                return Err(Error::InvalidResponse);
            }
            let includes_ram = match entry.vmstate {
                None => None,
                Some(Memory::Boolean(value)) => Some(value),
                Some(Memory::Integer(0)) => Some(false),
                Some(Memory::Integer(1)) => Some(true),
                _ => return Err(Error::InvalidResponse),
            };
            snapshots.push(Snapshot {
                name: entry.name,
                description: entry.description,
                created_at: entry.snaptime.filter(|time| *time != 0),
                includes_ram,
                parent: entry.parent,
                provider_state: entry.snapstate,
            });
        }
        snapshots.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(snapshots)
    }
}

impl Proxmox {
    /// The provider evaluates actual VM storage/configuration, not a local type guess.
    pub(crate) async fn snapshot_supported(&self, target: &VmReference) -> Result<bool, Error> {
        #[derive(Deserialize)]
        struct Feature {
            #[serde(rename = "hasFeature", deserialize_with = "super::deserialize_flag")]
            supported: u8,
        }
        let feature: Feature = self
            .get_query(
                &[
                    "nodes",
                    target.node(),
                    "qemu",
                    &target.vmid().to_string(),
                    "feature",
                ],
                &[("feature", "snapshot".into())],
            )
            .await?;
        Ok(feature.supported == 1)
    }
}

impl Proxmox {
    /// Fixed one-shot transport. Caller must durably mark dispatch before this POST.
    /// Transport errors cannot establish non-acceptance; never replay automatically.
    pub async fn dispatch_snapshot(
        &self,
        target: &VmReference,
        request: &crate::snapshot::CreateRequest,
    ) -> Result<super::TaskReference, Error> {
        if !request.is_valid() {
            return Err(Error::Configuration);
        }
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .map_err(|_| Error::Configuration)?
            .extend([
                "api2",
                "json",
                "nodes",
                target.node(),
                "qemu",
                &target.vmid().to_string(),
                "snapshot",
            ]);
        let body = form_urlencoded::Serializer::new(String::new())
            .append_pair("snapname", &request.name)
            .append_pair("description", &request.description)
            .append_pair("vmstate", "0")
            .finish();
        let response = self
            .client
            .post(url)
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
        let task =
            super::TaskReference::new(target.node(), &upid).map_err(|_| Error::InvalidResponse)?;
        if !task.matches_snapshot(target) {
            return Err(Error::InvalidResponse);
        }
        Ok(task)
    }
}

pub(crate) struct RestoreSnapshotIdentity {
    pub identity: super::identity::Identity,
    pub disk_only: bool,
    pub complete: bool,
}
// Explicit recursive ordering keeps the hash stable if serde_json preserve_order
// is enabled by another workspace consumer. Values are never logged/returned.
fn canonical(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let ordered: std::collections::BTreeMap<_, _> = map.iter().collect();
            serde_json::Value::Object(
                ordered
                    .into_iter()
                    .map(|(key, value)| (key.clone(), canonical(value)))
                    .collect(),
            )
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(canonical).collect())
        }
        value => value.clone(),
    }
}
impl Proxmox {
    pub(crate) async fn restore_snapshot_identity(
        &self,
        target: &VmReference,
        name: &str,
        marker: &str,
    ) -> Result<RestoreSnapshotIdentity, Error> {
        if !crate::snapshot::valid_name(name) {
            return Err(Error::Configuration);
        }
        let value: serde_json::Value = self
            .get(&[
                "nodes",
                target.node(),
                "qemu",
                &target.vmid().to_string(),
                "snapshot",
                name,
                "config",
            ])
            .await?;
        let config = value.as_object().ok_or(Error::InvalidResponse)?;
        let disk_only = match config.get("vmstate") {
            None => true,
            Some(serde_json::Value::String(volume)) if !volume.is_empty() => false,
            _ => return Err(Error::InvalidResponse),
        };
        let complete = match config.get("snapstate") {
            None => true,
            Some(serde_json::Value::String(state)) if !state.is_empty() => false,
            _ => return Err(Error::InvalidResponse),
        };
        use sha2::{Digest, Sha256};
        let mut material = b"vmanyware-restore-snapshot-config-v1\0".to_vec();
        material.extend(name.as_bytes());
        material.push(0);
        material
            .extend(serde_json::to_vec(&canonical(&value)).map_err(|_| Error::InvalidResponse)?);
        let hash = format!("{:x}", Sha256::digest(material));
        let identity = super::identity::decode_identity(&value, marker, hash)?;
        Ok(RestoreSnapshotIdentity {
            identity,
            disk_only,
            complete,
        })
    }
    pub(crate) async fn snapshot_support_at(
        &self,
        target: &VmReference,
        name: &str,
    ) -> Result<bool, Error> {
        if !crate::snapshot::valid_name(name) {
            return Err(Error::Configuration);
        }
        #[derive(Deserialize)]
        struct Feature {
            #[serde(rename = "hasFeature", deserialize_with = "super::deserialize_flag")]
            supported: u8,
        }
        let feature: Feature = self
            .get_query(
                &[
                    "nodes",
                    target.node(),
                    "qemu",
                    &target.vmid().to_string(),
                    "feature",
                ],
                &[("feature", "snapshot".into()), ("snapname", name.into())],
            )
            .await?;
        Ok(feature.supported == 1)
    }
}

impl Proxmox {
    /// One fixed rollback POST. The executor must persist its dispatch marker first.
    /// RAM snapshots must be excluded by revalidation: start=0 alone cannot prevent
    /// upstream RAM-state rollback from starting a VM.
    pub async fn dispatch_restore(
        &self,
        target: &VmReference,
        request: &crate::snapshot_restore::Request,
    ) -> Result<super::TaskReference, Error> {
        if !request.is_valid() {
            return Err(Error::Configuration);
        }
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .map_err(|_| Error::Configuration)?
            .extend([
                "api2",
                "json",
                "nodes",
                target.node(),
                "qemu",
                &target.vmid().to_string(),
                "snapshot",
                &request.snapshot_name,
                "rollback",
            ]);
        let response = self
            .client
            .post(url)
            .header(super::header::AUTHORIZATION, self.authorization.clone())
            .header(
                super::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body("start=0")
            .send()
            .await
            .map_err(super::transport_error)?;
        let upid: String = Self::decode_response(response).await?;
        let task =
            super::TaskReference::new(target.node(), &upid).map_err(|_| Error::InvalidResponse)?;
        if !task.matches_restore(target) {
            return Err(Error::InvalidResponse);
        }
        Ok(task)
    }
}
