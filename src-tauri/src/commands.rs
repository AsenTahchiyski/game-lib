use std::fs;
use std::time::UNIX_EPOCH;

use serde_json::Value;
use tauri::Manager;

/// Read a library JSON file from an arbitrary path (e.g. a NAS mount) and return
/// it as opaque JSON. The frontend owns the schema; Rust only does the IO.
#[tauri::command]
pub fn read_library(path: String) -> Result<Value, String> {
    let data = fs::read_to_string(&path).map_err(|e| format!("Failed to read {path}: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("Invalid library JSON in {path}: {e}"))
}

/// Write the library JSON to `path`, pretty-printed. Writes to a temp file first
/// and renames, so a crash mid-write can't corrupt the existing library.
#[tauri::command]
pub fn write_library(path: String, library: Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(&library).map_err(|e| e.to_string())?;
    let tmp = format!("{path}.tmp");
    fs::write(&tmp, text).map_err(|e| format!("Failed to write {tmp}: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Failed to finalize {path}: {e}"))?;
    Ok(())
}

/// Last-modified time of a file in milliseconds since the Unix epoch, or `None`
/// if the file doesn't exist. Used to detect a newer copy on disk (NAS sync).
#[tauri::command]
pub fn file_mtime(path: String) -> Result<Option<u64>, String> {
    match fs::metadata(&path) {
        Ok(meta) => {
            let modified = meta.modified().map_err(|e| e.to_string())?;
            let ms = modified
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis() as u64;
            Ok(Some(ms))
        }
        Err(_) => Ok(None),
    }
}

fn settings_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

/// Load app settings (Steam key, tokens, last library path) from the OS app
/// config dir. Kept OUT of the shared library file so it's safe to share.
#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<Value, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Value) -> Result<(), String> {
    let path = settings_path(&app)?;
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}
