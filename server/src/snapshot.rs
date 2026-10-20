//! Read-only prerequisites for disk-only snapshot creation; not authorization.
use crate::{
    instance::{ObservationError, Summary},
    lifecycle::{Check, Status},
    proxmox::{Error, Proxmox},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}
pub(crate) fn valid_name(name: &str) -> bool {
    (2..=40).contains(&name.len())
        && name != "current"
        && name.as_bytes()[0].is_ascii_alphabetic()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
impl CreateRequest {
    pub fn is_valid(&self) -> bool {
        valid_name(&self.name)
            && self.description.len() <= 4096
            && !self
                .description
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t')
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub node: String,
    pub vmid: u32,
    pub config_digest: String,
    pub request: CreateRequest,
    pub includes_ram: bool,
    pub checks: Vec<Check>,
    pub creation_preconditions: Status,
    pub execution_enabled: bool,
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
impl Summary {
    pub async fn snapshot_preflight(
        &self,
        provider: &Proxmox,
        request: CreateRequest,
    ) -> Result<Preflight, ObservationError> {
        if !request.is_valid() {
            return Err(ObservationError::InvalidReference);
        }
        let target = self.target(provider)?;
        tokio::time::timeout(Duration::from_secs(20), async {
            let lifecycle = self.lifecycle_preflight(provider).await?;
            let supported = provider.snapshot_supported(&target).await.map_err(ObservationError::Provider)?;
            let snapshots = provider.snapshots(&target).await.map_err(ObservationError::Provider)?;
            let pending = provider.vm_has_pending_changes(&target).await.map_err(ObservationError::Provider)?;
            let after = provider.vm_identity(&target, &format!("vmanyware-{}", self.id)).await.map_err(ObservationError::Provider)?;
            let mut checks = lifecycle.checks;
            checks.extend([
                check("snapshot_supported", supported, "Provider storage/configuration must support snapshots."),
                check("snapshot_name_available", !snapshots.iter().any(|snapshot| snapshot.name == request.name), "The exact snapshot name must not already exist."),
                check("snapshot_target_unchanged", lifecycle.config_digest == after.digest && after.instance_marker && !after.template && !after.locked && !pending, "Target identity/configuration must remain unchanged and unlocked through snapshot reads."),
                check("snapshot_tree_complete", snapshots.iter().all(|snapshot| snapshot.provider_state.is_none()), "Existing incomplete snapshot work must be resolved first."),
            ]);
            let creation_preconditions = if checks.iter().all(|check| check.status == Status::Passed) {Status::Passed} else {Status::Failed};
            Ok(Preflight { instance_id:self.id.clone(), provider_scope_id:provider.scope_id(), definition_hash:self.definition_hash.clone(), node:self.provider_reference.node.clone(), vmid:self.provider_reference.vmid, config_digest:after.digest, request, includes_ram:false, checks, creation_preconditions, execution_enabled:false })
        }).await.map_err(|_| ObservationError::Provider(Error::Timeout))?
    }
}

/// Exact caller-confirmed target and disk-only request; storage assigns its age.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Approval {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub node: String,
    pub vmid: u32,
    pub config_digest: String,
    pub request: CreateRequest,
    pub confirmed: bool,
}
impl Approval {
    pub fn is_valid(&self) -> bool {
        self.confirmed
            && self.request.is_valid()
            && self.instance_id.len() == 32
            && self
                .instance_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && crate::instance::valid_hash(&self.provider_scope_id)
            && crate::instance::valid_hash(&self.definition_hash)
            && self.config_digest.len() == 40
            && self
                .config_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && crate::proxmox::VmReference::new(&self.node, self.vmid).is_ok()
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
        let mut material = b"vmanyware-snapshot-create-v1\0".to_vec();
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
#[serde(rename_all = "camelCase")]
pub struct ResultEvidence {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub node: String,
    pub vmid: u32,
    pub task_status: crate::proxmox::TaskStatus,
    pub snapshot: crate::proxmox::Snapshot,
}
pub enum Outcome {
    Succeeded(ResultEvidence),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}

/// Saved operation context; it is not a new approval or a current provider observation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovered {
    pub operation: crate::operation::Operation,
    pub target: RecoveryTarget,
    pub request: CreateRequest,
    pub submission_started: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryTarget {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub node: String,
    pub vmid: u32,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub instance_id: String,
    pub operations: Vec<HistoryEntry>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub name: String,
    pub status: crate::operation::Status,
    pub failure: Option<crate::operation::Failure>,
    pub provider_task_known: bool,
    pub created_at: String,
}
