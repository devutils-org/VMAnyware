//! Isolated SSH and native Unix PTY feasibility fixtures. No product integration.
#![cfg(all(test, unix))]

use russh::{
    client,
    keys::{Algorithm, PrivateKey, PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate},
    server, Channel, ChannelId, ChannelMsg, Disconnect, Pty,
};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::mpsc,
    task::{JoinHandle, JoinSet},
    time::timeout,
};

const LIMIT: Duration = Duration::from_secs(10);

fn ephemeral_key() -> PrivateKey {
    PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap()
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Auth(bool),
    Pty { term: String, cols: u32, rows: u32 },
    Shell,
    Data(Vec<u8>),
    Resize(u32, u32),
    Closed,
}

#[derive(Clone)]
struct FixtureHandler {
    os_shell: bool,
    size: (u16, u16),
    pty: Option<Arc<std::sync::Mutex<os_pty::Shell>>>,
    shells: Arc<std::sync::Mutex<Vec<Arc<std::sync::Mutex<os_pty::Shell>>>>>,
    allowed_key: PublicKey,
    events: mpsc::UnboundedSender<Event>,
    auth_attempts: Arc<AtomicUsize>,
}

impl server::Handler for FixtureHandler {
    type Error = russh::Error;

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        self.auth_attempts.fetch_add(1, Ordering::SeqCst);
        let accepted = user == "fixture" && key.key_data() == self.allowed_key.key_data();
        let _ = self.events.send(Event::Auth(accepted));
        Ok(if accepted {
            server::Auth::Accept
        } else {
            server::Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<server::Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        term: &str,
        cols: u32,
        rows: u32,
        _pixels_w: u32,
        _pixels_h: u32,
        _modes: &[(Pty, u32)],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        self.size = (cols as u16, rows as u16);
        let _ = self.events.send(Event::Pty {
            term: term.into(),
            cols,
            rows,
        });
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        if self.os_shell {
            let shell = Arc::new(std::sync::Mutex::new(os_pty::Shell::start(
                self.size,
                channel,
                session.handle(),
            )));
            self.shells
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(shell.clone());
            self.pty = Some(shell);
        }
        let _ = self.events.send(Event::Shell);
        session.channel_success(channel)?;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        let _ = self.events.send(Event::Data(data.to_vec()));
        if let Some(pty) = &self.pty {
            pty.lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .write(data);
        } else {
            // Echo protocol bytes exactly; no POSIX signal interpretation.
            session.data(channel, data.to_vec())?;
        }
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _channel: ChannelId,
        cols: u32,
        rows: u32,
        _pixels_w: u32,
        _pixels_h: u32,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        if let Some(pty) = &self.pty {
            pty.lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .resize(cols as u16, rows as u16);
        }
        let _ = self.events.send(Event::Resize(cols, rows));
        Ok(())
    }

    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        if let Some(pty) = self.pty.take() {
            pty.lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .close();
        }
        let _ = self.events.send(Event::Closed);
        Ok(())
    }
}

struct Fixture {
    shells: Arc<std::sync::Mutex<Vec<Arc<std::sync::Mutex<os_pty::Shell>>>>>,
    addr: SocketAddr,
    trusted_host: PublicKey,
    user_key: Arc<PrivateKey>,
    auth_attempts: Arc<AtomicUsize>,
    events: mpsc::UnboundedReceiver<Event>,
    task: JoinHandle<()>,
}

impl Fixture {
    async fn start() -> Self {
        Self::with_os_shell(false).await
    }

    async fn with_os_shell(os_shell: bool) -> Self {
        let host_key = ephemeral_key();
        let trusted_host = host_key.public_key().clone();
        let user_key = Arc::new(ephemeral_key());
        let config = Arc::new(server::Config {
            keys: vec![host_key],
            inactivity_timeout: Some(LIMIT),
            auth_rejection_time: Duration::from_millis(10),
            auth_rejection_time_initial: Some(Duration::from_millis(10)),
            ..Default::default()
        });
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (sender, events) = mpsc::unbounded_channel();
        let auth_attempts = Arc::new(AtomicUsize::new(0));
        let shells = Arc::new(std::sync::Mutex::new(Vec::new()));
        let handler = FixtureHandler {
            os_shell,
            size: (80, 24),
            pty: None,
            shells: shells.clone(),
            allowed_key: user_key.public_key().clone(),
            events: sender,
            auth_attempts: auth_attempts.clone(),
        };
        let task = tokio::spawn(async move {
            let mut sessions = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (socket, _) = accepted.unwrap();
                        let config = config.clone();
                        let handler = handler.clone();
                        sessions.spawn(async move {
                            if let Ok(session) = server::run_stream(config, socket, handler).await {
                                // Rejected host keys intentionally cause a transport/session error.
                                let _ = session.await;
                            }
                        });
                    },
                    _ = sessions.join_next(), if !sessions.is_empty() => {},
                }
            }
        });
        Self {
            shells,
            addr,
            trusted_host,
            user_key,
            auth_attempts,
            events,
            task,
        }
    }

    async fn next_event(&mut self) -> Event {
        timeout(LIMIT, self.events.recv()).await.unwrap().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for shell in self
            .shells
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .iter()
        {
            shell
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .close();
        }
        self.task.abort();
    }
}

struct VerifiedClient {
    expected: Option<PublicKey>,
    checks: Arc<AtomicUsize>,
}

impl client::Handler for VerifiedClient {
    type Error = russh::Error;
    async fn check_server_key(
        &mut self,
        actual: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        // The endpoint-scoped expected public key is explicit, never accept-all/TOFU.
        Ok(match (self.expected.as_ref(), actual) {
            (Some(expected), PublicKeyOrCertificate::PublicKey { key, .. }) => {
                expected.key_data() == key.key_data()
            }
            _ => false,
        })
    }
}

async fn connect(
    fixture: &Fixture,
    expected: Option<PublicKey>,
    checks: Arc<AtomicUsize>,
) -> Result<client::Handle<VerifiedClient>, russh::Error> {
    client::connect(
        Arc::new(client::Config {
            inactivity_timeout: Some(LIMIT),
            ..Default::default()
        }),
        fixture.addr,
        VerifiedClient { expected, checks },
    )
    .await
}

async fn authenticate(handle: &mut client::Handle<VerifiedClient>, key: Arc<PrivateKey>) -> bool {
    handle
        .authenticate_publickey("fixture", PrivateKeyWithHashAlg::new(key, None))
        .await
        .unwrap()
        .success()
}

async fn channel_success(channel: &mut Channel<client::Msg>) {
    assert!(matches!(channel.wait().await, Some(ChannelMsg::Success)));
}

#[tokio::test]
async fn unknown_and_changed_host_keys_are_rejected_before_authentication() {
    timeout(LIMIT, async {
        let fixture = Fixture::start().await;
        for expected in [None, Some(ephemeral_key().public_key().clone())] {
            let checks = Arc::new(AtomicUsize::new(0));
            assert!(matches!(
                connect(&fixture, expected, checks.clone()).await,
                Err(russh::Error::UnknownKey)
            ));
            assert_eq!(checks.load(Ordering::SeqCst), 1);
            assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn wrong_client_key_is_rejected_and_ephemeral_authorized_key_authenticates() {
    timeout(LIMIT, async {
        let mut fixture = Fixture::start().await;
        let mut handle = connect(
            &fixture,
            Some(fixture.trusted_host.clone()),
            Arc::new(AtomicUsize::new(0)),
        )
        .await
        .unwrap();
        assert!(!authenticate(&mut handle, Arc::new(ephemeral_key())).await);
        assert_eq!(fixture.next_event().await, Event::Auth(false));
        assert!(authenticate(&mut handle, fixture.user_key.clone()).await);
        assert_eq!(fixture.next_event().await, Event::Auth(true));
        handle
            .disconnect(Disconnect::ByApplication, "fixture complete", "en")
            .await
            .unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn pty_binary_data_ctrl_c_resize_disconnect_and_fresh_reconnect() {
    timeout(LIMIT, async {
        let mut fixture = Fixture::start().await;
        for _ in 0..2 {
            let mut handle = connect(&fixture, Some(fixture.trusted_host.clone()), Arc::new(AtomicUsize::new(0))).await.unwrap();
            assert!(authenticate(&mut handle, fixture.user_key.clone()).await);
            assert_eq!(fixture.next_event().await, Event::Auth(true));
            let mut channel = handle.channel_open_session().await.unwrap();
            channel.request_pty(true, "xterm-256color", 80, 24, 0, 0, &[]).await.unwrap();
            channel_success(&mut channel).await;
            assert_eq!(fixture.next_event().await, Event::Pty { term: "xterm-256color".into(), cols:80, rows:24 });
            channel.request_shell(true).await.unwrap();
            channel_success(&mut channel).await;
            assert_eq!(fixture.next_event().await, Event::Shell);
            let data = b"hello\x00\xff\x03\r\n";
            channel.data(&data[..]).await.unwrap();
            assert_eq!(fixture.next_event().await, Event::Data(data.to_vec()));
            assert!(matches!(channel.wait().await, Some(ChannelMsg::Data { data: actual }) if &actual[..] == data));
            channel.window_change(132, 43, 0, 0).await.unwrap();
            assert_eq!(fixture.next_event().await, Event::Resize(132,43));
            channel.close().await.unwrap();
            assert_eq!(fixture.next_event().await, Event::Closed);
            handle.disconnect(Disconnect::ByApplication, "fixture complete", "en").await.unwrap();
        }
        assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 2);
    }).await.unwrap();
}

#[cfg(unix)]
mod os_pty;

mod trust;

struct PersistedClient {
    store: Arc<trust::Store>,
    endpoint: trust::Endpoint,
    observation: Arc<std::sync::Mutex<Option<trust::Confirmation>>>,
}
impl client::Handler for PersistedClient {
    type Error = russh::Error;
    async fn check_server_key(
        &mut self,
        actual: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = actual else {
            return Ok(false);
        };
        match self.store.inspect(self.endpoint.clone(), key) {
            Ok(trust::Inspection::Trusted) => Ok(true),
            Ok(trust::Inspection::Unknown(confirmation)) => {
                *self.observation.lock().unwrap() = Some(confirmation);
                Ok(false)
            }
            Err(_) => Ok(false),
        }
    }
}
#[tokio::test]
async fn persisted_first_use_confirmation_reopen_and_changed_key_before_auth() {
    timeout(LIMIT, async {
        let fixture = Fixture::start().await;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("managed");
        let store = Arc::new(trust::Store::create(&path).unwrap());
        let endpoint =
            trust::Endpoint::new(&fixture.addr.ip().to_string(), fixture.addr.port()).unwrap();
        let observed = Arc::new(std::sync::Mutex::new(None));
        let connect_persisted = |store: Arc<trust::Store>, endpoint: trust::Endpoint| {
            client::connect(
                Arc::new(client::Config::default()),
                fixture.addr,
                PersistedClient {
                    store,
                    endpoint,
                    observation: observed.clone(),
                },
            )
        };
        assert!(matches!(
            connect_persisted(store.clone(), endpoint.clone()).await,
            Err(russh::Error::UnknownKey)
        ));
        assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 0);
        let confirmation = observed.lock().unwrap().take().unwrap();
        assert_eq!(
            confirmation.fingerprint(),
            fixture
                .trusted_host
                .fingerprint(russh::keys::HashAlg::Sha256)
                .to_string()
        );
        let fingerprint = confirmation.fingerprint().to_owned();
        store.confirm(confirmation, &fingerprint).unwrap();
        drop(store);
        let reopened = Arc::new(trust::Store::open(&path).unwrap());
        let mut handle = connect_persisted(reopened.clone(), endpoint.clone())
            .await
            .unwrap();
        assert!(handle
            .authenticate_publickey(
                "fixture",
                PrivateKeyWithHashAlg::new(fixture.user_key.clone(), None)
            )
            .await
            .unwrap()
            .success());
        handle
            .disconnect(Disconnect::ByApplication, "fixture complete", "en")
            .await
            .unwrap();
        assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 1);
        // A second independent server presents a different key for the same logical
        // endpoint identity. The fixture transport is deliberately injectable.
        let replacement = Fixture::start().await;
        let result = client::connect(
            Arc::new(client::Config::default()),
            replacement.addr,
            PersistedClient {
                store: reopened.clone(),
                endpoint: endpoint.clone(),
                observation: observed.clone(),
            },
        )
        .await;
        assert!(matches!(result, Err(russh::Error::UnknownKey)));
        assert_eq!(replacement.auth_attempts.load(Ordering::SeqCst), 0);
        assert!(observed.lock().unwrap().is_none());
        assert!(matches!(
            reopened.inspect(endpoint, &fixture.trusted_host).unwrap(),
            trust::Inspection::Trusted
        ));
    })
    .await
    .unwrap();
}
