#[path = "fixture.rs"]
mod fixture;
use super::*;
use crate::ssh::trust::Store;
use fixture::*;
fn manager(socket: PathBuf) -> (tempfile::TempDir, AgentManager, Store) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("trust");
    let store = Store::create(&path).unwrap();
    let manager = AgentManager::new(HostManager::new(path), Some(socket));
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

async fn shell_output_until(
    sessions: &crate::ssh::session::SessionManager,
    id: &str,
    expected: &str,
) -> String {
    use crate::ssh::session::Event;
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut output = Vec::new();
        loop {
            match sessions.read("main", id).await.unwrap() {
                Some(Event::Data { bytes } | Event::Stderr { bytes }) => output.extend(bytes),
                Some(Event::Closed | Event::Failed) => {
                    panic!("shell transport closed before expected output")
                }
                _ => {}
            }
            assert!(output.len() <= 65536, "shell output exceeded fixture bound");
            let text = String::from_utf8_lossy(&output);
            if text.contains(expected) {
                return text.into_owned();
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("missing real shell output: {expected}"))
}
#[tokio::test]
async fn native_agent_pty_real_shell_resize_interrupt_reconnect_and_process_cleanup() {
    use crate::ssh::session::SessionManager;
    let agent = DisposableAgent::start().await;
    let allowed = key();
    let selected = fingerprint(allowed.public_key());
    agent.add(&allowed).await;
    let fixture = Fixture::with_configuration(allowed.public_key().clone(), true, None, true).await;
    drop(allowed);
    let (_root, manager, _store) = manager(agent.socket.clone());
    let inspected = manager
        .hosts
        .inspect("main", "127.0.0.1", fixture.port)
        .await
        .unwrap();
    assert_eq!(inspected.status, "unknown");
    manager
        .hosts
        .confirm(
            "main",
            inspected.challenge_id.as_deref().unwrap(),
            &inspected.fingerprint,
        )
        .await
        .unwrap();
    let sessions = SessionManager::new(manager.clone());
    for destroy_window in [false, true] {
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
        shell_output_until(&sessions, &id, "fixture> ").await;
        sessions
            .write("main", &id, b"stty -echo; printf 'READY\\n'\n".to_vec())
            .unwrap();
        shell_output_until(&sessions, &id, "\r\nREADY\r\n").await;
        sessions
            .write("main", &id, b"printf 'STDOUT_OK\\n'; stty size\n".to_vec())
            .unwrap();
        let output = shell_output_until(&sessions, &id, "24 80").await;
        assert!(output.contains("STDOUT_OK"));
        sessions.resize("main", &id, 132, 43).unwrap();
        sessions
            .write("main", &id, b"stty size\n".to_vec())
            .unwrap();
        shell_output_until(&sessions, &id, "43 132").await;
        let shell = fixture.shells.lock().unwrap().last().unwrap().clone();
        let shell_pid = shell.lock().unwrap().process_id().unwrap();
        sessions
            .write("main", &id, b"printf 'SLEEPING\\n'; sleep 5\n".to_vec())
            .unwrap();
        shell_output_until(&sessions, &id, "SLEEPING\r\n").await;
        let foreground = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(pid) = shell.lock().unwrap().foreground_child() {
                    break pid;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let interrupted_at = Instant::now();
        sessions.write("main", &id, vec![3]).unwrap();
        shell_output_until(&sessions, &id, "fixture> ").await;
        sessions
            .write("main", &id, b"printf 'INTERRUPTED:%s\\n' \"$?\"\n".to_vec())
            .unwrap();
        shell_output_until(&sessions, &id, "INTERRUPTED:130").await;
        assert!(interrupted_at.elapsed() < Duration::from_secs(2));
        sessions
            .write("main", &id, b"printf 'CLOSING\\n'; sleep 30\n".to_vec())
            .unwrap();
        shell_output_until(&sessions, &id, "CLOSING\r\n").await;
        let running_at_close = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(pid) = shell.lock().unwrap().foreground_child() {
                    break pid;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        if destroy_window {
            sessions.forget_owner("main");
        } else {
            sessions.close("main", &id).unwrap();
        }
        wait_closed(&fixture).await;
        assert!(shell.lock().unwrap().is_closed());
        for pid in [shell_pid, foreground, running_at_close] {
            tokio::time::timeout(Duration::from_secs(2), async {
                // SAFETY: signal zero only observes the fixture-created process ID.
                while unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("owned shell/foreground child still exists after close");
        }
    }
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn native_real_shell_stalled_output_fails_visibly_and_closes_owned_processes() {
    use crate::ssh::session::{Event, SessionManager};
    let agent = DisposableAgent::start().await;
    let allowed = key();
    let selected = fingerprint(allowed.public_key());
    agent.add(&allowed).await;
    let fixture = Fixture::with_configuration(allowed.public_key().clone(), true, None, true).await;
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
    shell_output_until(&sessions, &id, "fixture> ").await;
    sessions
        .write("main", &id, b"stty -echo; printf 'READY\\n'\n".to_vec())
        .unwrap();
    shell_output_until(&sessions, &id, "\r\nREADY\r\n").await;
    sessions
        .write("main", &id, b"yes vmanyware-pressure\n".to_vec())
        .unwrap();
    let shell = fixture.shells.lock().unwrap().last().unwrap().clone();
    tokio::time::timeout(Duration::from_secs(1), async {
        while shell.lock().unwrap().foreground_child().is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    // Intentionally stop pulling output so every bounded layer can fill.
    tokio::time::sleep(Duration::from_millis(250)).await;
    // russh's full channel queue can prevent processing queued input. Do not
    // falsely claim Ctrl-C delivery; the output-stall policy must close instead.
    sessions.write("main", &id, vec![3]).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !shell.lock().unwrap().is_closed() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("stalled output did not close the owned shell");
    let mut drained = 0;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sessions.read("main", &id).await.unwrap() {
                Some(Event::Data { bytes }) => {
                    drained += bytes.len();
                    assert!(
                        drained < 2 * 1024 * 1024,
                        "pressure backlog exceeded fixture budget"
                    );
                }
                Some(Event::Failed) => break,
                Some(Event::Closed) => panic!("stalled output was reported as normal closure"),
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        sessions.read("main", &id).await.unwrap(),
        Some(Event::Closed)
    ));
    assert!(sessions.write("main", &id, vec![3]).is_err());
    sessions.close("main", &id).unwrap();
    wait_closed(&fixture).await;
    // Freshly selected connections remain usable; active readers can interrupt
    // a fast producer without the paused-output failure policy firing.
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
    shell_output_until(&sessions, &id, "fixture> ").await;
    sessions
        .write("main", &id, b"stty -echo; printf 'READY\\n'\n".to_vec())
        .unwrap();
    shell_output_until(&sessions, &id, "\r\nREADY\r\n").await;
    sessions
        .write("main", &id, b"yes vmanyware-pressure\n".to_vec())
        .unwrap();
    assert!(matches!(
        next_output(&sessions, &id).await,
        Event::Data { .. }
    ));
    let interrupted_at = Instant::now();
    sessions.write("main", &id, vec![3]).unwrap();
    let mut tail = String::new();
    let mut drained = 0;
    tokio::time::timeout(Duration::from_secs(2), async {
        while !tail.contains("fixture> ") {
            let Event::Data { bytes } = next_output(&sessions, &id).await else {
                panic!("streaming shell failed")
            };
            drained += bytes.len();
            assert!(drained < 2 * 1024 * 1024);
            tail.push_str(&String::from_utf8_lossy(&bytes));
            if tail.len() > 128 {
                tail = tail[tail.len() - 128..].to_owned();
            }
        }
    })
    .await
    .unwrap();
    assert!(interrupted_at.elapsed() < Duration::from_secs(2));
    sessions
        .write("main", &id, b"printf 'RECOVERED\\n'\n".to_vec())
        .unwrap();
    shell_output_until(&sessions, &id, "RECOVERED\r\n").await;
    sessions.close("main", &id).unwrap();
    wait_closed(&fixture).await;
}
