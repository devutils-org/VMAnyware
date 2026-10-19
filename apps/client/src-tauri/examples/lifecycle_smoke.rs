//! Opt-in Linux WebView/real IPC smoke; never linked into the normal application.
#[cfg(target_os = "linux")]
#[path = "../src/control_plane.rs"]
#[allow(dead_code)]
mod control_plane;

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
    let script = format!(
        "window.__lifecycleFixturePort = {port};\n{}",
        include_str!("../../../../experiments/lifecycle-native/scenario.js")
    );
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "dev.vmanyware.lifecycle-fixture".into();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            runtime_platform,
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
