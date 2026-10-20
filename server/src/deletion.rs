//! Deletion target/volume review only. Destructive dispatch is not implemented.
use crate::{
    instance::{ObservationError, Summary},
    lifecycle::{Check, Status},
    proxmox::{DeletionVolume, Error, PowerState, Proxmox, QmpState, VmObservation},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub node: String,
    pub vmid: u32,
    pub config_digest: String,
    pub scope_hash: String,
    pub observation: VmObservation,
    pub volumes: Vec<DeletionVolume>,
    pub snapshots: Vec<String>,
    pub preserved_media: Vec<String>,
    pub preserved_owned_volumes: Vec<String>,
    pub effects: Effects,
    pub ha_managed: bool,
    pub replication_jobs: Vec<String>,
    pub destructive: bool,
    pub definition_retained: bool,
    pub purge: bool,
    pub destroy_unreferenced_disks: bool,
    pub checks: Vec<Check>,
    pub deletion_preconditions: Status,
    pub execution_enabled: bool,
}
/// Fixed backend contract, not caller-selectable delete options.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Effects {
    pub version: u32,
    pub remove_vm_configuration: bool,
    pub remove_snapshot_metadata: bool,
    pub remove_vm_permissions: bool,
    pub remove_vm_firewall: bool,
    pub cleanup_ipam: bool,
    pub cleanup_recorded_fleecing_images: bool,
    pub purge_jobs: bool,
    pub destroy_unreferenced_disks: bool,
    pub retain_definition_and_history: bool,
}
impl Effects {
    fn fixed() -> Self {
        Self {
            version: 1,
            remove_vm_configuration: true,
            remove_snapshot_metadata: true,
            remove_vm_permissions: true,
            remove_vm_firewall: true,
            cleanup_ipam: true,
            cleanup_recorded_fleecing_images: true,
            purge_jobs: false,
            destroy_unreferenced_disks: false,
            retain_definition_and_history: true,
        }
    }
}
fn check(code: &'static str, passed: bool, message: &'static str) -> Check {
    Check {
        code,
        status: if passed {
            Status::Passed
        } else {
            Status::Failed
        },
        message,
    }
}
fn stopped(observation: &VmObservation) -> bool {
    observation.status == PowerState::Stopped
        && matches!(observation.qmp_status, None | Some(QmpState::Stopped))
}
impl Summary {
    pub async fn deletion_preflight(
        &self,
        provider: &Proxmox,
    ) -> Result<Preflight, ObservationError> {
        let target = self.target(provider)?;
        tokio::time::timeout(Duration::from_secs(20),async {
            let live=self.lifecycle_preflight(provider).await?;
            let marker=format!("vmanyware-{}",self.id);
            let management_before=provider.deletion_management(&target).await.map_err(ObservationError::Provider)?;
            let before=provider.deletion_scope(&target,&marker).await.map_err(ObservationError::Provider)?;
            let after=provider.deletion_scope(&target,&marker).await.map_err(ObservationError::Provider)?;
            let management_after=provider.deletion_management(&target).await.map_err(ObservationError::Provider)?;
            let pending=provider.vm_has_pending_changes(&target).await.map_err(ObservationError::Provider)?;
            let identity=provider.vm_identity(&target,&marker).await.map_err(ObservationError::Provider)?;
            let power=provider.vm_status(&target).await.map_err(ObservationError::Provider)?;
            let mut checks=live.checks;
            checks.extend([
                check("delete_vm_stopped",stopped(&live.observation)&&stopped(&power),"The VM must already be stopped; no automatic power action is permitted."),
                check("delete_not_protected",!before.protected&&!after.protected,"Provider deletion protection must be off; it is never removed automatically."),
                check("delete_scope_complete",before.complete&&after.complete,"Every live/snapshot disk reference must have a supported explicit scope."),
                check("delete_volumes_owned",before.volumes.iter().chain(after.volumes.iter()).all(|volume|volume.owner_verified),"Storage inventory must confirm this VMID owns every reviewed image; filenames are not ownership evidence."),
                check("delete_snapshots_complete",before.snapshots_complete&&after.snapshots_complete&&before.snapshot_identity&&after.snapshot_identity,"Snapshot work must be complete, with saved configurations belonging to this instance."),
                check("delete_scope_stable",before.fingerprint==after.fingerprint,"Reviewed live/snapshot configuration and volume scope must stay unchanged."),
                check("delete_target_unchanged",live.config_digest==before.identity.digest&&before.identity.digest==after.identity.digest&&after.identity.digest==identity.digest&&before.identity.instance_marker&&after.identity.instance_marker&&identity.instance_marker&&!before.identity.locked&&!after.identity.locked&&!identity.locked&&!before.identity.template&&!after.identity.template&&!identity.template&&!pending,"Retained target identity/configuration must remain unchanged, unlocked and without pending changes."),
            ]);
            checks.extend([
                check("delete_not_ha_managed",!management_before.ha_managed&&!management_after.ha_managed,"The VM must not remain configured as an HA resource, even if disabled."),
                check("delete_no_replication_jobs",management_before.replication_jobs.is_empty()&&management_after.replication_jobs.is_empty(),"No replication job may reference this VM, including disabled or removal-pending jobs."),
                check("delete_management_stable",management_before==management_after,"Target HA/replication membership must remain stable through the volume review."),
            ]);
            checks.push(check("delete_provider_effect_scope",before.effects_complete&&after.effects_complete,"Recorded fleecing cleanup and every candidate effect must be understood; unknown sections or snapshot-only disk cleanup block deletion."));
            let effects=Effects::fixed();
            use sha2::{Digest,Sha256};
            let mut material=b"vmanyware-delete-management-scope-v1\0".to_vec();
            material.extend(after.fingerprint.as_bytes());
            material.extend(serde_json::to_vec(&management_after).map_err(|_|ObservationError::Provider(Error::InvalidResponse))?);
            material.extend(serde_json::to_vec(&effects).map_err(|_|ObservationError::Provider(Error::InvalidResponse))?);
            let scope_hash=format!("{:x}",Sha256::digest(material));
            let deletion_preconditions=if checks.iter().any(|check|check.status==Status::Failed){Status::Failed}else{Status::Passed};
            Ok(Preflight{instance_id:self.id.clone(),provider_scope_id:provider.scope_id(),definition_hash:self.definition_hash.clone(),node:self.provider_reference.node.clone(),vmid:self.provider_reference.vmid,config_digest:identity.digest,scope_hash,ha_managed:management_after.ha_managed,replication_jobs:management_after.replication_jobs,observation:power,volumes:after.volumes,snapshots:after.snapshots,preserved_media:after.preserved_media,preserved_owned_volumes:after.preserved_owned_volumes,effects,destructive:true,definition_retained:true,purge:false,destroy_unreferenced_disks:false,checks,deletion_preconditions,execution_enabled:false})
        }).await.map_err(|_|ObservationError::Provider(Error::Timeout))?
    }
}

/// Reviewed public scope retained for reconciliation after the VM no longer exists.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Plan {
    pub scope_hash: String,
    pub volumes: Vec<DeletionVolume>,
    pub snapshots: Vec<String>,
    pub preserved_media: Vec<String>,
    pub preserved_owned_volumes: Vec<String>,
    pub effects: Effects,
}
impl Plan {
    fn valid(&self) -> bool {
        fn sorted<T: Ord>(items: &[T]) -> bool {
            items.windows(2).all(|pair| pair[0] < pair[1])
        }
        fn snapshot(name: &str) -> bool {
            !name.is_empty()
                && name != "current"
                && name.len() <= 255
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
        }
        use crate::proxmox::{
            deletion::{supported_drive_key, volume_id},
            DeletionVolumeEffect,
        };
        if !crate::instance::valid_hash(&self.scope_hash)
            || self.effects != Effects::fixed()
            || self.volumes.len() + self.preserved_owned_volumes.len() > 256
            || self.snapshots.len() > 64
            || self.preserved_media.len() > 256
            || !sorted(&self.snapshots)
            || !self.snapshots.iter().all(|name| snapshot(name))
            || !sorted(&self.preserved_media)
            || !sorted(&self.preserved_owned_volumes)
            || !self
                .volumes
                .windows(2)
                .all(|pair| pair[0].volume < pair[1].volume)
        {
            return false;
        }
        let mut all = std::collections::BTreeSet::new();
        for volume in self
            .preserved_media
            .iter()
            .chain(&self.preserved_owned_volumes)
        {
            if volume_id(volume).is_none() || !all.insert(volume.as_str()) {
                return false;
            }
        }
        for volume in &self.volumes {
            if volume_id(&volume.volume).is_none()
                || !volume.owner_verified
                || !all.insert(&volume.volume)
                || volume.references.is_empty()
                || !sorted(&volume.references)
            {
                return false;
            }
            for reference in &volume.references {
                let valid = if let Some(key) = reference.strip_prefix("live:") {
                    key == "special/fleecing" || supported_drive_key(key)
                } else if let Some((name, key)) = reference
                    .strip_prefix("snapshot/")
                    .and_then(|reference| reference.split_once(':'))
                {
                    self.snapshots.iter().any(|snapshot| snapshot == name)
                        && supported_drive_key(key)
                } else {
                    false
                };
                if !valid {
                    return false;
                }
            }
            let fleecing = volume
                .references
                .iter()
                .any(|reference| reference == "live:special/fleecing");
            match volume.effect {
                DeletionVolumeEffect::FleecingCleanup if fleecing => {}
                DeletionVolumeEffect::DeleteOwnedVolume
                    if !fleecing
                        && volume.references.iter().any(|reference| {
                            reference.starts_with("live:") || reference.ends_with(":vmstate")
                        }) => {}
                _ => return false,
            }
        }
        true
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Approval {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub node: String,
    pub vmid: u32,
    pub config_digest: String,
    pub plan: Plan,
    pub confirmed: bool,
    pub acknowledge_data_loss: bool,
}
impl Approval {
    pub fn is_valid(&self) -> bool {
        self.confirmed
            && self.acknowledge_data_loss
            && self.instance_id.len() == 32
            && self
                .instance_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && crate::instance::valid_hash(&self.provider_scope_id)
            && crate::instance::valid_hash(&self.definition_hash)
            && self.config_digest.len() == 40
            && self
                .config_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && crate::proxmox::VmReference::new(&self.node, self.vmid).is_ok()
            && self.plan.valid()
            && serde_json::to_vec(self).is_ok_and(|json| json.len() <= 64 * 1024)
    }
    pub fn matches_instance(&self, instance: &Summary) -> bool {
        self.instance_id == instance.id
            && self.provider_scope_id == instance.provider_reference.scope_id
            && self.definition_hash == instance.definition_hash
            && self.node == instance.provider_reference.node
            && self.vmid == instance.provider_reference.vmid
    }
    pub fn input_hash(&self) -> Result<String, serde_json::Error> {
        use sha2::{Digest, Sha256};
        let mut material = b"vmanyware-delete-v1\0".to_vec();
        material.extend(serde_json::to_vec(self)?);
        Ok(format!("{:x}", Sha256::digest(material)))
    }
}
#[derive(Debug)]
pub struct Job {
    pub operation: crate::operation::Operation,
    pub approval: Approval,
    pub confirmed_at: i64,
    pub dispatch_started: bool,
    pub provider_task: Option<crate::proxmox::TaskReference>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResultEvidence {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub node: String,
    pub vmid: u32,
    pub approval_hash: String,
    pub task_status: crate::proxmox::TaskStatus,
    pub vm_absence_verified: bool,
    pub volumes_absence_verified: bool,
    pub definition_retained: bool,
    pub association_retired: bool,
}
pub enum Outcome {
    Succeeded(crate::proxmox::DeletionCompletion),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}
