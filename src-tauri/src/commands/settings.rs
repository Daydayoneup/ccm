use crate::db::Database;
use tauri::State;

#[tauri::command]
pub fn get_app_setting(db: State<'_, Database>, key: String) -> Result<Option<String>, String> {
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_app_setting(db: State<'_, Database>, key: String, value: String) -> Result<(), String> {
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}
