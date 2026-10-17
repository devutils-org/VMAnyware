//! Opt-in real Tauri/IPC/SSH smoke. Never linked into the normal application.
#[cfg(target_os = "macos")]
#[path = "../src/ssh/agent/fixture.rs"]
#[allow(dead_code)]
mod fixture;
#[cfg(target_os = "macos")]
#[path = "../src/ssh/mod.rs"]
mod ssh;

#[cfg(target_os = "macos")]
mod smoke {
    use super::{fixture, ssh};
    use russh::keys::HashAlg;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };
    use tauri::Manager;
    struct State {
        fixture: fixture::Fixture,
        agent: Mutex<Option<fixture::DisposableAgent>>,
        directory: Mutex<Option<tempfile::TempDir>>,
        reported: AtomicBool,
        trust_before_rotation: Mutex<Option<Vec<u8>>>,
    }
    #[tauri::command]
    async fn smoke_rotate_host(state: tauri::State<'_, State>) -> Result<(), String> {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while state.fixture.connections.load(Ordering::SeqCst) != 0 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .map_err(|_| "Fixture connection did not close")?;
        let directory = state.directory.lock().unwrap();
        let path = directory
            .as_ref()
            .ok_or("Fixture directory unavailable")?
            .path()
            .join("trust/trust.json");
        let bytes = std::fs::read(path).map_err(|_| "Fixture trust unavailable")?;
        *state.trust_before_rotation.lock().unwrap() = Some(bytes);
        *state.fixture.host_key.lock().unwrap() = fixture::key();
        Ok(())
    }
    #[tauri::command]
    async fn smoke_report(
        app: tauri::AppHandle,
        state: tauri::State<'_, State>,
        ok: bool,
        step: u32,
        failure: Option<String>,
    ) -> Result<(), ()> {
        if state.reported.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(failure) = failure {
            eprintln!("Native smoke assertion: {failure}");
        }
        app.state::<ssh::SessionManager>().forget_owner("main");
        let clean = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while state.fixture.connections.load(Ordering::SeqCst) != 0 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok();
        let shells = state.fixture.shells.lock().unwrap();
        let shells_closed =
            !shells.is_empty() && shells.iter().all(|shell| shell.lock().unwrap().is_closed());
        drop(shells);
        let authenticated = state.fixture.auth.load(Ordering::SeqCst) == 2;
        let resized = state
            .fixture
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches!(event, fixture::Protocol::Resize(_, _)))
            .count()
            >= 2;
        let unchanged = {
            let directory = state.directory.lock().unwrap();
            let current = directory
                .as_ref()
                .and_then(|dir| std::fs::read(dir.path().join("trust/trust.json")).ok());
            current.is_some() && *state.trust_before_rotation.lock().unwrap() == current
        };
        state.fixture.shutdown();
        state.agent.lock().unwrap().take();
        state.directory.lock().unwrap().take();
        let passed = ok && clean && shells_closed && authenticated && resized && unchanged;
        eprintln!("Native Tauri smoke: {} (real IPC/agent/shell; step={step}, cleanup={clean}, shells={shells_closed}, auth={authenticated}, resize={resized}, trust_unchanged={unchanged})", if passed { "passed" } else { "FAILED" });
        app.exit(if passed { 0 } else { 1 });
        Ok(())
    }
    pub fn run() {
        let (fixture, agent, fingerprint, identity) = tauri::async_runtime::block_on(async {
            let key = fixture::key();
            let identity = key.public_key().fingerprint(HashAlg::Sha256).to_string();
            let agent = fixture::DisposableAgent::start().await;
            agent.add(&key).await;
            let fixture =
                fixture::Fixture::with_configuration(key.public_key().clone(), true, None, true)
                    .await;
            let fingerprint = fixture
                .host_key
                .lock()
                .unwrap()
                .public_key()
                .fingerprint(HashAlg::Sha256)
                .to_string();
            (fixture, agent, fingerprint, identity)
        });
        let directory = tempfile::tempdir().unwrap();
        let hosts = ssh::HostManager::new(directory.path().join("trust"));
        let agents = ssh::AgentManager::new(hosts.clone(), Some(agent.socket.clone()));
        let sessions = ssh::SessionManager::new(agents.clone());
        let parameters = serde_json::json!({"host":"127.0.0.1", "port":fixture.port, "fingerprint":fingerprint, "identity":identity});
        let script = format!(
            "window.__nativeSmoke = {parameters};\n{}",
            std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../experiments/terminal-native/scenario.js"
            ))
            .expect("repository-owned native scenario unavailable")
        );
        let mut context = tauri::generate_context!();
        context.config_mut().identifier = "dev.pocketlab.terminal-fixture".into();
        context.config_mut().app.windows.clear();
        tauri::Builder::default()
            .manage(hosts)
            .manage(agents)
            .manage(sessions)
            .manage(State {
                fixture,
                agent: Mutex::new(Some(agent)),
                directory: Mutex::new(Some(directory)),
                reported: AtomicBool::new(false),
                trust_before_rotation: Mutex::new(None),
            })
            .invoke_handler(tauri::generate_handler![
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
                smoke_report,
                smoke_rotate_host
            ])
            .setup(move |app| {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(45)).await;
                    eprintln!("Native Tauri smoke deadline exceeded");
                    handle.state::<ssh::SessionManager>().forget_owner("main");
                    handle.state::<State>().fixture.shutdown();
                    handle.state::<State>().agent.lock().unwrap().take();
                    handle.state::<State>().directory.lock().unwrap().take();
                    handle.exit(1);
                });
                tauri::WebviewWindowBuilder::new(
                    app,
                    "main",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("PocketLab disposable native smoke")
                .inner_size(960.0, 900.0)
                .incognito(true)
                .focused(false)
                .initialization_script(script)
                .build()?;
                Ok(())
            })
            .on_window_event(|window, event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    window
                        .state::<ssh::SessionManager>()
                        .forget_owner(window.label());
                    window
                        .state::<ssh::HostManager>()
                        .forget_owner(window.label());
                    window.state::<ssh::AgentManager>().cancel(window.label());
                    let state = window.state::<State>();
                    state.fixture.shutdown();
                    state.agent.lock().unwrap().take();
                    state.directory.lock().unwrap().take();
                }
            })
            .run(context)
            .expect("isolated native fixture failed");
    }
}
#[cfg(target_os = "macos")]
fn main() {
    smoke::run();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This isolated native smoke requires macOS");
    std::process::exit(1);
}
