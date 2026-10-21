//! Fixed Debian package installation; dispatch authorization and PID durability belong to the worker.
use super::{Error, Proxmox, ReadinessRequest};
use serde::Deserialize;
use std::{collections::HashSet, time::Duration};

#[derive(Debug)]
pub struct PackageRequest {
    guest: ReadinessRequest,
    packages: Vec<String>,
}
impl PackageRequest {
    pub(crate) fn new(guest: ReadinessRequest, packages: &[String]) -> Result<Self, Error> {
        if packages.is_empty() || !Self::valid_declarations(packages) {
            return Err(Error::Configuration);
        }
        Ok(Self {
            guest,
            packages: packages.to_vec(),
        })
    }
    pub(crate) fn valid_declarations(packages: &[String]) -> bool {
        let mut seen = HashSet::new();
        packages.len() <= 128
            && packages
                .iter()
                .all(|p| p.len() <= 128 && valid_package(p) && seen.insert(p))
    }
    fn command(&self) -> Vec<String> {
        // Package declarations are positional arguments, never script interpolation.
        // Suppress raw apt output: repository URLs and maintainer scripts may contain secrets.
        let mut command: Vec<String> = [
            "/usr/bin/timeout", "--kill-after=30", "900",
            "/usr/bin/env", "DEBIAN_FRONTEND=noninteractive", "APT_LISTCHANGES_FRONTEND=none",
            "/bin/sh", "-c",
            "exec >/dev/null 2>&1; /usr/bin/apt-get -o DPkg::Lock::Timeout=60 update && /usr/bin/apt-get -o DPkg::Lock::Timeout=60 -o Dpkg::Options::=--force-confold --assume-yes --no-remove --no-install-recommends install -- \"$@\" || exit 1; for declaration do name=${declaration%%=*}; status=$(/usr/bin/dpkg-query --show --showformat='${db:Status-Status}' -- \"$name\") || exit 1; [ \"$status\" = installed ] || exit 1; case $declaration in *=*) version=$(/usr/bin/dpkg-query --show --showformat='${Version}' -- \"$name\") || exit 1; [ \"$version\" = \"${declaration#*=}\" ] || exit 1;; esac; done",
            "vmanyware-packages",
        ].into_iter().map(str::to_owned).collect();
        command.extend(self.packages.iter().cloned());
        command
    }
}
fn valid_package(value: &str) -> bool {
    let (name, version) = value
        .split_once('=')
        .map_or((value, None), |(n, v)| (n, Some(v)));
    let (name, architecture) = name
        .split_once(':')
        .map_or((name, None), |(n, a)| (n, Some(a)));
    let identifier = |s: &str| {
        !s.is_empty()
            && s.as_bytes()[0].is_ascii_alphanumeric()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"+.-".contains(&b))
    };
    identifier(name)
        // Refuse removal syntax. The fixed script additionally verifies literal names
        // and exact versions, including genuine names ending in + (such as g++).
        && !name.ends_with('-')
        && architecture.is_none_or(|a| !a.is_empty() && a.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
        && version.is_none_or(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".+:~-".contains(&b)))
}

#[derive(Debug, PartialEq, Eq)]
pub enum PackageProgress {
    Pending,
    Completed,
    Failed,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Process {
    pid: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ProcessStatus {
    #[serde(deserialize_with = "super::deserialize_flag")]
    exited: u8,
    exitcode: Option<u32>,
    signal: Option<u32>,
    out_data: Option<String>,
    err_data: Option<String>,
    #[serde(default, deserialize_with = "super::deserialize_flag")]
    out_truncated: u8,
    #[serde(default, deserialize_with = "super::deserialize_flag")]
    err_truncated: u8,
}
impl Proxmox {
    /// One mutation, never retried here. Persist a dispatch marker before calling,
    /// and persist the returned PID before observing. A lost reply is uncertain.
    pub async fn dispatch_packages(&self, request: &PackageRequest) -> Result<u32, Error> {
        tokio::time::timeout(Duration::from_secs(20), async {
            self.readiness_guard(&request.guest).await?;
            let target = request.guest.target();
            let mut url = self.endpoint.clone();
            url.path_segments_mut()
                .map_err(|_| Error::Configuration)?
                .extend([
                    "api2",
                    "json",
                    "nodes",
                    target.node(),
                    "qemu",
                    &target.vmid().to_string(),
                    "agent",
                    "exec",
                ]);
            let response = self
                .client
                .post(url)
                .header(super::header::AUTHORIZATION, self.authorization.clone())
                .json(&serde_json::json!({"command": request.command()}))
                .send()
                .await
                .map_err(super::transport_error)?;
            let process: Process = Self::decode_response(response).await?;
            if process.pid == 0 || process.pid > i32::MAX as u32 {
                return Err(Error::InvalidResponse);
            }
            Ok(process.pid)
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    /// Observe only the retained PID. Neither guest output nor repository details escape.
    pub async fn observe_packages(
        &self,
        request: &PackageRequest,
        pid: u32,
    ) -> Result<PackageProgress, Error> {
        if pid == 0 || pid > i32::MAX as u32 {
            return Err(Error::Configuration);
        }
        tokio::time::timeout(Duration::from_secs(20), async {
            self.readiness_guard(&request.guest).await?;
            let target = request.guest.target();
            let status: ProcessStatus = self
                .get_query(
                    &[
                        "nodes",
                        target.node(),
                        "qemu",
                        &target.vmid().to_string(),
                        "agent",
                        "exec-status",
                    ],
                    &[("pid", pid.to_string())],
                )
                .await?;
            if status.out_data.as_ref().is_some_and(|s| !s.is_empty())
                || status.err_data.as_ref().is_some_and(|s| !s.is_empty())
                || status.out_truncated != 0
                || status.err_truncated != 0
            {
                return Err(Error::InvalidResponse);
            }
            let result = if status.exited == 0 {
                if status.exitcode.is_some() || status.signal.is_some() {
                    return Err(Error::InvalidResponse);
                }
                PackageProgress::Pending
            } else {
                match (status.exitcode, status.signal) {
                    (Some(0), None) => PackageProgress::Completed,
                    (Some(_), None) | (None, Some(_)) => PackageProgress::Failed,
                    _ => return Err(Error::InvalidResponse),
                }
            };
            self.readiness_guard(&request.guest).await?;
            Ok(result)
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
}
