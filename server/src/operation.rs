//! Operation status describes job execution, never guest/SSH readiness.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
}

impl Status {
    pub fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Running)
                | (
                    Self::Running,
                    Self::Succeeded | Self::Failed | Self::Interrupted
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
}

impl Failure {
    pub fn message(self) -> &'static str {
        match self {
            Self::ProviderNotConfigured => "Proxmox is not configured on the control plane.",
            Self::ProviderUnauthorized => "Proxmox rejected the configured API token.",
            Self::ProviderForbidden => "The Proxmox token lacks inventory permissions.",
            Self::ProviderTimeout => "Proxmox inventory timed out.",
            Self::ProviderUnavailable => "Cannot read Proxmox inventory; check endpoint and TLS trust.",
            Self::ProcessInterrupted => "The service stopped during this operation. Its outcome is unconfirmed; start a new read-only refresh.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub kind: String,
    pub status: Status,
    pub result: Option<serde_json::Value>,
    pub failure: Option<Failure>,
    pub created_at: String,
    pub updated_at: String,
}

impl Operation {
    pub fn message(&self) -> Option<&'static str> {
        self.failure.map(Failure::message)
    }
}
