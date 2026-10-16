//! Deterministic offline planning. This module never contacts a provider.
use crate::definition::{Definition, Resources};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt, io::Read};

const MAX_CATALOG_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidCatalog,
    ImageNotMapped,
    InvalidVmid,
    DiskShrink,
    InvalidDefinition,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidCatalog => "Invalid template catalog; check the documented schema and required fields.",
            Self::ImageNotMapped => "No template is mapped to the definition's image.",
            Self::InvalidVmid => "Choose a target VMID between 100 and 999999999, different from the source template.",
            Self::DiskShrink => "Requested disk is smaller than the source template disk; shrinking is unsupported.",
            Self::InvalidDefinition => "Definition is invalid.",
        })
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Catalog {
    api_version: CatalogVersion,
    kind: CatalogKind,
    images: Vec<ImageMapping>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
enum CatalogVersion {
    #[serde(rename = "pocketlab.dev/v1")]
    V1,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
enum CatalogKind {
    TemplateCatalog,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImageMapping {
    image: String,
    node: String,
    template_vmid: u32,
    template_version: String,
    storage: String,
    bridge: String,
    disk_device: DiskDevice,
    #[serde(rename = "sourceDiskGiB")]
    source_disk_gib: u32,
    guest_user: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum DiskDevice {
    Scsi0,
    Virtio0,
    Sata0,
}

impl Catalog {
    pub fn parse(yaml: &str) -> Result<Self, Error> {
        if yaml.len() > MAX_CATALOG_BYTES {
            return Err(Error::InvalidCatalog);
        }
        let catalog: Self = serde_norway::from_str(yaml).map_err(|_| Error::InvalidCatalog)?;
        if catalog.images.len() > 128 {
            return Err(Error::InvalidCatalog);
        }
        let mut images = HashSet::new();
        for mapping in &catalog.images {
            if mapping.image.is_empty()
                || mapping.image.len() > 64
                || !mapping
                    .image
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-".contains(&b))
                || !images.insert(&mapping.image)
                || !identifier(&mapping.node, 63)
                || !identifier(&mapping.storage, 64)
                || !identifier(&mapping.bridge, 15)
                || !(100..=999_999_999).contains(&mapping.template_vmid)
                || !(4..=4096).contains(&mapping.source_disk_gib)
                || mapping.template_version.is_empty()
                || mapping.template_version.len() > 128
                || !mapping
                    .template_version
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
                || mapping.guest_user.is_empty()
                || mapping.guest_user.len() > 32
                || !mapping
                    .guest_user
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_lowercase() || b == b'_')
                || !mapping
                    .guest_user
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
            {
                return Err(Error::InvalidCatalog);
            }
        }
        Ok(catalog)
    }

    pub fn from_environment() -> Result<Option<Self>, Error> {
        let path = match std::env::var("POCKETLAB_TEMPLATES") {
            Ok(path) if !path.is_empty() => path,
            Err(std::env::VarError::NotPresent) => return Ok(None),
            _ => return Err(Error::InvalidCatalog),
        };
        let file = std::fs::File::open(path).map_err(|_| Error::InvalidCatalog)?;
        let mut yaml = String::new();
        file.take(MAX_CATALOG_BYTES as u64 + 1)
            .read_to_string(&mut yaml)
            .map_err(|_| Error::InvalidCatalog)?;
        Self::parse(&yaml).map(Some)
    }

    pub fn plan(&self, definition: &Definition, target_vmid: u32) -> Result<Plan, Error> {
        definition
            .validate()
            .map_err(|_| Error::InvalidDefinition)?;
        let mapping = self
            .images
            .iter()
            .find(|mapping| mapping.image == definition.spec.image)
            .ok_or(Error::ImageNotMapped)?;
        if !(100..=999_999_999).contains(&target_vmid) || target_vmid == mapping.template_vmid {
            return Err(Error::InvalidVmid);
        }
        if definition.spec.resources.disk_gib < mapping.source_disk_gib {
            return Err(Error::DiskShrink);
        }
        let definition_hash = definition.hash().map_err(|_| Error::InvalidDefinition)?;
        let material = serde_json::to_vec(&(&definition_hash, mapping, target_vmid))
            .map_err(|_| Error::InvalidDefinition)?;
        let mut steps = vec!["full_clone", "configure_resources"];
        if definition.spec.resources.disk_gib > mapping.source_disk_gib {
            steps.push("expand_disk");
        }
        steps.extend([
            "configure_cloud_init",
            "start_vm",
            "wait_for_guest_readiness",
        ]);
        if !definition.spec.provisioning.packages.is_empty() {
            steps.push("install_declared_packages");
        }
        if definition.spec.lifecycle.snapshot_after_provisioning {
            steps.push("snapshot_after_provisioning");
        }
        Ok(Plan {
            definition_name: definition.metadata.name.clone(),
            definition_hash,
            plan_hash: format!("{:x}", Sha256::digest(material)),
            provider: "proxmox",
            source: mapping.clone(),
            target: Target {
                node: mapping.node.clone(),
                vmid: target_vmid,
                name: definition.metadata.name.clone(),
                storage: mapping.storage.clone(),
                bridge: mapping.bridge.clone(),
                resources: definition.spec.resources.clone(),
                guest_user: mapping.guest_user.clone(),
                ipv4: "dhcp",
            },
            ssh_key_count: definition.spec.provisioning.ssh_authorized_keys.len(),
            packages: definition.spec.provisioning.packages.clone(),
            snapshot_after_provisioning: definition.spec.lifecycle.snapshot_after_provisioning,
            steps,
            live_verified: false,
            vmid_reserved: false,
            creation_enabled: false,
        })
    }
}

fn identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    node: String,
    vmid: u32,
    name: String,
    storage: String,
    bridge: String,
    resources: Resources,
    guest_user: String,
    ipv4: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub definition_name: String,
    pub definition_hash: String,
    pub plan_hash: String,
    provider: &'static str,
    source: ImageMapping,
    target: Target,
    ssh_key_count: usize,
    packages: Vec<String>,
    snapshot_after_provisioning: bool,
    steps: Vec<&'static str>,
    live_verified: bool,
    vmid_reserved: bool,
    creation_enabled: bool,
}
