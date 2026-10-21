//! Opt-in Linux WebView/real IPC smoke; never linked into the normal application.
#[cfg(target_os = "linux")]
#[path = "../src/ssh/mod.rs"]
#[allow(dead_code)]
mod ssh;
#[cfg(target_os = "linux")]
#[path = "../src/ssh/agent/fixture.rs"]
#[allow(dead_code)]
mod ssh_fixture;
#[cfg(target_os = "linux")]
use tauri::Manager;
#[cfg(target_os = "linux")]
struct ManualSsh {
    peer: ssh_fixture::Fixture,
    directory: std::sync::Mutex<Option<tempfile::TempDir>>,
    trusted: std::sync::Mutex<Option<Vec<u8>>>,
    identity: ssh::identity::IdentityStore,
}
#[cfg(target_os = "linux")]
#[tauri::command]
fn smoke_rotate_host(state: tauri::State<'_, ManualSsh>) -> String {
    use russh::keys::HashAlg;
    let path = state
        .directory
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .path()
        .join("trust/trust.json");
    *state.trusted.lock().unwrap() = Some(std::fs::read(path).unwrap());
    *state.peer.host_key.lock().unwrap() = ssh_fixture::key();
    state
        .peer
        .host_key
        .lock()
        .unwrap()
        .public_key()
        .fingerprint(HashAlg::Sha256)
        .to_string()
}
#[cfg(target_os = "linux")]
#[path = "../src/control_plane.rs"]
#[allow(dead_code)]
mod control_plane;

#[cfg(target_os = "linux")]
#[path = "../src/remote_connection.rs"]
#[allow(dead_code)]
mod remote_connection;

#[cfg(target_os = "linux")]
#[tauri::command]
fn runtime_platform() -> &'static str {
    std::env::consts::OS
}
#[cfg(target_os = "linux")]
#[tauri::command]
fn smoke_report(app: tauri::AppHandle, ok: bool, failure: Option<String>) {
    if let Some(state) = app.try_state::<ManualSsh>() {
        use std::sync::atomic::Ordering;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while state.peer.connections.load(Ordering::SeqCst) != 0
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if ok {
            assert!(
                state.peer.auth.load(Ordering::SeqCst) > 0,
                "No authentication"
            );
            assert_eq!(state.peer.channels.load(Ordering::SeqCst), 1);
            assert_eq!(
                state.peer.connections.load(Ordering::SeqCst),
                0,
                "SSH connection left open"
            );
            let events = state.peer.events.lock().unwrap();
            assert!(events
                .iter()
                .any(|event| matches!(event, ssh_fixture::Protocol::Pty(80, 24))));
            assert!(events
                .iter()
                .any(|event| matches!(event, ssh_fixture::Protocol::Shell)));
            assert!(events
                .iter()
                .any(|event| matches!(event, ssh_fixture::Protocol::Resize(_, _))));
        }
        let directory = state.directory.lock().unwrap().take().unwrap();
        if ok {
            let saved = std::fs::read(directory.path().join("trust/trust.json")).unwrap();
            assert_eq!(
                Some(saved),
                *state.trusted.lock().unwrap(),
                "Changed key overwrote trust"
            );
        }
        state.peer.shutdown();
        drop(directory);
        if ok {
            eprintln!(
            "Owned manual SSH: application identity, PTY/shell/resize, closed connection, changed-key trust unchanged"
        );
        }
    }
    if let Some(failure) = failure {
        eprintln!("Native lifecycle assertion: {failure}");
    }
    eprintln!(
        "Native lifecycle smoke: {}",
        if ok { "passed" } else { "FAILED" }
    );
    app.exit(if ok { 0 } else { 1 });
}
#[cfg(target_os = "linux")]
fn main() {
    let port: u16 = std::env::var("VMANYWARE_FIXTURE_PORT")
        .expect("Explicit fixture port required")
        .parse()
        .expect("Invalid fixture port");
    assert_ne!(port, 0);
    let mut snapshots = std::env::var("VMANYWARE_SNAPSHOT_FIXTURE").ok();
    let manual = if std::env::var_os("VMANYWARE_MANUAL_SSH_FIXTURE").is_some() {
        use russh::keys::HashAlg;
        let directory = tempfile::tempdir().unwrap();
        let identity = ssh::identity::IdentityStore::new(directory.path().join("identity"));
        let password = "owned-fixture-only-passphrase";
        let public = serde_json::to_value(
            identity
                .create(zeroize::Zeroizing::new(password.into()))
                .unwrap(),
        )
        .unwrap();
        let allowed =
            russh::keys::PublicKey::from_openssh(public["publicKey"].as_str().unwrap()).unwrap();
        let peer = tauri::async_runtime::block_on(ssh_fixture::Fixture::with_timeout(
            allowed,
            true,
            None,
            true,
            std::time::Duration::from_secs(60),
        ));
        let fingerprint = peer
            .host_key
            .lock()
            .unwrap()
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string();
        let mut config: serde_json::Value =
            serde_json::from_str(snapshots.as_ref().unwrap()).unwrap();
        config["sshPassphrase"] = password.into();
        config["sshHost"] = "127.0.0.1".into();
        config["sshPort"] = peer.port.into();
        config["sshFingerprint"] = fingerprint.into();
        snapshots = Some(config.to_string());
        Some(ManualSsh {
            peer,
            directory: std::sync::Mutex::new(Some(directory)),
            identity,
            trusted: Default::default(),
        })
    } else {
        None
    };
    let script = if let Some(configuration) = snapshots.as_ref() {
        let scenario = serde_json::from_str::<serde_json::Value>(configuration)
            .expect("Invalid owned snapshot configuration");
        let creation = scenario["creation"].as_bool().unwrap_or(false);
        let recovery = scenario["recovery"].as_bool().unwrap_or(false);
        let restore = scenario["restore"].as_bool().unwrap_or(false);
        let restore_recovery = scenario["restoreRecovery"].as_bool().unwrap_or(false);
        format!(
            "window.__snapshotFixture = {configuration};\n{}\n{}",
            include_str!("../../../../experiments/lifecycle-native/instance-detail.js"),
            if scenario["manualSsh"].as_bool().unwrap_or(false) {
                include_str!("../../../../experiments/lifecycle-native/manual-ssh.js")
            } else if scenario["lifecycleRecovery"].as_bool().unwrap_or(false) {
                include_str!("../../../../experiments/lifecycle-native/lifecycle-recovery.js")
            } else if scenario["remoteLifecycle"].as_bool().unwrap_or(false) {
                include_str!("../../../../experiments/lifecycle-native/remote-lifecycle.js")
            } else if scenario["recreationRecovery"].as_bool().unwrap_or(false) {
                include_str!("../../../../experiments/lifecycle-native/recreation-recovery.js")
            } else if scenario["recreation"].as_bool().unwrap_or(false) {
                include_str!("../../../../experiments/lifecycle-native/recreation.js")
            } else if scenario["deletionRecovery"].as_bool().unwrap_or(false) {
                include_str!("../../../../experiments/lifecycle-native/deletion-recovery.js")
            } else if scenario["deletion"].as_bool().unwrap_or(false) {
                include_str!("../../../../experiments/lifecycle-native/deletion.js")
            } else if restore_recovery {
                include_str!("../../../../experiments/lifecycle-native/restore-recovery.js")
            } else if restore {
                include_str!("../../../../experiments/lifecycle-native/snapshot-restore.js")
            } else if recovery {
                include_str!("../../../../experiments/lifecycle-native/snapshot-recovery.js")
            } else if creation {
                include_str!("../../../../experiments/lifecycle-native/snapshot-create.js")
            } else {
                include_str!("../../../../experiments/lifecycle-native/snapshots.js")
            }
        )
    } else {
        format!(
            "window.__lifecycleFixturePort = {port};\n{}",
            include_str!("../../../../experiments/lifecycle-native/scenario.js")
        )
    };
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "dev.vmanyware.lifecycle-fixture".into();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .manage(remote_connection::RemoteConnection::default())
        .invoke_handler(tauri::generate_handler![
            runtime_platform,
            ssh::identity_support,
            remote_connection::lifecycle_recovery::read_remote_lifecycle_history,
            remote_connection::lifecycle_recovery::recover_remote_lifecycle,
            remote_connection::lifecycle::prepare_remote_lifecycle,
            remote_connection::lifecycle::clear_remote_lifecycle,
            remote_connection::lifecycle::confirm_remote_lifecycle,
            remote_connection::lifecycle::read_remote_lifecycle_operation,
            remote_connection::instance_status::read_remote_instance_status,
            remote_connection::instance_detail::read_remote_instance_detail,
            remote_connection::connect_remote,
            remote_connection::disconnect_remote,
            remote_connection::read_remote_images,
            remote_connection::read_remote_environments,
            remote_connection::recreation_recovery::read_remote_recreation_history,
            remote_connection::recreation_recovery::recover_remote_recreation,
            remote_connection::recreation::prepare_remote_recreation,
            remote_connection::recreation::confirm_remote_recreation,
            remote_connection::recreation::read_remote_recreation_operation,
            remote_connection::recreation::clear_remote_recreation,
            remote_connection::deletion::prepare_remote_deletion,
            remote_connection::deletion::confirm_remote_deletion,
            remote_connection::deletion::read_remote_deletion_operation,
            remote_connection::deletion::clear_remote_deletion,
            remote_connection::deletion_recovery::read_remote_deletion_history,
            remote_connection::deletion_recovery::recover_remote_deletion,
            remote_connection::snapshots::read_remote_snapshots,
            remote_connection::snapshot_restore::prepare_remote_snapshot_restore,
            remote_connection::snapshot_restore::confirm_remote_snapshot_restore,
            remote_connection::snapshot_restore::read_remote_snapshot_restore_operation,
            remote_connection::snapshot_restore::clear_remote_snapshot_restore,
            remote_connection::snapshot_create::prepare_remote_snapshot,
            remote_connection::snapshot_create::confirm_remote_snapshot,
            remote_connection::snapshot_create::read_remote_snapshot_operation,
            remote_connection::snapshot_create::clear_remote_snapshot,
            remote_connection::restore_recovery::read_remote_restore_history,
            remote_connection::restore_recovery::recover_remote_restore,
            remote_connection::snapshot_recovery::read_remote_snapshot_history,
            remote_connection::snapshot_recovery::recover_remote_snapshot,
            ssh::ssh_host_support,
            ssh::ssh_agent_support,
            ssh::inspect_ssh_host,
            ssh::confirm_ssh_host,
            ssh::cancel_ssh_host,
            ssh::list_app_ssh_identity,
            ssh::check_ssh_authentication,
            ssh::cancel_ssh_authentication,
            ssh::start_ssh_session,
            ssh::read_ssh_session,
            ssh::write_ssh_session,
            ssh::resize_ssh_session,
            ssh::close_ssh_session,
            smoke_rotate_host,
            control_plane::list_instances,
            control_plane::check_instance_lifecycle,
            control_plane::begin_lifecycle_action,
            control_plane::read_lifecycle_operation,
            smoke_report
        ])
        .setup(move |app| {
            if let Some(state) = manual {
                let path = state.directory.lock().unwrap().as_ref().unwrap().path().join("trust");
                let hosts = ssh::HostManager::new(path.clone());
                let agents = ssh::AgentManager::new(hosts.clone(), Some(path.join("no-personal-agent"))).with_identity(state.identity.clone());
                app.manage(ssh::SessionManager::new(agents.clone()));
                app.manage(agents);
                app.manage(hosts);
                app.manage(state);
            } else {
                app.manage(ssh::HostManager::new(app.path().app_local_data_dir()?.join("fixture-host-trust")));
            }
                // Example-only reporting; the production application never grants this.
                tauri::Manager::add_capability(app, r#"{"identifier":"owned-smoke-report","windows":["main"],"permissions":["owned-fixture-report"]}"#)?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(45));
                handle.exit(1);
            });
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("VMAnyware disposable lifecycle smoke")
            .inner_size(1100.0, 900.0)
            .incognito(true)
            .focused(false)
            .initialization_script(script)
            .build()?;
            Ok(())
        })
        .run(context)
        .expect("Native lifecycle fixture failed");
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This isolated lifecycle smoke requires Linux");
    std::process::exit(1);
}
