//! Read-only task observation; successful provider completion is not guest readiness.

use super::{Error, Proxmox};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, PartialEq, Eq)]
pub struct TaskReference {
    node: String,
    upid: String,
}

impl std::fmt::Debug for TaskReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TaskReference { identity: [REDACTED] }")
    }
}

impl TaskReference {
    pub(crate) fn upid(&self) -> &str {
        &self.upid
    }
    pub(crate) fn matches_action(
        &self,
        target: &super::VmReference,
        action: crate::lifecycle::Action,
    ) -> bool {
        let fields: Vec<_> = self.upid.split(':').collect();
        self.node == target.node()
            && fields[5] == action.task_kind()
            && fields[6] == target.vmid().to_string()
    }

    pub(crate) fn matches_snapshot(&self, target: &super::VmReference) -> bool {
        let fields: Vec<_> = self.upid.split(':').collect();
        self.node == target.node()
            && fields[5] == "qmsnapshot"
            && fields[6] == target.vmid().to_string()
    }

    pub(crate) fn matches_restore(&self, target: &super::VmReference) -> bool {
        let fields: Vec<_> = self.upid.split(':').collect();
        self.node == target.node()
            && fields[5] == "qmrollback"
            && fields[6] == target.vmid().to_string()
    }

    pub(crate) fn matches_deletion(&self, target: &super::VmReference) -> bool {
        let fields: Vec<_> = self.upid.split(':').collect();
        self.node == target.node()
            && fields[5] == "qmdestroy"
            && fields[6] == target.vmid().to_string()
    }

    pub(crate) fn matches_clone(&self, source: &super::VmReference) -> bool {
        let fields: Vec<_> = self.upid.split(':').collect();
        self.node == source.node()
            && fields[5] == "qmclone"
            && fields[6] == source.vmid().to_string()
    }

    pub(crate) fn matches_resize(&self, target: &super::VmReference) -> bool {
        let fields: Vec<_> = self.upid.split(':').collect();
        self.node == target.node()
            && fields[5] == "resize"
            && fields[6] == target.vmid().to_string()
    }

    pub fn new(node: &str, upid: &str) -> Result<Self, Error> {
        let fields: Vec<_> = upid.split(':').collect();
        let valid_node = !node.is_empty()
            && node.len() <= 63
            && node.as_bytes()[0].is_ascii_alphanumeric()
            && node
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
        if !valid_node
            || upid.len() > 512
            || fields.len() != 9
            || fields[0] != "UPID"
            || fields[1] != node
            || !fields[8].is_empty()
            || fields[2].len() != 8
            || !(8..=9).contains(&fields[3].len())
            || fields[4].len() != 8
            || fields[2..5]
                .iter()
                .any(|s| !s.bytes().all(|b| b.is_ascii_hexdigit()))
            || fields[5].is_empty()
            || fields[7].is_empty()
            || !fields[5..8].iter().all(|s| {
                s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-@!".contains(&b))
            })
        {
            return Err(Error::Configuration);
        }
        Ok(Self {
            node: node.into(),
            upid: upid.into(),
        })
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Succeeded,
    /// Raw provider exit messages may contain sensitive host or guest details.
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct TaskPollOptions {
    interval: Duration,
    timeout: Duration,
}

impl TaskPollOptions {
    /// Bounded polling: 100ms..60s between reads, at most one hour overall.
    pub fn new(interval: Duration, timeout: Duration) -> Result<Self, Error> {
        if interval < Duration::from_millis(100)
            || interval > Duration::from_secs(60)
            || timeout < interval
            || timeout > Duration::from_secs(3600)
        {
            return Err(Error::Configuration);
        }
        Ok(Self { interval, timeout })
    }
}

#[derive(Deserialize)]
struct ProviderTaskStatus {
    node: String,
    upid: String,
    status: String,
    exitstatus: Option<String>,
}

impl Proxmox {
    pub async fn task_status(&self, task: &TaskReference) -> Result<TaskStatus, Error> {
        let response: ProviderTaskStatus = self
            .get(&["nodes", &task.node, "tasks", &task.upid, "status"])
            .await?;
        if response.node != task.node || response.upid != task.upid {
            return Err(Error::InvalidResponse);
        }
        match (response.status.as_str(), response.exitstatus.as_deref()) {
            ("running", None) => Ok(TaskStatus::Running),
            ("stopped", Some("OK")) => Ok(TaskStatus::Succeeded),
            ("stopped", Some(message)) if !message.trim().is_empty() => Ok(TaskStatus::Failed),
            _ => Err(Error::InvalidResponse),
        }
    }

    /// Stop on terminal status or the first provider error. No retries or mutations.
    pub async fn wait_for_task(
        &self,
        task: &TaskReference,
        options: TaskPollOptions,
    ) -> Result<TaskStatus, Error> {
        tokio::time::timeout(options.timeout, async {
            loop {
                let status = self.task_status(task).await?;
                if status != TaskStatus::Running {
                    return Ok(status);
                }
                tokio::time::sleep(options.interval).await;
            }
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
