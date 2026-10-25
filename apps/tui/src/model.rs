//! Read-only control-plane projections. Registration and guest hints are not live evidence.
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub fn hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// Strip terminal control sequences and bidi/format controls from remote display text.
pub fn display(value: &str) -> String {
    value.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .filter(|c| !matches!(*c, '\u{7f}'..='\u{9f}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}'))
        .collect()
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub provider: String,
    pub scope_id: String,
    pub node: String,
    pub vmid: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Association {
    Registered,
    Deleted,
    Released,
}
impl Association {
    pub fn label(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Deleted => "deleted",
            Self::Released => "released",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Guest {
    pub address: std::net::Ipv4Addr,
    pub user: String,
    pub observed_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub definition_name: String,
    pub definition_hash: String,
    pub definition_revision: u32,
    pub provider_reference: Provider,
    pub status: Association,
    #[serde(default)]
    pub last_guest: Option<Guest>,
    pub created_at: String,
}
impl Environment {
    pub fn valid(&self) -> bool {
        let p = &self.provider_reference;
        hex(&self.id, 32)
            && identifier(&self.definition_name, 63)
            && hex(&self.definition_hash, 64)
            && self.definition_revision > 0
            && p.provider == "proxmox"
            && hex(&p.scope_id, 64)
            && identifier(&p.node, 63)
            && (100..=999_999_999).contains(&p.vmid)
            && !self.created_at.is_empty()
            && self.last_guest.as_ref().is_none_or(|g| {
                identifier(&g.user, 32)
                    && !g.observed_at.is_empty()
                    && !g.address.is_unspecified()
                    && !g.address.is_loopback()
                    && !g.address.is_multicast()
                    && !g.address.is_broadcast()
            })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    #[serde(flatten)]
    pub environment: Environment,
    pub definition_yaml: String,
}
impl Detail {
    pub fn matches(&self, selected: &Environment) -> bool {
        let e = &self.environment;
        e.valid()
            && e.id == selected.id
            && e.provider_reference == selected.provider_reference
            && e.definition_name == selected.definition_name
            && e.definition_hash == selected.definition_hash
            && e.definition_revision == selected.definition_revision
            && !self.definition_yaml.is_empty()
            && self.definition_yaml.len() <= 64 * 1024
            && format!("{:x}", Sha256::digest(self.definition_yaml.as_bytes())) == e.definition_hash
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Power {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub observation: PowerObservation,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerObservation {
    pub node: String,
    pub vmid: u32,
    pub status: String,
    pub qmp_status: Option<String>,
}
impl Power {
    pub fn matches(&self, e: &Environment) -> bool {
        self.instance_id == e.id
            && self.provider_scope_id == e.provider_reference.scope_id
            && self.observation.node == e.provider_reference.node
            && self.observation.vmid == e.provider_reference.vmid
            && matches!(self.observation.status.as_str(), "running" | "stopped")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
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
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
            Self::Uncertain => "uncertain",
        }
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub kind: String,
    pub instance_id: Option<String>,
    pub status: Status,
    pub failure: Option<String>,
    #[serde(default)]
    pub provider_task_known: bool,
    pub created_at: String,
    pub updated_at: String,
}
impl Operation {
    pub fn valid(&self) -> bool {
        hex(&self.id, 32)
            && identifier(&self.kind, 64)
            && self.instance_id.as_ref().is_none_or(|id| hex(id, 32))
            && self.failure.as_ref().is_none_or(|f| identifier(f, 80))
            && !self.created_at.is_empty()
            && !self.updated_at.is_empty()
    }
    pub fn provisioning(&self) -> bool {
        matches!(self.kind.as_str(), "vm_provision" | "vm_recreate")
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub operation_id: String,
    pub instance_id: String,
    pub provider_scope_id: String,
    pub definition_hash: String,
    pub plan_hash: String,
    pub node: String,
    pub vmid: u32,
    pub status: Status,
    pub stages: Vec<Stage>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub position: usize,
    pub kind: String,
    pub status: Status,
    pub submission_started: bool,
}
impl Progress {
    pub fn matches(&self, op: &Operation, e: &Environment) -> bool {
        if self.operation_id != op.id
            || Some(&self.instance_id) != op.instance_id.as_ref()
            || self.instance_id != e.id
            || self.provider_scope_id != e.provider_reference.scope_id
            || self.node != e.provider_reference.node
            || self.vmid != e.provider_reference.vmid
            || self.definition_hash != e.definition_hash
            || !hex(&self.plan_hash, 64)
        {
            return false;
        }
        let mut kinds = vec!["full_clone", "configure_resources"];
        if self.stages.iter().any(|s| s.kind == "expand_disk") {
            kinds.push("expand_disk");
        }
        kinds.extend([
            "configure_cloud_init",
            "start_vm",
            "wait_for_guest_readiness",
        ]);
        if self
            .stages
            .iter()
            .any(|s| s.kind == "install_declared_packages")
        {
            kinds.push("install_declared_packages");
        }
        if self
            .stages
            .iter()
            .any(|s| s.kind == "snapshot_after_provisioning")
        {
            kinds.push("snapshot_after_provisioning");
        }
        if self.stages.len() != kinds.len()
            || self
                .stages
                .iter()
                .zip(kinds)
                .enumerate()
                .any(|(i, (s, k))| s.position != i || s.kind != k)
        {
            return false;
        }
        let first = self
            .stages
            .iter()
            .position(|s| s.status != Status::Succeeded);
        if first.is_some_and(|i| {
            self.stages[i + 1..]
                .iter()
                .any(|s| s.status != Status::Queued || s.submission_started)
        }) {
            return false;
        }
        match (self.status, first.map(|i| self.stages[i].status)) {
            (Status::Succeeded, None) => true,
            (Status::Queued, Some(Status::Queued)) => self
                .stages
                .iter()
                .all(|s| s.status == Status::Queued && !s.submission_started),
            (Status::Running, Some(Status::Running | Status::Queued)) => true,
            (Status::Failed, Some(Status::Failed)) => true,
            (Status::Interrupted, Some(Status::Interrupted)) => true,
            (Status::Uncertain, Some(Status::Uncertain | Status::Interrupted)) => true,
            _ => false,
        }
    }
}
