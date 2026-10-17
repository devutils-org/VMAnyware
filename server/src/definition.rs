//! Versioned desired configuration, independent of Proxmox runtime state.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt};

pub const MAX_YAML_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiVersion {
    #[serde(rename = "vmanyware.dev/v1")]
    V1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Environment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Definition {
    pub api_version: ApiVersion,
    pub kind: Kind,
    pub metadata: Metadata,
    pub spec: Spec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub image: String,
    pub resources: Resources,
    pub provisioning: Provisioning,
    #[serde(default)]
    pub lifecycle: Lifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub cpu: u16,
    #[serde(rename = "memoryMiB")]
    pub memory_mib: u32,
    #[serde(rename = "diskGiB")]
    pub disk_gib: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Provisioning {
    pub cloud_init: bool,
    pub ssh_authorized_keys: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Lifecycle {
    #[serde(default)]
    pub snapshot_after_provisioning: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}
impl std::error::Error for ValidationError {}

fn invalid(field: &'static str, message: &'static str) -> ValidationError {
    ValidationError { field, message }
}

impl Definition {
    pub fn parse(yaml: &str) -> Result<Self, ValidationError> {
        if yaml.len() > MAX_YAML_BYTES {
            return Err(invalid("yaml", "Definition exceeds 64 KiB."));
        }
        if yaml.contains("PRIVATE KEY") {
            return Err(invalid("yaml", "Private keys are forbidden."));
        }
        let value: Self = serde_norway::from_str(yaml).map_err(|_| {
            invalid(
                "yaml",
                "Invalid schema, unknown field or unsupported version. Use vmanyware.dev/v1.",
            )
        })?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        let name = &self.metadata.name;
        if name.is_empty()
            || name.len() > 63
            || name.starts_with('-')
            || name.ends_with('-')
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(invalid(
                "metadata.name",
                "Use 1–63 lowercase letters, digits or internal hyphens.",
            ));
        }
        if self.metadata.description.len() > 1024 {
            return Err(invalid(
                "metadata.description",
                "Description exceeds 1024 bytes.",
            ));
        }
        let image = &self.spec.image;
        if image.is_empty()
            || image.len() > 64
            || !image
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-".contains(&b))
        {
            return Err(invalid(
                "spec.image",
                "Use a 1–64 character lowercase image identifier.",
            ));
        }
        let resources = &self.spec.resources;
        if !(1..=64).contains(&resources.cpu) {
            return Err(invalid(
                "spec.resources.cpu",
                "CPU must be between 1 and 64.",
            ));
        }
        if !(256..=262_144).contains(&resources.memory_mib) {
            return Err(invalid(
                "spec.resources.memoryMiB",
                "Memory must be between 256 and 262144 MiB.",
            ));
        }
        if !(4..=4096).contains(&resources.disk_gib) {
            return Err(invalid(
                "spec.resources.diskGiB",
                "Disk must be between 4 and 4096 GiB.",
            ));
        }
        let provisioning = &self.spec.provisioning;
        if !provisioning.cloud_init {
            return Err(invalid(
                "spec.provisioning.cloudInit",
                "Cloud-Init is required for v1.",
            ));
        }
        if !(1..=16).contains(&provisioning.ssh_authorized_keys.len()) {
            return Err(invalid(
                "spec.provisioning.sshAuthorizedKeys",
                "Provide 1–16 inline OpenSSH public keys.",
            ));
        }
        let mut keys = HashSet::new();
        for key in &provisioning.ssh_authorized_keys {
            let parsed = ssh_key::PublicKey::from_openssh(key).map_err(|_| {
                invalid(
                    "spec.provisioning.sshAuthorizedKeys",
                    "Invalid OpenSSH public key; file paths are not supported.",
                )
            })?;
            if key.len() > 8192
                || key.contains(['\n', '\r'])
                || !keys.insert(parsed.key_data().clone())
            {
                return Err(invalid(
                    "spec.provisioning.sshAuthorizedKeys",
                    "Keys must be unique single-line values up to 8192 bytes.",
                ));
            }
        }
        if provisioning.packages.len() > 128 {
            return Err(invalid(
                "spec.provisioning.packages",
                "At most 128 package declarations are supported.",
            ));
        }
        let mut packages = HashSet::new();
        for package in &provisioning.packages {
            if package.is_empty()
                || package.len() > 128
                || !package.as_bytes()[0].is_ascii_alphanumeric()
                || !package
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"+.-=:~".contains(&b))
                || !packages.insert(package)
            {
                return Err(invalid("spec.provisioning.packages", "Use unique Debian package names with optional =version; shell syntax is forbidden."));
            }
        }
        Ok(())
    }

    pub fn canonical_yaml(&self) -> Result<String, ValidationError> {
        self.validate()?;
        serde_norway::to_string(self).map_err(|_| invalid("yaml", "Cannot serialize definition."))
    }

    pub fn hash(&self) -> Result<String, ValidationError> {
        Ok(format!(
            "{:x}",
            Sha256::digest(self.canonical_yaml()?.as_bytes())
        ))
    }
}
