use tauri::State;
use crate::adapters::{AdapterRegistry, LinkType, TargetScope, normalize_link_type};
use crate::adapters::file_based::copy_dir_recursive;
use crate::db::Database;
use crate::models::v2::{Resource, Project, ResourceType, ResourceScope, ResourceLink};
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

    // Scan and insert project-level MCP servers from .mcp.json
    let mut found_mcp_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mcp_path = Path::new(&path).join(".mcp.json");
    if mcp_path.is_file() {
        let mcp_servers = scanner::mcp::parse_mcp_file(
            mcp_path.to_str().unwrap_or_default(),
            Some(project.id.clone()),
        );
        for scanned in mcp_servers {
            found_mcp_names.insert(scanned.name.clone());
            let server = crate::models::v2::McpServer {
                id: uuid::Uuid::new_v4().to_string(),
                name: scanned.name,
                project_id: Some(project.id.clone()),
                server_type: scanned.server_type,
                command: scanned.command,
                args: scanned.args,
                url: scanned.url,
                env: scanned.env,
                source_path: scanned.source_path,
                registry_plugin_id: None,
            };
            let _ = db.insert_mcp_server(&server);
        }
    }

    // Also parse .claude/settings.local.json for enabledMcpjsonServers
    let settings_path = Path::new(&path).join(".claude").join("settings.local.json");
    if settings_path.is_file() {
        let enabled_names = scanner::mcp::parse_enabled_mcp_from_settings(
            settings_path.to_str().unwrap_or_default(),
        );
        for name in enabled_names {
            if found_mcp_names.contains(&name) {
                continue;
            }
            let server = crate::models::v2::McpServer {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                project_id: Some(project.id.clone()),
                server_type: None,
                command: None,
                args: None,
                url: None,
                env: None,
                source_path: settings_path.to_string_lossy().to_string(),
                registry_plugin_id: None,
            };
            let _ = db.insert_mcp_server(&server);
        }
    }

    Ok(project)
}

/// List global MCP servers (project_id IS NULL)
#[tauri::command]
pub fn list_global_mcp_servers(db: State<Database>) -> Result<Vec<crate::models::v2::McpServer>, String> {
    db.list_global_mcp_servers().map_err(|e| e.to_string())
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

    // Delete project MCP servers from DB
    let mcp_servers = db.list_mcp_servers_by_project(&id).map_err(|e| e.to_string())?;
    for s in mcp_servers {
        let _ = db.delete_mcp_server(&s.id);
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

/// List MCP servers for a specific project
#[tauri::command]
pub fn list_project_mcp_servers(db: State<Database>, project_id: String) -> Result<Vec<crate::models::v2::McpServer>, String> {
    db.list_mcp_servers_by_project(&project_id).map_err(|e| e.to_string())
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
pub fn scan_and_discover_projects(db: State<Database>, directories: Vec<String>) -> Result<Vec<Project>, String> {
    let registered = db.list_projects().map_err(|e| e.to_string())?;
    let registered_paths: std::collections::HashSet<String> = registered.iter().map(|p| p.path.clone()).collect();

    let mut discovered = Vec::new();
    for dir in directories {
        let results = scanner::project::scan_directory_v2(&dir);
        for (project, _resources) in results {
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
            seen_ids.insert(resource.id.clone());
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

    let rtype = ResourceType::from_str(&resource_type)
        .ok_or_else(|| format!("Invalid resource type: {}", resource_type))?;

    // Validate content via adapter
    let adapter = adapter_registry.get(&rtype)
        .ok_or_else(|| format!("No adapter for resource type: {}", resource_type))?;
    adapter.validate_content(&content)?;

    let claude_dir = Path::new(&project.path).join(".claude");

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

    fs::write(&file_path, &content).map_err(|e| e.to_string())?;

    let source_path = match rtype {
        ResourceType::Skill => file_path.parent().unwrap().to_string_lossy().to_string(),
        _ => file_path.to_string_lossy().to_string(),
    };

    let hash = scanner::compute_file_hash(&source_path);
    let now = chrono::Utc::now().to_rfc3339();

    let resource = Resource {
        id: uuid::Uuid::new_v4().to_string(),
        resource_type: rtype,
        name,
        description: None,
        scope: ResourceScope::Project,
        source_path,
        content_hash: hash,
        metadata: None,
        created_at: now.clone(),
        updated_at: now,
    };

    db.insert_resource(&resource).map_err(|e| e.to_string())?;
    Ok(resource)
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
    // For non-local resources (from registry/library), just remove the DB record

    db.delete_resource(&resource_id).map_err(|e| e.to_string())
}

/// Publish a project resource to the central library (copy + optional symlink)
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
        let dest = target_dir.join(&resource.name);
        copy_dir_recursive(source, &dest)?;
        dest
    } else {
        let file_name = source.file_name().ok_or("Invalid source path")?;
        let dest = target_dir.join(file_name);
        fs::copy(source, &dest).map_err(|e| e.to_string())?;
        dest
    };

    let target_path_str = target.to_string_lossy().to_string();
    let hash = scanner::compute_file_hash(&target_path_str);
    let now = chrono::Utc::now().to_rfc3339();

    let library_resource = Resource {
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
    };

    db.insert_resource(&library_resource).map_err(|e| e.to_string())?;

    // Optionally replace original with symlink
    if replace_with_symlink {
        let original = Path::new(&resource.source_path);
        if original.is_dir() {
            fs::remove_dir_all(original).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(original).map_err(|e| e.to_string())?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, original).map_err(|e| e.to_string())?;

        // Record the link
        let link = ResourceLink {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: library_resource.id.clone(),
            target_scope: "project".to_string(),
            target_path: resource.source_path.clone(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: now,
        };
        db.insert_link(&link).map_err(|e| e.to_string())?;
    }

    Ok(library_resource)
}

/// Install a library resource into a project via adapter
#[tauri::command]
pub fn install_from_library(
    db: State<Database>,
    adapter_registry: State<'_, AdapterRegistry>,
    library_resource_id: String,
    project_id: String,
    link_type: String,
) -> Result<(), String> {
    let lib_resource = db.get_resource(&library_resource_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library resource not found: {}", library_resource_id))?;

    if lib_resource.scope != ResourceScope::Library {
        return Err("Resource is not a library resource".to_string());
    }

    let project = db.get_project(&project_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let adapter = adapter_registry.get(&lib_resource.resource_type)
        .ok_or_else(|| "No adapter for resource type".to_string())?;

    let requested_link_type = LinkType::from_str(&link_type)
        .ok_or_else(|| "Invalid link type".to_string())?;
    let effective_link_type = normalize_link_type(adapter, requested_link_type);

    let target = adapter.resolve_target(
        &TargetScope::Project,
        &lib_resource.name,
        Some(&project),
    )?;

    let mut link = adapter.install(&lib_resource, &target, &effective_link_type)?;
    link.target_scope = "project".to_string();
    link.project_id = Some(project_id.clone());

    db.insert_link(&link).map_err(|e| e.to_string())?;

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

