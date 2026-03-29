use tauri::State;
use crate::adapters::AdapterRegistry;
use crate::adapters::file_based::copy_dir_recursive;
use crate::db::Database;
use crate::models::v2::{Resource, Project, ResourceType, ResourceScope};
use crate::scanner;
use std::path::Path;
use std::fs;

/// List all registered projects from DB
#[tauri::command]
pub fn list_projects_v2(db: State<Database>) -> Result<Vec<Project>, String> {
    db.list_projects().map_err(|e| e.to_string())
}

/// Register a project: scan it, insert project + its resources + MCP servers into DB
#[tauri::command]
pub fn register_project_v2(
    db: State<Database>,
    adapter_registry: State<'_, AdapterRegistry>,
    path: String,
) -> Result<Project, String> {
    // Check if already registered
    if let Some(existing) = db.get_project_by_path(&path).map_err(|e| e.to_string())? {
        return Ok(existing);
    }

    let (project, resources) = scanner::project::scan_project_v3(&path, &adapter_registry)?;

    db.insert_project(&project).map_err(|e| e.to_string())?;

    // Insert scanned resources with scope=project
    let now = chrono::Utc::now().to_rfc3339();
    for mut resource in resources {
        // Assign a fresh ID and timestamps; adapter.scan() may have set placeholders
        resource.id = uuid::Uuid::new_v4().to_string();
        resource.created_at = now.clone();
        resource.updated_at = now.clone();
        let _ = db.insert_resource(&resource);
    }

    Ok(project)
}

/// Result of removing a project — DB deletion always succeeds if Ok, warnings collect disk errors.
#[derive(serde::Serialize)]
pub struct RemoveResult {
    pub warnings: Vec<String>,
}

/// Remove a project and its resources from DB, optionally delete from disk.
#[tauri::command]
pub fn remove_project_v2(
    db: State<Database>,
    id: String,
    delete_from_disk: Option<bool>,
) -> Result<RemoveResult, String> {
    let project = db.get_project(&id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", id))?;

    // Delete project resources from DB (those whose source_path starts with project.path)
    let project_resources = db.list_resources_by_scope(&ResourceScope::Project)
        .map_err(|e| e.to_string())?;
    for r in project_resources {
        if r.source_path.starts_with(&project.path) {
            let _ = db.delete_resource(&r.id);
        }
    }

    db.delete_project(&id).map_err(|e| e.to_string())?;

    let mut warnings = Vec::new();

    if delete_from_disk.unwrap_or(false) {
        // 1. Clean up ~/.claude/settings.json
        if let Err(e) = clean_claude_settings(&project.path) {
            warnings.push(format!("Failed to clean ~/.claude/settings.json: {}", e));
        }

        // 2. Clean up ~/.claude/projects/ cache
        if let Err(e) = clean_claude_projects_cache(&project.path) {
            warnings.push(format!("Failed to clean ~/.claude/projects/ cache: {}", e));
        }

        // 3. Delete project directory
        let project_dir = Path::new(&project.path);
        if project_dir.is_dir() {
            if let Err(e) = fs::remove_dir_all(project_dir) {
                warnings.push(format!("Failed to delete project directory {}: {}", project.path, e));
            }
        }
    }

    Ok(RemoveResult { warnings })
}

/// Remove project entries from ~/.claude/settings.json (projectSettings key).
fn clean_claude_settings(project_path: &str) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let settings_path = home.join(".claude").join("settings.json");

    if !settings_path.is_file() {
        return Ok(());
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Read failed: {}", e))?;

    let mut root: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Parse failed: {}", e))?;

    let mut modified = false;

    // Remove from projectSettings
    if let Some(ps) = root.get_mut("projectSettings").and_then(|v| v.as_object_mut()) {
        if ps.remove(project_path).is_some() {
            modified = true;
        }
    }

    if modified {
        let out = serde_json::to_string_pretty(&root)
            .map_err(|e| format!("Serialize failed: {}", e))?;
        fs::write(&settings_path, out)
            .map_err(|e| format!("Write failed: {}", e))?;
    }

    Ok(())
}

/// Remove matching cache directories from ~/.claude/projects/.
fn clean_claude_projects_cache(project_path: &str) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let projects_dir = home.join(".claude").join("projects");

    if !projects_dir.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(&projects_dir)
        .map_err(|e| format!("Read dir failed: {}", e))?;

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        // Use the same logic as scanner to resolve the project path
        if let Some(resolved) = scanner::project::resolve_project_path(&entry_path) {
            if resolved == project_path {
                let _ = fs::remove_dir_all(&entry_path);
            }
        }
    }

    Ok(())
}

/// Rescan a project's .claude/ directory and reconcile with DB.
/// Adds new resources found on disk, removes DB records for resources no longer on disk.
/// Returns the count of added and removed resources.
#[tauri::command]
pub fn rescan_project(
    db: State<Database>,
    adapter_registry: State<'_, AdapterRegistry>,
    project_id: String,
) -> Result<RescanResult, String> {
    let project = db.get_project(&project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    // 1. Scan filesystem using adapters
    let scanned = scanner::scan_claude_dir_v3(
        Path::new(&project.path),
        &ResourceScope::Project,
        &adapter_registry,
    );

    // Build a set of source_paths found on disk
    let disk_paths: std::collections::HashSet<String> = scanned.iter()
        .map(|r| r.source_path.clone())
        .collect();

    // 2. Get existing DB records for this project (across all relevant scopes)
    let mut all_existing = Vec::new();
    for scope in &[ResourceScope::Project, ResourceScope::Registry, ResourceScope::Library] {
        if let Ok(resources) = db.list_resources_by_scope(scope) {
            all_existing.extend(resources);
        }
    }
    let project_existing: Vec<&Resource> = all_existing.iter()
        .filter(|r| r.source_path.starts_with(&project.path))
        .collect();

    let existing_paths: std::collections::HashSet<String> = project_existing.iter()
        .map(|r| r.source_path.clone())
        .collect();

    // 3. Add resources found on disk but missing from DB
    let now = chrono::Utc::now().to_rfc3339();
    let mut added = 0;
    for resource in scanned {
        if !existing_paths.contains(&resource.source_path) {
            let new_resource = Resource {
                id: uuid::Uuid::new_v4().to_string(),
                resource_type: resource.resource_type,
                name: resource.name,
                description: resource.description,
                scope: resource.scope,  // Preserve detected scope (registry/library for symlinks)
                source_path: resource.source_path,
                content_hash: resource.content_hash,
                metadata: resource.metadata,
                created_at: now.clone(),
                updated_at: now.clone(),
                version: None,
                is_draft: 1,
            installed_from_id: None,
            };
            if db.insert_resource(&new_resource).is_ok() {
                added += 1;
            }
        }
    }

    // 4. Remove DB records whose files no longer exist on disk
    let mut removed = 0;
    for existing_res in &project_existing {
        if !disk_paths.contains(&existing_res.source_path) {
            // Verify the file is truly gone (not just a different scope)
            let path = Path::new(&existing_res.source_path);
            if !path.exists() && path.symlink_metadata().is_err() {
                if db.delete_resource(&existing_res.id).is_ok() {
                    removed += 1;
                }
            }
        }
    }

    // 5. Update project's last_scanned timestamp
    let mut updated_project = project.clone();
    updated_project.last_scanned = Some(now);
    let _ = db.update_project(&updated_project);

    Ok(RescanResult { added, removed })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RescanResult {
    pub added: usize,
    pub removed: usize,
}

/// Discover projects from ~/.claude/projects/ that are not yet registered
#[tauri::command]
pub fn discover_claude_projects(db: State<Database>) -> Result<Vec<scanner::project::DiscoveredProject>, String> {
    let discovered = scanner::project::discover_from_claude_projects();
    let registered = db.list_projects().map_err(|e| e.to_string())?;
    let registered_paths: std::collections::HashSet<String> = registered.iter().map(|p| p.path.clone()).collect();

    Ok(discovered.into_iter()
        .filter(|d| !registered_paths.contains(&d.path))
        .collect())
}

/// Scan directories for projects not yet registered
#[tauri::command]
pub fn scan_and_discover_projects(
    db: State<Database>,
    adapter_registry: State<'_, AdapterRegistry>,
    directories: Vec<String>,
) -> Result<Vec<Project>, String> {
    let registered = db.list_projects().map_err(|e| e.to_string())?;
    let registered_paths: std::collections::HashSet<String> = registered.iter().map(|p| p.path.clone()).collect();

    let mut discovered = Vec::new();
    for dir in directories {
        let projects = scanner::project::scan_directory_v3(&dir, &adapter_registry);
        for project in projects {
            if !registered_paths.contains(&project.path) {
                discovered.push(project);
            }
        }
    }
    Ok(discovered)
}

/// List resources for a specific project
/// Returns both project-owned resources AND resources linked from library
#[tauri::command]
pub fn list_project_resources(
    db: State<Database>,
    project_id: String,
    resource_type: Option<String>,
) -> Result<Vec<Resource>, String> {
    let project = db.get_project(&project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    // 1. Get resources whose source_path is under this project
    // This includes scope=project (native), scope=registry (symlinked from registry),
    // and scope=library (symlinked from library)
    let rtype_filter = match &resource_type {
        Some(rt) => Some(ResourceType::from_str(rt)
            .ok_or_else(|| format!("Invalid resource type: {}", rt))?),
        None => None,
    };

    let mut results: Vec<Resource> = Vec::new();
    for scope in &[ResourceScope::Project, ResourceScope::Registry, ResourceScope::Library] {
        let resources = match &rtype_filter {
            Some(rtype) => db.list_resources_by_scope_and_type(scope, rtype)
                .map_err(|e| e.to_string())?,
            None => db.list_resources_by_scope(scope)
                .map_err(|e| e.to_string())?,
        };
        results.extend(
            resources.into_iter()
                .filter(|r| r.source_path.starts_with(&project.path))
        );
    }

    // 2. Get resources linked/installed from library to this project
    let links = db.list_links_by_project(&project_id)
        .map_err(|e| e.to_string())?;

    let mut seen_ids: std::collections::HashSet<String> = results.iter().map(|r| r.id.clone()).collect();
    // Track MCP server names to avoid duplicates (project-scoped vs linked library)
    let mut seen_mcp_names: std::collections::HashSet<String> = results.iter()
        .filter(|r| r.resource_type == ResourceType::McpServer)
        .map(|r| r.name.clone())
        .collect();

    for link in &links {
        if seen_ids.contains(&link.resource_id) {
            continue;
        }
        if let Ok(Some(resource)) = db.get_resource(&link.resource_id) {
            // Filter by resource_type if specified
            if let Some(ref rt) = resource_type {
                if resource.resource_type.as_str() != rt.as_str() {
                    continue;
                }
            }
            // Skip linked MCP resources if a project-scoped one with same name already exists
            if resource.resource_type == ResourceType::McpServer
                && seen_mcp_names.contains(&resource.name)
            {
                continue;
            }
            seen_ids.insert(resource.id.clone());
            if resource.resource_type == ResourceType::McpServer {
                seen_mcp_names.insert(resource.name.clone());
            }
            results.push(resource);
        }
    }

    Ok(results)
}

/// Create a resource in a project's .claude/ directory
#[tauri::command]
pub fn create_project_resource(
    db: State<Database>,
    adapter_registry: State<AdapterRegistry>,
    project_id: String,
    resource_type: String,
    name: String,
    content: String,
) -> Result<Resource, String> {
    let project = db.get_project(&project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let claude_dir = Path::new(&project.path).join(".claude");

    super::resource_ops::create_resource_at(
        &db,
        &adapter_registry,
        &resource_type,
        &name,
        None,
        &content,
        &claude_dir,
        ResourceScope::Project,
    )
}

/// Delete a project resource (file/symlink + DB record)
/// For resources whose source_path is in a project directory: deletes the file and DB record.
/// For linked resources (source_path outside project): only removes the DB record, not the source file.
#[tauri::command]
pub fn delete_project_resource(db: State<Database>, resource_id: String) -> Result<(), String> {
    let resource = db.get_resource(&resource_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    let path = Path::new(&resource.source_path);

    // Only delete the file if source_path looks like a project-local path
    // (contains .claude/ — avoids deleting registry/library source files)
    let is_project_local = resource.source_path.contains("/.claude/");
    let is_registry_source = resource.source_path.contains("/.claude-manager/registries/")
        || resource.source_path.contains("/.claude-manager/library/");

    // ConfigBased resources (MCP): remove entry from JSON config file
    if resource.resource_type == ResourceType::McpServer {
        let server_name = &resource.name;

        // Find the project this resource belongs to
        let project = db.list_projects()
            .unwrap_or_default()
            .into_iter()
            .find(|p| resource.source_path.starts_with(&p.path) || is_project_local);
        let project_id = project.as_ref().map(|p| p.id.clone());

        // Determine the config file to edit
        let config_file = if resource.source_path.ends_with(".mcp.json") || resource.source_path.ends_with(".claude.json") {
            Some(std::path::PathBuf::from(&resource.source_path))
        } else if let Some(ref proj) = project {
            Some(std::path::PathBuf::from(&proj.path).join(".mcp.json"))
        } else {
            None
        };

        // Remove entry from the config file
        if let Some(config_path) = config_file {
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(servers) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                            servers.remove(server_name);
                            let _ = fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap_or_default());
                        }
                    }
                }
            }
        }

        // Clean up resource_links: find links targeting this project for this MCP name.
        // Links are keyed by SOURCE resource_id (library), not the project resource,
        // so we search by project_id + config_key match.
        if let Some(ref pid) = project_id {
            let project_links = db.list_links_by_project(pid).unwrap_or_default();
            for link in project_links {
                if link.link_type == "config_merge"
                    && link.config_key.as_deref() == Some(&format!("mcpServers.{}", server_name))
                {
                    let _ = db.delete_link(&link.id);
                }
            }
        }

        return db.delete_resource(&resource_id).map_err(|e| e.to_string());
    }

    if is_project_local && !is_registry_source {
        if path.exists() || path.symlink_metadata().is_ok() {
            if path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                fs::remove_file(path).map_err(|e| e.to_string())?;
            } else if path.is_dir() {
                fs::remove_dir_all(path).map_err(|e| e.to_string())?;
            } else {
                fs::remove_file(path).map_err(|e| e.to_string())?;
            }
        }
    }

    db.delete_resource(&resource_id).map_err(|e| e.to_string())
}

/// Publish a project resource to the central library (copy + optional install back via symlink)
#[tauri::command]
pub fn publish_to_library(
    db: State<Database>,
    resource_id: String,
    replace_with_symlink: bool,
) -> Result<Resource, String> {
    let resource = db.get_resource(&resource_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let library_base = home.join(".claude-manager").join("library");

    let adapter_registry = AdapterRegistry::new();
    let type_dir = adapter_registry
        .get(&resource.resource_type)
        .map(|a| a.type_dir())
        .ok_or_else(|| format!("No adapter for {:?}", resource.resource_type))?;
    let target_dir = library_base.join(type_dir);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    // Resolve symlinks to get real source content (avoid self-referential copy)
    let source = Path::new(&resource.source_path);
    let real_source = fs::canonicalize(source)
        .map_err(|e| format!("Cannot resolve source path: {}", e))?;

    // Copy source to library
    let target = if real_source.is_dir() {
        let dest = target_dir.join(&resource.name);
        if dest.exists() {
            fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
        }
        copy_dir_recursive(&real_source, &dest)?;
        dest
    } else {
        let file_name = real_source.file_name().ok_or("Invalid source path")?;
        let dest = target_dir.join(file_name);
        fs::copy(&real_source, &dest).map_err(|e| e.to_string())?;
        dest
    };

    let target_path_str = target.to_string_lossy().to_string();
    let hash = scanner::compute_file_hash(&target_path_str);
    let now = chrono::Utc::now().to_rfc3339();

    // Check if a library resource with same name+type already exists — update instead of duplicate
    let existing_lib = db.list_resources_by_scope(&ResourceScope::Library)
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.name == resource.name && r.resource_type == resource.resource_type);

    let library_resource = if let Some(mut existing) = existing_lib {
        existing.content_hash = hash;
        existing.source_path = target_path_str;
        existing.updated_at = now.clone();
        db.update_resource(&existing).map_err(|e| e.to_string())?;
        existing
    } else {
        let new_resource = Resource {
            id: uuid::Uuid::new_v4().to_string(),
            resource_type: resource.resource_type.clone(),
            name: resource.name.clone(),
            description: resource.description.clone(),
            scope: ResourceScope::Library,
            source_path: target_path_str,
            content_hash: hash,
            metadata: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&new_resource).map_err(|e| e.to_string())?;
        new_resource
    };

    // Optionally replace original with symlink via the standard install flow
    if replace_with_symlink {
        // Determine install scope from the original resource
        let scope = if resource.scope == ResourceScope::Global {
            crate::install::InstallScope::Global
        } else {
            // Find the project this resource belongs to
            let project_id = db.list_projects()
                .unwrap_or_default()
                .into_iter()
                .find(|p| resource.source_path.starts_with(&p.path))
                .map(|p| p.id);
            match project_id {
                Some(pid) => {
                    let project = db.get_project(&pid).map_err(|e| e.to_string())?
                        .ok_or("Project not found")?;
                    crate::install::InstallScope::Project {
                        id: pid,
                        path: project.path,
                    }
                }
                None => return Ok(library_resource), // Can't determine project, skip symlink
            }
        };

        // Remove original (file or directory)
        let original = Path::new(&resource.source_path);
        if original.symlink_metadata().is_ok() {
            if original.is_dir() && !original.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                fs::remove_dir_all(original).map_err(|e| e.to_string())?;
            } else {
                fs::remove_file(original).map_err(|e| e.to_string())?;
            }
        }

        // Use standard install flow: library → installed/ → symlink
        let _ = crate::install_service::install(&db, &library_resource, scope, &adapter_registry)?;

        // Update the original resource record to reflect it's now installed from library
        let mut updated_original = db.get_resource(&resource_id)
            .map_err(|e| e.to_string())?
            .ok_or("Original resource disappeared")?;
        updated_original.installed_from_id = Some(library_resource.id.clone());
        updated_original.updated_at = chrono::Utc::now().to_rfc3339();
        db.update_resource(&updated_original).map_err(|e| e.to_string())?;
    }

    Ok(library_resource)
}

/// Install a library resource into a project
#[tauri::command]
pub fn install_from_library(
    db: State<Database>,
    adapter_registry: State<AdapterRegistry>,
    library_resource_id: String,
    project_id: String,
    link_type: String,
) -> Result<(), String> {
    let _ = link_type;

    let lib_resource = db.get_resource(&library_resource_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library resource not found: {}", library_resource_id))?;

    if lib_resource.scope != ResourceScope::Library {
        return Err("Resource is not a library resource".to_string());
    }

    let project = db.get_project(&project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let scope = crate::install::InstallScope::Project {
        id: project_id,
        path: project.path.clone(),
    };

    super::resource_ops::install_resource_to(&db, &lib_resource, scope, &adapter_registry)?;

    Ok(())
}

/// Get project permissions from .claude/settings.local.json
#[tauri::command]
pub fn get_project_permissions(db: State<Database>, project_id: String) -> Result<serde_json::Value, String> {
    let project = db.get_project(&project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let settings_path = Path::new(&project.path).join(".claude").join("settings.local.json");
    if !settings_path.is_file() {
        return Ok(serde_json::json!({ "allow": [], "deny": [] }));
    }

    let content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
    let data: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let permissions = data.get("permissions").cloned()
        .unwrap_or_else(|| serde_json::json!({ "allow": [], "deny": [] }));

    // Ensure both allow and deny arrays exist
    let allow = permissions.get("allow").cloned().unwrap_or_else(|| serde_json::json!([]));
    let deny = permissions.get("deny").cloned().unwrap_or_else(|| serde_json::json!([]));

    Ok(serde_json::json!({ "allow": allow, "deny": deny }))
}

#[tauri::command]
pub fn toggle_project_pin(db: State<'_, Database>, project_id: String) -> Result<Project, String> {
    db.toggle_pin(&project_id).map_err(|e| e.to_string())?.ok_or_else(|| format!("Project not found: {}", project_id))
}

#[tauri::command]
pub fn list_projects_ranked(db: State<'_, Database>) -> Result<Vec<Project>, String> {
    db.list_projects_ranked().map_err(|e| e.to_string())
}

/// Update project permissions in .claude/settings.local.json (preserves other fields)
#[tauri::command]
pub fn update_project_permissions(
    db: State<Database>,
    project_id: String,
    allow: Vec<String>,
    deny: Vec<String>,
) -> Result<(), String> {
    let project = db.get_project(&project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let settings_path = Path::new(&project.path).join(".claude").join("settings.local.json");
    let claude_dir = Path::new(&project.path).join(".claude");
    fs::create_dir_all(&claude_dir).map_err(|e| e.to_string())?;

    // Read existing file to preserve other fields
    let mut data: serde_json::Value = if settings_path.is_file() {
        let content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Update only the permissions field
    data["permissions"] = serde_json::json!({
        "allow": allow,
        "deny": deny,
    });

    let output = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    fs::write(&settings_path, output).map_err(|e| e.to_string())?;

    Ok(())
}

