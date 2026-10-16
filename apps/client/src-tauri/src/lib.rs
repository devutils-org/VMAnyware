mod control_plane;
mod ssh;
use tauri::Manager;

#[tauri::command]
fn runtime_platform() -> &'static str {
    std::env::consts::OS
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.manage(ssh::HostManager::new(
                app.path()
                    .app_local_data_dir()
                    .map_err(|_| std::io::Error::other("SSH application data path unavailable"))?
                    .join("ssh-host-trust-v1"),
            ));
            #[cfg(not(target_os = "macos"))]
            app.manage(ssh::HostManager);
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::WindowEvent::Destroyed) {
                if let Some(manager) = window.try_state::<ssh::HostManager>() {
                    manager.forget_owner(window.label());
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            runtime_platform,
            ssh::ssh_host_support,
            ssh::inspect_ssh_host,
            ssh::confirm_ssh_host,
            ssh::cancel_ssh_host,
            control_plane::inspect_control_plane,
            control_plane::list_definitions,
            control_plane::save_definition,
            control_plane::validate_definition,
            control_plane::begin_inventory_refresh,
            control_plane::read_operation,
            control_plane::preview_provisioning,
            control_plane::preflight_provisioning
        ])
        .run(tauri::generate_context!())
        .expect("failed to run PocketLab");
}
