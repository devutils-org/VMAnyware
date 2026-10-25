//! Destructive restore prerequisites and durable approval; no rollback transport is exposed.
use crate::{
    instance::{ObservationError, Summary},
    lifecycle::{Check, Status},
    proxmox::{Error, PowerState, Proxmox, QmpState, Snapshot},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Request {
    pub snapshot_name: String,
}
impl Request {
    pub fn is_valid(&self) -> bool {
        crate::snapshot::valid_name(&self.snapshot_name)
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
    pub observation: crate::proxmox::VmObservation,
    pub request: Request,
    pub selected_snapshot: Option<Snapshot>,
    pub snapshot_config_hash: Option<String>,
    pub destructive: bool,
    pub starts_vm: bool,
    pub checks: Vec<Check>,
    pub restore_preconditions: Status,
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
fn stopped(status: PowerState, qmp: Option<QmpState>) -> bool {
    status == PowerState::Stopped && matches!(qmp, None | Some(QmpState::Stopped))
}
impl Summary {
    pub async fn snapshot_restore_preflight(
        &self,
        provider: &Proxmox,
        request: Request,
    ) -> Result<Preflight, ObservationError> {
        if !request.is_valid() {
            return Err(ObservationError::InvalidReference);
        }
        let target = self.target(provider)?;
        tokio::time::timeout(Duration::from_secs(20),async {
            let live=self.lifecycle_preflight(provider).await?;
            let snapshots=provider.snapshots(&target).await.map_err(ObservationError::Provider)?;
            let selected=snapshots.iter().find(|snapshot|snapshot.name==request.snapshot_name).cloned();
            let marker=format!("vmanyware-{}",self.id);
            let (before,after,supported)=if selected.is_some() {
                let before=provider.restore_snapshot_identity(&target,&request.snapshot_name,&marker).await.map_err(ObservationError::Provider)?;
                // Proxmox reports snapshot features per state; LVM-thin reports none for an existing
                // snapshot (snapname) although rollback works. Observe the current storage instead.
                let supported=provider.snapshot_supported(&target).await.map_err(ObservationError::Provider)?;
                let after=provider.restore_snapshot_identity(&target,&request.snapshot_name,&marker).await.map_err(ObservationError::Provider)?;
                (Some(before),Some(after),supported)
            } else {(None,None,false)};
            let final_snapshots=provider.snapshots(&target).await.map_err(ObservationError::Provider)?;
            let final_selected=final_snapshots.iter().find(|snapshot|snapshot.name==request.snapshot_name);
            let pending=provider.vm_has_pending_changes(&target).await.map_err(ObservationError::Provider)?;
            let final_identity=provider.vm_identity(&target,&marker).await.map_err(ObservationError::Provider)?;
            let final_power=provider.vm_status(&target).await.map_err(ObservationError::Provider)?;
            let complete=snapshots.iter().chain(final_snapshots.iter()).all(|snapshot|snapshot.provider_state.is_none()) && before.as_ref().is_some_and(|snapshot|snapshot.complete) && after.as_ref().is_some_and(|snapshot|snapshot.complete);
            let disk_only=selected.as_ref().is_some_and(|snapshot|snapshot.includes_ram==Some(false)) && final_selected.is_some_and(|snapshot|snapshot.includes_ram==Some(false)) && before.as_ref().is_some_and(|snapshot|snapshot.disk_only) && after.as_ref().is_some_and(|snapshot|snapshot.disk_only);
            let snapshot_identity=before.as_ref().is_some_and(|snapshot|snapshot.identity.instance_marker && !snapshot.identity.template && !snapshot.identity.locked) && after.as_ref().is_some_and(|snapshot|snapshot.identity.instance_marker && !snapshot.identity.template && !snapshot.identity.locked);
            let stable=selected.as_ref().is_some_and(|snapshot|Some(snapshot)==final_selected) && before.as_ref().zip(after.as_ref()).is_some_and(|(before,after)|before.identity.digest==after.identity.digest);
            let mut checks=live.checks;
            checks.extend([
                checked("restore_vm_stopped",stopped(live.observation.status,live.observation.qmp_status) && stopped(final_power.status,final_power.qmp_status),"Initial restore requires a VM already stopped; no automatic shutdown/start is permitted."),
                checked("restore_snapshot_exists",selected.is_some() && final_selected.is_some(),"The exact selected snapshot must exist during both reads."),
                checked("restore_snapshot_complete",complete,"Resolve incomplete snapshot work before restore."),
                checked("restore_snapshot_disk_only",disk_only,"Initial restore supports only explicitly disk-only snapshots, without RAM state."),
                checked("restore_snapshot_identity",snapshot_identity,"Saved configuration must retain this instance marker and be a non-template/unlocked VM."),
                checked("restore_snapshot_stable",stable,"Selected metadata and canonical saved-configuration hash must remain unchanged."),
                checked("restore_snapshot_storage",supported,"Provider must report snapshot support for the saved configuration; disk contents/rollback remain unverified."),
                checked("restore_target_unchanged",live.config_digest==final_identity.digest && final_identity.instance_marker && !final_identity.template && !final_identity.locked && !pending,"Live target/configuration must remain unchanged, without lock or pending changes."),
            ]);
            let restore_preconditions=if checks.iter().all(|check|check.status==Status::Passed){Status::Passed}else{Status::Failed};
            Ok(Preflight{instance_id:self.id.clone(),provider_scope_id:provider.scope_id(),definition_hash:self.definition_hash.clone(),node:self.provider_reference.node.clone(),vmid:self.provider_reference.vmid,config_digest:final_identity.digest,observation:final_power,request,selected_snapshot:selected,snapshot_config_hash:after.map(|snapshot|snapshot.identity.digest),destructive:true,starts_vm:false,checks,restore_preconditions,execution_enabled:false})
        }).await.map_err(|_|ObservationError::Provider(Error::Timeout))?
    }
}

/// Both confirmations are required; neither permits automatic shutdown or start.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Approval {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub node: String,
    pub vmid: u32,
    pub config_digest: String,
    pub request: Request,
    pub snapshot_config_hash: String,
    pub confirmed: bool,
    pub acknowledge_data_loss: bool,
}
impl Approval {
    pub fn is_valid(&self) -> bool {
        // Reuse the existing exact target/digest grammar, with a distinct hash domain.
        self.acknowledge_data_loss
            && crate::instance::valid_hash(&self.snapshot_config_hash)
            && self.target_approval().is_valid()
    }
    fn target_approval(&self) -> crate::snapshot::Approval {
        crate::snapshot::Approval {
            instance_id: self.instance_id.clone(),
            provider_scope_id: self.provider_scope_id.clone(),
            definition_hash: self.definition_hash.clone(),
            node: self.node.clone(),
            vmid: self.vmid,
            config_digest: self.config_digest.clone(),
            request: crate::snapshot::CreateRequest {
                name: self.request.snapshot_name.clone(),
                description: String::new(),
            },
            confirmed: self.confirmed,
        }
    }
    pub fn matches_instance(&self, instance: &Summary) -> bool {
        self.target_approval().matches_instance(instance)
    }
    pub fn input_hash(&self) -> Result<String, serde_json::Error> {
        use sha2::{Digest, Sha256};
        let mut material = b"vmanyware-snapshot-restore-v1\0".to_vec();
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
/// Provider completion and independent post-task observations, not guest-content proof.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultEvidence {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub node: String,
    pub vmid: u32,
    pub snapshot_name: String,
    pub snapshot_config_hash: String,
    pub task_status: crate::proxmox::TaskStatus,
    pub observation: crate::proxmox::VmObservation,
}
pub enum Outcome {
    Succeeded(ResultEvidence),
    Failed(crate::operation::Failure),
    Uncertain(crate::operation::Failure),
}

/// Historical journal context, never a replacement approval or current VM observation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovered {
    pub operation: crate::operation::Operation,
    pub target: crate::snapshot::RecoveryTarget,
    pub request: Request,
    pub snapshot_config_hash: String,
    pub submission_started: bool,
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
    pub snapshot_name: String,
    pub status: crate::operation::Status,
    pub failure: Option<crate::operation::Failure>,
    pub provider_task_known: bool,
    pub created_at: String,
}
