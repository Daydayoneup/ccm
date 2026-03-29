use tauri::State;
use crate::adapters::AdapterRegistry;
use crate::adapters::file_based::copy_dir_recursive;
use crate::db::Database;
use crate::models::v2::{Resource, ResourceType, ResourceScope};
use crate::scanner;
use std::path::Path;
use std::fs;

/// List global resources, optionally filtered by type.
/// Returns global-owned resources, resources linked from library,
/// and non-Global resources (e.g. Registry) whose files are in ~/.claude/.
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
    let mut seen_ids: std::collections::HashSet<String> = results.iter().map(|r| r.id.clone()).collect();

    // 2. Get resources linked/deployed from library to global
    let links = db.list_global_links()
        .map_err(|e| e.to_string())?;

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
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let claude_dir = home.join(".claude");

    super::resource_ops::create_resource_at(
        &db,
        &adapter_registry,
        &resource_type,
        &name,
        None,
        &content,
        &claude_dir,
        ResourceScope::Global,
    )
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

/// Backup a global resource to the central library.
/// When `replace_with_link` is true, the original global resource is deleted
/// and replaced with a symlink pointing to the library copy.
#[tauri::command]
pub fn backup_to_library(
    db: State<Database>,
    resource_id: String,
    replace_with_link: Option<bool>,
) -> Result<Resource, String> {
    let resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if resource.scope != ResourceScope::Global {
        return Err("Resource is not a global resource".to_string());
    }

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let library_base = home.join(".claude-manager").join("library");

    // Determine target path in library
    let adapter_registry = AdapterRegistry::new();
    let type_dir = adapter_registry
        .get(&resource.resource_type)
        .map(|a| a.type_dir())
        .ok_or_else(|| format!("No adapter for {:?}", resource.resource_type))?;
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
        source_path: target_path.clone(),
        content_hash: hash,
        metadata: None,
        created_at: now.clone(),
        updated_at: now,
        version: None,
        is_draft: 1,
            installed_from_id: None,
    };

    db.insert_resource(&library_resource).map_err(|e| e.to_string())?;

    // If replace_with_link is requested, delete the original and install from library
    if replace_with_link.unwrap_or(false) {
        // Delete the original global resource from filesystem
        if source.is_dir() {
            fs::remove_dir_all(source)
                .map_err(|e| format!("Failed to remove original directory: {}", e))?;
        } else {
            fs::remove_file(source)
                .map_err(|e| format!("Failed to remove original file: {}", e))?;
        }

        // Delete the old global resource record from DB
        db.delete_resource(&resource.id).map_err(|e| e.to_string())?;

        // Install the library copy to global via proper install flow
        // (copies to installed/, writes manifest, creates symlink)
        let scope = crate::install::InstallScope::Global;
        super::resource_ops::install_resource_to(
            &db, &library_resource, scope, &adapter_registry,
        )?;
    }

    Ok(library_resource)
}

