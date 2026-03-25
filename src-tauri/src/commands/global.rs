use tauri::State;
use crate::adapters::AdapterRegistry;
use crate::adapters::file_based::copy_dir_recursive;
use crate::db::Database;
use crate::models::v2::{Resource, ResourceType, ResourceScope};
use crate::scanner;
use std::path::Path;
use std::fs;

/// List global resources, optionally filtered by type
/// Returns both global-owned resources AND resources linked/deployed from library
#[tauri::command]
pub fn list_global_resources(db: State<Database>, resource_type: Option<String>) -> Result<Vec<Resource>, String> {
    // 1. Get global-owned resources
    let owned_resources = match &resource_type {
        Some(rt) => {
            let rtype = ResourceType::from_str(rt)
                .ok_or_else(|| format!("Invalid resource type: {}", rt))?;
            db.list_resources_by_scope_and_type(&ResourceScope::Global, &rtype)
                .map_err(|e| e.to_string())?
        }
        None => {
            db.list_resources_by_scope(&ResourceScope::Global)
                .map_err(|e| e.to_string())?
        }
    };

    let mut results = owned_resources;

    // 2. Get resources linked/deployed from library to global
    let links = db.list_global_links()
        .map_err(|e| e.to_string())?;

    let mut seen_ids: std::collections::HashSet<String> = results.iter().map(|r| r.id.clone()).collect();

    for link in &links {
        if seen_ids.contains(&link.resource_id) {
            continue;
        }
        if let Ok(Some(resource)) = db.get_resource(&link.resource_id) {
            if let Some(ref rt) = resource_type {
                if resource.resource_type.as_str() != rt.as_str() {
                    continue;
                }
            }
            seen_ids.insert(resource.id.clone());
            results.push(resource);
        }
    }

    Ok(results)
}

/// Create a new global resource — writes file to ~/.claude/<type>/ and inserts DB record
#[tauri::command]
pub fn create_global_resource(
    db: State<Database>,
    adapter_registry: State<AdapterRegistry>,
    resource_type: String,
    name: String,
    content: String,
) -> Result<Resource, String> {
    let rtype = ResourceType::from_str(&resource_type)
        .ok_or_else(|| format!("Invalid resource type: {}", resource_type))?;

    // Validate content via adapter
    let adapter = adapter_registry.get(&rtype)
        .ok_or_else(|| format!("No adapter for resource type: {}", resource_type))?;
    adapter.validate_content(&content)?;

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let claude_dir = home.join(".claude");

    // Determine file path based on resource type
    let file_path = match rtype {
        ResourceType::Skill => {
            let skill_dir = claude_dir.join("skills").join(&name);
            fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;
            skill_dir.join("SKILL.md")
        }
        ResourceType::Agent => {
            let dir = claude_dir.join("agents");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.join(format!("{}.md", name))
        }
        ResourceType::Rule => {
            let dir = claude_dir.join("rules");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.join(format!("{}.md", name))
        }
        ResourceType::Hook => {
            let dir = claude_dir.join("hooks");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.join(format!("{}.json", name))
        }
        ResourceType::Command => {
            let dir = claude_dir.join("commands");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.join(format!("{}.md", name))
        }
        ResourceType::McpServer => {
            return Err("McpServer resources cannot be created via this command".to_string());
        }
    };

    // Write file
    fs::write(&file_path, &content).map_err(|e| e.to_string())?;

    // For skills, the source_path should be the skill directory, not the SKILL.md file
    let source_path = match rtype {
        ResourceType::Skill => file_path.parent().unwrap().to_string_lossy().to_string(),
        _ => file_path.to_string_lossy().to_string(),
    };

    // Compute hash
    let hash = scanner::compute_file_hash(&source_path);

    // Create resource record
    let now = chrono::Utc::now().to_rfc3339();
    let resource = Resource {
        id: uuid::Uuid::new_v4().to_string(),
        resource_type: rtype,
        name: name.clone(),
        description: None,
        scope: ResourceScope::Global,
        source_path,
        content_hash: hash,
        metadata: None,
        created_at: now.clone(),
        updated_at: now,
        version: None,
        is_draft: 1,
    };

    db.insert_resource(&resource).map_err(|e| e.to_string())?;
    Ok(resource)
}

/// Delete a global resource — optionally removes file from filesystem, always removes DB record.
/// For library resources deployed to global via links, removes the global links (symlinks/copies)
/// without deleting the library source.
#[tauri::command]
pub fn delete_global_resource(db: State<Database>, id: String, delete_from_disk: Option<bool>) -> Result<(), String> {
    let resource = db.get_resource(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", id))?;

    // Library resource deployed to global: unlink instead of delete
    if resource.scope == ResourceScope::Library {
        let links = db.list_links_by_resource(&id).map_err(|e| e.to_string())?;
        let global_links: Vec<_> = links.into_iter()
            .filter(|l| l.target_scope == "global")
            .collect();

        if global_links.is_empty() {
            return Err("Resource has no global deployment to remove".to_string());
        }

        for link in &global_links {
            // Remove the target (symlink, copy, or config entry)
            let target = Path::new(&link.target_path);
            if target.exists() || target.symlink_metadata().is_ok() {
                if target.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                    fs::remove_file(target).map_err(|e| e.to_string())?;
                } else if target.is_dir() {
                    fs::remove_dir_all(target).map_err(|e| e.to_string())?;
                } else {
                    fs::remove_file(target).map_err(|e| e.to_string())?;
                }
            }
            // Remove link record from DB
            db.delete_link(&link.id).map_err(|e| e.to_string())?;
        }

        return Ok(());
    }

    if resource.scope != ResourceScope::Global {
        return Err("Resource is not a global resource".to_string());
    }

    // Check for links
    let links = db.list_links_by_resource(&id).map_err(|e| e.to_string())?;
    if !links.is_empty() {
        return Err(format!("Resource is linked to {} locations. Unlink first.", links.len()));
    }

    // Delete from filesystem if requested
    if delete_from_disk.unwrap_or(false) {
        let path = Path::new(&resource.source_path);
        if path.exists() {
            if path.is_dir() {
                fs::remove_dir_all(path).map_err(|e| e.to_string())?;
            } else {
                fs::remove_file(path).map_err(|e| e.to_string())?;
            }
        }
    }

    // Delete from DB
    db.delete_resource(&id).map_err(|e| e.to_string())?;
    Ok(())
}

/// Backup a global resource to the central library
#[tauri::command]
pub fn backup_to_library(db: State<Database>, resource_id: String) -> Result<Resource, String> {
    let resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if resource.scope != ResourceScope::Global {
        return Err("Resource is not a global resource".to_string());
    }

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let library_base = home.join(".claude-manager").join("library");

    // Determine target path in library
    let type_dir = match resource.resource_type {
        ResourceType::Skill => "skills",
        ResourceType::Agent => "agents",
        ResourceType::Rule => "rules",
        ResourceType::Hook => "hooks",
        ResourceType::Command => "commands",
        ResourceType::McpServer => "mcp_servers",
    };
    let target_dir = library_base.join(type_dir);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let source = Path::new(&resource.source_path);
    let target = if source.is_dir() {
        // Skill: copy entire directory
        let dest = target_dir.join(&resource.name);
        copy_dir_recursive(source, &dest)?;
        dest
    } else {
        // Single file: copy it
        let file_name = source.file_name().ok_or("Invalid source path")?;
        let dest = target_dir.join(file_name);
        fs::copy(source, &dest).map_err(|e| e.to_string())?;
        dest
    };

    let target_path = target.to_string_lossy().to_string();
    let hash = scanner::compute_file_hash(&target_path);

    // Create library resource record
    let now = chrono::Utc::now().to_rfc3339();
    let library_resource = Resource {
        id: uuid::Uuid::new_v4().to_string(),
        resource_type: resource.resource_type.clone(),
        name: resource.name.clone(),
        description: resource.description.clone(),
        scope: ResourceScope::Library,
        source_path: target_path,
        content_hash: hash,
        metadata: None,
        created_at: now.clone(),
        updated_at: now,
        version: None,
        is_draft: 1,
    };

    db.insert_resource(&library_resource).map_err(|e| e.to_string())?;
    Ok(library_resource)
}

