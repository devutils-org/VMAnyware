//! Authentication-only macOS bridge. Never loads keys or opens SSH channels.
use super::{host::HostManager, trust::Endpoint};
use russh::{
    keys::{
        agent::{client::AgentClient, AgentIdentity},
        HashAlg, PublicKey,
    },
    Disconnect, Signer,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

const LIMIT: Duration = Duration::from_secs(10);
const TTL: Duration = Duration::from_secs(120);
const STALE: &str = "SSH identity selection expired or cancelled; list identities again";
const UNAVAILABLE: &str =
    "SSH agent unavailable; launch the application with SSH_AUTH_SOCK configured";
type Agent = AgentClient<Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>>;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    algorithm: String,
    fingerprint: String,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    id: String,
    host: String,
    port: u16,
    username: String,
    identities: Vec<Identity>,
    expires_in_seconds: u64,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Authentication {
    host: String,
    port: u16,
    username: String,
    fingerprint: String,
}
pub(super) struct Authenticated {
    pub handle: russh::client::Handle<super::host::ProbeClient>,
    pub shutdown: super::host::ProbeSocket,
    pub identity: Authentication,
}
struct Ticket {
    endpoint: Endpoint,
    username: String,
    socket: PathBuf,
    keys: Vec<PublicKey>,
}
struct Operation {
    id: String,
    expires: Instant,
    active: Arc<AtomicBool>,
    ticket: Option<Ticket>,
}
#[derive(Clone)]
pub struct AgentManager {
    hosts: HostManager,
    owners: Arc<Mutex<HashMap<String, Operation>>>,
    permits: Arc<tokio::sync::Semaphore>,
    // Tests supply their disposable socket; production resolves env only on click.
    socket: Option<PathBuf>,
}
impl AgentManager {
    pub fn new(hosts: HostManager) -> Self {
        Self {
            hosts,
            owners: Arc::default(),
            permits: Arc::new(tokio::sync::Semaphore::new(8)),
            socket: None,
        }
    }
    pub fn cancel(&self, owner: &str) {
        if let Ok(mut owners) = self.owners.lock() {
            if let Some(operation) = owners.remove(owner) {
                operation.active.store(false, Ordering::SeqCst);
            }
        }
    }
    pub fn cancel_selection(&self, owner: &str, id: &str) {
        if let Ok(mut owners) = self.owners.lock() {
            if owners
                .get(owner)
                .is_some_and(|operation| operation.id == id)
            {
                if let Some(operation) = owners.remove(owner) {
                    operation.active.store(false, Ordering::SeqCst);
                }
            }
        }
    }
    fn begin(&self, owner: &str) -> Result<(String, Arc<AtomicBool>), String> {
        let mut owners = self.owners.lock().map_err(|_| STALE)?;
        owners.retain(|_, operation| {
            let keep = operation.expires > Instant::now();
            if !keep {
                operation.active.store(false, Ordering::SeqCst);
            }
            keep
        });
        if !owners.contains_key(owner) && owners.len() >= 8 {
            return Err("Too many SSH agent operations".into());
        }
        if let Some(old) = owners.remove(owner) {
            old.active.store(false, Ordering::SeqCst);
        }
        let id = Uuid::new_v4().to_string();
        let active = Arc::new(AtomicBool::new(true));
        owners.insert(
            owner.into(),
            Operation {
                id: id.clone(),
                expires: Instant::now() + TTL,
                active: active.clone(),
                ticket: None,
            },
        );
        Ok((id, active))
    }
    pub async fn list(
        &self,
        owner: &str,
        host: &str,
        port: u16,
        username: &str,
    ) -> Result<Selection, String> {
        let (id, active) = self.begin(owner)?;
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| "Too many SSH agent operations")?;
        let endpoint = Endpoint::new(host, port).map_err(|e| e.to_string())?;
        // Conservative portable login syntax. No command/URL/options are accepted.
        if username.is_empty()
            || username.len() > 64
            || username.starts_with('-')
            || !username
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
        {
            return Err(
                "Enter an SSH username of at most 64 letters, digits, dots, underscores or hyphens"
                    .into(),
            );
        }
        let (socket, keys) = tokio::time::timeout(LIMIT, async {
            let (handle, _shutdown) = self.hosts.connect_trusted(endpoint.clone()).await?;
            ensure_active(&active)?;
            // Resolve and connect only after a completed, persisted-trust handshake.
            let socket = self
                .socket
                .clone()
                .or_else(|| std::env::var_os("SSH_AUTH_SOCK").map(PathBuf::from))
                .filter(|p| p.is_absolute())
                .ok_or(UNAVAILABLE)?;
            let mut agent = AgentClient::connect_uds(&socket)
                .await
                .map_err(|_| UNAVAILABLE)?
                .dynamic();
            let keys = identities(&mut agent).await?;
            handle
                .disconnect(Disconnect::ByApplication, "identity listing complete", "en")
                .await
                .map_err(|_| "SSH disconnect failed")?;
            Ok::<_, String>((socket, keys))
        })
        .await
        .map_err(|_| "SSH identity listing timed out")??;
        let identities = keys
            .iter()
            .map(|key| Identity {
                algorithm: key.algorithm().to_string(),
                fingerprint: fingerprint(key),
            })
            .collect();
        let mut owners = self.owners.lock().map_err(|_| STALE)?;
        let operation = owners
            .get_mut(owner)
            .filter(|op| op.id == id && op.expires > Instant::now())
            .ok_or(STALE)?;
        ensure_active(&active)?;
        operation.expires = Instant::now() + TTL;
        operation.ticket = Some(Ticket {
            endpoint: endpoint.clone(),
            username: username.into(),
            socket,
            keys,
        });
        Ok(Selection {
            id,
            host: endpoint.host().into(),
            port,
            username: username.into(),
            identities,
            expires_in_seconds: TTL.as_secs(),
        })
    }
    pub async fn authenticate(
        &self,
        owner: &str,
        id: &str,
        selected: &str,
    ) -> Result<Authentication, String> {
        tokio::time::timeout(LIMIT, async {
            let connection = self.connect_authenticated(owner, id, selected).await?;
            connection
                .handle
                .disconnect(
                    Disconnect::ByApplication,
                    "authentication check complete",
                    "en",
                )
                .await
                .map_err(|_| "SSH disconnect failed")?;
            Ok(connection.identity)
        })
        .await
        .map_err(|_| "SSH authentication check timed out")?
    }
    pub(super) async fn connect_authenticated(
        &self,
        owner: &str,
        id: &str,
        selected: &str,
    ) -> Result<Authenticated, String> {
        if id.len() > 64 || selected.len() > 128 {
            return Err(STALE.into());
        }
        let (ticket, active) = {
            let mut owners = self.owners.lock().map_err(|_| STALE)?;
            let operation = owners
                .get_mut(owner)
                .filter(|op| op.id == id && op.expires > Instant::now())
                .ok_or(STALE)?;
            (
                operation.ticket.take().ok_or(STALE)?,
                operation.active.clone(),
            )
        };
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| "Too many SSH agent operations")?;
        let selected = ticket
            .keys
            .into_iter()
            .find(|key| fingerprint(key) == selected)
            .ok_or("Selected SSH identity unavailable")?;
        tokio::time::timeout(LIMIT, async {
            ensure_active(&active)?;
            let (mut handle, _shutdown) =
                self.hosts.connect_trusted(ticket.endpoint.clone()).await?;
            ensure_active(&active)?;
            let mut agent = AgentClient::connect_uds(&ticket.socket)
                .await
                .map_err(|_| UNAVAILABLE)?
                .dynamic();
            let current = identities(&mut agent).await?;
            if !current
                .iter()
                .any(|key| key.key_data() == selected.key_data())
            {
                return Err("Selected SSH identity unavailable".into());
            }
            let mut signer = SelectedSigner {
                agent,
                selected: selected.clone(),
                active: active.clone(),
            };
            let result = handle
                .authenticate_publickey_with(&ticket.username, selected.clone(), None, &mut signer)
                .await
                .map_err(|_| "SSH authentication or agent signing failed")?;
            if !result.success() {
                return Err("SSH authentication rejected for the selected identity".into());
            }
            ensure_active(&active)?;
            Ok(Authenticated {
                handle,
                shutdown: _shutdown,
                identity: Authentication {
                    host: ticket.endpoint.host().into(),
                    port: ticket.endpoint.port(),
                    username: ticket.username,
                    fingerprint: fingerprint(&selected),
                },
            })
        })
        .await
        .map_err(|_| "SSH authentication check timed out")?
    }
}
fn ensure_active(active: &AtomicBool) -> Result<(), String> {
    if active.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err(STALE.into())
    }
}
fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}
async fn identities(agent: &mut Agent) -> Result<Vec<PublicKey>, String> {
    // russh also bounds each agent response frame to 256 KiB before allocation.
    let identities = agent.request_identities().await.map_err(|_| UNAVAILABLE)?;
    if identities.len() > 128 {
        return Err("SSH agent returned too many identities".into());
    }
    let mut keys: Vec<PublicKey> = Vec::new();
    for identity in identities {
        if let AgentIdentity::PublicKey { key, .. } = identity {
            if !keys
                .iter()
                .any(|existing| existing.key_data() == key.key_data())
            {
                keys.push(key);
            }
        }
    }
    Ok(keys)
}
struct SelectedSigner {
    agent: Agent,
    selected: PublicKey,
    active: Arc<AtomicBool>,
}
struct SigningFailed;
impl From<russh::SendError> for SigningFailed {
    fn from(_: russh::SendError) -> Self {
        Self
    }
}
impl Signer for SelectedSigner {
    type Error = SigningFailed;
    async fn auth_sign(
        &mut self,
        identity: &AgentIdentity,
        hash: Option<HashAlg>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, Self::Error> {
        if !self.active.load(Ordering::SeqCst) {
            return Err(SigningFailed);
        }
        let AgentIdentity::PublicKey { key, .. } = identity else {
            return Err(SigningFailed);
        };
        if key.key_data() != self.selected.key_data() {
            return Err(SigningFailed);
        }
        self.agent
            .sign_request(identity, hash, data)
            .await
            .map_err(|_| SigningFailed)
    }
}
#[cfg(test)]
mod tests;
