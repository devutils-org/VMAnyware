//! Operation status describes job execution, never guest/SSH readiness.
use serde::{Deserialize, Serialize};

pub(crate) fn valid_request_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
    Uncertain,
}

impl Status {
    pub fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Running)
                | (Self::Uncertain, Self::Running)
                | (
                    Self::Running,
                    Self::Succeeded | Self::Failed | Self::Interrupted | Self::Uncertain
                )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    ProviderNotConfigured,
    ProviderUnauthorized,
    ProviderForbidden,
    ProviderTimeout,
    ProviderUnavailable,
    ProcessInterrupted,
    ProviderScopeChanged,
    InstanceReferenceInvalid,
    LifecycleDisabled,
    SnapshotDisabled,
    RestoreDisabled,
    DeletionDisabled,
    DeletionTargetChanged,
    DeletionPreconditionsFailed,
    RestoreTargetChanged,
    RestorePreconditionsFailed,
    SnapshotTargetChanged,
    SnapshotPreconditionsFailed,
    ProvisioningDisabled,
    ProvisioningPlanChanged,
    ProvisioningPreconditionsFailed,
    ApprovalExpired,
    LifecycleTargetChanged,
    LifecyclePreconditionsFailed,
    ProviderSubmissionUnconfirmed,
    ProviderTaskFailed,
    ProviderTaskUnconfirmed,
    PowerStateUnconfirmed,
    SnapshotStateUnconfirmed,
    RestoreStateUnconfirmed,
    DeletionStateUnconfirmed,
    ResourceStateUnconfirmed,
    DiskStateUnconfirmed,
    CloudInitStateUnconfirmed,
    SeedRegenerationUnconfirmed,
    GuestReadinessUnconfirmed,
    GuestReadinessTimedOut,
    GuestReadinessFailed,
}

impl Failure {
    pub fn message(self) -> &'static str {
        match self {
            Self::ProviderNotConfigured => "Proxmox is not configured on the control plane.",
            Self::ProviderUnauthorized => "Proxmox rejected the configured API token.",
            Self::ProviderForbidden => "The Proxmox token lacks permissions for this read.",
            Self::ProviderTimeout => "Proxmox read timed out.",
            Self::ProviderUnavailable => "Cannot read Proxmox; check endpoint and TLS trust.",
            Self::ProviderScopeChanged => "The configured Proxmox endpoint differs from the registered instance.",
            Self::InstanceReferenceInvalid => "The registered instance reference cannot be used; no provider request was made.",
            Self::ProvisioningDisabled => "Provisioning dispatch is disabled on this control plane.",
            Self::ProvisioningPlanChanged => "The confirmed provisioning plan differs from the configured catalog.",
            Self::ProvisioningPreconditionsFailed => "Current source, storage, network or target checks do not permit this provisioning stage.",
            Self::DeletionDisabled => "Deletion dispatch is disabled on this control plane.",
            Self::DeletionTargetChanged => "The confirmed deletion target, configuration or reviewed cleanup scope changed.",
            Self::DeletionPreconditionsFailed => "Current stopped-target, ownership, cleanup scope or read permissions do not permit deletion.",
            Self::RestoreDisabled => "Snapshot restore dispatch is disabled on this control plane.",
            Self::RestoreTargetChanged => "The confirmed restore target, live configuration or selected snapshot changed.",
            Self::RestorePreconditionsFailed => "Current stopped-VM/disk-only snapshot checks do not permit restore.",
            Self::SnapshotDisabled => "Snapshot creation dispatch is disabled on this control plane.",
            Self::SnapshotTargetChanged => "The confirmed snapshot target or configuration changed.",
            Self::SnapshotPreconditionsFailed => "Snapshot capability, name or target checks do not permit creation.",
            Self::LifecycleDisabled => "Lifecycle dispatch is disabled on this control plane.",
            Self::ApprovalExpired => "The undispatched action approval expired. Review the target before submitting a new action.",
            Self::LifecycleTargetChanged => "The approved target or configuration differs from the current instance.",
            Self::LifecyclePreconditionsFailed => "Current lifecycle preconditions do not permit this action.",
            Self::ProviderSubmissionUnconfirmed => "A provider submission may have occurred. It will not be repeated automatically.",
            Self::ProviderTaskFailed => "The provider task failed. Inspect the target before another action.",
            Self::ProviderTaskUnconfirmed => "The submitted provider task outcome remains unconfirmed. No write will be repeated.",
            Self::GuestReadinessUnconfirmed => "Guest initialization is not confirmed. The retained probe and deadline remain unchanged.",
            Self::GuestReadinessFailed => "Guest readiness checks failed. Inspect the retained VM and its Cloud-Init status before retrying.",
            Self::GuestReadinessTimedOut => "Guest readiness did not complete before its retained deadline. The VM is retained for inspection.",
            Self::SeedRegenerationUnconfirmed => "Cloud-Init seed regeneration remains unconfirmed. No write will be repeated.",
            Self::CloudInitStateUnconfirmed => "Requested network and Cloud-Init settings remain unconfirmed. No write will be repeated.",
            Self::DiskStateUnconfirmed => "Requested VM disk size remains unconfirmed. No write will be repeated.",
            Self::ResourceStateUnconfirmed => "Requested VM resources remain unconfirmed. No write will be repeated.",
            Self::DeletionStateUnconfirmed => "VM or reviewed volume absence remains unconfirmed. No delete will be repeated; inspect partial cleanup.",
            Self::RestoreStateUnconfirmed => "Provider rollback completion or the selected snapshot/stopped target remains unconfirmed. No write will be repeated.",
            Self::SnapshotStateUnconfirmed => "Provider task completed, but the exact disk-only snapshot metadata remains unconfirmed. No write will be repeated.",
            Self::PowerStateUnconfirmed => "Provider task completion was observed, but the requested VM power state remains unconfirmed.",
            Self::ProcessInterrupted => "The service stopped during this operation. Its outcome is unconfirmed; start a new read-only refresh.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub provider_task_known: bool,
    pub status: Status,
    pub result: Option<serde_json::Value>,
    pub failure: Option<Failure>,
    pub created_at: String,
    pub updated_at: String,
}

/// Bounded history excludes potentially large inventory result payloads.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub provider_task_known: bool,
    pub status: Status,
    pub failure: Option<Failure>,
    pub created_at: String,
    pub updated_at: String,
}

impl Operation {
    pub fn message(&self) -> Option<&'static str> {
        self.failure.map(Failure::message)
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
