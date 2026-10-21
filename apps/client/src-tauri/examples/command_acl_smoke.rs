//! Owned two-window IPC boundary check. No network, key store or VM access.
#[cfg(target_os = "linux")]
#[tauri::command]
fn runtime_platform() -> &'static str {
    std::env::consts::OS
}

#[cfg(target_os = "linux")]
fn main() {
    use std::sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    };
    use tauri::Manager;
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "dev.vmanyware.command-acl-fixture".into();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![runtime_platform])
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(30));
                eprintln!("Command ACL fixture deadline exceeded");
                handle.exit(1);
            });
            let passed = Arc::new(AtomicU8::new(0));
            for (label, bit, expected) in [("main", 1, "ACL_MAIN_ALLOWED"), ("untrusted", 2, "ACL_OTHER_DENIED")] {
                let passed = passed.clone();
                let script = format!(r#"
                    window.addEventListener('DOMContentLoaded', async () => {{
                        try {{
                            const value = await window.__TAURI_INTERNALS__.invoke('runtime_platform');
                            document.title = {label:?} === 'main' && value === 'linux' ? 'ACL_MAIN_ALLOWED' : 'ACL_FAIL';
                        }} catch (error) {{
                            document.title = {label:?} === 'untrusted' && String(error).includes('not allowed') ? 'ACL_OTHER_DENIED' : 'ACL_FAIL';
                        }}
                    }});
                "#);
                tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("index.html".into()))
                    .title("Owned IPC permission fixture")
                    .incognito(true)
                    .focused(false)
                    .initialization_script(script)
                    .on_document_title_changed(move |window, title| {
                        if title == "ACL_FAIL" {
                            eprintln!("Command ACL fixture: unexpected IPC result");
                            window.app_handle().exit(1);
                        } else if title == expected {
                            eprintln!("Command ACL fixture: {title}");
                            if passed.fetch_or(bit, Ordering::SeqCst) | bit == 3 {
                                eprintln!("Command ACL main allow / other window deny: passed");
                                window.app_handle().exit(0);
                            }
                        }
                    })
                    .build()?;
            }
            Ok(())
        })
        .run(context)
        .expect("Command ACL fixture failed");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This owned IPC fixture requires Linux WebKitGTK");
    std::process::exit(1);
}
