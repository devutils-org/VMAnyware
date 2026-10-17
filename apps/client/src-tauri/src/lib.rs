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
            {
                let hosts = ssh::HostManager::new(
                    app.path()
                        .app_local_data_dir()
                        .map_err(|_| {
                            std::io::Error::other("SSH application data path unavailable")
                        })?
                        .join("ssh-host-trust-v1"),
                );
                let agents = ssh::AgentManager::new(hosts.clone(), None);
                app.manage(ssh::SessionManager::new(agents.clone()));
                app.manage(agents);
                app.manage(hosts);
            }
            #[cfg(not(target_os = "macos"))]
            app.manage(ssh::HostManager);
            #[cfg(not(target_os = "macos"))]
            app.manage(ssh::AgentManager);
            #[cfg(not(target_os = "macos"))]
            app.manage(ssh::SessionManager);
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::WindowEvent::Destroyed) {
                if let Some(manager) = window.try_state::<ssh::HostManager>() {
                    manager.forget_owner(window.label());
                }
                if let Some(manager) = window.try_state::<ssh::AgentManager>() {
                    manager.cancel(window.label());
                }
                if let Some(manager) = window.try_state::<ssh::SessionManager>() {
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
            ssh::list_ssh_identities,
            ssh::check_ssh_authentication,
            ssh::cancel_ssh_authentication,
            ssh::start_ssh_session,
            ssh::read_ssh_session,
            ssh::write_ssh_session,
            ssh::resize_ssh_session,
            ssh::close_ssh_session,
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
