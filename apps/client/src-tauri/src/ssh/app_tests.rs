use super::fixture;
use super::*;
use crate::ssh::{
    identity::IdentityStore,
    session::{Event, SessionManager},
    trust::Store,
};
use fixture::*;

fn password() -> zeroize::Zeroizing<String> {
    zeroize::Zeroizing::new("disposable app test phrase 42".into())
}
fn setup() -> (tempfile::TempDir, AgentManager, PublicKey, Store) {
    let root = tempfile::tempdir().unwrap();
    let identity = IdentityStore::new(root.path().join("identity"));
    let exported = serde_json::to_value(identity.create(password()).unwrap()).unwrap();
    let public = PublicKey::from_openssh(exported["publicKey"].as_str().unwrap()).unwrap();
    let path = root.path().join("trust");
    let trust = Store::create(&path).unwrap();
    let manager = AgentManager::new(HostManager::new(path), None).with_identity(identity);
    (root, manager, public, trust)
}
#[tokio::test]
async fn app_identity_verifies_host_before_unlock_and_rechecks_before_authentication() {
    let (_root, manager, public, trust) = setup();
    let fixture = Fixture::start(public).await;
    // Even a wrong password cannot be evaluated before a trusted handshake.
    let error = manager
        .list_application("main", "127.0.0.1", fixture.port, "fixture", password())
        .await
        .err()
        .unwrap();
    assert!(error.contains("host verification"));
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    fixture.trust(&trust);
    assert!(manager
        .list_application(
            "main",
            "127.0.0.1",
            fixture.port,
            "fixture",
            zeroize::Zeroizing::new("incorrect passphrase 42".into())
        )
        .await
        .is_err());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    let selection = manager
        .list_application("main", "127.0.0.1", fixture.port, "fixture", password())
        .await
        .unwrap();
    let selected = &selection.identities[0].fingerprint;
    assert!(manager
        .authenticate("other", &selection.id, selected)
        .await
        .is_err());
    *fixture.host_key.lock().unwrap() = key();
    assert!(manager
        .authenticate("main", &selection.id, selected)
        .await
        .is_err());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.channels.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn app_identity_selection_is_exact_single_use_and_cancellable() {
    let (_root, manager, public, trust) = setup();
    let fixture = Fixture::start(public).await;
    fixture.trust(&trust);
    let selection = manager
        .list_application("main", "127.0.0.1", fixture.port, "fixture", password())
        .await
        .unwrap();
    assert!(manager
        .authenticate("main", &selection.id, "SHA256:other")
        .await
        .is_err());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    assert!(manager
        .authenticate("main", &selection.id, &selection.identities[0].fingerprint)
        .await
        .is_err());
    let selection = manager
        .list_application("main", "127.0.0.1", fixture.port, "fixture", password())
        .await
        .unwrap();
    manager.cancel_selection("main", &selection.id);
    assert!(manager
        .authenticate("main", &selection.id, &selection.identities[0].fingerprint)
        .await
        .is_err());
    let expired = manager
        .list_application("main", "127.0.0.1", fixture.port, "fixture", password())
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
        .authenticate("main", &expired.id, &expired.identities[0].fingerprint)
        .await
        .is_err());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    let rejected = manager
        .list_application("main", "127.0.0.1", fixture.port, "wrong-user", password())
        .await
        .unwrap();
    assert!(manager
        .authenticate("main", &rejected.id, &rejected.identities[0].fingerprint)
        .await
        .is_err());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 1);
    let selection = manager
        .list_application("main", "127.0.0.1", fixture.port, "fixture", password())
        .await
        .unwrap();
    manager
        .authenticate("main", &selection.id, &selection.identities[0].fingerprint)
        .await
        .unwrap();
    assert!(manager
        .authenticate("main", &selection.id, &selection.identities[0].fingerprint)
        .await
        .is_err());
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.channels.load(Ordering::SeqCst), 0);
}
async fn output(sessions: &SessionManager, id: &str, expected: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = Vec::new();
        loop {
            match sessions.read("main", id).await.unwrap() {
                Some(Event::Data { bytes: chunk }) | Some(Event::Stderr { bytes: chunk }) => {
                    bytes.extend(chunk)
                }
                Some(Event::Failed | Event::Closed) => {
                    panic!("session closed before expected output")
                }
                _ => {}
            }
            if String::from_utf8_lossy(&bytes).contains(expected) {
                break;
            }
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn app_identity_opens_real_shell_resizes_interrupts_and_reconnects() {
    let (_root, manager, public, trust) = setup();
    let fixture = Fixture::with_configuration(public, true, None, true).await;
    fixture.trust(&trust);
    let sessions = SessionManager::new(manager.clone());
    for _ in 0..2 {
        let selection = manager
            .list_application("main", "127.0.0.1", fixture.port, "fixture", password())
            .await
            .unwrap();
        let started = sessions
            .start(
                "main",
                &selection.id,
                &selection.identities[0].fingerprint,
                80,
                24,
            )
            .await
            .unwrap();
        let id = serde_json::to_value(started).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        output(&sessions, &id, "fixture> ").await;
        assert!(sessions.write("other", &id, vec![3]).is_err());
        sessions
            .write("main", &id, b"printf 'app-%s\n' identity\n".to_vec())
            .unwrap();
        output(&sessions, &id, "app-identity").await;
        sessions.resize("main", &id, 100, 37).unwrap();
        sessions
            .write("main", &id, b"stty size\n".to_vec())
            .unwrap();
        output(&sessions, &id, "37 100").await;
        sessions
            .write("main", &id, b"printf '\\nSLEEPING\\n'; sleep 30\n".to_vec())
            .unwrap();
        output(&sessions, &id, "SLEEPING\r\n").await;
        let shell = fixture.shells.lock().unwrap().last().unwrap().clone();
        tokio::time::timeout(Duration::from_secs(1), async {
            while shell.lock().unwrap().foreground_child().is_none() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let interrupted = Instant::now();
        sessions.write("main", &id, vec![3]).unwrap();
        output(&sessions, &id, "fixture> ").await;
        sessions
            .write("main", &id, b"printf 'interrupt-%s\\n' \"$?\"\n".to_vec())
            .unwrap();
        output(&sessions, &id, "interrupt-130").await;
        assert!(interrupted.elapsed() < Duration::from_secs(2));
        sessions.close("main", &id).unwrap();
        assert!(sessions.read("main", &id).await.is_err());
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        while fixture.connections.load(Ordering::SeqCst) != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 2);
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires an isolated owned Secret Service; use the vault runner"]
async fn owned_vault_ssh_selection_checks_host_owner_and_actual_authentication() {
    let root = crate::ssh::identity::vault::tests::require_owned_bus();
    let identity = IdentityStore::new(root.join("ssh-identity"));
    let exported = serde_json::to_value(identity.create(password()).unwrap()).unwrap();
    let public = PublicKey::from_openssh(exported["publicKey"].as_str().unwrap()).unwrap();
    identity.remember(password()).unwrap();
    let path = root.join("ssh-trust");
    let trust = Store::create(&path).unwrap();
    let manager = AgentManager::new(HostManager::new(path), None).with_identity(identity.clone());
    let fixture = Fixture::start(public).await;
    assert!(manager
        .list_system_application("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .err()
        .unwrap()
        .contains("host verification"));
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 0);
    fixture.trust(&trust);
    let selection = manager
        .list_system_application("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    assert!(manager
        .authenticate("other", &selection.id, &selection.identities[0].fingerprint)
        .await
        .is_err());
    manager.cancel_selection("main", &selection.id);
    let selection = manager
        .list_system_application("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .unwrap();
    manager
        .authenticate("main", &selection.id, &selection.identities[0].fingerprint)
        .await
        .unwrap();
    assert_eq!(fixture.auth.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.channels.load(Ordering::SeqCst), 0);
    identity.forget(true).unwrap();
    assert!(manager
        .list_system_application("main", "127.0.0.1", fixture.port, "fixture")
        .await
        .is_err());
}
