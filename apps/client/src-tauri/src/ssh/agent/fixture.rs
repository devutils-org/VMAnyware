#[path = "../../../../../../experiments/ssh/src/shell.rs"]
mod shell;
use russh::keys::{agent::client::AgentClient, PublicKey};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};
const LIMIT: Duration = Duration::from_secs(10);
#[cfg(test)]
use crate::ssh::trust::{Endpoint, Inspection, Store};
use russh::{
    keys::{Algorithm, PrivateKey},
    server,
};
use std::{
    process::{Child, Command, Stdio},
    sync::atomic::AtomicUsize,
};
use tokio::{net::TcpListener, task::JoinSet};
pub(crate) fn key() -> PrivateKey {
    PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap()
}
pub(crate) struct DisposableAgent {
    child: Child,
    pub(crate) socket: PathBuf,
    _directory: tempfile::TempDir,
}
impl DisposableAgent {
    pub(crate) async fn start() -> Self {
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
    pub(crate) async fn add(&self, key: &PrivateKey) {
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
pub(crate) enum Protocol {
    Pty(u32, u32),
    Shell,
    Resize(u32, u32),
    Data(Vec<u8>),
}
pub(crate) struct Fixture {
    pub(crate) port: u16,
    pub(crate) host_key: Arc<Mutex<PrivateKey>>,
    pub(crate) auth: Arc<AtomicUsize>,
    pub(crate) channels: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
    pub(crate) events: Arc<Mutex<Vec<Protocol>>>,
    pub(crate) connections: Arc<AtomicUsize>,
    pub(crate) shells: Arc<Mutex<Vec<Arc<Mutex<shell::Shell>>>>>,
}
#[derive(Clone)]
struct Handler {
    os_shell: bool,
    size: (u16, u16),
    shell: Option<Arc<Mutex<shell::Shell>>>,
    pub(crate) shells: Arc<Mutex<Vec<Arc<Mutex<shell::Shell>>>>>,
    pty: bool,
    refuse: Option<&'static str>,
    pub(crate) events: Arc<Mutex<Vec<Protocol>>>,
    allowed: PublicKey,
    pub(crate) auth: Arc<AtomicUsize>,
    pub(crate) channels: Arc<AtomicUsize>,
}
impl Drop for Handler {
    fn drop(&mut self) {
        if let Some(shell) = &self.shell {
            shell.lock().unwrap().close();
        }
    }
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
        self.size = (cols as u16, rows as u16);
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
            if self.os_shell {
                let shell = Arc::new(Mutex::new(shell::Shell::start(
                    self.size,
                    channel,
                    session.handle(),
                )));
                self.shells.lock().unwrap().push(shell.clone());
                self.shell = Some(shell);
            }
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
        if let Some(shell) = &self.shell {
            shell.lock().unwrap().resize(cols as u16, rows as u16);
        }
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
        if let Some(shell) = &self.shell {
            shell.lock().unwrap().write(data);
        } else if data == b"fixture-exit" {
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
    pub(crate) async fn start(allowed: PublicKey) -> Self {
        Self::with_pty(allowed, false).await
    }
    pub(crate) async fn with_pty(allowed: PublicKey, pty: bool) -> Self {
        Self::with_policy(allowed, pty, None).await
    }
    pub(crate) async fn with_policy(
        allowed: PublicKey,
        pty: bool,
        refuse: Option<&'static str>,
    ) -> Self {
        Self::with_configuration(allowed, pty, refuse, false).await
    }
    pub(crate) async fn with_configuration(
        allowed: PublicKey,
        pty: bool,
        refuse: Option<&'static str>,
        os_shell: bool,
    ) -> Self {
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
        let shells = Arc::new(Mutex::new(Vec::new()));
        let handler = Handler {
            os_shell,
            size: (80, 24),
            shell: None,
            shells: shells.clone(),
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
            shells,
        }
    }
    #[cfg(test)]
    pub(crate) fn trust(&self, store: &Store) {
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
impl Fixture {
    pub(crate) fn shutdown(&self) {
        let mut shells = self.shells.lock().unwrap();
        for shell in shells.iter() {
            shell.lock().unwrap().close();
        }
        shells.clear();
        self.task.abort();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.shutdown();
    }
}
