mod commands;
mod epic;
mod gog;
mod ign;
mod steam;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
            steam::sync_steam,
            gog::gog_login_url,
            gog::gog_exchange_code,
            gog::gog_sync,
            epic::epic_login_url,
            epic::epic_exchange_code,
            epic::epic_sync,
            ign::ign_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
