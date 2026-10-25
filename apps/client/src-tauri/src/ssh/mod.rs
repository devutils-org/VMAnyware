#[cfg(unix)]
pub(crate) mod identity;
// Unix host trust, window-bound identities and bounded PTY transport.
#[cfg(unix)]
mod session;
#[cfg(unix)]
pub use session::{Event as SessionEvent, SessionManager, Started};
#[cfg(not(unix))]
pub struct SessionManager;
#[cfg(not(unix))]
pub type Started = ();
#[cfg(not(unix))]
pub type SessionEvent = ();
#[cfg(unix)]
mod agent;
#[cfg(unix)]
pub use agent::{AgentManager, Authentication, Selection};
#[cfg(not(unix))]
pub struct AgentManager;
#[cfg(not(unix))]
pub type Selection = ();
#[cfg(not(unix))]
pub type Authentication = ();
#[cfg(unix)]
mod host;
#[cfg(unix)]
pub(crate) mod trust;
#[cfg(unix)]
pub use host::HostManager;
#[cfg(not(unix))]
pub struct HostManager;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInspection {
    host: String,
    port: u16,
    algorithm: String,
    fingerprint: String,
    status: &'static str,
    previous_fingerprint: Option<String>,
    challenge_id: Option<String>,
    expires_in_seconds: u64,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedHost {
    host: String,
    port: u16,
    algorithm: String,
    fingerprint: String,
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
    #[cfg(unix)]
    {
        manager.inspect(window.label(), &host, port).await
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, host, port);
        Err("SSH host inspection is unavailable on this platform".into())
    }
}
#[tauri::command]
pub async fn confirm_ssh_host(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, HostManager>,
    challenge_id: String,
    fingerprint: String,
) -> Result<HostConfirmation, String> {
    #[cfg(unix)]
    {
        manager
            .confirm(window.label(), &challenge_id, &fingerprint)
            .await
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, challenge_id, fingerprint);
        Err("SSH host inspection is unavailable on this platform".into())
    }
}
#[tauri::command]
pub async fn replace_ssh_host(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, HostManager>,
    challenge_id: String,
    previous_fingerprint: String,
    fingerprint: String,
) -> Result<HostConfirmation, String> {
    #[cfg(unix)]
    {
        manager
            .replace(
                window.label(),
                &challenge_id,
                &previous_fingerprint,
                &fingerprint,
            )
            .await
    }
    #[cfg(not(unix))]
    {
        let _ = (
            window,
            manager,
            challenge_id,
            previous_fingerprint,
            fingerprint,
        );
        Err("SSH host inspection is unavailable on this platform".into())
    }
}
#[tauri::command]
pub async fn list_ssh_hosts(
    manager: tauri::State<'_, HostManager>,
) -> Result<Vec<TrustedHost>, String> {
    #[cfg(unix)]
    {
        manager.trusted().await
    }
    #[cfg(not(unix))]
    {
        let _ = manager;
        Err("SSH host trust is unavailable on this platform".into())
    }
}
#[tauri::command]
pub async fn forget_ssh_host(
    manager: tauri::State<'_, HostManager>,
    host: String,
    port: u16,
    fingerprint: String,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        manager.forget(&host, port, &fingerprint).await
    }
    #[cfg(not(unix))]
    {
        let _ = (manager, host, port, fingerprint);
        Err("SSH host trust is unavailable on this platform".into())
    }
}
#[tauri::command]
pub async fn cancel_ssh_host(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, HostManager>,
    challenge_id: String,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        manager.cancel(window.label(), &challenge_id)
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, challenge_id);
        Err("SSH host inspection is unavailable on this platform".into())
    }
}

#[tauri::command]
pub fn ssh_host_support() -> bool {
    cfg!(unix)
}

#[tauri::command]
pub async fn list_ssh_identities(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, AgentManager>,
    host: String,
    port: u16,
    username: String,
) -> Result<Selection, String> {
    #[cfg(unix)]
    {
        manager.list(window.label(), &host, port, &username).await
    }
    #[cfg(not(unix))]
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
    #[cfg(unix)]
    {
        manager
            .authenticate(window.label(), &selection_id, &fingerprint)
            .await
    }
    #[cfg(not(unix))]
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
    #[cfg(unix)]
    {
        manager.cancel_selection(window.label(), &selection_id);
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, selection_id);
    }
}

#[tauri::command]
pub async fn start_ssh_session(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, SessionManager>,
    selection_id: String,
    fingerprint: String,
    cols: u32,
    rows: u32,
) -> Result<Started, String> {
    #[cfg(unix)]
    {
        manager
            .start(window.label(), &selection_id, &fingerprint, cols, rows)
            .await
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, selection_id, fingerprint, cols, rows);
        Err("SSH sessions are unavailable on this platform".into())
    }
}
#[tauri::command]
pub async fn read_ssh_session(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, SessionManager>,
    session_id: String,
) -> Result<Option<SessionEvent>, String> {
    #[cfg(unix)]
    {
        manager.read(window.label(), &session_id).await
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, session_id);
        Err("SSH sessions are unavailable on this platform".into())
    }
}
#[tauri::command]
pub fn write_ssh_session(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, SessionManager>,
    session_id: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        manager.write(window.label(), &session_id, bytes)
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, session_id, bytes);
        Err("SSH sessions are unavailable on this platform".into())
    }
}
#[tauri::command]
pub fn resize_ssh_session(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, SessionManager>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        manager.resize(window.label(), &session_id, cols, rows)
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, session_id, cols, rows);
        Err("SSH sessions are unavailable on this platform".into())
    }
}
#[tauri::command]
pub fn close_ssh_session(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, SessionManager>,
    session_id: String,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        manager.close(window.label(), &session_id)
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, session_id);
        Err("SSH sessions are unavailable on this platform".into())
    }
}

#[tauri::command]
pub fn identity_support() -> bool {
    cfg!(unix)
}

#[tauri::command]
pub async fn create_app_identity(
    app: tauri::AppHandle,
    passphrase: String,
) -> Result<serde_json::Value, String> {
    app_identity_action(app, passphrase, true).await
}
#[tauri::command]
pub async fn unlock_app_identity(
    app: tauri::AppHandle,
    passphrase: String,
) -> Result<serde_json::Value, String> {
    app_identity_action(app, passphrase, false).await
}
async fn app_identity_action(
    app: tauri::AppHandle,
    passphrase: String,
    create: bool,
) -> Result<serde_json::Value, String> {
    #[cfg(unix)]
    {
        use tauri::Manager;
        let passphrase = zeroize::Zeroizing::new(passphrase);
        tauri::async_runtime::spawn_blocking(move || {
            let store = app.state::<identity::IdentityStore>();
            let identity = if create {
                store.create(passphrase)?
            } else {
                store.unlock(passphrase)?
            };
            serde_json::to_value(identity).map_err(|_| "SSH identity unavailable".into())
        })
        .await
        .map_err(|_| "SSH identity unavailable")?
    }
    #[cfg(not(unix))]
    {
        let _ = (app, passphrase, create);
        Err("Application SSH identity is not available on this platform".into())
    }
}

#[tauri::command]
pub fn ssh_agent_support() -> bool {
    cfg!(target_os = "macos")
}

#[tauri::command]
pub async fn list_app_ssh_identity(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, AgentManager>,
    host: String,
    port: u16,
    username: String,
    passphrase: String,
) -> Result<Selection, String> {
    #[cfg(unix)]
    {
        manager
            .list_application(
                window.label(),
                &host,
                port,
                &username,
                zeroize::Zeroizing::new(passphrase),
            )
            .await
    }
    #[cfg(not(unix))]
    {
        let _ = (window, manager, host, port, username, passphrase);
        Err("Application SSH identity unavailable on this platform".into())
    }
}

#[tauri::command]
pub fn system_identity_support() -> bool {
    cfg!(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos"
    ))
}
#[tauri::command]
pub async fn remember_app_identity(
    app: tauri::AppHandle,
    passphrase: String,
    confirmed: bool,
) -> Result<serde_json::Value, String> {
    if !confirmed {
        return Err("Confirm saving this identity's unlock secret in the system key store.".into());
    }
    system_identity_action(app, Some(passphrase), false, false).await
}
#[tauri::command]
pub async fn unlock_system_app_identity(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    system_identity_action(app, None, false, false).await
}
#[tauri::command]
pub async fn forget_app_identity(
    app: tauri::AppHandle,
    confirmed: bool,
) -> Result<serde_json::Value, String> {
    system_identity_action(app, None, true, confirmed).await
}
async fn system_identity_action(
    app: tauri::AppHandle,
    passphrase: Option<String>,
    forget: bool,
    confirmed: bool,
) -> Result<serde_json::Value, String> {
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
    {
        use tauri::Manager;
        let passphrase = passphrase.map(zeroize::Zeroizing::new);
        tauri::async_runtime::spawn_blocking(move || {
            let store = app.state::<identity::IdentityStore>();
            if forget {
                store.forget(confirmed)?;
                return Ok(serde_json::Value::Null);
            }
            let public = match passphrase {
                Some(secret) => store.remember(secret)?,
                None => store.unlock_system()?,
            };
            serde_json::to_value(public).map_err(|_| "System SSH identity unavailable".into())
        })
        .await
        .map_err(|_| "System SSH identity unavailable")?
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
    {
        let _ = (app, passphrase, forget, confirmed);
        Err("System SSH identity storage is unavailable on this platform".into())
    }
}
#[tauri::command]
pub async fn list_system_ssh_identity(
    window: tauri::WebviewWindow,
    manager: tauri::State<'_, AgentManager>,
    host: String,
    port: u16,
    username: String,
) -> Result<Selection, String> {
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
    {
        manager
            .list_system_application(window.label(), &host, port, &username)
            .await
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
    {
        let _ = (window, manager, host, port, username);
        Err("System SSH identity storage is unavailable on this platform".into())
    }
}
