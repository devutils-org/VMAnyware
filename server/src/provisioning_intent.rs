//! Confirmed provisioning intent and private per-stage task journal.
use crate::{
    definition::Definition,
    operation::{Operation, Status},
    provisioning::{Catalog, Plan},
    proxmox::TaskReference,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Approval {
    pub definition_name: String,
    pub definition_hash: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub target_node: String,
    pub target_vmid: u32,
    pub source_vmid: u32,
    pub storage: String,
    pub bridge: String,
    pub confirmed: bool,
}
impl Approval {
    pub fn valid(&self) -> bool {
        self.confirmed
            && crate::instance::valid_hash(&self.definition_hash)
            && crate::instance::valid_hash(&self.provider_scope_id)
            && crate::instance::valid_hash(&self.plan_hash)
            && crate::proxmox::VmReference::new(&self.target_node, self.target_vmid).is_ok()
            && (100..=999999999).contains(&self.source_vmid)
            && self.source_vmid != self.target_vmid
            && !self.definition_name.is_empty()
            && self.definition_name.len() <= 63
            && !self.definition_name.starts_with('-')
            && !self.definition_name.ends_with('-')
            && self
                .definition_name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !self.storage.is_empty()
            && self.storage.len() <= 64
            && self.storage.as_bytes()[0].is_ascii_alphanumeric()
            && self
                .storage
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            && !self.bridge.is_empty()
            && self.bridge.len() <= 15
            && self.bridge.as_bytes()[0].is_ascii_alphanumeric()
            && self
                .bridge
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    }
    pub fn matches(&self, plan: &Plan) -> bool {
        self.definition_name == plan.definition_name
            && self.definition_hash == plan.definition_hash
            && self.plan_hash == plan.plan_hash
            && self.target_node == plan.target.node
            && self.target_vmid == plan.target.vmid
            && self.source_vmid == plan.source.template_vmid
            && self.storage == plan.target.storage
            && self.bridge == plan.target.bridge
    }
    pub fn input_hash(&self) -> Result<String, serde_json::Error> {
        use sha2::{Digest, Sha256};
        let mut bytes = b"vmanyware-provision-v1\0".to_vec();
        bytes.extend(serde_json::to_vec(self)?);
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}
#[derive(Debug)]
pub struct Stage {
    pub position: usize,
    pub kind: String,
    pub status: Status,
    pub dispatch_started: bool,
    pub task: Option<TaskReference>,
    pub result: Option<serde_json::Value>,
    pub config_digest: Option<String>,
}
#[derive(Debug)]
pub struct Job {
    pub recreated_from: Option<String>,
    pub operation: Operation,
    pub approval: Approval,
    pub confirmed_at: i64,
    pub definition: Definition,
    pub catalog: Catalog,
    pub plan: Plan,
    pub stages: Vec<Stage>,
    pub disk_dispatch: Option<DiskDispatch>,
    pub cloud_init_dispatch: Option<CloudInitDispatch>,
    pub(crate) seed_journal: Option<SeedJournal>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneResult {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub task_status: crate::proxmox::TaskStatus,
    pub target: crate::proxmox::CloneObservation,
}
pub enum CloneOutcome {
    Completed(CloneResult),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceResult {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub resources: crate::proxmox::ResourceObservation,
}
pub enum ResourceOutcome {
    Completed(ResourceResult),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}

impl Job {
    /// Validate retained clone evidence before advancing to a new mutating stage.
    pub fn cloned_config_digest(&self) -> Option<String> {
        use crate::proxmox::{PowerState, QmpState, TaskStatus};
        let clone = self.stages.first()?;
        if clone.status != Status::Succeeded || !clone.dispatch_started || clone.task.is_none() {
            return None;
        }
        let evidence: CloneResult = serde_json::from_value(clone.result.clone()?).ok()?;
        if Some(evidence.instance_id.as_str()) != self.operation.instance_id.as_deref()
            || evidence.provider_scope_id != self.approval.provider_scope_id
            || evidence.plan_hash != self.approval.plan_hash
            || evidence.task_status != TaskStatus::Succeeded
            || evidence.target.observation.node != self.approval.target_node
            || evidence.target.observation.vmid != self.approval.target_vmid
            || evidence.target.observation.status != PowerState::Stopped
            || !matches!(
                evidence.target.observation.qmp_status,
                None | Some(QmpState::Stopped)
            )
            || evidence.target.config_digest.len() != 40
            || !evidence
                .target
                .config_digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return None;
        }
        Some(evidence.target.config_digest)
    }
}

#[derive(Debug, Clone)]
pub struct DiskDispatch {
    pub config_digest: String,
    pub volume: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskResult {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub task_status: crate::proxmox::TaskStatus,
    pub disk: crate::proxmox::DiskObservation,
}
pub enum DiskOutcome {
    Completed(DiskResult),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}
impl Job {
    pub fn resource_config_digest(&self) -> Option<String> {
        use crate::proxmox::{PowerState, QmpState};
        let source = self.cloned_config_digest()?;
        let stage = self.stages.get(1)?;
        if stage.status != Status::Succeeded
            || !stage.dispatch_started
            || stage.config_digest.as_deref() != Some(source.as_str())
        {
            return None;
        }
        let evidence: ResourceResult = serde_json::from_value(stage.result.clone()?).ok()?;
        let target = &evidence.resources.target;
        if Some(evidence.instance_id.as_str()) != self.operation.instance_id.as_deref()
            || evidence.provider_scope_id != self.approval.provider_scope_id
            || evidence.plan_hash != self.approval.plan_hash
            || evidence.resources.cpu != self.plan.target.resources.cpu
            || evidence.resources.memory_mib != self.plan.target.resources.memory_mib
            || target.observation.node != self.approval.target_node
            || target.observation.vmid != self.approval.target_vmid
            || target.observation.status != PowerState::Stopped
            || !matches!(
                target.observation.qmp_status,
                None | Some(QmpState::Stopped)
            )
            || target.config_digest.len() != 40
            || !target.config_digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return None;
        }
        Some(target.config_digest.clone())
    }
}

#[derive(Debug, Clone)]
pub struct CloudInitDispatch {
    pub config_digest: String,
    pub volume: String,
    pub network: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitResult {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub settings: crate::proxmox::CloudInitObservation,
}
pub enum CloudInitOutcome {
    Completed(Box<CloudInitResult>),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}
impl Job {
    /// Source revision and, when expanded, exact volume after validated prior stages.
    pub fn cloud_init_source(&self) -> Option<(String, Option<String>)> {
        use crate::proxmox::{PowerState, QmpState, TaskStatus};
        let digest = self.resource_config_digest()?;
        if !self.stages.iter().any(|s| s.kind == "expand_disk") {
            return Some((digest, None));
        }
        let stage = self.stages.get(2)?;
        let retained = self.disk_dispatch.as_ref()?;
        if stage.status != Status::Succeeded
            || !stage.dispatch_started
            || stage.task.is_none()
            || retained.config_digest != digest
        {
            return None;
        }
        let evidence: DiskResult = serde_json::from_value(stage.result.clone()?).ok()?;
        let target = &evidence.disk.resources.target;
        let device = match self.plan.source.disk_device {
            crate::provisioning::DiskDevice::Scsi0 => "scsi0",
            crate::provisioning::DiskDevice::Virtio0 => "virtio0",
            crate::provisioning::DiskDevice::Sata0 => "sata0",
        };
        if Some(evidence.instance_id.as_str()) != self.operation.instance_id.as_deref()
            || evidence.provider_scope_id != self.approval.provider_scope_id
            || evidence.plan_hash != self.approval.plan_hash
            || evidence.task_status != TaskStatus::Succeeded
            || evidence.disk.device != device
            || evidence.disk.volume != retained.volume
            || evidence.disk.size_bytes
                != u64::from(self.plan.target.resources.disk_gib) * 1024_u64.pow(3)
            || evidence.disk.resources.cpu != self.plan.target.resources.cpu
            || evidence.disk.resources.memory_mib != self.plan.target.resources.memory_mib
            || target.observation.node != self.approval.target_node
            || target.observation.vmid != self.approval.target_vmid
            || target.observation.status != PowerState::Stopped
            || !matches!(
                target.observation.qmp_status,
                None | Some(QmpState::Stopped)
            )
            || target.config_digest.len() != 40
            || !target.config_digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return None;
        }
        Some((target.config_digest.clone(), Some(retained.volume.clone())))
    }
}

/// Private durable substep of start_vm. A settings-only result is never seed proof.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct SeedJournal {
    pub expected: crate::proxmox::SeedPreflight,
    pub completed: Option<crate::proxmox::SeedPreflight>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<PowerDispatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness: Option<ReadinessJournal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packages: Option<PackageJournal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_digest: Option<String>,
}
pub enum SeedOutcome {
    Completed(crate::proxmox::SeedPreflight),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}
impl Job {
    pub fn cloud_init_settings(&self) -> Option<CloudInitResult> {
        use crate::proxmox::{PowerState, QmpState};
        self.cloud_init_source()?;
        let stage = self
            .stages
            .iter()
            .find(|s| s.kind == "configure_cloud_init")?;
        if stage.status != Status::Succeeded || !stage.dispatch_started {
            return None;
        }
        let retained = self.cloud_init_dispatch.as_ref()?;
        let evidence: CloudInitResult = serde_json::from_value(stage.result.clone()?).ok()?;
        let settings = &evidence.settings;
        let target = &settings.resources.target;
        let request = self
            .plan
            .cloud_init_request(
                &self.definition,
                self.operation.instance_id.as_deref()?,
                &retained.config_digest,
                &retained.volume,
                &retained.network,
            )
            .ok()?;
        if Some(evidence.instance_id.as_str()) != self.operation.instance_id.as_deref()
            || evidence.provider_scope_id != self.approval.provider_scope_id
            || evidence.plan_hash != self.approval.plan_hash
            || settings.primary_volume != retained.volume
            || settings.disk_size_bytes
                != u64::from(self.plan.target.resources.disk_gib) * 1024_u64.pow(3)
            || settings.network != request.expected_network()
            || settings.guest_user != self.plan.target.guest_user
            || settings.ssh_key_count != self.definition.spec.provisioning.ssh_authorized_keys.len()
            || settings.guest_ready
            || settings.resources.cpu != self.plan.target.resources.cpu
            || settings.resources.memory_mib != self.plan.target.resources.memory_mib
            || target.observation.node != self.approval.target_node
            || target.observation.vmid != self.approval.target_vmid
            || target.observation.status != PowerState::Stopped
            || !matches!(
                target.observation.qmp_status,
                None | Some(QmpState::Stopped)
            )
            || target.config_digest.len() != 40
            || !target.config_digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return None;
        }
        Some(evidence)
    }
    pub(crate) fn valid_seed(&self, proof: &crate::proxmox::SeedPreflight) -> bool {
        use crate::proxmox::SeedUserFormat;
        proof.compatible
            && !proof.guest_ready
            && proof.config_digest.len() == 40
            && proof.config_digest.bytes().all(|b| b.is_ascii_hexdigit())
            && crate::instance::valid_hash(&proof.user_data_hash)
            && crate::instance::valid_hash(&proof.network_data_hash)
            && proof.user_format
                == if self.plan.source.image_default_user.is_some() {
                    SeedUserFormat::ImageDefault
                } else {
                    SeedUserFormat::Named
                }
    }
    /// Necessary start gate; the start executor must still check current provider state.
    pub fn regenerated_seed(&self) -> Option<crate::proxmox::SeedPreflight> {
        let settings = self.cloud_init_settings()?;
        let stage = self.stages.iter().find(|s| s.kind == "start_vm")?;
        if !stage.dispatch_started {
            return None;
        }
        let journal = self.seed_journal.as_ref()?;
        let completed = journal.completed.as_ref()?;
        if !self.valid_seed(&journal.expected)
            || !self.valid_seed(completed)
            || journal.expected.config_digest != settings.settings.resources.target.config_digest
            || journal.expected.user_data_hash != completed.user_data_hash
            || journal.expected.network_data_hash != completed.network_data_hash
            || journal.expected.user_format != completed.user_format
        {
            return None;
        }
        Some(completed.clone())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PowerDispatch {
    pub config_digest: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StartResult {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub task_status: crate::proxmox::TaskStatus,
    pub observation: crate::proxmox::VmObservation,
    pub guest_ready: bool,
}
pub enum StartOutcome {
    Completed(StartResult),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}
impl Job {
    pub fn power_dispatch_started(&self) -> bool {
        self.seed_journal
            .as_ref()
            .and_then(|j| j.power.as_ref())
            .is_some_and(|p| p.config_digest.is_some())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ReadinessJournal {
    pub started_at: i64,
    pub deadline: i64,
    pub probe_started: bool,
    pub pid: Option<u32>,
    pub attempts: u16,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReadinessResult {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub guest: crate::proxmox::GuestReadiness,
    pub guest_ready: bool,
}
pub enum ReadinessOutcome {
    Completed(ReadinessResult),
    Pending { process_exited: bool },
    Uncertain,
    TimedOut,
    Rejected,
}
impl Job {
    pub fn started(&self) -> Option<StartResult> {
        use crate::proxmox::{PowerState, QmpState, TaskStatus};
        self.regenerated_seed()?;
        let stage = self.stages.iter().find(|s| s.kind == "start_vm")?;
        if stage.status != Status::Succeeded
            || stage.task.is_none()
            || !self.power_dispatch_started()
        {
            return None;
        }
        let proof: StartResult = serde_json::from_value(stage.result.clone()?).ok()?;
        if Some(proof.instance_id.as_str()) != self.operation.instance_id.as_deref()
            || proof.provider_scope_id != self.approval.provider_scope_id
            || proof.plan_hash != self.approval.plan_hash
            || proof.task_status != TaskStatus::Succeeded
            || proof.observation.node != self.approval.target_node
            || proof.observation.vmid != self.approval.target_vmid
            || proof.observation.status != PowerState::Running
            || !matches!(proof.observation.qmp_status, None | Some(QmpState::Running))
            || proof.guest_ready
        {
            return None;
        }
        Some(proof)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecreationHistory {
    pub source_instance_id: String,
    pub operations: Vec<RecreationSummary>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecreationSummary {
    pub id: String,
    pub instance_id: String,
    pub status: Status,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct PackageJournal {
    pub started_at: i64,
    pub deadline: i64,
    pub pid: Option<u32>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PackageResult {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub plan_hash: String,
    pub package_count: usize,
    pub installed: bool,
}
pub enum PackageOutcome {
    Completed,
    Pending,
    Uncertain,
    Failed(crate::operation::Failure),
    TimedOut,
}
impl Job {
    pub fn guest_readiness(&self) -> Option<ReadinessResult> {
        self.started()?;
        let stage = self
            .stages
            .iter()
            .find(|s| s.kind == "wait_for_guest_readiness")?;
        let journal = self.seed_journal.as_ref()?.readiness.as_ref()?;
        if stage.status != Status::Succeeded
            || !stage.dispatch_started
            || !journal.probe_started
            || journal.pid.is_none()
        {
            return None;
        }
        let proof: ReadinessResult = serde_json::from_value(stage.result.clone()?).ok()?;
        let ip = proof.guest.ipv4;
        if Some(proof.instance_id.as_str()) != self.operation.instance_id.as_deref()
            || proof.provider_scope_id != self.approval.provider_scope_id
            || proof.plan_hash != self.approval.plan_hash
            || !proof.guest_ready
            || !proof.guest.cloud_init_clean
            || proof.guest.cloud_init_exit_code != 0
            || ip.is_unspecified()
            || ip.is_loopback()
            || ip.is_link_local()
            || ip.is_broadcast()
            || ip.octets()[0] == 0
            || ip.octets()[0] >= 224
            || proof.guest.interface.is_empty()
            || proof.guest.interface.len() > 15
            || !proof
                .guest
                .interface
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return None;
        }
        Some(proof)
    }
    pub fn installed_packages(&self) -> Option<PackageResult> {
        self.guest_readiness()?;
        let stage = self
            .stages
            .iter()
            .find(|s| s.kind == "install_declared_packages")?;
        if stage.status != Status::Succeeded
            || !stage.dispatch_started
            || self.seed_journal.as_ref()?.packages.as_ref()?.pid.is_none()
        {
            return None;
        }
        let proof: PackageResult = serde_json::from_value(stage.result.clone()?).ok()?;
        if Some(proof.instance_id.as_str()) != self.operation.instance_id.as_deref()
            || proof.provider_scope_id != self.approval.provider_scope_id
            || proof.plan_hash != self.approval.plan_hash
            || !proof.installed
            || proof.package_count == 0
            || proof.package_count != self.definition.spec.provisioning.packages.len()
        {
            return None;
        }
        Some(proof)
    }
}

impl Job {
    pub fn final_snapshot_request(&self) -> Option<crate::snapshot::CreateRequest> {
        if !self.definition.spec.lifecycle.snapshot_after_provisioning
            || self.operation.id.len() != 32
            || !self
                .operation
                .id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return None;
        }
        Some(crate::snapshot::CreateRequest {
            name: format!("ready_{}", self.operation.id),
            description: format!(
                "VMAnyware provisioning {}\ndefinition-sha256: {}\nplan-sha256: {}",
                self.operation.id, self.approval.definition_hash, self.approval.plan_hash
            ),
        })
    }
    pub fn final_snapshot_prerequisites(&self) -> bool {
        self.final_snapshot_request().is_some()
            && self.guest_readiness().is_some()
            && (self.definition.spec.provisioning.packages.is_empty()
                || self.installed_packages().is_some())
    }
    pub(crate) fn valid_final_snapshot(&self, proof: &crate::snapshot::ResultEvidence) -> bool {
        let Some(request) = self.final_snapshot_request() else {
            return false;
        };
        self.final_snapshot_prerequisites()
            && Some(proof.instance_id.as_str()) == self.operation.instance_id.as_deref()
            && proof.provider_scope_id == self.approval.provider_scope_id
            && proof.node == self.approval.target_node
            && proof.vmid == self.approval.target_vmid
            && proof.task_status == crate::proxmox::TaskStatus::Succeeded
            && proof.snapshot.name == request.name
            && proof.snapshot.description == request.description
            && proof.snapshot.includes_ram == Some(false)
            && proof.snapshot.provider_state.is_none()
    }
}
