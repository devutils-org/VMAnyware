//! Independent GET-only observations; provider errors never mean absence.
use super::{deletion::volume_id, Error, Proxmox, TaskReference, TaskStatus, VmReference};
use crate::deletion::Approval;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, Eq)]
pub struct DeletionAbsence {
    pub approval_hash: String,
    pub remaining_volumes: Vec<String>,
}
/// Constructed only by independent provider reads; not deserializable or caller-forgeable.
#[derive(Debug)]
pub struct DeletionCompletion {
    approval_hash: String,
    task: TaskReference,
    observed_at: Instant,
}
impl DeletionCompletion {
    pub(crate) fn matches(&self, approval: &Approval, task: &TaskReference) -> bool {
        self.task == *task
            && approval
                .input_hash()
                .is_ok_and(|hash| hash == self.approval_hash)
            && self.observed_at.elapsed() <= Duration::from_secs(30)
    }
}
#[derive(Deserialize)]
struct Image {
    volid: String,
}
impl Proxmox {
    /// Refuse dispatch if current permissions already prevent later complete absence reads.
    pub(crate) async fn deletion_absence_visibility(
        &self,
        approval: &Approval,
    ) -> Result<(), Error> {
        if !approval.is_valid() || approval.provider_scope_id != self.scope_id() {
            return Err(Error::Configuration);
        }
        tokio::time::timeout(Duration::from_secs(20), async {
            let storages = approval
                .plan
                .volumes
                .iter()
                .map(|volume| volume_id(&volume.volume).ok_or(Error::Configuration))
                .collect::<Result<BTreeSet<_>, _>>()?;
            for storage in storages {
                self.deletion_storage_visibility(storage).await?;
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    /// A successful destroy task alone is insufficient: verify approved VM/volume absence too.
    pub async fn verify_deletion_completion(
        &self,
        approval: &Approval,
        task: &TaskReference,
    ) -> Result<DeletionCompletion, Error> {
        if !approval.is_valid()
            || approval.provider_scope_id != self.scope_id()
            || !task.matches_deletion(&VmReference::new(&approval.node, approval.vmid)?)
        {
            return Err(Error::Configuration);
        }
        if self.task_status(task).await? != TaskStatus::Succeeded {
            return Err(Error::InvalidResponse);
        }
        let absence = self.deletion_absence(approval).await?;
        if !absence.remaining_volumes.is_empty() {
            return Err(Error::InvalidResponse);
        }
        Ok(DeletionCompletion {
            approval_hash: absence.approval_hash,
            task: task.clone(),
            observed_at: Instant::now(),
        })
    }
    async fn deletion_storage_visibility(&self, storage: &str) -> Result<(), Error> {
        let path = format!("/storage/{storage}");
        let permissions: Value = self
            .get_query(&["access", "permissions"], &[("path", path.clone())])
            .await?;
        let paths = permissions.as_object().ok_or(Error::InvalidResponse)?;
        let grants = paths
            .get(&path)
            .ok_or(Error::Forbidden)?
            .as_object()
            .ok_or(Error::InvalidResponse)?;
        // Allocate bypasses per-volume VM ACL filtering, including after VM ACL removal.
        // Merely possessing Audit does not guarantee a complete image listing.
        match grants.get("Datastore.Allocate").ok_or(Error::Forbidden)? {
            Value::Bool(_) => Ok(()),
            value if matches!(value.as_u64(), Some(0 | 1)) => Ok(()),
            _ => Err(Error::InvalidResponse),
        }
    }
    /// Requires full retained approval; does not complete an operation or mutate a VM.
    /// Observations are point-in-time, not a reservation against external actors.
    pub async fn deletion_absence(&self, approval: &Approval) -> Result<DeletionAbsence, Error> {
        if !approval.is_valid() || approval.provider_scope_id != self.scope_id() {
            return Err(Error::Configuration);
        }
        tokio::time::timeout(Duration::from_secs(20), async {
            if self.available_vmid(approval.vmid).await? != approval.vmid {
                return Err(Error::InvalidResponse);
            }
            let storages = approval
                .plan
                .volumes
                .iter()
                .map(|volume| volume_id(&volume.volume).ok_or(Error::Configuration))
                .collect::<Result<BTreeSet<_>, _>>()?;
            let mut remaining_volumes = BTreeSet::new();
            for storage in storages {
                self.deletion_storage_visibility(storage).await?;
                // No VMID filter: an ownership change must not hide a reviewed volume.
                let images: Vec<Image> = self
                    .get_query(
                        &["nodes", &approval.node, "storage", storage, "content"],
                        &[("content", "images".into())],
                    )
                    .await?;
                if images.len() > 4096 {
                    return Err(Error::ResponseTooLarge);
                }
                let mut seen = BTreeSet::new();
                for image in images {
                    if volume_id(&image.volid) != Some(storage) || !seen.insert(image.volid.clone())
                    {
                        return Err(Error::InvalidResponse);
                    }
                    if approval
                        .plan
                        .volumes
                        .iter()
                        .any(|volume| volume.volume == image.volid)
                    {
                        remaining_volumes.insert(image.volid);
                    }
                }
                self.deletion_storage_visibility(storage).await?;
            }
            if self.available_vmid(approval.vmid).await? != approval.vmid {
                return Err(Error::InvalidResponse);
            }
            Ok(DeletionAbsence {
                approval_hash: approval.input_hash().map_err(|_| Error::Configuration)?,
                remaining_volumes: remaining_volumes.into_iter().collect(),
            })
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deletion_completion_proof_expires_and_cannot_be_rebound() {
        let approval: Approval=serde_json::from_value(serde_json::json!({"instanceId":"a".repeat(32),"providerScopeId":"b".repeat(64),"definitionHash":"c".repeat(64),"node":"pve-lab","vmid":202,"configDigest":"d".repeat(40),"confirmed":true,"acknowledgeDataLoss":true,"plan":{"scopeHash":"e".repeat(64),"volumes":[],"snapshots":[],"preservedMedia":[],"preservedOwnedVolumes":[],"effects":{"version":1,"removeVmConfiguration":true,"removeSnapshotMetadata":true,"removeVmPermissions":true,"removeVmFirewall":true,"cleanupIpam":true,"cleanupRecordedFleecingImages":true,"purgeJobs":false,"destroyUnreferencedDisks":false,"retainDefinitionAndHistory":true}}})).unwrap();
        assert!(approval.is_valid());
        let task = TaskReference::new(
            "pve-lab",
            "UPID:pve-lab:00000001:00000002:00000003:qmdestroy:202:fixture@pve:",
        )
        .unwrap();
        let mut proof = DeletionCompletion {
            approval_hash: approval.input_hash().unwrap(),
            task: task.clone(),
            observed_at: Instant::now(),
        };
        assert!(proof.matches(&approval, &task));
        proof.observed_at = Instant::now() - Duration::from_secs(31);
        assert!(!proof.matches(&approval, &task));
        proof.observed_at = Instant::now();
        let mut changed = approval.clone();
        changed.plan.scope_hash = "f".repeat(64);
        assert!(!proof.matches(&changed, &task));
        let other = TaskReference::new(
            "pve-lab",
            "UPID:pve-lab:00000004:00000002:00000003:qmdestroy:202:fixture@pve:",
        )
        .unwrap();
        assert!(!proof.matches(&approval, &other));
    }
}
