use tauri::State;
use crate::db::Database;
use crate::models::v2::{LibraryPlugin, LibraryPluginResource, Resource};

#[tauri::command]
pub fn create_library_plugin(
    db: State<Database>,
    name: String,
    description: Option<String>,
    category: Option<String>,
) -> Result<LibraryPlugin, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let plugin = LibraryPlugin {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description,
        category,
        created_at: now.clone(),
        updated_at: now,
    };
    db.insert_library_plugin(&plugin)
        .map_err(|e| format!("Failed to create library plugin: {}", e))?;
    Ok(plugin)
}

#[tauri::command]
pub fn delete_library_plugin(
    db: State<Database>,
    id: String,
) -> Result<(), String> {
    db.delete_library_plugin(&id)
        .map_err(|e| format!("Failed to delete library plugin: {}", e))
}

#[tauri::command]
pub fn list_library_plugins(
    db: State<Database>,
) -> Result<Vec<LibraryPlugin>, String> {
    db.list_library_plugins()
        .map_err(|e| format!("Failed to list library plugins: {}", e))
}

#[tauri::command]
pub fn add_resource_to_library_plugin(
    db: State<Database>,
    plugin_id: String,
    resource_id: String,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let link = LibraryPluginResource {
        id: uuid::Uuid::new_v4().to_string(),
        plugin_id,
        resource_id,
        created_at: now,
    };
    db.add_resource_to_library_plugin(&link)
        .map_err(|e| format!("Failed to add resource to plugin: {}", e))
}

#[tauri::command]
pub fn remove_resource_from_library_plugin(
    db: State<Database>,
    plugin_id: String,
    resource_id: String,
) -> Result<(), String> {
    db.remove_resource_from_library_plugin(&plugin_id, &resource_id)
        .map_err(|e| format!("Failed to remove resource from plugin: {}", e))
}

#[tauri::command]
pub fn get_library_plugin_resources(
    db: State<Database>,
    plugin_id: String,
) -> Result<Vec<Resource>, String> {
    db.list_library_plugin_resources(&plugin_id)
        .map_err(|e| format!("Failed to get plugin resources: {}", e))
}
