use crate::db::Database;
use crate::models::v2::{EnvVar, MergedEnvVar};
use tauri::State;

#[tauri::command]
pub fn list_env_vars(
    db: State<'_, Database>,
    project_id: Option<String>,
) -> Result<Vec<EnvVar>, String> {
    db.list_env_vars(project_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_env_var(
    db: State<'_, Database>,
    project_id: Option<String>,
    key: String,
    value: String,
) -> Result<EnvVar, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let env_var = EnvVar {
        id: id.clone(),
        project_id,
        key,
        value,
    };
    db.insert_env_var(&env_var).map_err(|e| e.to_string())?;
    Ok(env_var)
}

#[tauri::command]
pub fn delete_env_var(
    db: State<'_, Database>,
    id: String,
) -> Result<(), String> {
    db.delete_env_var(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_merged_env_vars(
    db: State<'_, Database>,
    project_id: String,
) -> Result<Vec<MergedEnvVar>, String> {
    db.list_merged_env_vars(&project_id)
        .map_err(|e| e.to_string())
}
