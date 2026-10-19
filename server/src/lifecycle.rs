//! Read-only lifecycle preconditions. A matching tag is a declaration, not authorization.
use crate::{
    instance::{ObservationError, Summary},
    proxmox::{Error, PowerState, Proxmox, QmpState, VmObservation},
};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub code: &'static str,
    pub status: Status,
    pub message: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preflight {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub expected_marker: String,
    pub config_digest: String,
    pub observation: VmObservation,
    pub checks: Vec<Check>,
    pub start_preconditions: Status,
    pub shutdown_preconditions: Status,
    pub execution_enabled: bool,
}

fn checked(code: &'static str, passed: bool, message: &'static str) -> Check {
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
fn combined(checks: &[Check], power: Status) -> Status {
    if power == Status::Failed || checks.iter().any(|check| check.status == Status::Failed) {
        Status::Failed
    } else if power == Status::Unknown || checks.iter().any(|check| check.status == Status::Unknown)
    {
        Status::Unknown
    } else {
        Status::Passed
    }
}

impl Summary {
    pub async fn lifecycle_preflight(
        &self,
        provider: &Proxmox,
    ) -> Result<Preflight, ObservationError> {
        let target = self.target(provider)?;
        let expected_marker = format!("vmanyware-{}", self.id);
        tokio::time::timeout(Duration::from_secs(20), async {
            let before = provider.vm_identity(&target, &expected_marker).await?;
            let pending = provider.vm_has_pending_changes(&target).await?;
            let observation = provider.vm_status(&target).await?;
            let after = provider.vm_identity(&target, &expected_marker).await?;
            let checks = vec![
                checked(
                    "instance_marker",
                    before.instance_marker && after.instance_marker,
                    "The sole VMAnyware tag must exactly match this registered instance.",
                ),
                checked(
                    "not_template",
                    !before.template && !after.template,
                    "Lifecycle actions require a VM, not a template.",
                ),
                checked(
                    "unlocked",
                    !before.locked && !after.locked,
                    "No provider configuration lock may be present.",
                ),
                checked(
                    "no_pending_changes",
                    !pending,
                    "No pending configuration or deletion may be present.",
                ),
                checked(
                    "configuration_unchanged",
                    before.digest == after.digest,
                    "Configuration digest must remain unchanged during these reads.",
                ),
            ];
            let start = match (observation.status, observation.qmp_status) {
                (PowerState::Stopped, None | Some(QmpState::Stopped)) => Status::Passed,
                (PowerState::Stopped, _) => Status::Unknown,
                (PowerState::Running, _) => Status::Failed,
            };
            let shutdown = match (observation.status, observation.qmp_status) {
                (PowerState::Running, Some(QmpState::Running)) => Status::Passed,
                (PowerState::Running, None | Some(QmpState::Unknown)) => Status::Unknown,
                _ => Status::Failed,
            };
            Ok(Preflight {
                instance_id: self.id.clone(),
                provider_scope_id: self.provider_reference.scope_id.clone(),
                definition_hash: self.definition_hash.clone(),
                expected_marker,
                config_digest: after.digest,
                start_preconditions: combined(&checks, start),
                shutdown_preconditions: combined(&checks, shutdown),
                observation,
                checks,
                execution_enabled: false,
            })
        })
        .await
        .map_err(|_| ObservationError::Provider(Error::Timeout))?
        .map_err(ObservationError::Provider)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Start,
    Shutdown,
}
impl Action {
    pub fn operation_kind(self) -> &'static str {
        match self {
            Self::Start => "vm_start",
            Self::Shutdown => "vm_shutdown",
        }
    }
    pub(crate) fn task_kind(self) -> &'static str {
        match self {
            Self::Start => "qmstart",
            Self::Shutdown => "qmshutdown",
        }
    }
    pub fn desired_power(self) -> PowerState {
        match self {
            Self::Start => PowerState::Running,
            Self::Shutdown => PowerState::Stopped,
        }
    }
}

/// Explicit caller confirmation of the whole target; timestamps are assigned by storage.
#[derive(Debug, Clone, serde::Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Approval {
    pub action: Action,
    pub instance_id: String,
    pub provider_scope_id: String,
    pub node: String,
    pub vmid: u32,
    pub definition_hash: String,
    pub config_digest: String,
    pub confirmed: bool,
}
impl Approval {
    pub fn is_valid(&self) -> bool {
        self.confirmed
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
            && self.node == instance.provider_reference.node
            && self.vmid == instance.provider_reference.vmid
            && self.definition_hash == instance.definition_hash
    }
    pub fn input_hash(&self) -> Result<String, serde_json::Error> {
        use sha2::{Digest, Sha256};
        let mut material = b"vmanyware-lifecycle-v1\0".to_vec();
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultEvidence {
    pub action: Action,
    pub instance_id: String,
    pub provider_scope_id: String,
    pub task_status: crate::proxmox::TaskStatus,
    pub observation: VmObservation,
}

pub enum Outcome {
    Succeeded(ResultEvidence),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}
