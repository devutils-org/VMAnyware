use super::*;
use russh::{
    keys::{Algorithm, PrivateKey},
    server,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{net::TcpListener, task::JoinSet};
fn key() -> PrivateKey {
    PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap()
}
struct Fixture {
    addr: std::net::SocketAddr,
    key: Arc<Mutex<PrivateKey>>,
    auth: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
#[derive(Clone)]
struct Handler {
    auth: Arc<AtomicUsize>,
}
impl server::Handler for Handler {
    type Error = russh::Error;
    async fn auth_none(&mut self, _user: &str) -> Result<server::Auth, Self::Error> {
        self.auth.fetch_add(1, Ordering::SeqCst);
        Ok(server::Auth::reject())
    }
    async fn auth_password(
        &mut self,
        _user: &str,
        _password: &str,
    ) -> Result<server::Auth, Self::Error> {
        self.auth.fetch_add(1, Ordering::SeqCst);
        Ok(server::Auth::reject())
    }
    async fn auth_publickey(
        &mut self,
        _user: &str,
        _key: &russh::keys::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        self.auth.fetch_add(1, Ordering::SeqCst);
        Ok(server::Auth::reject())
    }
}
impl Fixture {
    async fn start() -> Self {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let key = Arc::new(Mutex::new(key()));
        let auth = Arc::new(AtomicUsize::new(0));
        let keys = key.clone();
        let attempts = auth.clone();
        let task = tokio::spawn(async move {
            let mut sessions = JoinSet::new();
            loop {
                tokio::select! {
                    accepted=listener.accept()=>{
                        let (socket,_)=accepted.unwrap();
                        let config=Arc::new(server::Config {keys:vec![keys.lock().unwrap().clone()],inactivity_timeout:Some(PROBE_LIMIT),..Default::default()});
                        let handler=Handler {auth:attempts.clone()};
                        sessions.spawn(async move {if let Ok(session)=server::run_stream(config,socket,handler).await {let _=session.await;}});
                    },
                    _=sessions.join_next(),if !sessions.is_empty()=>{},
                }
            }
        });
        Self {
            addr,
            key,
            auth,
            task,
        }
    }
    async fn inspect(&self, manager: &HostManager, owner: &str) -> HostInspection {
        manager
            .inspect(owner, &self.addr.ip().to_string(), self.addr.port())
            .await
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn manager() -> (tempfile::TempDir, HostManager) {
    let root = tempfile::tempdir().unwrap();
    let manager = HostManager::new(root.path().join("parent").join("managed"));
    (root, manager)
}
fn expire(manager: &HostManager, owner: &str) {
    manager
        .inner
        .owners
        .lock()
        .unwrap()
        .get_mut(owner)
        .unwrap()
        .expires = Instant::now() - Duration::from_secs(1);
}
#[tokio::test]
async fn real_handshake_confirmation_reopen_changed_and_no_authentication() {
    let fixture = Fixture::start().await;
    let (_root, manager) = manager();
    assert!(!manager.inner.path.exists());
    let unknown = fixture.inspect(&manager, "main").await;
    assert_eq!(unknown.status, "unknown");
    assert_eq!(unknown.expires_in_seconds, 120);
    assert_eq!(
        unknown.fingerprint,
        fixture
            .key
            .lock()
            .unwrap()
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string()
    );
    assert_eq!(unknown.algorithm, "ssh-ed25519");
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    let id = unknown.challenge_id.unwrap();
    let confirmation = manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .unwrap();
    assert_eq!(confirmation.host, "127.0.0.1");
    assert_eq!(confirmation.port, fixture.addr.port());
    assert_eq!(confirmation.fingerprint, unknown.fingerprint);
    assert!(manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .is_err());
    let reopened = HostManager::new(manager.inner.path.clone());
    let trusted = fixture.inspect(&reopened, "main").await;
    assert_eq!(trusted.status, "trusted");
    assert!(trusted.challenge_id.is_none());
    assert_eq!(trusted.expires_in_seconds, 0);
    let before = std::fs::read(manager.inner.path.join("trust.json")).unwrap();
    *fixture.key.lock().unwrap() = key();
    let changed = fixture.inspect(&reopened, "main").await;
    assert_eq!(changed.status, "changed");
    assert_eq!(
        changed.previous_fingerprint.as_deref(),
        Some(unknown.fingerprint.as_str())
    );
    assert_ne!(changed.fingerprint, unknown.fingerprint);
    assert_eq!(
        before,
        std::fs::read(manager.inner.path.join("trust.json")).unwrap()
    );
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    let serialized = serde_json::to_value(&changed).unwrap();
    assert!(serialized.get("publicKey").is_none());
    let id = changed.challenge_id.clone().unwrap();
    // First-use confirmation never consumes or approves a replacement challenge.
    assert!(reopened
        .confirm("main", &id, &changed.fingerprint)
        .await
        .is_err());
    assert_eq!(
        before,
        std::fs::read(manager.inner.path.join("trust.json")).unwrap()
    );
    let replaced = reopened
        .replace("main", &id, &unknown.fingerprint, &changed.fingerprint)
        .await
        .unwrap();
    assert_eq!(replaced.fingerprint, changed.fingerprint);
    assert!(reopened
        .replace("main", &id, &unknown.fingerprint, &changed.fingerprint)
        .await
        .is_err());
    let trusted = fixture.inspect(&reopened, "main").await;
    assert_eq!(trusted.status, "trusted");
    assert!(trusted.previous_fingerprint.is_none());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn replacement_requires_replace_challenge_and_both_fingerprints() {
    let fixture = Fixture::start().await;
    let (_root, manager) = manager();
    let unknown = fixture.inspect(&manager, "main").await;
    let id = unknown.challenge_id.unwrap();
    // A first-use challenge cannot be used to replace.
    assert!(manager
        .replace("main", &id, &unknown.fingerprint, &unknown.fingerprint)
        .await
        .is_err());
    manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .unwrap();
    let trusted = std::fs::read(manager.inner.path.join("trust.json")).unwrap();
    *fixture.key.lock().unwrap() = key();
    for (previous, current) in [
        ("SHA256:incorrect", None),
        (unknown.fingerprint.as_str(), Some("SHA256:incorrect")),
    ] {
        let changed = fixture.inspect(&manager, "main").await;
        assert!(manager
            .replace(
                "main",
                &changed.challenge_id.unwrap(),
                previous,
                current.unwrap_or(&changed.fingerprint),
            )
            .await
            .is_err());
        assert_eq!(
            trusted,
            std::fs::read(manager.inner.path.join("trust.json")).unwrap()
        );
    }
    let changed = fixture.inspect(&manager, "main").await;
    expire(&manager, "main");
    assert!(manager
        .replace(
            "main",
            &changed.challenge_id.unwrap(),
            &unknown.fingerprint,
            &changed.fingerprint
        )
        .await
        .is_err());
    assert_eq!(
        trusted,
        std::fs::read(manager.inner.path.join("trust.json")).unwrap()
    );
}
#[tokio::test]
async fn owner_fingerprint_expiry_cancel_and_new_inspection_bind_challenges() {
    let fixture = Fixture::start().await;
    let (_root, manager) = manager();
    let unknown = fixture.inspect(&manager, "main").await;
    let id = unknown.challenge_id.unwrap();
    assert!(manager
        .confirm("other", &id, &unknown.fingerprint)
        .await
        .is_err());
    assert!(manager.cancel("other", &id).is_err());
    assert!(manager
        .confirm("main", &id, "wrong fingerprint")
        .await
        .is_err());
    assert!(manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .is_err());
    let unknown = fixture.inspect(&manager, "main").await;
    let id = unknown.challenge_id.unwrap();
    expire(&manager, "main");
    assert!(manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .is_err());
    let unknown = fixture.inspect(&manager, "main").await;
    let id = unknown.challenge_id.unwrap();
    manager.cancel("main", &id).unwrap();
    assert!(manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .is_err());
    let unknown = fixture.inspect(&manager, "main").await;
    let id = unknown.challenge_id.unwrap();
    // Even an invalid subsequent endpoint invalidates the old observation.
    assert!(manager.inspect("main", "ssh://unsafe", 22).await.is_err());
    assert!(manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .is_err());
    let unknown = fixture.inspect(&manager, "main").await;
    let id = unknown.challenge_id.unwrap();
    manager.forget_owner("main");
    assert!(manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .is_err());
    // Reusing a destroyed window's label must not revive its old challenge.
    let current = fixture.inspect(&manager, "main").await;
    assert!(manager
        .confirm("main", &id, &unknown.fingerprint)
        .await
        .is_err());
    manager
        .cancel("main", current.challenge_id.as_deref().unwrap())
        .unwrap();
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manager.inner.path.join("trust.json")).unwrap())
            .unwrap();
    assert_eq!(document["entries"], serde_json::json!([]));
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn bounded_owners_and_out_of_order_results_cannot_replace_current() {
    let (_root, manager) = manager();
    let store = manager.store().await.unwrap();
    let old = manager.begin("main").unwrap();
    let latest = manager.begin("main").unwrap();
    let endpoint = trust::Endpoint::new("localhost", 22).unwrap();
    let observe = || {
        let key = key();
        let trust::Inspection::Unknown(c) =
            store.inspect(endpoint.clone(), key.public_key()).unwrap()
        else {
            panic!("unknown expected")
        };
        Observed {
            algorithm: "ssh-ed25519".into(),
            fingerprint: c.fingerprint().into(),
            inspection: Checked::Unknown(c),
        }
    };
    assert!(manager
        .finish("main", old, endpoint.clone(), observe())
        .is_err());
    let current = manager
        .finish("main", latest, endpoint.clone(), observe())
        .unwrap();
    let current_id = current.challenge_id.unwrap();
    assert!(manager
        .finish("main", old, endpoint.clone(), observe())
        .is_err());
    assert_eq!(
        manager.inner.owners.lock().unwrap()["main"]
            .challenge
            .as_ref()
            .unwrap()
            .id,
        current_id
    );
    for n in 0..7 {
        manager.begin(&format!("owner-{n}")).unwrap();
    }
    assert!(manager.begin("overflow").is_err());
    expire(&manager, "owner-0");
    assert!(manager.begin("overflow").is_ok());
    manager.cancel("main", &current_id).unwrap();
}
#[tokio::test]
async fn incomplete_handshake_and_corrupt_store_are_safe_failures() {
    let (_root, manager) = manager();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
    });
    assert_eq!(
        manager
            .inspect("main", "127.0.0.1", addr.port())
            .await
            .err()
            .unwrap(),
        "SSH host inspection connection failed"
    );
    task.await.unwrap();
    std::fs::write(
        manager.inner.path.join("trust.json"),
        b"malformed secret input",
    )
    .unwrap();
    let fixture = Fixture::start().await;
    let error = manager
        .inspect("main", "127.0.0.1", fixture.addr.port())
        .await
        .err()
        .unwrap();
    assert_eq!(error, "Invalid host trust store");
    assert!(!error.contains("secret"));
}

#[tokio::test]
async fn invalid_input_and_probe_capacity_reject_before_filesystem_or_network() {
    let (_root, manager) = manager();
    for (host, port) in [("../injected", 22), ("valid", 0), ("ssh://host", 22)] {
        assert_eq!(
            manager.inspect("main", host, port).await.err().unwrap(),
            "Invalid SSH endpoint"
        );
    }
    assert!(!manager.inner.path.exists());
    let permits = manager.inner.probes.acquire_many(8).await.unwrap();
    assert_eq!(
        manager
            .inspect("main", "127.0.0.1", 22)
            .await
            .err()
            .unwrap(),
        "Too many active SSH inspections"
    );
    assert!(!manager.inner.path.exists());
    drop(permits);
}
