//! Retained full-scope destructive consent. No provider URL or credentials enter IPC.
use super::{hex, identifier, snapshot_restore::Status, Profile, RemoteConnection, RemoteProvider};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Instant};
#[derive(Default)]
pub(super) struct DeletionState {
    revision: u64,
    prepared: Option<Prepared>,
}
struct Prepared {
    key: String,
    review: Review,
    submitted: bool,
    inflight: bool,
    operation: Option<String>,
    terminal: bool,
    prepared_at: Instant,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Passed,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Check {
    code: String,
    status: CheckStatus,
}
const CHECKS: [&str; 16] = [
    "instance_marker",
    "not_template",
    "unlocked",
    "no_pending_changes",
    "configuration_unchanged",
    "delete_vm_stopped",
    "delete_not_protected",
    "delete_scope_complete",
    "delete_volumes_owned",
    "delete_snapshots_complete",
    "delete_scope_stable",
    "delete_target_unchanged",
    "delete_not_ha_managed",
    "delete_no_replication_jobs",
    "delete_management_stable",
    "delete_provider_effect_scope",
];
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    node: String,
    vmid: u32,
    status: String,
    qmp_status: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Effects {
    version: u32,
    remove_vm_configuration: bool,
    remove_snapshot_metadata: bool,
    remove_vm_permissions: bool,
    remove_vm_firewall: bool,
    cleanup_ipam: bool,
    cleanup_recorded_fleecing_images: bool,
    purge_jobs: bool,
    destroy_unreferenced_disks: bool,
    retain_definition_and_history: bool,
}
impl Effects {
    fn valid(&self) -> bool {
        self.version == 1
            && self.remove_vm_configuration
            && self.remove_snapshot_metadata
            && self.remove_vm_permissions
            && self.remove_vm_firewall
            && self.cleanup_ipam
            && self.cleanup_recorded_fleecing_images
            && !self.purge_jobs
            && !self.destroy_unreferenced_disks
            && self.retain_definition_and_history
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Effect {
    DeleteOwnedVolume,
    FleecingCleanup,
    SnapshotReference,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Volume {
    volume: String,
    references: Vec<String>,
    owner_verified: bool,
    effect: Effect,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Plan {
    scope_hash: String,
    volumes: Vec<Volume>,
    snapshots: Vec<String>,
    preserved_media: Vec<String>,
    preserved_owned_volumes: Vec<String>,
    effects: Effects,
}
fn volume_id(value: &str) -> bool {
    let Some((storage, name)) = value.split_once(':') else {
        return false;
    };
    identifier(storage, 64)
        && storage.as_bytes()[0].is_ascii_alphabetic()
        && !storage.contains('.')
        && value.len() <= 256
        && !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
        && name
            .split('/')
            .all(|part| !part.is_empty() && !matches!(part, "." | ".."))
}
fn drive_key(key: &str) -> bool {
    key == "vmstate"
        || [
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
                    .parse::<u32>()
                    .is_ok_and(|value| value < *limit && value.to_string() == index)
            })
        })
}
fn sorted<T: Ord>(items: &[T]) -> bool {
    items.windows(2).all(|pair| pair[0] < pair[1])
}
impl Plan {
    fn valid(&self, complete: bool) -> bool {
        if !hex(&self.scope_hash, 64)
            || !self.effects.valid()
            || self.volumes.len() + self.preserved_owned_volumes.len() > 256
            || self.snapshots.len() > 64
            || self.preserved_media.len() > 256
            || !sorted(&self.snapshots)
            || !sorted(&self.preserved_media)
            || !sorted(&self.preserved_owned_volumes)
            || !self.snapshots.iter().all(|name| {
                !name.is_empty()
                    && name.len() <= 255
                    && name != "current"
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            })
            || !self
                .volumes
                .windows(2)
                .all(|pair| pair[0].volume < pair[1].volume)
        {
            return false;
        }
        let mut seen = BTreeSet::new();
        for name in self
            .preserved_media
            .iter()
            .chain(&self.preserved_owned_volumes)
        {
            if !volume_id(name) || !seen.insert(name.as_str()) {
                return false;
            }
        }
        for volume in &self.volumes {
            if !volume_id(&volume.volume)
                || !seen.insert(&volume.volume)
                || (complete && !volume.owner_verified)
                || volume.references.is_empty()
                || !sorted(&volume.references)
            {
                return false;
            }
            for reference in &volume.references {
                let valid = if let Some(key) = reference.strip_prefix("live:") {
                    key == "special/fleecing" || drive_key(key)
                } else if let Some((name, key)) = reference
                    .strip_prefix("snapshot/")
                    .and_then(|r| r.split_once(':'))
                {
                    self.snapshots.iter().any(|s| s == name) && drive_key(key)
                } else {
                    false
                };
                if !valid {
                    return false;
                }
            }
            if complete {
                let fleecing = volume
                    .references
                    .iter()
                    .any(|r| r == "live:special/fleecing");
                match volume.effect {
                    Effect::FleecingCleanup if fleecing => {}
                    Effect::DeleteOwnedVolume
                        if !fleecing
                            && volume
                                .references
                                .iter()
                                .any(|r| r.starts_with("live:") || r.ends_with(":vmstate")) => {}
                    _ => return false,
                }
            }
        }
        true
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    instance_id: String,
    provider_scope_id: String,
    definition_hash: String,
    node: String,
    vmid: u32,
    config_digest: String,
    scope_hash: String,
    observation: Observation,
    volumes: Vec<Volume>,
    snapshots: Vec<String>,
    preserved_media: Vec<String>,
    preserved_owned_volumes: Vec<String>,
    effects: Effects,
    ha_managed: bool,
    replication_jobs: Vec<String>,
    destructive: bool,
    definition_retained: bool,
    purge: bool,
    destroy_unreferenced_disks: bool,
    checks: Vec<Check>,
    deletion_preconditions: CheckStatus,
    execution_enabled: bool,
}
// Field order matches the server's versioned SHA-256 deletion approval contract.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Approval {
    pub(super) instance_id: String,
    pub(super) provider_scope_id: String,
    pub(super) definition_hash: String,
    pub(super) node: String,
    pub(super) vmid: u32,
    pub(super) config_digest: String,
    pub(super) plan: Plan,
    pub(super) confirmed: bool,
    pub(super) acknowledge_data_loss: bool,
}
impl Approval {
    pub(super) fn valid(&self) -> bool {
        hex(&self.instance_id, 32)
            && hex(&self.provider_scope_id, 64)
            && hex(&self.definition_hash, 64)
            && identifier(&self.node, 63)
            && (100..=999999999).contains(&self.vmid)
            && hex(&self.config_digest, 40)
            && self.confirmed
            && self.acknowledge_data_loss
            && self.plan.valid(true)
            && serde_json::to_vec(self).is_ok_and(|bytes| bytes.len() <= 64 * 1024)
    }
    pub(super) fn hash(&self) -> Option<String> {
        use sha2::{Digest, Sha256};
        let mut material = b"vmanyware-delete-v1\0".to_vec();
        material.extend(serde_json::to_vec(self).ok()?);
        Some(format!("{:x}", Sha256::digest(material)))
    }
}

impl Review {
    fn plan(&self) -> Plan {
        Plan {
            scope_hash: self.scope_hash.clone(),
            volumes: self.volumes.clone(),
            snapshots: self.snapshots.clone(),
            preserved_media: self.preserved_media.clone(),
            preserved_owned_volumes: self.preserved_owned_volumes.clone(),
            effects: self.effects.clone(),
        }
    }
    fn approval(&self) -> Approval {
        Approval {
            instance_id: self.instance_id.clone(),
            provider_scope_id: self.provider_scope_id.clone(),
            definition_hash: self.definition_hash.clone(),
            node: self.node.clone(),
            vmid: self.vmid,
            config_digest: self.config_digest.clone(),
            plan: self.plan(),
            confirmed: true,
            acknowledge_data_loss: true,
        }
    }
    fn approval_hash(&self) -> Option<String> {
        self.approval().hash()
    }
    fn valid(&self) -> bool {
        let passed = self.deletion_preconditions == CheckStatus::Passed;
        hex(&self.instance_id, 32)
            && hex(&self.provider_scope_id, 64)
            && hex(&self.definition_hash, 64)
            && identifier(&self.node, 63)
            && (100..=999999999).contains(&self.vmid)
            && hex(&self.config_digest, 40)
            && self.destructive
            && self.definition_retained
            && !self.purge
            && !self.destroy_unreferenced_disks
            && self.plan().valid(passed)
            && self.observation.node == self.node
            && self.observation.vmid == self.vmid
            && matches!(self.observation.status.as_str(), "running" | "stopped")
            && self
                .observation
                .qmp_status
                .as_ref()
                .is_none_or(|s| s.len() <= 64 && !s.chars().any(char::is_control))
            && self.replication_jobs.len() <= 4096
            && sorted(&self.replication_jobs)
            && self
                .replication_jobs
                .iter()
                .all(|id| id.len() <= 64 && id.bytes().all(|b| b.is_ascii_digit() || b == b'-'))
            && self.checks.len() == CHECKS.len()
            && CHECKS.iter().all(|code| {
                self.checks
                    .iter()
                    .filter(|check| check.code == *code)
                    .count()
                    == 1
            })
            && (!passed
                || (!self.ha_managed
                    && self.replication_jobs.is_empty()
                    && self.observation.status == "stopped"
                    && self
                        .observation
                        .qmp_status
                        .as_deref()
                        .is_none_or(|s| s == "stopped")
                    && self.checks.iter().all(|c| c.status == CheckStatus::Passed)))
            && serde_json::to_vec(&self.approval()).is_ok_and(|bytes| bytes.len() <= 64 * 1024)
    }
    fn eligible(&self) -> bool {
        self.valid() && self.execution_enabled && self.deletion_preconditions == CheckStatus::Passed
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TargetStatus {
    Registered,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    id: String,
    definition_hash: String,
    provider_reference: RemoteProvider,
    status: TargetStatus,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Failure {
    DeletionDisabled,
    DeletionTargetChanged,
    DeletionPreconditionsFailed,
    ApprovalExpired,
    ProviderNotConfigured,
    ProviderUnauthorized,
    ProviderForbidden,
    ProviderTimeout,
    ProviderUnavailable,
    ProviderScopeChanged,
    InstanceReferenceInvalid,
    ProviderSubmissionUnconfirmed,
    ProviderTaskFailed,
    ProviderTaskUnconfirmed,
    DeletionStateUnconfirmed,
    ProcessInterrupted,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    id: String,
    kind: String,
    instance_id: String,
    status: Status,
    #[serde(default)]
    provider_task_known: bool,
    result: Option<Evidence>,
    failure: Option<Failure>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Evidence {
    instance_id: String,
    provider_scope_id: String,
    node: String,
    vmid: u32,
    approval_hash: String,
    task_status: TaskStatus,
    vm_absence_verified: bool,
    volumes_absence_verified: bool,
    definition_retained: bool,
    association_retired: bool,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Succeeded,
}
impl Operation {
    fn valid(&self, review: &Review, expected: Option<&str>) -> bool {
        review.approval_hash().is_some() && self.valid_context(&review.approval(), expected)
    }
    pub(super) fn valid_context(&self, approval: &Approval, expected: Option<&str>) -> bool {
        if !approval.valid()
            || !hex(&self.id, 32)
            || self.kind != "vm_delete"
            || self.instance_id != approval.instance_id
            || expected.is_some_and(|id| id != self.id)
            || (self.status == Status::Queued && self.provider_task_known)
        {
            return false;
        }
        match self.status {
            Status::Succeeded => {
                self.provider_task_known
                    && self.failure.is_none()
                    && self.result.as_ref().is_some_and(|r| {
                        r.instance_id == approval.instance_id
                            && r.provider_scope_id == approval.provider_scope_id
                            && r.node == approval.node
                            && r.vmid == approval.vmid
                            && Some(r.approval_hash.clone()) == approval.hash()
                            && r.vm_absence_verified
                            && r.volumes_absence_verified
                            && r.definition_retained
                            && r.association_retired
                    })
            }
            Status::Queued | Status::Running => self.result.is_none() && self.failure.is_none(),
            Status::Interrupted => {
                !self.provider_task_known
                    && self.result.is_none()
                    && self.failure == Some(Failure::ProcessInterrupted)
            }
            Status::Uncertain => {
                self.result.is_none()
                    && self.failure.as_ref().is_some_and(|f| {
                        if !self.provider_task_known {
                            *f == Failure::ProviderSubmissionUnconfirmed
                        } else {
                            matches!(
                                f,
                                Failure::ProviderTaskUnconfirmed
                                    | Failure::DeletionStateUnconfirmed
                                    | Failure::ProcessInterrupted
                                    | Failure::ProviderScopeChanged
                                    | Failure::ProviderNotConfigured
                                    | Failure::ProviderTaskFailed
                            )
                        }
                    })
            }
            Status::Failed => {
                !self.provider_task_known
                    && self.result.is_none()
                    && self.failure.as_ref().is_some_and(|f| {
                        matches!(
                            f,
                            Failure::DeletionDisabled
                                | Failure::DeletionTargetChanged
                                | Failure::DeletionPreconditionsFailed
                                | Failure::ApprovalExpired
                                | Failure::ProviderNotConfigured
                                | Failure::ProviderScopeChanged
                                | Failure::InstanceReferenceInvalid
                                | Failure::ProviderUnauthorized
                                | Failure::ProviderForbidden
                                | Failure::ProviderTimeout
                                | Failure::ProviderUnavailable
                        )
                    })
            }
        }
    }
    pub(super) fn consistent_submission(&self, started: bool) -> bool {
        (!self.provider_task_known || started)
            && match self.status {
                Status::Queued | Status::Interrupted => !started,
                Status::Succeeded | Status::Uncertain => started,
                Status::Running => true,
                Status::Failed => {
                    !started
                        || matches!(
                            self.failure,
                            Some(Failure::ProviderUnauthorized | Failure::ProviderForbidden)
                        )
                }
            }
    }
    fn terminal(&self) -> bool {
        matches!(
            self.status,
            Status::Succeeded | Status::Failed | Status::Interrupted
        )
    }
}
impl RemoteConnection {
    fn deletion_revision(&self, id: u64, revision: u64) -> Result<(), String> {
        let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id || state.deletion.revision != revision {
            return Err("Discarded an outdated deletion preparation.".into());
        }
        Ok(())
    }
    async fn prepare_deletion(
        &self,
        id: u64,
        key: String,
        instance_id: String,
    ) -> Result<Review, String> {
        if key.is_empty()
            || key.len() > 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || !hex(&instance_id, 32)
        {
            return Err("Supply a valid deletion request, instance and request key.".into());
        }
        let profile = self.snapshot(id)?;
        let revision = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let deletion = &mut state.deletion;
            if deletion
                .prepared
                .as_ref()
                .is_some_and(|intent| intent.submitted)
            {
                return Err("Read or retry the submitted deletion before preparing another. Disconnect does not cancel it.".into());
            }
            deletion.revision = deletion
                .revision
                .checked_add(1)
                .ok_or("Deletion state unavailable.")?;
            deletion.prepared = None;
            deletion.revision
        };
        let target: Target = profile
            .deletion_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/environments/{instance_id}"))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await?;
        self.deletion_revision(id, revision)?;
        let reference = &target.provider_reference;
        if target.id != instance_id
            || !hex(&target.definition_hash, 64)
            || !matches!(target.status, TargetStatus::Registered)
            || reference.provider != "proxmox"
            || !hex(&reference.scope_id, 64)
            || !identifier(&reference.node, 63)
            || !(100..=999999999).contains(&reference.vmid)
        {
            return Err("The server returned an invalid deletion target.".into());
        }
        let review: Review = profile
            .deletion_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!(
                            "api/v1/environments/{instance_id}/delete-preflight"
                        ))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await?;
        self.deletion_revision(id, revision)?;
        if !review.valid()
            || review.instance_id != instance_id
            || review.provider_scope_id != reference.scope_id
            || review.node != reference.node
            || review.vmid != reference.vmid
            || review.definition_hash != target.definition_hash
        {
            return Err("The server returned a mismatched deletion review.".into());
        }
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id || state.deletion.revision != revision {
            return Err("Discarded an outdated deletion preparation.".into());
        }
        state.deletion.prepared = Some(Prepared {
            key,
            review: review.clone(),
            submitted: false,
            inflight: false,
            operation: None,
            terminal: false,
            prepared_at: std::time::Instant::now(),
        });
        Ok(review)
    }
    async fn confirm_deletion(
        &self,
        id: u64,
        key: &str,
        confirmed: bool,
        acknowledge_data_loss: bool,
    ) -> Result<Operation, String> {
        if !confirmed || !acknowledge_data_loss {
            return Err("Confirm the displayed VM and reviewed volumes, and acknowledge permanent data loss.".into());
        }
        let profile = self.snapshot(id)?;
        let review = {
            let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let intent = state
                .deletion
                .prepared
                .as_mut()
                .filter(|intent| intent.key == key)
                .ok_or("Prepare this deletion first.")?;
            if !intent.review.eligible() {
                return Err(
                    "Deletion prerequisites or server capability do not permit deletion.".into(),
                );
            }
            if !intent.submitted
                && intent.prepared_at.elapsed() > std::time::Duration::from_secs(120)
            {
                return Err("Deletion review expired. Review this target again.".into());
            }
            if intent.inflight {
                return Err("Deletion submission is already in progress.".into());
            }
            if intent.operation.is_some() {
                return Err(
                    "Read the retained deletion operation instead of submitting again.".into(),
                );
            }
            intent.submitted = true;
            intent.inflight = true;
            intent.review.clone()
        };
        let result: Result<Operation, String> = profile
            .deletion_json(
                profile
                    .client
                    .post(
                        profile
                            .endpoint
                            .join(&format!(
                                "api/v1/environments/{}/delete",
                                review.instance_id
                            ))
                            .map_err(|_| "Invalid server address.")?,
                    )
                    .header("idempotency-key", key)
                    .json(&review.approval()),
                true,
            )
            .await;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err(
                "Connection changed; the submitted deletion may continue on the server.".into(),
            );
        }
        let intent = state
            .deletion
            .prepared
            .as_mut()
            .filter(|intent| intent.key == key)
            .ok_or("Deletion state unavailable.")?;
        intent.inflight = false;
        let operation = result?;
        if !operation.valid(&intent.review, None) {
            return Err(
                "The server returned a mismatched deletion operation. Retain the request key."
                    .into(),
            );
        }
        intent.operation = Some(operation.id.clone());
        intent.terminal = operation.terminal();
        Ok(operation)
    }
    async fn deletion_operation(&self, id: u64, key: &str) -> Result<Operation, String> {
        let profile = self.snapshot(id)?;
        let (review, operation_id) = {
            let state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
            if state.generation != id {
                return Err("The connection changed.".into());
            }
            let intent = state
                .deletion
                .prepared
                .as_ref()
                .filter(|intent| intent.key == key)
                .ok_or("Prepare this deletion first.")?;
            (
                intent.review.clone(),
                intent.operation.clone().ok_or(
                    "No operation ID was received. Retry only the same confirmed request.",
                )?,
            )
        };
        let result: Operation = profile
            .deletion_json(
                profile.client.get(
                    profile
                        .endpoint
                        .join(&format!("api/v1/operations/{operation_id}"))
                        .map_err(|_| "Invalid server address.")?,
                ),
                false,
            )
            .await?;
        self.current(id)?;
        if !result.valid(&review, Some(&operation_id)) {
            return Err("The server returned a mismatched deletion operation.".into());
        }
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed.".into());
        }
        let intent = state
            .deletion
            .prepared
            .as_mut()
            .filter(|intent| {
                intent.key == key && intent.operation.as_deref() == Some(&operation_id)
            })
            .ok_or("Deletion state changed.")?;
        intent.terminal = result.terminal();
        Ok(result)
    }
    fn clear_deletion(&self, id: u64, key: &str) -> Result<(), String> {
        self.current(id)?;
        let mut state = self.0.lock().map_err(|_| "Connection state unavailable.")?;
        if state.generation != id {
            return Err("The connection changed.".into());
        }
        let deletion = &mut state.deletion;
        let intent = deletion
            .prepared
            .as_ref()
            .filter(|intent| intent.key == key)
            .ok_or("Deletion state changed.")?;
        if intent.inflight || (intent.submitted && !intent.terminal) {
            return Err("Read or retry the retained deletion before clearing it. Disconnect does not cancel it.".into());
        }
        deletion.revision = deletion
            .revision
            .checked_add(1)
            .ok_or("Deletion state unavailable.")?;
        deletion.prepared = None;
        Ok(())
    }
}
#[tauri::command]
pub async fn prepare_remote_deletion(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    instance_id: String,
) -> Result<Review, String> {
    state
        .prepare_deletion(connection_id, request_key, instance_id)
        .await
}
#[tauri::command]
pub async fn confirm_remote_deletion(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
    confirmed: bool,
    acknowledge_data_loss: bool,
) -> Result<Operation, String> {
    state
        .confirm_deletion(
            connection_id,
            &request_key,
            confirmed,
            acknowledge_data_loss,
        )
        .await
}
#[tauri::command]
pub async fn read_remote_deletion_operation(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<Operation, String> {
    state.deletion_operation(connection_id, &request_key).await
}
#[tauri::command]
pub fn clear_remote_deletion(
    state: tauri::State<'_, RemoteConnection>,
    connection_id: u64,
    request_key: String,
) -> Result<(), String> {
    state.clear_deletion(connection_id, &request_key)
}

impl Profile {
    pub(super) async fn deletion_json<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        accepted: bool,
    ) -> Result<T, String> {
        let mut response=request.header(reqwest::header::AUTHORIZATION,self.authorization.clone()).send().await.map_err(|_|"Deletion request interrupted. If confirmed, retain the same request key and retry only that request.")?;
        if response.status().as_u16() == 429 {
            return Err(super::RATE_LIMITED.into());
        }
        if response.status().as_u16() == 401 {
            return Err("The server rejected the access token.".into());
        }
        if response
            .headers()
            .get("x-vmanyware-authenticated")
            .and_then(|value| value.to_str().ok())
            != Some("bearer")
        {
            return Err("The server does not confirm access-token authentication.".into());
        }
        if response.status().as_u16() != if accepted { 202 } else { 200 } {
            return Err(match response.status().as_u16() {403=>"Deletion action is disabled or access was denied.",409=>"Deletion inputs conflict with the server. Retain the original request key and inspect its operation.",_=>"Cannot complete the deletion request. If confirmed, retain the same request key."}.into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Deletion response interrupted; retain the request key.")?
        {
            if bytes.len() + chunk.len() > 256 * 1024 {
                return Err("The server returned an invalid deletion response.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "The server returned an invalid deletion response.".into())
    }
}
#[cfg(test)]
#[path = "deletion_tests.rs"]
mod tests;

#[cfg(test)]
impl RemoteConnection {
    pub(super) fn deletion_is_empty(&self) -> bool {
        self.0.lock().unwrap().deletion.prepared.is_none()
    }
}
