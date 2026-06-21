mod commands;
mod steam;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::read_library,
            commands::write_library,
            commands::file_mtime,
            commands::load_settings,
            commands::save_settings,
            steam::sync_steam,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
