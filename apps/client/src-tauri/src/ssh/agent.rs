//! Window-bound identity selection: macOS agent or encrypted app-owned key.
use super::{host::HostManager, trust::Endpoint};
use russh::{
    keys::{
        agent::{client::AgentClient, AgentIdentity},
        HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey,
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
enum Source {
    Agent(PathBuf),
    Application(Arc<PrivateKey>),
}
struct Ticket {
    endpoint: Endpoint,
    username: String,
    source: Source,
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
    identity: Option<super::identity::IdentityStore>,
}
impl AgentManager {
    pub fn new(hosts: HostManager, socket: Option<PathBuf>) -> Self {
        Self {
            hosts,
            owners: Arc::default(),
            permits: Arc::new(tokio::sync::Semaphore::new(8)),
            socket,
            identity: None,
        }
    }
    pub fn with_identity(mut self, identity: super::identity::IdentityStore) -> Self {
        self.identity = Some(identity);
        self
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
        validate_username(username)?;
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
            source: Source::Agent(socket),
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
    pub async fn list_application(
        &self,
        owner: &str,
        host: &str,
        port: u16,
        username: &str,
        password: zeroize::Zeroizing<String>,
    ) -> Result<Selection, String> {
        self.list_application_inner(owner, host, port, username, Some(password))
            .await
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub async fn list_system_application(
        &self,
        owner: &str,
        host: &str,
        port: u16,
        username: &str,
    ) -> Result<Selection, String> {
        self.list_application_inner(owner, host, port, username, None)
            .await
    }
    async fn list_application_inner(
        &self,
        owner: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<zeroize::Zeroizing<String>>,
    ) -> Result<Selection, String> {
        let (id, active) = self.begin(owner)?;
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| "Too many SSH identity operations")?;
        let endpoint = Endpoint::new(host, port).map_err(|error| error.to_string())?;
        validate_username(username)?;
        let store = self
            .identity
            .clone()
            .ok_or("Application SSH identity unavailable")?;
        let key = tokio::time::timeout(LIMIT, async {
            // Complete persisted-trust verification before touching private material.
            let (handle, _shutdown) = self.hosts.connect_trusted(endpoint.clone()).await?;
            ensure_active(&active)?;
            handle
                .disconnect(Disconnect::ByApplication, "identity selection", "en")
                .await
                .map_err(|_| "SSH disconnect failed")?;
            ensure_active(&active)?;
            let unlocking = active.clone();
            let key = tokio::task::spawn_blocking(move || {
                ensure_active(&unlocking)?;
                match password {
                    Some(password) => store.signing_key(password),
                    #[cfg(any(target_os = "linux", target_os = "android"))]
                    None => store.signing_system(),
                    #[cfg(not(any(target_os = "linux", target_os = "android")))]
                    None => {
                        Err("System SSH identity storage is unavailable on this platform".into())
                    }
                }
            })
            .await
            .map_err(|_| "Application SSH identity unavailable")??;
            ensure_active(&active)?;
            Ok::<_, String>(Arc::new(key))
        })
        .await
        .map_err(|_| "SSH identity unlock timed out")??;
        let selected = key.public_key().clone();
        let identities = vec![Identity {
            algorithm: selected.algorithm().to_string(),
            fingerprint: fingerprint(&selected),
        }];
        let expires = Instant::now() + TTL;
        {
            let mut owners = self.owners.lock().map_err(|_| STALE)?;
            let operation = owners
                .get_mut(owner)
                .filter(|op| op.id == id && op.expires > Instant::now())
                .ok_or(STALE)?;
            ensure_active(&active)?;
            operation.expires = expires;
            operation.ticket = Some(Ticket {
                endpoint: endpoint.clone(),
                username: username.into(),
                source: Source::Application(key),
                keys: vec![selected],
            });
        }
        // Public agent selections did not need a timer. Private application keys
        // must be dropped at expiry even if no later user action occurs.
        let owners = Arc::downgrade(&self.owners);
        let owner = owner.to_owned();
        let expiring_id = id.clone();
        tokio::spawn(async move {
            tokio::time::sleep_until(tokio::time::Instant::from_std(expires)).await;
            if let Some(owners) = owners.upgrade() {
                if let Ok(mut owners) = owners.lock() {
                    if owners
                        .get(&owner)
                        .is_some_and(|op| op.id == expiring_id && op.expires <= Instant::now())
                    {
                        if let Some(expired) = owners.remove(&owner) {
                            expired.active.store(false, Ordering::SeqCst);
                        }
                    }
                }
            }
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
            let result = match ticket.source {
                Source::Agent(socket) => {
                    let mut agent = AgentClient::connect_uds(&socket)
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
                    handle
                        .authenticate_publickey_with(
                            &ticket.username,
                            selected.clone(),
                            None,
                            &mut signer,
                        )
                        .await
                        .map_err(|_| "SSH authentication or agent signing failed")?
                }
                Source::Application(key) => {
                    if key.public_key().key_data() != selected.key_data() {
                        return Err("Selected SSH identity unavailable".into());
                    }
                    ensure_active(&active)?;
                    handle
                        .authenticate_publickey(
                            &ticket.username,
                            PrivateKeyWithHashAlg::new(key, None),
                        )
                        .await
                        .map_err(|_| "SSH application identity authentication failed")?
                }
            };
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
fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty()
        || username.len() > 64
        || username.starts_with('-')
        || !username
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
    {
        Err(
            "Enter an SSH username of at most 64 letters, digits, dots, underscores or hyphens"
                .into(),
        )
    } else {
        Ok(())
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
#[path = "app_tests.rs"]
mod app_tests;
#[cfg(all(test, target_os = "macos"))]
mod tests;
