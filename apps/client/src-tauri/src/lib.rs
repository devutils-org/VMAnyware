mod control_plane;

#[tauri::command]
fn runtime_platform() -> &'static str {
    std::env::consts::OS
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            runtime_platform,
            control_plane::inspect_control_plane,
            control_plane::list_definitions,
            control_plane::save_definition,
            control_plane::begin_inventory_refresh,
            control_plane::read_operation,
            control_plane::preview_provisioning
        ])
        .run(tauri::generate_context!())
        .expect("failed to run PocketLab");
}
