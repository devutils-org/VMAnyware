use super::*;
use crate::ssh::trust::{Inspection, Store};
use russh::{
    keys::{Algorithm, PrivateKey},
    server,
};
use std::{
    process::{Child, Command, Stdio},
    sync::atomic::AtomicUsize,
};
use tokio::{net::TcpListener, task::JoinSet};
fn key() -> PrivateKey {
    PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap()
}
struct DisposableAgent {
    child: Child,
    socket: PathBuf,
    _directory: tempfile::TempDir,
}
impl DisposableAgent {
    async fn start() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("pl-native-agent-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let socket = directory.path().join("a");
        let child = Command::new("/usr/bin/ssh-agent")
            .args(["-D", "-U", "-a"])
            .arg(&socket)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut agent = Self {
            child,
            socket,
            _directory: directory,
        };
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                assert!(agent.child.try_wait().unwrap().is_none());
                if tokio::net::UnixStream::connect(&agent.socket).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        agent
    }
    async fn add(&self, key: &PrivateKey) {
        tokio::time::timeout(Duration::from_secs(2), async {
            AgentClient::connect_uds(&self.socket)
                .await
                .unwrap()
                .add_identity(key, &[])
                .await
                .unwrap();
        })
        .await
        .unwrap();
    }
}
impl Drop for DisposableAgent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
#[derive(Debug, PartialEq)]
enum Protocol {
    Pty(u32, u32),
    Shell,
    Resize(u32, u32),
    Data(Vec<u8>),
}
struct Fixture {
    port: u16,
    host_key: Arc<Mutex<PrivateKey>>,
    auth: Arc<AtomicUsize>,
    channels: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
    events: Arc<Mutex<Vec<Protocol>>>,
    connections: Arc<AtomicUsize>,
}
#[derive(Clone)]
struct Handler {
    pty: bool,
    refuse: Option<&'static str>,
    events: Arc<Mutex<Vec<Protocol>>>,
    allowed: PublicKey,
    auth: Arc<AtomicUsize>,
    channels: Arc<AtomicUsize>,
}
impl server::Handler for Handler {
    type Error = russh::Error;
    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        self.auth.fetch_add(1, Ordering::SeqCst);
        Ok(
            if user == "fixture" && key.key_data() == self.allowed.key_data() {
                server::Auth::Accept
            } else {
                server::Auth::reject()
            },
        )
    }
    async fn channel_open_session(
        &mut self,
        _: russh::Channel<server::Msg>,
        reply: server::ChannelOpenHandle,
        _: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.channels.fetch_add(1, Ordering::SeqCst);
        if self.pty {
            reply.accept().await;
        }
        Ok(())
    }
    async fn pty_request(
        &mut self,
        channel: russh::ChannelId,
        _: &str,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &[(russh::Pty, u32)],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.events.lock().unwrap().push(Protocol::Pty(cols, rows));
        if self.refuse == Some("pty") {
            session.channel_failure(channel)?;
        } else {
            session.channel_success(channel)?;
        }
        Ok(())
    }
    async fn shell_request(
        &mut self,
        channel: russh::ChannelId,
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.events.lock().unwrap().push(Protocol::Shell);
        if self.refuse == Some("shell") {
            session.channel_failure(channel)?;
        } else {
            session.channel_success(channel)?;
        }
        Ok(())
    }
    async fn window_change_request(
        &mut self,
        _: russh::ChannelId,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.events
            .lock()
            .unwrap()
            .push(Protocol::Resize(cols, rows));
        Ok(())
    }
    async fn data(
        &mut self,
        channel: russh::ChannelId,
        data: &[u8],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.events
            .lock()
            .unwrap()
            .push(Protocol::Data(data.to_vec()));
        if data == b"fixture-exit" {
            session.exit_status_request(channel, 42)?;
            session.eof(channel)?;
            session.close(channel)?;
        } else {
            session.data(channel, data.to_vec())?;
        }
        Ok(())
    }
}
impl Fixture {
    async fn start(allowed: PublicKey) -> Self {
        Self::with_pty(allowed, false).await
    }
    async fn with_pty(allowed: PublicKey, pty: bool) -> Self {
        Self::with_policy(allowed, pty, None).await
    }
    async fn with_policy(allowed: PublicKey, pty: bool, refuse: Option<&'static str>) -> Self {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let host_key = Arc::new(Mutex::new(key()));
        let auth = Arc::new(AtomicUsize::new(0));
        let channels = Arc::new(AtomicUsize::new(0));
        let keys = host_key.clone();
        let events = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let active = connections.clone();
        let handler = Handler {
            pty,
            refuse,
            events: events.clone(),
            allowed,
            auth: auth.clone(),
            channels: channels.clone(),
        };
        let task = tokio::spawn(async move {
            let mut sessions = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (socket, _) = accepted.unwrap();
                        let config = Arc::new(server::Config { keys: vec![keys.lock().unwrap().clone()], auth_rejection_time: Duration::ZERO, auth_rejection_time_initial: Some(Duration::ZERO), inactivity_timeout: Some(LIMIT), ..Default::default() });
                        let handler = handler.clone();
                        let active = active.clone();
                        sessions.spawn(async move {
                            struct Active(Arc<AtomicUsize>);
                            impl Drop for Active { fn drop(&mut self) { self.0.fetch_sub(1, Ordering::SeqCst); } }
                            active.fetch_add(1, Ordering::SeqCst);
                            let _active = Active(active);
                            if let Ok(session) = server::run_stream(config, socket, handler).await { let _ = session.await; }
                        });
                    },
                    _ = sessions.join_next(), if !sessions.is_empty() => {},
                }
            }
        });
        Self {
            port,
            host_key,
            auth,
            channels,
            task,
            events,
            connections,
        }
    }
    fn trust(&self, store: &Store) {
        let host_key = self.host_key.lock().unwrap();
        let Inspection::Unknown(confirmation) = store
            .inspect(
                Endpoint::new("127.0.0.1", self.port).unwrap(),
                host_key.public_key(),
            )
            .unwrap()
        else {
            panic!("new host expected")
        };
        let fingerprint = confirmation.fingerprint().to_owned();
        store.confirm(confirmation, &fingerprint).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn manager(socket: PathBuf) -> (tempfile::TempDir, AgentManager, Store) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("trust");
    let store = Store::create(&path).unwrap();
    let mut manager = AgentManager::new(HostManager::new(path));
    manager.socket = Some(socket);
    (root, manager, store)
}
#[tokio::test]
async fn native_selection_authenticates_only_explicit_identity_once_without_channels() {
    let agent = DisposableAgent::start().await;
    let allowed = key();
    let wrong = key();
    let selected = fingerprint(allowed.public_key());
    let wrong_fp = fingerprint(wrong.public_key());
    let fixture = Fixture::start(allowed.public_key().clone()).await;
    agent.add(&allowed).await;
    agent.add(&wrong).await;
    drop(allowed);
    drop(wrong);
    let (_root, manager, store) = manager(agent.socket.clone());
    fixture.trust(&store);
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    assert_eq!(selection.identities.len(), 2);
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    assert!(manager
        .authenticate("other", &selection.id, &selected)
        .await
        .is_err());
    let authenticated = manager
        .authenticate("main", &selection.id, &selected)
        .await
        .unwrap();
    assert_eq!(authenticated.fingerprint, selected);
    assert_eq!(authenticated.username, "fixture");
    assert!(manager
        .authenticate("main", &selection.id, &selected)
        .await
        .is_err());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 1);
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    assert_eq!(
        manager
            .authenticate("main", &selection.id, &wrong_fp)
            .await
            .err()
            .unwrap(),
        "SSH authentication rejected for the selected identity"
    );
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.channels.load(Ordering::SeqCst), 0);
    assert!(fixture.events.lock().unwrap().is_empty());
    wait_closed(&fixture).await;
    let json = serde_json::to_value(selection).unwrap();
    assert!(json["identities"][0].get("publicKey").is_none());
    assert!(json["identities"][0].get("comment").is_none());
    assert!(json.get("socket").is_none());
}
#[tokio::test]
async fn unknown_and_changed_hosts_never_contact_the_agent() {
    let root_socket = tempfile::Builder::new()
        .prefix("pl-native-socket-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let socket = root_socket.path().join("a");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let fixture = Fixture::start(key().public_key().clone()).await;
    let (_root, manager, store) = manager(socket);
    assert!(manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .is_err());
    fixture.trust(&store);
    *fixture.host_key.lock().unwrap() = key();
    assert!(manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(50), listener.accept())
            .await
            .is_err()
    );
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn selection_rechecks_host_and_rejects_removed_keys_cancel_expiry_and_replay() {
    let agent = DisposableAgent::start().await;
    let allowed = key();
    let selected = fingerprint(allowed.public_key());
    let fixture = Fixture::start(allowed.public_key().clone()).await;
    let (_root, manager, store) = manager(agent.socket.clone());
    fixture.trust(&store);
    agent.add(&allowed).await;
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    manager.cancel("main");
    assert!(manager
        .authenticate("main", &selection.id, &selected)
        .await
        .is_err());
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    manager
        .owners
        .lock()
        .unwrap()
        .get_mut("main")
        .unwrap()
        .expires = Instant::now() - Duration::from_secs(1);
    assert!(manager
        .authenticate("main", &selection.id, &selected)
        .await
        .is_err());
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    assert!(manager
        .list("main", "invalid/path", fixture.port, "fixture")
        .await
        .is_err());
    assert!(manager
        .authenticate("main", &selection.id, &selected)
        .await
        .is_err());
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    AgentClient::connect_uds(&agent.socket)
        .await
        .unwrap()
        .remove_identity(allowed.public_key())
        .await
        .unwrap();
    assert_eq!(
        manager
            .authenticate("main", &selection.id, &selected)
            .await
            .err()
            .unwrap(),
        "Selected SSH identity unavailable"
    );
    agent.add(&allowed).await;
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    *fixture.host_key.lock().unwrap() = key();
    assert!(manager
        .authenticate("main", &selection.id, &selected)
        .await
        .err()
        .unwrap()
        .contains("host verification failed"));
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn stalled_agent_deadline_closes_socket_without_authentication() {
    use tokio::io::AsyncReadExt;
    let directory = tempfile::Builder::new()
        .prefix("pl-native-stall-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let socket = directory.path().join("a");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let fixture = Fixture::start(key().public_key().clone()).await;
    let (_root, manager, store) = manager(socket);
    fixture.trust(&store);
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 64];
        assert!(stream.read(&mut buffer).await.unwrap() > 0);
        assert_eq!(stream.read(&mut buffer).await.unwrap(), 0);
    });
    assert_eq!(
        manager
            .list("main", "127.0.0.1", fixture.port, "fixture")
            .await
            .err()
            .unwrap(),
        "SSH identity listing timed out"
    );
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancellation_discards_inflight_listing_and_old_cancel_cannot_cancel_new_selection() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let directory = tempfile::Builder::new()
        .prefix("pl-native-cancel-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let socket = directory.path().join("a");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let fixture = Fixture::start(key().public_key().clone()).await;
    let (_root, manager, store) = manager(socket);
    fixture.trust(&store);
    let pending = {
        let manager = manager.clone();
        let port = fixture.port;
        tokio::spawn(async move { manager.list("main", "127.0.0.1", port, "fixture").await })
    };
    let (mut stream, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut request = [0; 5];
    stream.read_exact(&mut request).await.unwrap();
    assert_eq!(request, [0, 0, 0, 1, 11]);
    let old = manager.owners.lock().unwrap()["main"].id.clone();
    manager.cancel_selection("main", &old);
    let (latest, active) = manager.begin("main").unwrap();
    manager.cancel_selection("main", &old);
    assert!(active.load(Ordering::SeqCst));
    assert_eq!(manager.owners.lock().unwrap()["main"].id, latest);
    stream
        .write_all(&[0, 0, 0, 5, 12, 0, 0, 0, 0])
        .await
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), pending)
            .await
            .unwrap()
            .unwrap()
            .err()
            .unwrap(),
        STALE
    );
    manager.cancel("main");
    assert!(!active.load(Ordering::SeqCst));
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn invalid_inputs_capacity_and_unlisted_identity_fail_before_authentication() {
    let agent = DisposableAgent::start().await;
    let fixture = Fixture::start(key().public_key().clone()).await;
    let (_root, manager, store) = manager(agent.socket.clone());
    fixture.trust(&store);
    for username in [
        "",
        "-option",
        "user@host",
        "user;command",
        " user",
        "user\n",
    ] {
        assert!(manager
            .list("main", "127.0.0.1", fixture.port, username)
            .await
            .is_err());
    }
    let empty = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    assert!(empty.identities.is_empty());
    assert_eq!(
        manager
            .authenticate("main", &empty.id, "SHA256:unlisted")
            .await
            .err()
            .unwrap(),
        "Selected SSH identity unavailable"
    );
    assert!(manager
        .authenticate("main", &empty.id, "SHA256:unlisted")
        .await
        .is_err());
    for n in 0..7 {
        manager.begin(&format!("owner-{n}")).unwrap();
    }
    assert!(manager.begin("overflow").is_err());
    manager.cancel("owner-0");
    assert!(manager.begin("overflow").is_ok());
    let permits = manager.permits.acquire_many(8).await.unwrap();
    assert_eq!(
        manager
            .list("main", "127.0.0.1", fixture.port, "fixture")
            .await
            .err()
            .unwrap(),
        "Too many SSH agent operations"
    );
    drop(permits);
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
}

async fn wait_closed(fixture: &Fixture) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while fixture.connections.load(Ordering::SeqCst) != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
async fn next_output(
    sessions: &crate::ssh::session::SessionManager,
    id: &str,
) -> crate::ssh::session::Event {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(event) = sessions.read("main", id).await.unwrap() {
                break event;
            }
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn native_pty_binary_ctrl_c_resize_disconnect_and_reconnect() {
    use crate::ssh::session::{Event, SessionManager};
    let agent = DisposableAgent::start().await;
    let allowed = key();
    let selected = fingerprint(allowed.public_key());
    agent.add(&allowed).await;
    let fixture = Fixture::with_pty(allowed.public_key().clone(), true).await;
    drop(allowed);
    let (_root, manager, store) = manager(agent.socket.clone());
    fixture.trust(&store);
    let sessions = SessionManager::new(manager.clone());
    for _ in 0..2 {
        let selection = manager
            .list("main", "127.0.0.1", fixture.port, "fixture")
            .await
            .unwrap();
        let started = sessions
            .start("main", &selection.id, &selected, 80, 24)
            .await
            .unwrap();
        let id = serde_json::to_value(started).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(sessions.read("other", &id).await.is_err());
        assert!(sessions.write("other", &id, vec![3]).is_err());
        assert!(sessions.resize("other", &id, 90, 30).is_err());
        assert!(sessions.close("other", &id).is_err());
        for bytes in [vec![0, 255, 3, 13, 10], vec![65; 32768]] {
            sessions.write("main", &id, bytes.clone()).unwrap();
            let mut received = Vec::new();
            while received.len() < bytes.len() {
                let Event::Data { bytes } = next_output(&sessions, &id).await else {
                    panic!("binary output expected")
                };
                received.extend(bytes);
            }
            assert_eq!(received, bytes);
        }
        assert!(sessions.write("main", &id, vec![]).is_err());
        assert!(sessions.write("main", &id, vec![0; 32769]).is_err());
        assert!(sessions.resize("main", &id, 0, 25).is_err());
        sessions.resize("main", &id, 132, 43).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !fixture
                .events
                .lock()
                .unwrap()
                .contains(&Protocol::Resize(132, 43))
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        sessions.close("main", &id).unwrap();
        assert!(sessions.read("main", &id).await.is_err());
        assert!(sessions.write("main", &id, vec![3]).is_err());
        wait_closed(&fixture).await;
    }
    assert_eq!(fixture.channels.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 2);
    assert_eq!(
        fixture
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| **event == Protocol::Pty(80, 24))
            .count(),
        2
    );
}
#[tokio::test]
async fn native_pty_remote_exit_and_window_cleanup_close_owned_transport() {
    use crate::ssh::session::{Event, SessionManager};
    let agent = DisposableAgent::start().await;
    let allowed = key();
    let selected = fingerprint(allowed.public_key());
    agent.add(&allowed).await;
    let fixture = Fixture::with_pty(allowed.public_key().clone(), true).await;
    let (_root, manager, store) = manager(agent.socket.clone());
    fixture.trust(&store);
    let sessions = SessionManager::new(manager.clone());
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    let started = sessions
        .start("main", &selection.id, &selected, 80, 24)
        .await
        .unwrap();
    let id = serde_json::to_value(started).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    sessions
        .write("main", &id, b"fixture-exit".to_vec())
        .unwrap();
    assert!(matches!(
        next_output(&sessions, &id).await,
        Event::Exit { status: 42 }
    ));
    assert!(matches!(next_output(&sessions, &id).await, Event::Eof));
    assert!(matches!(next_output(&sessions, &id).await, Event::Closed));
    wait_closed(&fixture).await;
    let selection = manager
        .list("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    let started = sessions
        .start("main", &selection.id, &selected, 80, 24)
        .await
        .unwrap();
    let id = serde_json::to_value(started).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    sessions.forget_owner("main");
    assert!(sessions.read("main", &id).await.is_err());
    wait_closed(&fixture).await;
    // A server that refuses channels must never yield a started session.
    let refusing = Fixture::start(allowed.public_key().clone()).await;
    refusing.trust(&store);
    let selection = manager
        .list("main", "127.0.0.1", refusing.port, "fixture")
        .await
        .unwrap();
    assert_eq!(
        sessions
            .start("main", &selection.id, &selected, 80, 24)
            .await
            .err()
            .unwrap(),
        "SSH session channel refused"
    );
    assert_eq!(refusing.auth.load(Ordering::SeqCst), 1);
    assert_eq!(refusing.channels.load(Ordering::SeqCst), 1);
    wait_closed(&refusing).await;
    for request in ["pty", "shell"] {
        let refusing =
            Fixture::with_policy(allowed.public_key().clone(), true, Some(request)).await;
        refusing.trust(&store);
        let selection = manager
            .list("main", "127.0.0.1", refusing.port, "fixture")
            .await
            .unwrap();
        assert_eq!(
            sessions
                .start("main", &selection.id, &selected, 80, 24)
                .await
                .err()
                .unwrap(),
            "SSH server refused or interrupted the PTY/shell request"
        );
        wait_closed(&refusing).await;
    }
}
