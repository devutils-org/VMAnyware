//! macOS-only disposable OpenSSH-agent experiment. Never discovers a user's agent.
use super::*;
use russh::{
    keys::{
        agent::{client::AgentClient, AgentIdentity},
        HashAlg,
    },
    Signer,
};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Agent = AgentClient<Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>>;

// Keep the socket owned even if russh detaches an internal task on cancellation.
struct Connection(std::net::TcpStream);
impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.0.shutdown(std::net::Shutdown::Both);
    }
}

struct DisposableAgent {
    child: Child,
    socket: PathBuf,
    _directory: tempfile::TempDir,
}

impl DisposableAgent {
    async fn start() -> Self {
        // Short private path avoids the Unix socket path-length limit.
        let directory = tempfile::Builder::new()
            .prefix("pl-agent-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let socket = directory.path().join("a");
        let child = Command::new("/usr/bin/ssh-agent")
            // -U prevents cleanup of personal ~/.ssh/agent even with -a/env_clear.
            .args(["-D", "-U", "-a"])
            .arg(&socket)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("disposable ssh-agent must be available with -U support");
        let mut agent = Self {
            child,
            socket,
            _directory: directory,
        };
        timeout(Duration::from_secs(2), async {
            loop {
                assert!(
                    agent.child.try_wait().unwrap().is_none(),
                    "disposable agent exited before startup"
                );
                if tokio::net::UnixStream::connect(&agent.socket).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("disposable agent startup deadline");
        agent
    }

    async fn add(&self, key: &PrivateKey) {
        timeout(Duration::from_secs(2), async {
            let mut client = AgentClient::connect_uds(&self.socket).await.unwrap();
            client.add_identity(key, &[]).await.unwrap();
        })
        .await
        .expect("disposable identity setup deadline");
    }
}

impl Drop for DisposableAgent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "owned ssh-agent did not terminate"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum AgentError {
    HostRejected,
    AgentUnavailable,
    IdentityUnavailable,
    SigningRefused,
    AuthenticationRejected,
    Timeout,
}
impl From<russh::SendError> for AgentError {
    fn from(_: russh::SendError) -> Self {
        Self::AuthenticationRejected
    }
}

#[derive(Default)]
struct AccessCounts {
    connections: AtomicUsize,
    lists: AtomicUsize,
    signatures: AtomicUsize,
}

#[derive(Clone, Copy)]
enum Fault {
    None,
    RemoveSelectedBeforeSign,
    DisconnectBeforeSign,
}

struct SelectedSigner<'a> {
    agent: Option<Agent>,
    selected: PublicKey,
    counts: &'a AccessCounts,
    fault: Fault,
}

impl Signer for SelectedSigner<'_> {
    type Error = AgentError;
    async fn auth_sign(
        &mut self,
        identity: &AgentIdentity,
        hash: Option<HashAlg>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, Self::Error> {
        let AgentIdentity::PublicKey { key, .. } = identity else {
            return Err(AgentError::SigningRefused);
        };
        if key.key_data() != self.selected.key_data() {
            return Err(AgentError::SigningRefused);
        }
        match self.fault {
            Fault::None => {}
            Fault::RemoveSelectedBeforeSign => self
                .agent
                .as_mut()
                .unwrap()
                .remove_identity(&self.selected)
                .await
                .map_err(|_| AgentError::SigningRefused)?,
            Fault::DisconnectBeforeSign => {
                let mut stream = self.agent.take().unwrap().into_inner();
                stream
                    .shutdown()
                    .await
                    .map_err(|_| AgentError::AgentUnavailable)?;
                self.agent = Some(AgentClient::connect(stream));
            }
        }
        self.counts.signatures.fetch_add(1, Ordering::SeqCst);
        self.agent
            .as_mut()
            .unwrap()
            .sign_request(identity, hash, data)
            .await
            .map_err(|_| AgentError::SigningRefused)
    }
}

/// Explicit endpoint/key/socket/identity only. No environment/config discovery.
async fn authenticate_via_agent(
    fixture: &Fixture,
    trusted_host: Option<PublicKey>,
    agent_socket: &Path,
    selected_fingerprint: &str,
    counts: &AccessCounts,
    fault: Fault,
) -> Result<(client::Handle<VerifiedClient>, Connection), AgentError> {
    timeout(Duration::from_secs(3), async {
        // Complete trusted key exchange before even opening the agent socket.
        let socket = tokio::net::TcpStream::connect(fixture.addr)
            .await
            .map_err(|_| AgentError::HostRejected)?
            .into_std()
            .map_err(|_| AgentError::HostRejected)?;
        let connection = Connection(socket.try_clone().map_err(|_| AgentError::HostRejected)?);
        let socket =
            tokio::net::TcpStream::from_std(socket).map_err(|_| AgentError::HostRejected)?;
        let mut handle = client::connect_stream(
            Arc::new(client::Config {
                inactivity_timeout: Some(LIMIT),
                ..Default::default()
            }),
            socket,
            VerifiedClient {
                expected: trusted_host,
                checks: Arc::new(AtomicUsize::new(0)),
            },
        )
        .await
        .map_err(|_| AgentError::HostRejected)?;
        counts.connections.fetch_add(1, Ordering::SeqCst);
        let mut agent = AgentClient::connect_uds(agent_socket)
            .await
            .map_err(|_| AgentError::AgentUnavailable)?
            .dynamic();
        counts.lists.fetch_add(1, Ordering::SeqCst);
        let identities = agent
            .request_identities()
            .await
            .map_err(|_| AgentError::AgentUnavailable)?;
        let selected: Vec<_> = identities
            .into_iter()
            .filter_map(|identity| match identity {
                AgentIdentity::PublicKey { key, .. }
                    if key.fingerprint(HashAlg::Sha256).to_string() == selected_fingerprint =>
                {
                    Some(key)
                }
                _ => None,
            })
            .collect();
        if selected.len() != 1 {
            return Err(AgentError::IdentityUnavailable);
        }
        let selected = selected.into_iter().next().unwrap();
        let mut signer = SelectedSigner {
            agent: Some(agent),
            selected: selected.clone(),
            counts,
            fault,
        };
        let authenticated = handle
            .authenticate_publickey_with("fixture", selected, None, &mut signer)
            .await?;
        if !authenticated.success() {
            return Err(AgentError::AuthenticationRejected);
        }
        Ok((handle, connection))
    })
    .await
    .map_err(|_| AgentError::Timeout)?
}

fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

#[tokio::test]
async fn disposable_real_agent_authenticates_selected_key_then_opens_pty_after_private_key_drop() {
    timeout(LIMIT, async {
        let key = ephemeral_key();
        let selected = fingerprint(key.public_key());
        let mut fixture = Fixture::with_user_public_key(key.public_key().clone()).await;
        let agent = DisposableAgent::start().await;
        agent.add(&key).await;
        drop(key); // Client path now receives public metadata and agent socket only.
        for _ in 0..2 {
            let counts = AccessCounts::default();
            let (handle, _connection) = authenticate_via_agent(&fixture, Some(fixture.trusted_host.clone()), &agent.socket, &selected, &counts, Fault::None).await.unwrap();
            assert_eq!(fixture.next_event().await, Event::Auth(true));
            assert_eq!(counts.connections.load(Ordering::SeqCst), 1);
            assert_eq!(counts.lists.load(Ordering::SeqCst), 1);
            assert_eq!(counts.signatures.load(Ordering::SeqCst), 1);
            let mut channel = handle.channel_open_session().await.unwrap();
            channel.request_pty(true,"xterm-256color",80,24,0,0,&[]).await.unwrap();
            channel_success(&mut channel).await;
            assert!(matches!(fixture.next_event().await, Event::Pty { .. }));
            channel.request_shell(true).await.unwrap();
            channel_success(&mut channel).await;
            assert_eq!(fixture.next_event().await, Event::Shell);
            channel.data(&b"agent-authenticated\x03"[..]).await.unwrap();
            assert_eq!(fixture.next_event().await, Event::Data(b"agent-authenticated\x03".to_vec()));
            assert!(matches!(channel.wait().await,Some(ChannelMsg::Data { data }) if &data[..] == b"agent-authenticated\x03"));
            channel.close().await.unwrap();
            assert_eq!(fixture.next_event().await, Event::Closed);
            handle.disconnect(Disconnect::ByApplication,"fixture complete","en").await.unwrap();
        }
    }).await.unwrap();
}

#[tokio::test]
async fn unknown_or_changed_host_never_opens_lists_or_signs_with_agent() {
    timeout(LIMIT, async {
        let fixture = Fixture::start().await;
        let agent = DisposableAgent::start().await;
        for trusted in [None, Some(ephemeral_key().public_key().clone())] {
            let counts = AccessCounts::default();
            assert_eq!(
                authenticate_via_agent(
                    &fixture,
                    trusted,
                    &agent.socket,
                    "SHA256:unused",
                    &counts,
                    Fault::None
                )
                .await
                .err()
                .unwrap(),
                AgentError::HostRejected
            );
            assert_eq!(counts.connections.load(Ordering::SeqCst), 0);
            assert_eq!(counts.lists.load(Ordering::SeqCst), 0);
            assert_eq!(counts.signatures.load(Ordering::SeqCst), 0);
            assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn empty_agent_missing_selected_identity_and_missing_socket_fail_without_auth_or_signature() {
    timeout(LIMIT, async {
        let fixture = Fixture::start().await;
        let agent = DisposableAgent::start().await;
        for populated in [false, true] {
            if populated {
                agent.add(&ephemeral_key()).await;
            }
            let counts = AccessCounts::default();
            assert_eq!(
                authenticate_via_agent(
                    &fixture,
                    Some(fixture.trusted_host.clone()),
                    &agent.socket,
                    "SHA256:explicitly-missing",
                    &counts,
                    Fault::None
                )
                .await
                .err()
                .unwrap(),
                AgentError::IdentityUnavailable
            );
            assert_eq!(counts.lists.load(Ordering::SeqCst), 1);
            assert_eq!(counts.signatures.load(Ordering::SeqCst), 0);
            assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 0);
        }
        let counts = AccessCounts::default();
        assert_eq!(
            authenticate_via_agent(
                &fixture,
                Some(fixture.trusted_host.clone()),
                &agent.socket.with_extension("missing"),
                "SHA256:missing",
                &counts,
                Fault::None
            )
            .await
            .err()
            .unwrap(),
            AgentError::AgentUnavailable
        );
        assert_eq!(counts.signatures.load(Ordering::SeqCst), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unauthorized_selected_agent_key_never_falls_back_to_authorized_other_identity() {
    timeout(LIMIT, async {
        let allowed = ephemeral_key();
        let wrong = ephemeral_key();
        let selected = fingerprint(wrong.public_key());
        let fixture = Fixture::with_user_public_key(allowed.public_key().clone()).await;
        let agent = DisposableAgent::start().await;
        agent.add(&allowed).await;
        agent.add(&wrong).await;
        drop(allowed);
        drop(wrong);
        let counts = AccessCounts::default();
        assert_eq!(
            authenticate_via_agent(
                &fixture,
                Some(fixture.trusted_host.clone()),
                &agent.socket,
                &selected,
                &counts,
                Fault::None
            )
            .await
            .err()
            .unwrap(),
            AgentError::AuthenticationRejected
        );
        assert_eq!(counts.signatures.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 1);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn real_agent_removed_identity_and_closed_connection_return_safe_signing_failure() {
    timeout(LIMIT, async {
        let key = ephemeral_key();
        let selected = fingerprint(key.public_key());
        let fixture = Fixture::with_user_public_key(key.public_key().clone()).await;
        for fault in [Fault::RemoveSelectedBeforeSign, Fault::DisconnectBeforeSign] {
            let agent = DisposableAgent::start().await;
            agent.add(&key).await;
            let counts = AccessCounts::default();
            let error = authenticate_via_agent(
                &fixture,
                Some(fixture.trusted_host.clone()),
                &agent.socket,
                &selected,
                &counts,
                fault,
            )
            .await
            .err()
            .unwrap();
            assert_eq!(error, AgentError::SigningRefused);
            assert_eq!(counts.signatures.load(Ordering::SeqCst), 1);
            assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unresponsive_agent_times_out_and_closes_its_connection() {
    timeout(LIMIT, async {
        let fixture = Fixture::start().await;
        let directory = tempfile::Builder::new()
            .prefix("pl-stalled-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let socket = directory.path().join("a");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 64];
            let mut received = false;
            loop {
                let n = stream.read(&mut buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                received = true;
                // Deliberately never reply to this disposable client's request.
            }
            assert!(received);
        });
        let counts = AccessCounts::default();
        assert_eq!(
            authenticate_via_agent(
                &fixture,
                Some(fixture.trusted_host.clone()),
                &socket,
                "SHA256:unused",
                &counts,
                Fault::None,
            )
            .await
            .err()
            .unwrap(),
            AgentError::Timeout
        );
        timeout(Duration::from_secs(1), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counts.lists.load(Ordering::SeqCst), 1);
        assert_eq!(counts.signatures.load(Ordering::SeqCst), 0);
        assert_eq!(fixture.auth_attempts.load(Ordering::SeqCst), 0);
    })
    .await
    .unwrap();
}
