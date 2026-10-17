use super::{trust, HostConfirmation, HostInspection};
use russh::{
    client,
    keys::{HashAlg, PublicKeyOrCertificate},
    Disconnect,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

const TTL: Duration = Duration::from_secs(120);
const PROBE_LIMIT: Duration = Duration::from_secs(10);
const INVALID_CHALLENGE: &str =
    "SSH confirmation expired, cancelled or unavailable; inspect the host again";
const STALE_INSPECTION: &str = "SSH inspection superseded; inspect the current endpoint again";

struct Challenge {
    id: String,
    endpoint: trust::Endpoint,
    observation: trust::Confirmation,
}
struct Owner {
    generation: Uuid,
    expires: Instant,
    challenge: Option<Challenge>,
}
struct Inner {
    path: PathBuf,
    probes: tokio::sync::Semaphore,
    owners: Mutex<HashMap<String, Owner>>,
    store: Mutex<Option<Arc<trust::Store>>>,
}
#[derive(Clone)]
pub struct HostManager {
    inner: Arc<Inner>,
}
impl HostManager {
    pub fn new(path: PathBuf) -> Self {
        Self {
            inner: Arc::new(Inner {
                path,
                probes: tokio::sync::Semaphore::new(8),
                owners: Mutex::new(HashMap::new()),
                store: Mutex::new(None),
            }),
        }
    }
    pub fn forget_owner(&self, owner: &str) {
        if let Ok(mut owners) = self.inner.owners.lock() {
            owners.remove(owner);
        }
    }
    pub(super) async fn connect_trusted(
        &self,
        endpoint: trust::Endpoint,
    ) -> Result<(client::Handle<ProbeClient>, ProbeSocket), String> {
        let store = self.store().await?;
        let observed = Arc::new(Mutex::new(None));
        let socket = tokio::net::TcpStream::connect((endpoint.host(), endpoint.port()))
            .await
            .map_err(|_| "SSH connection failed")?
            .into_std()
            .map_err(|_| "SSH connection failed")?;
        let shutdown = ProbeSocket(socket.try_clone().map_err(|_| "SSH connection failed")?);
        let socket =
            tokio::net::TcpStream::from_std(socket).map_err(|_| "SSH connection failed")?;
        let handle = client::connect_stream(
            Arc::new(client::Config {
                inactivity_timeout: Some(Duration::from_secs(300)),
                keepalive_interval: Some(Duration::from_secs(30)),
                keepalive_max: 3,
                channel_buffer_size: 16,
                window_size: 256 * 1024,
                maximum_packet_size: 32 * 1024,
                ..Default::default()
            }),
            socket,
            ProbeClient {
                store,
                endpoint,
                observed: observed.clone(),
            },
        )
        .await
        .map_err(|_| "SSH host verification failed; inspect and verify the host first")?;
        let trusted = matches!(
            observed
                .lock()
                .map_err(|_| "SSH verification unavailable")?
                .as_ref(),
            Some(Ok(Observed {
                inspection: Checked::Trusted,
                ..
            }))
        );
        if !trusted {
            return Err("SSH host verification failed".into());
        }
        Ok((handle, shutdown))
    }
    fn begin(&self, owner: &str) -> Result<Uuid, String> {
        let mut owners = self
            .inner
            .owners
            .lock()
            .map_err(|_| "SSH inspection unavailable")?;
        owners.retain(|_, value| value.expires > Instant::now());
        if !owners.contains_key(owner) && owners.len() >= 8 {
            return Err("Too many active SSH inspections".into());
        }
        let generation = Uuid::new_v4();
        owners.insert(
            owner.into(),
            Owner {
                generation,
                expires: Instant::now() + TTL,
                challenge: None,
            },
        );
        Ok(generation)
    }
    async fn store(&self) -> Result<Arc<trust::Store>, String> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let mut stored = inner
                .store
                .lock()
                .map_err(|_| "Host trust storage unavailable")?;
            if let Some(store) = stored.as_ref() {
                return Ok(store.clone());
            }
            let store = match std::fs::symlink_metadata(&inner.path) {
                Ok(_) => trust::Store::open(&inner.path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let parent = inner
                        .path
                        .parent()
                        .ok_or("Host trust storage unavailable")?;
                    std::fs::create_dir_all(parent)
                        .map_err(|_| "Host trust storage unavailable")?;
                    // Another process may have initialized the directory concurrently.
                    trust::Store::create(&inner.path).or_else(|_| trust::Store::open(&inner.path))
                }
                Err(_) => Err(trust::Error::Io),
            }
            .map_err(|error| error.to_string())?;
            let store = Arc::new(store);
            *stored = Some(store.clone());
            Ok(store)
        })
        .await
        .map_err(|_| "Host trust storage unavailable")?
    }
    pub async fn inspect(
        &self,
        owner: &str,
        host: &str,
        port: u16,
    ) -> Result<HostInspection, String> {
        let generation = self.begin(owner)?;
        let _permit = self
            .inner
            .probes
            .try_acquire()
            .map_err(|_| "Too many active SSH inspections")?;
        let endpoint = trust::Endpoint::new(host, port).map_err(|error| error.to_string())?;
        let result = tokio::time::timeout(PROBE_LIMIT, async {
            let store = self.store().await?;
            probe(store, endpoint.clone()).await
        })
        .await
        .map_err(|_| "SSH host inspection timed out")??;
        self.finish(owner, generation, endpoint, result)
    }
    fn finish(
        &self,
        owner: &str,
        generation: Uuid,
        endpoint: trust::Endpoint,
        observed: Observed,
    ) -> Result<HostInspection, String> {
        let mut owners = self
            .inner
            .owners
            .lock()
            .map_err(|_| "SSH inspection unavailable")?;
        let current = owners
            .get_mut(owner)
            .filter(|v| v.generation == generation && v.expires > Instant::now())
            .ok_or(STALE_INSPECTION)?;
        let (status, challenge_id, expires_in_seconds) = match observed.inspection {
            Checked::Trusted => ("trusted", None, 0),
            Checked::Changed => ("changed", None, 0),
            Checked::Unknown(observation) => {
                let id = Uuid::new_v4().to_string();
                current.challenge = Some(Challenge {
                    id: id.clone(),
                    endpoint: endpoint.clone(),
                    observation,
                });
                current.expires = Instant::now() + TTL;
                ("unknown", Some(id), TTL.as_secs())
            }
        };
        Ok(HostInspection {
            host: endpoint.host().into(),
            port: endpoint.port(),
            algorithm: observed.algorithm,
            fingerprint: observed.fingerprint,
            status,
            challenge_id,
            expires_in_seconds,
        })
    }
    pub async fn confirm(
        &self,
        owner: &str,
        id: &str,
        fingerprint: &str,
    ) -> Result<HostConfirmation, String> {
        if id.len() > 64 || fingerprint.len() > 128 {
            return Err(INVALID_CHALLENGE.into());
        }
        let inner = self.inner.clone();
        let owner = owner.to_owned();
        let id = id.to_owned();
        let fingerprint = fingerprint.to_owned();
        // Consuming the challenge commits approval; later cancellation cannot revoke
        // this operation. Never hold the owner-state mutex across filesystem I/O.
        tokio::task::spawn_blocking(move || {
            let mut owners = inner
                .owners
                .lock()
                .map_err(|_| "SSH inspection unavailable")?;
            let current = owners
                .get_mut(&owner)
                .filter(|v| v.expires > Instant::now())
                .ok_or(INVALID_CHALLENGE)?;
            if current.challenge.as_ref().map(|c| c.id.as_str()) != Some(&id) {
                return Err(INVALID_CHALLENGE.into());
            }
            let challenge = current.challenge.take().ok_or(INVALID_CHALLENGE)?;
            drop(owners);
            let store = inner
                .store
                .lock()
                .map_err(|_| "Host trust storage unavailable")?
                .clone()
                .ok_or("Host trust storage unavailable")?;
            let confirmed_fingerprint = challenge.observation.fingerprint().to_owned();
            store
                .confirm(challenge.observation, &fingerprint)
                .map_err(|error| error.to_string())?;
            Ok(HostConfirmation {
                host: challenge.endpoint.host().into(),
                port: challenge.endpoint.port(),
                fingerprint: confirmed_fingerprint,
            })
        })
        .await
        .map_err(|_| "SSH confirmation unavailable")?
    }
    pub fn cancel(&self, owner: &str, id: &str) -> Result<(), String> {
        if id.len() > 64 {
            return Err(INVALID_CHALLENGE.into());
        }
        let mut owners = self
            .inner
            .owners
            .lock()
            .map_err(|_| "SSH inspection unavailable")?;
        let current = owners.get_mut(owner).ok_or(INVALID_CHALLENGE)?;
        if current.challenge.as_ref().map(|c| c.id.as_str()) != Some(id) {
            return Err(INVALID_CHALLENGE.into());
        }
        current.challenge = None;
        Ok(())
    }
}
enum Checked {
    Trusted,
    Changed,
    Unknown(trust::Confirmation),
}
struct Observed {
    algorithm: String,
    fingerprint: String,
    inspection: Checked,
}
pub(super) struct ProbeClient {
    store: Arc<trust::Store>,
    endpoint: trust::Endpoint,
    observed: Arc<Mutex<Option<Result<Observed, String>>>>,
}
impl client::Handler for ProbeClient {
    type Error = russh::Error;
    async fn check_server_key(
        &mut self,
        actual: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = actual else {
            return Ok(false);
        };
        let key = key.clone();
        let endpoint = self.endpoint.clone();
        let store = self.store.clone();
        let result = tokio::task::spawn_blocking(move || {
            let inspection = match store.inspect(endpoint, &key) {
                Ok(trust::Inspection::Trusted) => Checked::Trusted,
                Ok(trust::Inspection::Unknown(c)) => Checked::Unknown(c),
                Err(trust::Error::ChangedKey) => Checked::Changed,
                Err(error) => return Err(error.to_string()),
            };
            Ok(Observed {
                algorithm: key.algorithm().to_string(),
                fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
                inspection,
            })
        })
        .await
        .unwrap_or_else(|_| Err("Host trust storage unavailable".into()));
        let trusted = matches!(
            &result,
            Ok(Observed {
                inspection: Checked::Trusted,
                ..
            })
        );
        *self.observed.lock().map_err(|_| russh::Error::UnknownKey)? = Some(result);
        Ok(trusted)
    }
}
// Own socket shutdown even when a timeout drops connect_stream while russh's
// internal session task is still running.
pub(super) struct ProbeSocket(std::net::TcpStream);
impl Drop for ProbeSocket {
    fn drop(&mut self) {
        let _ = self.0.shutdown(std::net::Shutdown::Both);
    }
}
async fn probe(store: Arc<trust::Store>, endpoint: trust::Endpoint) -> Result<Observed, String> {
    let observed = Arc::new(Mutex::new(None));
    let socket = tokio::net::TcpStream::connect((endpoint.host(), endpoint.port()))
        .await
        .map_err(|_| "SSH host inspection connection failed")?;
    let socket = socket
        .into_std()
        .map_err(|_| "SSH host inspection connection failed")?;
    let _shutdown = ProbeSocket(
        socket
            .try_clone()
            .map_err(|_| "SSH host inspection connection failed")?,
    );
    let socket = tokio::net::TcpStream::from_std(socket)
        .map_err(|_| "SSH host inspection connection failed")?;
    let result = client::connect_stream(
        Arc::new(client::Config {
            inactivity_timeout: Some(PROBE_LIMIT),
            ..Default::default()
        }),
        socket,
        ProbeClient {
            store,
            endpoint,
            observed: observed.clone(),
        },
    )
    .await;
    let observed = observed
        .lock()
        .map_err(|_| "SSH inspection unavailable")?
        .take();
    match result {
        Ok(handle) => {
            // This is intentionally the only post-handshake action: no authentication.
            handle
                .disconnect(Disconnect::ByApplication, "host inspection complete", "en")
                .await
                .map_err(|_| "SSH host inspection disconnect failed")?;
            match observed {
                Some(Ok(v)) if matches!(v.inspection, Checked::Trusted) => Ok(v),
                _ => Err("SSH host verification failed".into()),
            }
        }
        Err(russh::Error::UnknownKey) => match observed {
            Some(Ok(v)) if !matches!(v.inspection, Checked::Trusted) => Ok(v),
            Some(Err(error)) => Err(error),
            _ => Err("SSH host verification failed".into()),
        },
        Err(_) => Err("SSH host inspection connection failed".into()),
    }
}
#[cfg(test)]
mod tests;
