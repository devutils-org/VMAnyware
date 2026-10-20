//! Local definition/target association; registration is not verified VM ownership.
use crate::proxmox::VmReference;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Registration {
    pub definition_name: String,
    pub definition_hash: String,
    pub node: String,
    pub vmid: u32,
}

impl Registration {
    pub fn is_valid(&self) -> bool {
        let name = &self.definition_name;
        !name.is_empty()
            && name.len() <= 63
            && !name.starts_with('-')
            && !name.ends_with('-')
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && valid_hash(&self.definition_hash)
            && VmReference::new(&self.node, self.vmid).is_ok()
    }
}

pub(crate) fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderReference {
    pub provider: &'static str,
    pub scope_id: String,
    pub node: String,
    pub vmid: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Registered,
    Deleted,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub definition_name: String,
    pub definition_hash: String,
    pub definition_revision: u32,
    pub provider_reference: ProviderReference,
    pub status: Status,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    #[serde(flatten)]
    pub summary: Summary,
    pub definition_yaml: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub instance_id: String,
    pub provider_scope_id: String,
    pub observation: crate::proxmox::VmObservation,
}

#[derive(Debug)]
pub enum ObservationError {
    ScopeChanged,
    InvalidReference,
    Provider(crate::proxmox::Error),
}

impl Summary {
    pub(crate) fn target(
        &self,
        provider: &crate::proxmox::Proxmox,
    ) -> Result<VmReference, ObservationError> {
        let reference = &self.provider_reference;
        if self.status != Status::Registered {
            return Err(ObservationError::InvalidReference);
        }
        if reference.scope_id != provider.scope_id() {
            return Err(ObservationError::ScopeChanged);
        }
        if reference.provider != "proxmox"
            || !valid_hash(&self.definition_hash)
            || self.id.len() != 32
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ObservationError::InvalidReference);
        }
        VmReference::new(&reference.node, reference.vmid)
            .map_err(|_| ObservationError::InvalidReference)
    }

    pub async fn observe(
        &self,
        provider: &crate::proxmox::Proxmox,
    ) -> Result<Observation, ObservationError> {
        let reference = &self.provider_reference;
        let target = self.target(provider)?;
        let observation = provider
            .vm_status(&target)
            .await
            .map_err(ObservationError::Provider)?;
        Ok(Observation {
            instance_id: self.id.clone(),
            provider_scope_id: reference.scope_id.clone(),
            observation,
        })
    }
}
