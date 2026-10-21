#[cfg(target_os = "android")]
mod android_identity;
#[cfg(target_os = "android")]
mod android_tls;
mod control_plane;
mod remote_connection;
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
            app.manage(remote_connection::RemoteConnection::default());
            #[cfg(unix)]
            app.manage(ssh::identity::IdentityStore::new(
                app.path().app_local_data_dir()?.join("ssh-identity-v1"),
            ));
            #[cfg(unix)]
            {
                let hosts = ssh::HostManager::new(
                    app.path()
                        .app_local_data_dir()
                        .map_err(|_| {
                            std::io::Error::other("SSH application data path unavailable")
                        })?
                        .join("ssh-host-trust-v1"),
                );
                let agents = ssh::AgentManager::new(hosts.clone(), None)
                    .with_identity(app.state::<ssh::identity::IdentityStore>().inner().clone());
                app.manage(ssh::SessionManager::new(agents.clone()));
                app.manage(agents);
                app.manage(hosts);
            }
            #[cfg(not(unix))]
            app.manage(ssh::HostManager);
            #[cfg(not(unix))]
            app.manage(ssh::AgentManager);
            #[cfg(not(unix))]
            app.manage(ssh::SessionManager);
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(unix)]
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
            #[cfg(not(unix))]
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            runtime_platform,
            ssh::identity_support,
            ssh::system_identity_support,
            ssh::remember_app_identity,
            ssh::unlock_system_app_identity,
            ssh::forget_app_identity,
            ssh::list_system_ssh_identity,
            ssh::ssh_agent_support,
            ssh::list_app_ssh_identity,
            ssh::create_app_identity,
            ssh::unlock_app_identity,
            remote_connection::lifecycle::confirm_remote_lifecycle,
            remote_connection::lifecycle::read_remote_lifecycle_operation,
            remote_connection::lifecycle_recovery::read_remote_lifecycle_history,
            remote_connection::lifecycle_recovery::recover_remote_lifecycle,
            remote_connection::lifecycle::prepare_remote_lifecycle,
            remote_connection::lifecycle::clear_remote_lifecycle,
            remote_connection::instance_status::read_remote_instance_status,
            remote_connection::instance_detail::read_remote_instance_detail,
            remote_connection::connect_remote,
            remote_connection::disconnect_remote,
            remote_connection::recreation_recovery::read_remote_recreation_history,
            remote_connection::recreation_recovery::recover_remote_recreation,
            remote_connection::recreation::prepare_remote_recreation,
            remote_connection::recreation::confirm_remote_recreation,
            remote_connection::recreation::read_remote_recreation_operation,
            remote_connection::recreation::clear_remote_recreation,
            remote_connection::create::prepare_remote_create,
            remote_connection::create::confirm_remote_create,
            remote_connection::create::read_remote_create,
            remote_connection::create::read_remote_creations,
            remote_connection::create::recover_remote_creation,
            remote_connection::create::progress::read_remote_provisioning_progress,
            remote_connection::read_remote_images,
            remote_connection::read_remote_environments,
            remote_connection::snapshots::read_remote_snapshots,
            remote_connection::deletion_recovery::read_remote_deletion_history,
            remote_connection::deletion_recovery::recover_remote_deletion,
            remote_connection::deletion::prepare_remote_deletion,
            remote_connection::deletion::confirm_remote_deletion,
            remote_connection::deletion::read_remote_deletion_operation,
            remote_connection::deletion::clear_remote_deletion,
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
            control_plane::check_control_plane,
            control_plane::inspect_control_plane,
            control_plane::list_definitions,
            control_plane::save_definition,
            control_plane::validate_definition,
            control_plane::begin_inventory_refresh,
            control_plane::read_operation,
            control_plane::list_operations,
            control_plane::list_instances,
            control_plane::begin_instance_refresh,
            control_plane::read_instance_operation,
            control_plane::check_instance_lifecycle,
            control_plane::begin_lifecycle_action,
            control_plane::read_lifecycle_operation,
            control_plane::preview_provisioning,
            control_plane::preflight_provisioning
        ])
        .run(tauri::generate_context!())
        .expect("failed to run VMAnyware");
}
