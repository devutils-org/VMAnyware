//! Opt-in Linux WebView/real IPC smoke; never linked into the normal application.
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
fn identity_support() -> bool {
    false
}

#[cfg(target_os = "linux")]
#[tauri::command]
fn runtime_platform() -> &'static str {
    std::env::consts::OS
}
#[cfg(target_os = "linux")]
#[tauri::command]
fn ssh_host_support() -> bool {
    false
}
#[cfg(target_os = "linux")]
#[tauri::command]
fn smoke_report(app: tauri::AppHandle, ok: bool, failure: Option<String>) {
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
    let snapshots = std::env::var("VMANYWARE_SNAPSHOT_FIXTURE").ok();
    let script = if let Some(configuration) = snapshots.as_ref() {
        let scenario = serde_json::from_str::<serde_json::Value>(configuration)
            .expect("Invalid owned snapshot configuration");
        let creation = scenario["creation"].as_bool().unwrap_or(false);
        let recovery = scenario["recovery"].as_bool().unwrap_or(false);
        let restore = scenario["restore"].as_bool().unwrap_or(false);
        let restore_recovery = scenario["restoreRecovery"].as_bool().unwrap_or(false);
        format!(
            "window.__snapshotFixture = {configuration};\n{}",
            if restore_recovery {
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
            identity_support,
            remote_connection::connect_remote,
            remote_connection::disconnect_remote,
            remote_connection::read_remote_images,
            remote_connection::read_remote_environments,
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
            ssh_host_support,
            control_plane::list_instances,
            control_plane::check_instance_lifecycle,
            control_plane::begin_lifecycle_action,
            control_plane::read_lifecycle_operation,
            smoke_report
        ])
        .setup(move |app| {
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
