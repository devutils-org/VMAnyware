//! Read-only deletion scope. Storage inventory establishes ownership, not filenames.
use super::{Error, Proxmox, VmReference};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DeletionVolume {
    pub volume: String,
    pub references: Vec<String>,
    pub owner_verified: bool,
    pub effect: DeletionVolumeEffect,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionVolumeEffect {
    DeleteOwnedVolume,
    FleecingCleanup,
    SnapshotReference,
}
pub(crate) struct DeletionScope {
    pub identity: super::identity::Identity,
    pub protected: bool,
    pub effects_complete: bool,
    pub preserved_owned_volumes: Vec<String>,
    pub complete: bool,
    pub snapshots_complete: bool,
    pub snapshot_identity: bool,
    pub fingerprint: String,
    pub volumes: Vec<DeletionVolume>,
    pub snapshots: Vec<String>,
    pub preserved_media: Vec<String>,
}
pub(crate) fn volume_id(value: &str) -> Option<&str> {
    let (storage, name) = value.split_once(':')?;
    (!storage.is_empty()
        && storage.len() <= 64
        && storage.as_bytes()[0].is_ascii_alphabetic()
        && storage
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        && value.len() <= 256
        && !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
        && name
            .split('/')
            .all(|part| !part.is_empty() && !matches!(part, "." | "..")))
    .then_some(storage)
}
fn drive_key(key: &str) -> bool {
    [
        "ide", "scsi", "sata", "virtio", "unused", "efidisk", "tpmstate",
    ]
    .iter()
    .any(|prefix| {
        key.strip_prefix(prefix)
            .is_some_and(|index| !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()))
    }) || key == "vmstate"
}
pub(crate) fn supported_drive_key(key: &str) -> bool {
    if key == "vmstate" {
        return true;
    }
    [
        ("ide", 4),
        ("scsi", 31),
        ("sata", 6),
        ("virtio", 16),
        ("unused", 256),
        ("efidisk", 1),
        ("tpmstate", 1),
    ]
    .iter()
    .any(|(prefix, limit)| {
        key.strip_prefix(prefix).is_some_and(|index| {
            index
                .parse::<usize>()
                .is_ok_and(|number| number < *limit && number.to_string() == index)
        })
    })
}
fn references(
    value: &Value,
    source: &str,
    volumes: &mut BTreeMap<String, BTreeSet<String>>,
    media: &mut BTreeSet<String>,
) -> bool {
    let Some(config) = value.as_object() else {
        return false;
    };
    let mut complete = true;
    for (key, value) in config.iter().filter(|(key, _)| drive_key(key)) {
        if !supported_drive_key(key) {
            complete = false;
            continue;
        }
        let Some(raw) = value
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 8192 && !s.chars().any(char::is_control))
        else {
            complete = false;
            continue;
        };
        let mut fields = raw.split(',');
        let first = fields.next().unwrap_or("");
        let volume = first.strip_prefix("file=").unwrap_or(first);
        let mut options = BTreeMap::new();
        let valid = fields.all(|field| {
            field.split_once('=').is_some_and(|(key, value)| {
                !key.is_empty()
                    && !value.is_empty()
                    && key != "file"
                    && options.insert(key, value).is_none()
            })
        });
        if !valid {
            complete = false;
            continue;
        }
        if options.get("media") == Some(&"cdrom") && matches!(volume, "none" | "cdrom") {
            continue;
        }
        if volume_id(volume).is_none() {
            complete = false;
            continue;
        }
        let filename = volume.rsplit([':', '/']).next().unwrap_or("");
        let stem = filename
            .strip_suffix(".raw")
            .or_else(|| filename.strip_suffix(".qcow2"))
            .or_else(|| filename.strip_suffix(".vmdk"))
            .unwrap_or(filename);
        let cloudinit = stem == "cloudinit"
            || stem
                .strip_prefix("vm-")
                .and_then(|name| name.strip_suffix("-cloudinit"))
                .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()));
        if options.get("media") == Some(&"cdrom") && !cloudinit {
            media.insert(volume.into());
        } else {
            volumes
                .entry(volume.into())
                .or_default()
                .insert(format!("{source}:{key}"));
        }
    }
    complete
}
fn fleecing_references(value: &Value, volumes: &mut BTreeMap<String, BTreeSet<String>>) -> bool {
    let Some(sections) = value.get("special-sections") else {
        return true;
    };
    let Some(sections) = sections.as_object() else {
        return false;
    };
    if sections.iter().any(|(name, value)| {
        !matches!(name.as_str(), "cloudinit" | "fleecing") || !value.is_object()
    }) {
        return false;
    }
    let Some(fleecing) = sections.get("fleecing") else {
        return true;
    };
    let fleecing = fleecing.as_object().unwrap();
    if fleecing.keys().any(|key| key != "fleecing-images") {
        return false;
    }
    let Some(images) = fleecing.get("fleecing-images") else {
        return true;
    };
    let Some(images) = images
        .as_str()
        .filter(|value| value.len() <= 65536 && !value.chars().any(char::is_control))
    else {
        return false;
    };
    if images.is_empty() {
        return true;
    }
    for image in images.split(',').map(str::trim) {
        if volume_id(image).is_none() {
            return false;
        }
        volumes
            .entry(image.into())
            .or_default()
            .insert("live:special/fleecing".into());
    }
    true
}
#[derive(Deserialize)]
struct Inventory {
    volid: String,
    vmid: Option<u32>,
    format: String,
}
impl Proxmox {
    pub(crate) async fn deletion_scope(
        &self,
        target: &VmReference,
        marker: &str,
    ) -> Result<DeletionScope, Error> {
        let live = self.template_config(target.node(), target.vmid()).await?;
        let config = live.as_object().ok_or(Error::InvalidResponse)?;
        let digest = config
            .get("digest")
            .and_then(Value::as_str)
            .filter(|digest| digest.len() == 40 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or(Error::InvalidResponse)?;
        let identity = super::identity::decode_identity(&live, marker, digest.into())?;
        let protected = match config.get("protection") {
            None => false,
            Some(Value::Bool(value)) => *value,
            Some(value) if value.as_u64() == Some(0) => false,
            Some(value) if value.as_u64() == Some(1) => true,
            _ => return Err(Error::InvalidResponse),
        };
        let snapshots = self.snapshots(target).await?;
        if snapshots.len() > 64 {
            return Err(Error::ResponseTooLarge);
        }
        let snapshots_complete = snapshots
            .iter()
            .all(|snapshot| snapshot.provider_state.is_none());
        let mut volumes = BTreeMap::new();
        let mut media = BTreeSet::new();
        let mut complete = references(&live, "live", &mut volumes, &mut media);
        let mut effects_complete = fleecing_references(&live, &mut volumes);
        let mut snapshot_identity = true;
        let mut configs = BTreeMap::new();
        configs.insert("live".to_string(), live);
        for snapshot in &snapshots {
            let value: Value = self
                .get(&[
                    "nodes",
                    target.node(),
                    "qemu",
                    &target.vmid().to_string(),
                    "snapshot",
                    &snapshot.name,
                    "config",
                ])
                .await?;
            let saved = super::identity::decode_identity(&value, marker, String::new())?;
            snapshot_identity &= saved.instance_marker && !saved.template && !saved.locked;
            complete &= value.get("snapstate").is_none();
            complete &= references(
                &value,
                &format!("snapshot/{}", snapshot.name),
                &mut volumes,
                &mut media,
            );
            configs.insert(format!("snapshot/{}", snapshot.name), value);
        }
        if volumes.len() > 256 {
            return Err(Error::ResponseTooLarge);
        }
        let storages: BTreeSet<_> = volumes
            .keys()
            .filter_map(|volume| volume_id(volume).map(str::to_owned))
            .collect();
        let mut owners = BTreeMap::new();
        for storage in storages {
            let entries: Vec<Inventory> = self
                .get_query(
                    &["nodes", target.node(), "storage", &storage, "content"],
                    &[
                        ("content", "images".into()),
                        ("vmid", target.vmid().to_string()),
                    ],
                )
                .await?;
            if entries.len() > 1024 {
                return Err(Error::ResponseTooLarge);
            }
            for entry in entries {
                if volume_id(&entry.volid) != Some(storage.as_str())
                    || owners.contains_key(&entry.volid)
                {
                    return Err(Error::InvalidResponse);
                }
                owners.insert(
                    entry.volid,
                    entry.vmid == Some(target.vmid())
                        && matches!(entry.format.as_str(), "raw" | "qcow2" | "vmdk"),
                );
            }
        }
        let preserved_owned_volumes = owners
            .iter()
            .filter(|(volume, owned)| {
                **owned && !volumes.contains_key(*volume) && !media.contains(*volume)
            })
            .map(|(volume, _)| volume.clone())
            .collect::<Vec<_>>();
        if volumes.len() + preserved_owned_volumes.len() > 256 {
            return Err(Error::ResponseTooLarge);
        }
        let volumes = volumes
            .into_iter()
            .map(|(volume, references)| {
                let effect = if references.contains("live:special/fleecing") {
                    DeletionVolumeEffect::FleecingCleanup
                } else if references.iter().any(|reference| {
                    reference.starts_with("live:") || reference.ends_with(":vmstate")
                }) {
                    DeletionVolumeEffect::DeleteOwnedVolume
                } else {
                    effects_complete = false;
                    DeletionVolumeEffect::SnapshotReference
                };
                DeletionVolume {
                    owner_verified: owners.get(&volume) == Some(&true),
                    volume,
                    references: references.into_iter().collect(),
                    effect,
                }
            })
            .collect::<Vec<_>>();
        // Hash canonical API-visible config and normalized effects; raw config stays private.
        // BTreeMaps ensure deterministic key ordering even with preserve_order enabled.
        fn canonical(value: &Value) -> Value {
            match value {
                Value::Object(map) => Value::Object(
                    map.iter()
                        .collect::<BTreeMap<_, _>>()
                        .into_iter()
                        .map(|(key, value)| (key.clone(), canonical(value)))
                        .collect(),
                ),
                Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
                value => value.clone(),
            }
        }
        use sha2::{Digest, Sha256};
        let mut material = b"vmanyware-delete-scope-v2\0".to_vec();
        material.extend(
            serde_json::to_vec(&(
                configs
                    .into_iter()
                    .map(|(key, value)| (key, canonical(&value)))
                    .collect::<BTreeMap<_, _>>(),
                &snapshots,
                &volumes,
                &media,
                &preserved_owned_volumes,
                effects_complete,
            ))
            .map_err(|_| Error::InvalidResponse)?,
        );
        Ok(DeletionScope {
            identity,
            protected,
            effects_complete,
            preserved_owned_volumes,
            complete,
            snapshots_complete,
            snapshot_identity,
            fingerprint: format!("{:x}", Sha256::digest(material)),
            volumes,
            snapshots: snapshots
                .into_iter()
                .map(|snapshot| snapshot.name)
                .collect(),
            preserved_media: media.into_iter().collect(),
        })
    }
}
