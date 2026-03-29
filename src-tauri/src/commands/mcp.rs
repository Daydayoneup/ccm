use tauri::State;
use crate::db::Database;
use crate::models::v2::{Resource, ResourceType, ResourceScope};
use crate::adapters::config_based::{read_or_create_json, write_json};
use crate::scanner::compute_content_hash;
use std::path::Path;

/// Update an existing MCP server's config in .mcp.json and the resources table.
#[tauri::command]
pub fn update_mcp_server_config(
    db: State<Database>,
    resource_id: String,
    new_config_json: String,
) -> Result<Resource, String> {
    // Validate JSON
    let new_value: serde_json::Value = serde_json::from_str(&new_config_json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Get existing resource
    let mut resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if resource.resource_type != ResourceType::McpServer {
        return Err("Resource is not an MCP server".to_string());
    }

    // Library MCP resources store the inner config directly in the file
    if resource.scope == ResourceScope::Library {
        let config_path = Path::new(&resource.source_path);
        write_json(config_path, &new_value)?;
    } else {
        // Project/Global MCP resources are entries in a .mcp.json with mcpServers wrapper
        let config_path = Path::new(&resource.source_path);
        let mut config = read_or_create_json(config_path)?;

        let servers = config
            .as_object_mut()
            .ok_or("Config is not a JSON object")?
            .entry("mcpServers")
            .or_insert_with(|| serde_json::json!({}));

        let servers_obj = servers
            .as_object_mut()
            .ok_or("mcpServers is not a JSON object")?;

        if !servers_obj.contains_key(&resource.name) {
            return Err(format!("Server '{}' not found in {}", resource.name, resource.source_path));
        }

        servers_obj.insert(resource.name.clone(), new_value);
        write_json(config_path, &config)?;
    }

    // Update resource metadata and hash
    let content_hash = compute_content_hash(&new_config_json);
    resource.metadata = Some(new_config_json);
    resource.content_hash = Some(content_hash);
    resource.updated_at = chrono::Utc::now().to_rfc3339();
    db.update_resource(&resource).map_err(|e| e.to_string())?;

    Ok(resource)
}

/// Create a new MCP server in a project's .mcp.json (creates the file if needed).
#[tauri::command]
pub fn create_mcp_server(
    db: State<Database>,
    project_id: String,
    name: String,
    config_json: String,
) -> Result<Resource, String> {
    // Validate JSON
    let server_value: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Get project path
    let project = db.get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let mcp_path = Path::new(&project.path).join(".mcp.json");

    // Read or create the .mcp.json file
    let mut config = read_or_create_json(&mcp_path)?;

    let servers = config
        .as_object_mut()
        .ok_or("Config is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let servers_obj = servers
        .as_object_mut()
        .ok_or("mcpServers is not a JSON object")?;

    if servers_obj.contains_key(&name) {
        return Err(format!("Server '{}' already exists in {}", name, mcp_path.display()));
    }

    servers_obj.insert(name.clone(), server_value);
    write_json(&mcp_path, &config)?;

    // Insert resource into DB
    let content_hash = compute_content_hash(&config_json);
    let now = chrono::Utc::now().to_rfc3339();
    let resource = Resource {
        id: uuid::Uuid::new_v4().to_string(),
        resource_type: ResourceType::McpServer,
        name,
        description: None,
        scope: ResourceScope::Project,
        source_path: mcp_path.to_string_lossy().to_string(),
        content_hash: Some(content_hash),
        metadata: Some(config_json),
        created_at: now.clone(),
        updated_at: now,
        version: None,
        is_draft: 1,
            installed_from_id: None,
    };
    db.insert_resource(&resource).map_err(|e| e.to_string())?;

    Ok(resource)
}
