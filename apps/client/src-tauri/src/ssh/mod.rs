//! macOS host trust and explicit agent authentication. No terminal channels.
#[cfg(target_os = "macos")]
mod agent;
#[cfg(target_os = "macos")]
pub use agent::{AgentManager, Authentication, Selection};
#[cfg(not(target_os = "macos"))]
pub struct AgentManager;
#[cfg(not(target_os = "macos"))]
pub type Selection = ();
#[cfg(not(target_os = "macos"))]
pub type Authentication = ();
#[cfg(target_os = "macos")]
mod host;
#[cfg(target_os = "macos")]
mod trust;
#[cfg(target_os = "macos")]
pub use host::HostManager;
#[cfg(not(target_os = "macos"))]
pub struct HostManager;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInspection {
    host: String,
    port: u16,
    algorithm: String,
    fingerprint: String,
    status: &'static str,
    challenge_id: Option<String>,
    expires_in_seconds: u64,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostConfirmation {
    host: String,
    port: u16,
    fingerprint: String,
}

#[tauri::command]
pub async fn inspect_ssh_host(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, HostManager>,
    host: String,
    port: u16,
) -> Result<HostInspection, String> {
    #[cfg(target_os = "macos")]
    {
        manager.inspect(window.label(), &host, port).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, manager, host, port);
        Err("SSH host inspection is currently supported on macOS only".into())
    }
}
#[tauri::command]
pub async fn confirm_ssh_host(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, HostManager>,
    challenge_id: String,
    fingerprint: String,
) -> Result<HostConfirmation, String> {
    #[cfg(target_os = "macos")]
    {
        manager
            .confirm(window.label(), &challenge_id, &fingerprint)
            .await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, manager, challenge_id, fingerprint);
        Err("SSH host inspection is currently supported on macOS only".into())
    }
}
#[tauri::command]
pub async fn cancel_ssh_host(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, HostManager>,
    challenge_id: String,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        manager.cancel(window.label(), &challenge_id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, manager, challenge_id);
        Err("SSH host inspection is currently supported on macOS only".into())
    }
}

#[tauri::command]
pub fn ssh_host_support() -> bool {
    cfg!(target_os = "macos")
}

#[tauri::command]
pub async fn list_ssh_identities(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, AgentManager>,
    host: String,
    port: u16,
    username: String,
) -> Result<Selection, String> {
    #[cfg(target_os = "macos")]
    {
        manager.list(window.label(), &host, port, &username).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, manager, host, port, username);
        Err("SSH agent authentication is supported on macOS only".into())
    }
}

#[tauri::command]
pub async fn check_ssh_authentication(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, AgentManager>,
    selection_id: String,
    fingerprint: String,
) -> Result<Authentication, String> {
    #[cfg(target_os = "macos")]
    {
        manager
            .authenticate(window.label(), &selection_id, &fingerprint)
            .await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, manager, selection_id, fingerprint);
        Err("SSH agent authentication is supported on macOS only".into())
    }
}

#[tauri::command]
pub fn cancel_ssh_authentication(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, AgentManager>,
    selection_id: String,
) {
    #[cfg(target_os = "macos")]
    {
        manager.cancel_selection(window.label(), &selection_id);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, manager, selection_id);
    }
}
