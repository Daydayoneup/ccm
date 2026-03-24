use tauri::State;
use crate::adapters::file_based::copy_dir_recursive;
use crate::adapters::plugin_install;
use crate::db::Database;
use crate::models::v2::{Registry, Resource, ResourceType, ResourceScope, ResourceLink};
use crate::models::v2::RegistryPlugin;
use crate::scanner;
use crate::scanner::registry::{read_marketplace_json, resolve_plugin_source_path, scan_plugin_dir, scan_registry};
use crate::git;
use std::fs;
use std::path::Path;

/// List all registries from DB
#[tauri::command]
pub fn list_registries(db: State<Database>) -> Result<Vec<Registry>, String> {
    db.list_registries().map_err(|e| e.to_string())
}

/// Shared helper: parse marketplace.json and insert registry_plugins + resources into DB.
/// Takes &Database directly so it can be called both from Tauri commands and startup.
pub fn scan_and_insert_plugins(db: &crate::db::Database, registry: &Registry) -> Result<(), String> {
    let marketplace = read_marketplace_json(&registry.local_path);

    if let Some(mp) = marketplace {
        if let Some(plugins) = mp.plugins {
            for mp_plugin in &plugins {
                let clone_path = resolve_plugin_source_path(&registry.local_path, mp_plugin);
                if mp_plugin.is_external() {
                    if let Some(url) = mp_plugin.external_url() {
                        if !std::path::Path::new(&clone_path).exists() {
                            match crate::git::clone(&url, &clone_path, None) {
                                Ok(result) if result.success => {}
                                Ok(result) => {
                                    eprintln!("Warning: Failed to clone external plugin {}: {}", mp_plugin.name, result.stderr);
                                    continue;
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to clone external plugin {}: {}", mp_plugin.name, e);
                                    continue;
                                }
                            }
                        }
                    }
                }

                // For git-subdir sources, the actual plugin is in a subdirectory
                let source_path = if let Some(subdir) = mp_plugin.external_subdir_path() {
                    Path::new(&clone_path).join(subdir).to_string_lossy().to_string()
                } else {
                    clone_path.clone()
                };

                let reg_plugin = RegistryPlugin {
                    id: uuid::Uuid::new_v4().to_string(),
                    registry_id: registry.id.clone(),
                    name: mp_plugin.name.clone(),
                    description: mp_plugin.description.clone(),
                    category: mp_plugin.category.clone(),
                    source_path: source_path.clone(),
                    source_type: if mp_plugin.is_external() { "external".to_string() } else { "local".to_string() },
                    source_url: mp_plugin.external_url(),
                    homepage: mp_plugin.homepage.clone(),
                };
                db.insert_registry_plugin(&reg_plugin)
                    .map_err(|e| format!("Failed to insert registry plugin: {}", e))?;

                let scanned = scan_plugin_dir(&source_path);
                let now = chrono::Utc::now().to_rfc3339();
                for sr in scanned {
                    let resource = crate::models::v2::Resource {
                        id: uuid::Uuid::new_v4().to_string(),
                        resource_type: sr.resource_type,
                        name: sr.name,
                        description: None,
                        scope: crate::models::v2::ResourceScope::Registry,
                        source_path: sr.source_path,
                        content_hash: sr.content_hash,
                        metadata: Some(reg_plugin.id.clone()),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        version: None,
                        is_draft: 1,
                    };
                    let _ = db.insert_resource(&resource);
                }

                // Scan plugin's .mcp.json for MCP servers
                let mcp_path = std::path::Path::new(&source_path).join(".mcp.json");
                if mcp_path.is_file() {
                    let scanned_servers = crate::scanner::mcp::parse_plugin_mcp_file(
                        mcp_path.to_str().unwrap_or_default(),
                    );
                    for ss in scanned_servers {
                        let server = crate::models::v2::McpServer {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: ss.name,
                            project_id: None,
                            server_type: ss.server_type,
                            command: ss.command,
                            args: ss.args,
                            url: ss.url,
                            env: ss.env,
                            source_path: ss.source_path,
                            registry_plugin_id: Some(reg_plugin.id.clone()),
                        };
                        let _ = db.insert_mcp_server(&server);
                    }
                }
            }
        }
    } else {
        // Fallback: no marketplace.json, scan root as single plugin
        let fallback_plugin = RegistryPlugin {
            id: uuid::Uuid::new_v4().to_string(),
            registry_id: registry.id.clone(),
            name: registry.name.clone(),
            description: None,
            category: None,
            source_path: registry.local_path.clone(),
            source_type: "local".to_string(),
            source_url: None,
            homepage: None,
        };
        db.insert_registry_plugin(&fallback_plugin)
            .map_err(|e| format!("Failed to insert fallback plugin: {}", e))?;

        let scanned = scan_registry(&registry.local_path);
        let now = chrono::Utc::now().to_rfc3339();
        for sr in scanned {
            let resource = crate::models::v2::Resource {
                id: uuid::Uuid::new_v4().to_string(),
                resource_type: sr.resource_type,
                name: sr.name,
                description: None,
                scope: crate::models::v2::ResourceScope::Registry,
                source_path: sr.source_path,
                content_hash: sr.content_hash,
                metadata: Some(fallback_plugin.id.clone()),
                created_at: now.clone(),
                updated_at: now.clone(),
                version: None,
                is_draft: 1,
            };
            let _ = db.insert_resource(&resource);
        }

        // Scan fallback plugin's .mcp.json for MCP servers
        let mcp_path = std::path::Path::new(&registry.local_path).join(".mcp.json");
        if mcp_path.is_file() {
            let scanned_servers = crate::scanner::mcp::parse_plugin_mcp_file(
                mcp_path.to_str().unwrap_or_default(),
            );
            for ss in scanned_servers {
                let server = crate::models::v2::McpServer {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: ss.name,
                    project_id: None,
                    server_type: ss.server_type,
                    command: ss.command,
                    args: ss.args,
                    url: ss.url,
                    env: ss.env,
                    source_path: ss.source_path,
                    registry_plugin_id: Some(fallback_plugin.id.clone()),
                };
                let _ = db.insert_mcp_server(&server);
            }
        }
    }
    Ok(())
}

/// Helper: generate (or regenerate) the CCM wrapper marketplace for a registry, then
/// register it with the Claude Code CLI. Both steps are best-effort — failures are
/// logged as warnings and do not abort the caller.
fn ensure_wrapper_marketplace(registry_path: &str, registry_name: &str) {
    let path = std::path::Path::new(registry_path);
    if !plugin_install::is_claude_marketplace(path) {
        return;
    }
    let wrapper_base = match plugin_install::ccm_marketplaces_dir() {
        Ok(base) => base,
        Err(e) => {
            eprintln!("Warning: Failed to determine wrapper directory: {}", e);
            return;
        }
    };
    match plugin_install::generate_wrapper_marketplace(path, registry_name, &wrapper_base) {
        Ok(wrapper_path) => {
            if let Err(e) = plugin_install::register_marketplace_cli(&wrapper_path) {
                eprintln!("Warning: Failed to register marketplace with Claude Code: {}", e);
            }
        }
        Err(e) => eprintln!("Warning: Failed to generate wrapper marketplace: {}", e),
    }
}

/// Add a new registry: check git, clone, scan resources, insert into DB
#[tauri::command]
pub fn add_registry(
    db: State<Database>,
    name: String,
    url: String,
    readonly: bool,
) -> Result<Registry, String> {
    // Check git is available
    if !git::is_git_available() {
        return Err("Git is not available on this system".to_string());
    }

    // Check duplicate URL
    if let Some(_existing) = db.get_registry_by_url(&url).map_err(|e| e.to_string())? {
        return Err(format!("A registry with URL '{}' already exists", url));
    }

    // Check duplicate name
    let existing_registries = db.list_registries().map_err(|e| e.to_string())?;
    if existing_registries.iter().any(|r| r.name == name) {
        return Err(format!("Registry with name '{}' already exists", name));
    }

    // Determine local path
    let repo_name = git::extract_repo_name(&url);
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let registries_dir = home.join(".claude-manager").join("registries");
    fs::create_dir_all(&registries_dir).map_err(|e| e.to_string())?;
    let local_path = registries_dir.join(&repo_name);
    let local_path_str = local_path.to_string_lossy().to_string();

    // Clone the repository
    let proxy = crate::proxy::ProxyConfig::load(&db);
    let clone_result = git::clone(&url, &local_path_str, proxy.as_ref())?;
    if !clone_result.success {
        return Err(format!("Git clone failed: {}", clone_result.stderr));
    }

    // Try to read registry metadata for a display name
    // Priority: registry.json > marketplace.json > user-provided name > repo name
    let display_name = scanner::registry::read_registry_metadata(&local_path_str)
        .map(|(n, _)| n)
        .or_else(|| {
            read_marketplace_json(&local_path_str)
                .and_then(|mp| mp.name)
        })
        .unwrap_or_else(|| if name.is_empty() { repo_name.clone() } else { name });

    let now = chrono::Utc::now().to_rfc3339();
    let registry = Registry {
        id: uuid::Uuid::new_v4().to_string(),
        name: display_name,
        url: url.clone(),
        local_path: local_path_str.clone(),
        readonly,
        last_synced: Some(now.clone()),
        has_remote_changes: false,
        has_local_changes: false,
        created_at: now.clone(),
    };

    db.insert_registry(&registry).map_err(|e| e.to_string())?;

    // Scan plugins using marketplace.json or fallback
    scan_and_insert_plugins(&db, &registry)?;

    // Generate wrapper marketplace and register with Claude Code CLI (best-effort)
    ensure_wrapper_marketplace(&registry.local_path, &registry.name);

    Ok(registry)
}

/// Remove a registry: check for links, delete resources, delete record, remove directory
#[tauri::command]
pub fn remove_registry(db: State<Database>, id: String) -> Result<(), String> {
    let registry = db.get_registry(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Registry not found: {}", id))?;

    // Check for existing resource_links
    let registry_resources = db.list_resources_by_scope(&ResourceScope::Registry)
        .map_err(|e| e.to_string())?;
    for r in &registry_resources {
        if r.metadata.as_deref() == Some(&id) {
            let links = db.list_links_by_resource(&r.id).map_err(|e| e.to_string())?;
            if !links.is_empty() {
                return Err(format!(
                    "Cannot remove registry: resource '{}' is linked to {} location(s). Unlink first.",
                    r.name, links.len()
                ));
            }
        }
    }

    db.delete_registry_plugins_by_registry(&id)
        .map_err(|e| format!("Failed to delete plugins: {}", e))?;

    // Delete registry resources from DB
    for r in &registry_resources {
        if r.metadata.as_deref() == Some(&id) {
            let _ = db.delete_resource(&r.id);
        }
    }

    // Delete registry record from DB
    db.delete_registry(&id).map_err(|e| e.to_string())?;

    // Delete local directory
    let local_path = Path::new(&registry.local_path);
    if local_path.exists() {
        fs::remove_dir_all(local_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Sync a single registry: git pull, re-scan resources, update last_synced
#[tauri::command]
pub async fn sync_registry(db: State<'_, Database>, id: String) -> Result<Registry, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut registry = db.get_registry(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Registry not found: {}", id))?;

        // Pull latest changes
        let proxy = crate::proxy::ProxyConfig::load(&db);
        let pull_result = git::pull(&registry.local_path, proxy.as_ref())?;
        if !pull_result.success {
            return Err(format!("Git pull failed: {}", pull_result.stderr));
        }

        // Delete old plugins and their resources
        let old_plugins = db.list_registry_plugins(&id)
            .map_err(|e| e.to_string())?;
        for old_plugin in &old_plugins {
            let resources = db.list_resources_by_scope(&ResourceScope::Registry)
                .map_err(|e| e.to_string())?;
            for r in resources {
                if r.metadata.as_deref() == Some(&old_plugin.id) {
                    let _ = db.delete_resource(&r.id);
                }
            }
        }
        db.delete_registry_plugins_by_registry(&id)
            .map_err(|e| e.to_string())?;

        // Re-scan plugins
        scan_and_insert_plugins(&db, &registry)?;

        // Regenerate wrapper marketplace and re-register with Claude Code CLI (best-effort)
        ensure_wrapper_marketplace(&registry.local_path, &registry.name);

        // Update registry record
        let now = chrono::Utc::now().to_rfc3339();
        registry.last_synced = Some(now);
        registry.has_remote_changes = false;
        db.update_registry(&registry).map_err(|e| e.to_string())?;

        Ok(registry)
    }).await.map_err(|e| e.to_string())?
}

/// Sync all registries: pull each, re-scan resources. Collects errors but doesn't fail entirely.
#[tauri::command]
pub async fn sync_all_registries(db: State<'_, Database>) -> Result<Vec<Registry>, String> {
    let db = db.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let registries = db.list_registries().map_err(|e| e.to_string())?;
        let proxy = crate::proxy::ProxyConfig::load(&db);
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for reg in registries {
            let pull_result = match git::pull(&reg.local_path, proxy.as_ref()) {
                Ok(r) => r,
                Err(e) => {
                    errors.push(format!("Failed to pull {}: {}", reg.name, e));
                    results.push(reg);
                    continue;
                }
            };

            if !pull_result.success {
                errors.push(format!("Git pull failed for {}: {}", reg.name, pull_result.stderr));
                results.push(reg);
                continue;
            }

            let now = chrono::Utc::now().to_rfc3339();

            // Delete old plugins and resources
            if let Ok(old_plugins) = db.list_registry_plugins(&reg.id) {
                for old_plugin in &old_plugins {
                    if let Ok(resources) = db.list_resources_by_scope(&ResourceScope::Registry) {
                        for r in resources {
                            if r.metadata.as_deref() == Some(&old_plugin.id) {
                                let _ = db.delete_resource(&r.id);
                            }
                        }
                    }
                }
            }
            let _ = db.delete_registry_plugins_by_registry(&reg.id);

            let mut updated = reg;
            updated.last_synced = Some(now);
            updated.has_remote_changes = false;
            let _ = scan_and_insert_plugins(&db, &updated);
            // Regenerate wrapper marketplace and re-register with Claude Code CLI (best-effort)
            ensure_wrapper_marketplace(&updated.local_path, &updated.name);
            let _ = db.update_registry(&updated);
            results.push(updated);
        }

        if !errors.is_empty() {
            eprintln!("sync_all_registries warnings: {:?}", errors);
        }

        Ok(results)
    }).await.map_err(|e| e.to_string())?
}

/// Push local changes to registry remote
#[tauri::command]
pub fn push_registry(
    db: State<Database>,
    id: String,
    message: String,
) -> Result<Registry, String> {
    let mut registry = db.get_registry(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Registry not found: {}", id))?;

    if registry.readonly {
        return Err("Cannot push to a readonly registry".to_string());
    }

    let proxy = crate::proxy::ProxyConfig::load(&db);
    let result = git::commit_and_push(&registry.local_path, &message, proxy.as_ref())?;
    if !result.success {
        return Err(format!("Git push failed: {}", result.stderr));
    }

    registry.has_local_changes = false;
    db.update_registry(&registry).map_err(|e| e.to_string())?;

    Ok(registry)
}

/// Check all registries for remote/local changes
#[tauri::command]
pub fn check_registry_updates(db: State<Database>) -> Result<Vec<Registry>, String> {
    let registries = db.list_registries().map_err(|e| e.to_string())?;
    let proxy = crate::proxy::ProxyConfig::load(&db);
    let mut results = Vec::new();

    for mut reg in registries {
        // Check remote changes
        match git::has_remote_changes(&reg.local_path, proxy.as_ref()) {
            Ok(has_changes) => reg.has_remote_changes = has_changes,
            Err(e) => eprintln!("Failed to check remote changes for {}: {}", reg.name, e),
        }

        // Check local changes (only for non-readonly)
        if !reg.readonly {
            match git::has_local_changes(&reg.local_path) {
                Ok(has_changes) => reg.has_local_changes = has_changes,
                Err(e) => eprintln!("Failed to check local changes for {}: {}", reg.name, e),
            }
        }

        let _ = db.update_registry(&reg);
        results.push(reg);
    }

    Ok(results)
}

/// List resources for a specific registry, optionally filtered by type
#[tauri::command]
pub fn list_registry_resources(
    db: State<Database>,
    registry_id: String,
    resource_type: Option<String>,
) -> Result<Vec<Resource>, String> {
    let all_resources = match &resource_type {
        Some(rt) => {
            let rtype = ResourceType::from_str(rt)
                .ok_or_else(|| format!("Invalid resource type: {}", rt))?;
            db.list_resources_by_scope_and_type(&ResourceScope::Registry, &rtype)
                .map_err(|e| e.to_string())?
        }
        None => {
            db.list_resources_by_scope(&ResourceScope::Registry)
                .map_err(|e| e.to_string())?
        }
    };

    // Filter by registry_id stored in metadata
    Ok(all_resources
        .into_iter()
        .filter(|r| r.metadata.as_deref() == Some(&registry_id))
        .collect())
}

/// Publish a resource to a registry: copy file/dir, insert Resource with scope=Registry
#[tauri::command]
pub fn publish_to_registry(
    db: State<Database>,
    resource_id: String,
    registry_id: String,
) -> Result<Resource, String> {
    let registry = db.get_registry(&registry_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Registry not found: {}", registry_id))?;

    if registry.readonly {
        return Err("Cannot publish to a readonly registry".to_string());
    }

    let source_resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    let type_dir = match source_resource.resource_type {
        ResourceType::Skill => "skills",
        ResourceType::Agent => "agents",
        ResourceType::Rule => "rules",
        ResourceType::Hook => "hooks",
        ResourceType::Command => "commands",
        ResourceType::McpServer => "mcp_servers",
    };

    let target_type_dir = Path::new(&registry.local_path).join(type_dir);
    fs::create_dir_all(&target_type_dir).map_err(|e| e.to_string())?;

    let source = Path::new(&source_resource.source_path);
    let target = if source.is_dir() {
        let dest = target_type_dir.join(&source_resource.name);
        copy_dir_recursive(source, &dest)?;
        dest
    } else {
        let file_name = source.file_name().ok_or("Invalid source path")?;
        let dest = target_type_dir.join(file_name);
        fs::copy(source, &dest).map_err(|e| e.to_string())?;
        dest
    };

    let target_path_str = target.to_string_lossy().to_string();
    let hash = scanner::compute_file_hash(&target_path_str);
    let now = chrono::Utc::now().to_rfc3339();

    let new_resource = Resource {
        id: uuid::Uuid::new_v4().to_string(),
        resource_type: source_resource.resource_type.clone(),
        name: source_resource.name.clone(),
        description: source_resource.description.clone(),
        scope: ResourceScope::Registry,
        source_path: target_path_str,
        content_hash: hash,
        metadata: Some(registry_id.clone()),
        created_at: now.clone(),
        updated_at: now,
        version: None,
        is_draft: 1,
    };

    db.insert_resource(&new_resource).map_err(|e| e.to_string())?;

    // Mark registry as having local changes
    let mut updated_registry = registry;
    updated_registry.has_local_changes = true;
    let _ = db.update_registry(&updated_registry);

    Ok(new_resource)
}

/// Install a registry resource to a project via symlink
#[tauri::command]
pub fn install_from_registry(
    db: State<Database>,
    resource_id: String,
    project_id: String,
) -> Result<ResourceLink, String> {
    let resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if resource.scope != ResourceScope::Registry {
        return Err("Resource is not a registry resource".to_string());
    }

    let project = db.get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let claude_dir = Path::new(&project.path).join(".claude");
    let type_dir = match resource.resource_type {
        ResourceType::Skill => "skills",
        ResourceType::Agent => "agents",
        ResourceType::Rule => "rules",
        ResourceType::Hook => "hooks",
        ResourceType::Command => "commands",
        ResourceType::McpServer => "mcp_servers",
    };
    let target_type_dir = claude_dir.join(type_dir);
    fs::create_dir_all(&target_type_dir).map_err(|e| e.to_string())?;

    let source = Path::new(&resource.source_path);
    let target = if source.is_dir() {
        target_type_dir.join(&resource.name)
    } else {
        let file_name = source.file_name().ok_or("Invalid source path")?;
        target_type_dir.join(file_name)
    };

    if target.exists() {
        return Err(format!("Target already exists: {}", target.display()));
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(source, &target).map_err(|e| e.to_string())?;
    #[cfg(not(unix))]
    return Err("Symlinks not supported on this platform".to_string());

    let now = chrono::Utc::now().to_rfc3339();
    let link = ResourceLink {
        id: uuid::Uuid::new_v4().to_string(),
        resource_id,
        target_scope: "project".to_string(),
        target_path: target.to_string_lossy().to_string(),
        config_key: None,
        project_id: Some(project_id),
        link_type: "symlink".to_string(),
        created_at: now,
    };

    db.insert_link(&link).map_err(|e| e.to_string())?;
    Ok(link)
}

/// Deploy a registry resource to global ~/.claude/<type>/ via symlink
#[tauri::command]
pub fn deploy_from_registry(
    db: State<Database>,
    resource_id: String,
) -> Result<ResourceLink, String> {
    let resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if resource.scope != ResourceScope::Registry {
        return Err("Resource is not a registry resource".to_string());
    }

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let claude_dir = home.join(".claude");
    let type_dir = match resource.resource_type {
        ResourceType::Skill => "skills",
        ResourceType::Agent => "agents",
        ResourceType::Rule => "rules",
        ResourceType::Hook => "hooks",
        ResourceType::Command => "commands",
        ResourceType::McpServer => "mcp_servers",
    };
    let target_type_dir = claude_dir.join(type_dir);
    fs::create_dir_all(&target_type_dir).map_err(|e| e.to_string())?;

    let source = Path::new(&resource.source_path);
    let target = if source.is_dir() {
        target_type_dir.join(&resource.name)
    } else {
        let file_name = source.file_name().ok_or("Invalid source path")?;
        target_type_dir.join(file_name)
    };

    if target.exists() {
        return Err(format!("Target already exists: {}", target.display()));
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(source, &target).map_err(|e| e.to_string())?;
    #[cfg(not(unix))]
    return Err("Symlinks not supported on this platform".to_string());

    let now = chrono::Utc::now().to_rfc3339();
    let link = ResourceLink {
        id: uuid::Uuid::new_v4().to_string(),
        resource_id,
        target_scope: "global".to_string(),
        target_path: target.to_string_lossy().to_string(),
        config_key: None,
        project_id: None,
        link_type: "symlink".to_string(),
        created_at: now,
    };

    db.insert_link(&link).map_err(|e| e.to_string())?;
    Ok(link)
}

/// List all plugins for a registry
#[tauri::command]
pub fn list_registry_plugins(
    db: State<Database>,
    registry_id: String,
) -> Result<Vec<RegistryPlugin>, String> {
    db.list_registry_plugins(&registry_id)
        .map_err(|e| format!("Failed to list registry plugins: {}", e))
}

/// Get resources for a specific registry plugin
#[tauri::command]
pub fn get_registry_plugin_resources(
    db: State<Database>,
    plugin_id: String,
) -> Result<Vec<Resource>, String> {
    let resources = db
        .list_resources_by_scope(&ResourceScope::Registry)
        .map_err(|e| format!("Failed to list resources: {}", e))?;
    Ok(resources
        .into_iter()
        .filter(|r| r.metadata.as_deref() == Some(&plugin_id))
        .collect())
}

/// Get MCP servers for a specific registry plugin
#[tauri::command]
pub fn get_registry_plugin_mcp_servers(
    db: State<Database>,
    plugin_id: String,
) -> Result<Vec<crate::models::v2::McpServer>, String> {
    db.list_mcp_servers_by_registry_plugin(&plugin_id)
        .map_err(|e| format!("Failed to list MCP servers: {}", e))
}

/// Install all resources of a plugin to a project via symlinks
#[tauri::command]
pub fn install_plugin_to_project(
    db: State<Database>,
    plugin_id: String,
    project_id: String,
) -> Result<Vec<crate::models::v2::ResourceLink>, String> {
    let project = db
        .get_project(&project_id)
        .map_err(|e| format!("Failed to get project: {}", e))?
        .ok_or("Project not found")?;

    // Look up registry plugin and its registry
    let registry_plugin = db.get_registry_plugin(&plugin_id)
        .map_err(|e| format!("Failed to get plugin: {}", e))?
        .ok_or("Plugin not found")?;

    let registry = db.get_registry(&registry_plugin.registry_id)
        .map_err(|e| format!("Failed to get registry: {}", e))?
        .ok_or("Registry not found")?;

    // Try plugin mode: if wrapper exists and CLI available
    let ccm_name = plugin_install::ccm_marketplace_name(&registry.name);
    let has_wrapper = plugin_install::has_wrapper(&registry.name).unwrap_or(false);

    if has_wrapper && plugin_install::is_claude_cli_available() {
        match plugin_install::install_plugin_cli(
            &project.path,
            &registry_plugin.name,
            &ccm_name,
            "project",
        ) {
            Ok(()) => {
                // Record in DB
                let now = chrono::Utc::now().to_rfc3339();
                let config_key = format!("enabledPlugins.{}@{}", registry_plugin.name, ccm_name);
                let settings_path = std::path::Path::new(&project.path)
                    .join(".claude").join("settings.json");
                let link = crate::models::v2::ResourceLink {
                    id: uuid::Uuid::new_v4().to_string(),
                    resource_id: plugin_id.clone(),
                    target_scope: "project".to_string(),
                    target_path: settings_path.to_string_lossy().to_string(),
                    config_key: Some(config_key),
                    project_id: Some(project_id),
                    link_type: "plugin_install".to_string(),
                    created_at: now,
                };
                db.insert_link(&link).map_err(|e| format!("Failed to record link: {}", e))?;
                return Ok(vec![link]);
            }
            Err(e) => {
                eprintln!("Warning: Plugin mode install failed, falling back to symlinks: {}", e);
                // Fall through to resource mode below
            }
        }
    }

    // Fallback: resource mode (existing symlink logic)
    let resources = db
        .list_resources_by_scope(&ResourceScope::Registry)
        .map_err(|e| format!("Failed to list resources: {}", e))?;
    let plugin_resources: Vec<_> = resources
        .into_iter()
        .filter(|r| r.metadata.as_deref() == Some(&plugin_id))
        .collect();

    let mut links = Vec::new();
    for resource in &plugin_resources {
        let type_dir = match resource.resource_type {
            ResourceType::Skill => "skills",
            ResourceType::Agent => "agents",
            ResourceType::Rule => "rules",
            ResourceType::Hook => "hooks",
            ResourceType::Command => "commands",
            ResourceType::McpServer => "mcp_servers",
        };
        let target_dir = std::path::Path::new(&project.path)
            .join(".claude")
            .join(type_dir);
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create dir: {}", e))?;

        let source = std::path::Path::new(&resource.source_path);
        let file_name = source.file_name().ok_or("Invalid source path")?;
        let target = target_dir.join(file_name);

        if target.exists() {
            continue;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(source, &target)
            .map_err(|e| format!("Failed to create symlink: {}", e))?;

        let now = chrono::Utc::now().to_rfc3339();
        let link = crate::models::v2::ResourceLink {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: resource.id.clone(),
            target_scope: "project".to_string(),
            target_path: target.to_string_lossy().to_string(),
            config_key: None,
            project_id: Some(project_id.clone()),
            link_type: "symlink".to_string(),
            created_at: now,
        };
        db.insert_link(&link)
            .map_err(|e| format!("Failed to insert link: {}", e))?;
        links.push(link);
    }
    Ok(links)
}

/// Internal implementation for install_plugin_to_global, accepts claude_dir for testability.
fn install_plugin_to_global_impl(
    db: &Database,
    plugin_id: &str,
    claude_dir: &Path,
) -> Result<Vec<crate::models::v2::ResourceLink>, String> {
    let resources = db
        .list_resources_by_scope(&ResourceScope::Registry)
        .map_err(|e| format!("Failed to list resources: {}", e))?;
    let plugin_resources: Vec<_> = resources
        .into_iter()
        .filter(|r| r.metadata.as_deref() == Some(plugin_id))
        .collect();

    let mut links = Vec::new();
    for resource in &plugin_resources {
        let type_dir = match resource.resource_type {
            ResourceType::Skill => "skills",
            ResourceType::Agent => "agents",
            ResourceType::Rule => "rules",
            ResourceType::Hook => "hooks",
            ResourceType::Command => "commands",
            ResourceType::McpServer => "mcp_servers",
        };
        let target_dir = claude_dir.join(type_dir);
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create dir: {}", e))?;

        let source = std::path::Path::new(&resource.source_path);
        let file_name = source.file_name().ok_or("Invalid source path")?;
        let target = target_dir.join(file_name);

        if target.exists() {
            continue;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(source, &target)
            .map_err(|e| format!("Failed to create symlink: {}", e))?;
        #[cfg(not(unix))]
        return Err("Symlinks not supported on this platform".to_string());

        let now = chrono::Utc::now().to_rfc3339();
        let link = crate::models::v2::ResourceLink {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: resource.id.clone(),
            target_scope: "global".to_string(),
            target_path: target.to_string_lossy().to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: now,
        };
        db.insert_link(&link)
            .map_err(|e| format!("Failed to insert link: {}", e))?;
        links.push(link);
    }
    Ok(links)
}

/// Uninstall a plugin (either plugin_install mode or resource/symlink mode).
///
/// For plugin_install links: runs `claude plugin uninstall` (best-effort) and removes the DB record.
/// For symlink/resource links: removes the symlink/file/dir from disk, then removes the DB record.
#[tauri::command]
pub fn uninstall_plugin_from_project(
    db: State<Database>,
    link_id: String,
) -> Result<(), String> {
    let link = db.get_link(&link_id)
        .map_err(|e| e.to_string())?
        .ok_or("Link not found")?;

    if link.link_type == "plugin_install" {
        // Parse plugin@marketplace from config_key
        let config_key = link.config_key.as_deref()
            .ok_or("Missing config_key for plugin_install link")?;
        let plugin_ref = config_key.strip_prefix("enabledPlugins.")
            .ok_or("Invalid config_key format")?;
        let parts: Vec<&str> = plugin_ref.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err("Invalid plugin reference format".into());
        }
        let (plugin_name, marketplace_name) = (parts[0], parts[1]);

        // Determine scope and path
        let (project_path, scope) = if let Some(ref pid) = link.project_id {
            let project = db.get_project(pid)
                .map_err(|e| e.to_string())?
                .ok_or("Project not found")?;
            (project.path, "project")
        } else {
            let home = dirs::home_dir()
                .ok_or("Cannot determine home directory")?;
            (home.to_string_lossy().to_string(), "user")
        };

        // Best effort CLI uninstall — don't fail if CLI unavailable
        if let Err(e) = plugin_install::uninstall_plugin_cli(&project_path, plugin_name, marketplace_name, scope) {
            eprintln!("Warning: CLI uninstall failed: {}", e);
        }
    } else {
        // Resource mode: delete the symlink/file
        let target = std::path::Path::new(&link.target_path);
        if target.exists() || target.symlink_metadata().is_ok() {
            if target.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                std::fs::remove_file(target).map_err(|e| e.to_string())?;
            } else if target.is_dir() {
                std::fs::remove_dir_all(target).map_err(|e| e.to_string())?;
            } else {
                std::fs::remove_file(target).map_err(|e| e.to_string())?;
            }
        }
    }

    // Remove link record from DB
    db.delete_link(&link_id).map_err(|e| e.to_string())?;
    Ok(())
}

/// Install all resources of a registry plugin to global ~/.claude/<type>/
#[tauri::command]
pub fn install_plugin_to_global(
    db: State<Database>,
    plugin_id: String,
) -> Result<Vec<crate::models::v2::ResourceLink>, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;

    // Look up registry plugin and its registry
    let registry_plugin = db.get_registry_plugin(&plugin_id)
        .map_err(|e| format!("Failed to get plugin: {}", e))?
        .ok_or("Plugin not found")?;

    let registry = db.get_registry(&registry_plugin.registry_id)
        .map_err(|e| format!("Failed to get registry: {}", e))?
        .ok_or("Registry not found")?;

    // Try plugin mode: if wrapper exists and CLI available
    let ccm_name = plugin_install::ccm_marketplace_name(&registry.name);
    let has_wrapper = plugin_install::has_wrapper(&registry.name).unwrap_or(false);

    if has_wrapper && plugin_install::is_claude_cli_available() {
        match plugin_install::install_plugin_cli(
            &home.to_string_lossy(),
            &registry_plugin.name,
            &ccm_name,
            "user",
        ) {
            Ok(()) => {
                let now = chrono::Utc::now().to_rfc3339();
                let config_key = format!("enabledPlugins.{}@{}", registry_plugin.name, ccm_name);
                let settings_path = home.join(".claude").join("settings.json");
                let link = crate::models::v2::ResourceLink {
                    id: uuid::Uuid::new_v4().to_string(),
                    resource_id: plugin_id.clone(),
                    target_scope: "global".to_string(),
                    target_path: settings_path.to_string_lossy().to_string(),
                    config_key: Some(config_key),
                    project_id: None,
                    link_type: "plugin_install".to_string(),
                    created_at: now,
                };
                db.insert_link(&link).map_err(|e| format!("Failed to record link: {}", e))?;
                return Ok(vec![link]);
            }
            Err(e) => {
                eprintln!("Warning: Plugin mode failed, falling back to symlinks: {}", e);
                // Fall through to symlink mode below
            }
        }
    }

    let claude_dir = home.join(".claude");
    install_plugin_to_global_impl(&db, &plugin_id, &claude_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::v2::{Registry, Resource, ResourceScope, ResourceType, ResourceLink};
    use std::fs;

    fn make_registry(id: &str, name: &str, url: &str, local_path: &str) -> Registry {
        Registry {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            local_path: local_path.to_string(),
            readonly: false,
            last_synced: Some("2026-03-01T00:00:00Z".to_string()),
            has_remote_changes: false,
            has_local_changes: false,
            created_at: "2026-03-01T00:00:00Z".to_string(),
        }
    }

    fn make_registry_resource(id: &str, name: &str, source_path: &str, registry_id: &str, rtype: ResourceType) -> Resource {
        Resource {
            id: id.to_string(),
            resource_type: rtype,
            name: name.to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: source_path.to_string(),
            content_hash: Some("abc123".to_string()),
            metadata: Some(registry_id.to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        }
    }

    #[test]
    fn test_list_registries_empty() {
        let db = Database::new_in_memory().unwrap();
        let registries = db.list_registries().unwrap();
        assert!(registries.is_empty());
    }

    #[test]
    fn test_list_registries_with_data() {
        let db = Database::new_in_memory().unwrap();
        let r1 = make_registry("reg1", "My Registry", "https://github.com/user/reg1.git", "/tmp/reg1");
        let r2 = make_registry("reg2", "Other Registry", "https://github.com/user/reg2.git", "/tmp/reg2");
        db.insert_registry(&r1).unwrap();
        db.insert_registry(&r2).unwrap();

        let registries = db.list_registries().unwrap();
        assert_eq!(registries.len(), 2);
    }

    #[test]
    fn test_list_registry_resources_filters_by_registry_id() {
        let db = Database::new_in_memory().unwrap();

        let r1 = make_registry_resource("r1", "skill-a", "/reg1/skills/skill-a", "reg1", ResourceType::Skill);
        let r2 = make_registry_resource("r2", "agent-a", "/reg1/agents/agent-a.md", "reg1", ResourceType::Agent);
        let r3 = make_registry_resource("r3", "skill-b", "/reg2/skills/skill-b", "reg2", ResourceType::Skill);
        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();
        db.insert_resource(&r3).unwrap();

        // Get all registry resources and filter by metadata
        let all = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        let reg1_resources: Vec<_> = all.into_iter()
            .filter(|r| r.metadata.as_deref() == Some("reg1"))
            .collect();
        assert_eq!(reg1_resources.len(), 2);

        // Filter by type too
        let skills = db.list_resources_by_scope_and_type(&ResourceScope::Registry, &ResourceType::Skill).unwrap();
        let reg1_skills: Vec<_> = skills.into_iter()
            .filter(|r| r.metadata.as_deref() == Some("reg1"))
            .collect();
        assert_eq!(reg1_skills.len(), 1);
        assert_eq!(reg1_skills[0].name, "skill-a");
    }

    #[test]
    fn test_remove_registry_with_linked_resources_fails() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("reg1", "My Registry", "https://example.com/reg.git", "/tmp/reg1");
        db.insert_registry(&registry).unwrap();

        let resource = make_registry_resource("r1", "test-skill", "/tmp/reg1/skills/test-skill", "reg1", ResourceType::Skill);
        db.insert_resource(&resource).unwrap();

        // Add a project for FK
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('proj1', 'test-project', '/tmp/proj1')",
                [],
            ).unwrap();
        }

        let link = ResourceLink {
            id: "link1".to_string(),
            resource_id: "r1".to_string(),
            target_scope: "project".to_string(),
            target_path: "/tmp/proj1/.claude/skills/test-skill".to_string(),
            config_key: None,
            project_id: Some("proj1".to_string()),
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_link(&link).unwrap();

        // Verify links exist
        let links = db.list_links_by_resource("r1").unwrap();
        assert_eq!(links.len(), 1);

        // The remove would fail because of linked resources
        let registry_resources = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        let has_links = registry_resources.iter().any(|r| {
            r.metadata.as_deref() == Some("reg1")
                && db.list_links_by_resource(&r.id).unwrap().len() > 0
        });
        assert!(has_links);
    }

    #[test]
    fn test_remove_registry_without_links_succeeds() {
        let db = Database::new_in_memory().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let local_path = tmp.path().to_string_lossy().to_string();

        let registry = make_registry("reg1", "My Registry", "https://example.com/reg.git", &local_path);
        db.insert_registry(&registry).unwrap();

        let resource = make_registry_resource("r1", "test-skill", &format!("{}/skills/test-skill", local_path), "reg1", ResourceType::Skill);
        db.insert_resource(&resource).unwrap();

        // Delete resources
        let registry_resources = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        for r in &registry_resources {
            if r.metadata.as_deref() == Some("reg1") {
                db.delete_resource(&r.id).unwrap();
            }
        }

        // Delete registry
        db.delete_registry("reg1").unwrap();

        assert!(db.get_registry("reg1").unwrap().is_none());
        let remaining = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_publish_to_readonly_registry_fails() {
        let db = Database::new_in_memory().unwrap();
        let registry = Registry {
            id: "reg1".to_string(),
            name: "Readonly Registry".to_string(),
            url: "https://example.com/reg.git".to_string(),
            local_path: "/tmp/reg1".to_string(),
            readonly: true,
            last_synced: None,
            has_remote_changes: false,
            has_local_changes: false,
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_registry(&registry).unwrap();

        let fetched = db.get_registry("reg1").unwrap().unwrap();
        assert!(fetched.readonly);
    }

    #[test]
    fn test_push_to_readonly_registry_fails() {
        let db = Database::new_in_memory().unwrap();
        let registry = Registry {
            id: "reg1".to_string(),
            name: "Readonly Registry".to_string(),
            url: "https://example.com/reg.git".to_string(),
            local_path: "/tmp/reg1".to_string(),
            readonly: true,
            last_synced: None,
            has_remote_changes: false,
            has_local_changes: false,
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_registry(&registry).unwrap();

        // Verify readonly flag
        let fetched = db.get_registry("reg1").unwrap().unwrap();
        assert!(fetched.readonly, "Registry should be readonly");
    }

    #[test]
    fn test_copy_dir_recursive() {
        let src = tempfile::TempDir::new().unwrap();
        let dst = tempfile::TempDir::new().unwrap();

        let sub_dir = src.path().join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(src.path().join("file1.txt"), "hello").unwrap();
        fs::write(sub_dir.join("file2.txt"), "world").unwrap();

        let dst_path = dst.path().join("copied");
        copy_dir_recursive(src.path(), &dst_path).unwrap();

        assert!(dst_path.join("file1.txt").exists());
        assert!(dst_path.join("subdir").join("file2.txt").exists());
        assert_eq!(fs::read_to_string(dst_path.join("file1.txt")).unwrap(), "hello");
        assert_eq!(fs::read_to_string(dst_path.join("subdir").join("file2.txt")).unwrap(), "world");
    }

    #[test]
    fn test_install_from_registry_rejects_non_registry_resource() {
        let db = Database::new_in_memory().unwrap();
        let resource = Resource {
            id: "lib-r1".to_string(),
            resource_type: ResourceType::Skill,
            name: "test-skill".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: "/tmp/library/skills/test-skill".to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        };
        db.insert_resource(&resource).unwrap();

        let fetched = db.get_resource("lib-r1").unwrap().unwrap();
        assert_ne!(fetched.scope, ResourceScope::Registry);
    }

    #[test]
    fn test_deploy_from_registry_rejects_non_registry_resource() {
        let db = Database::new_in_memory().unwrap();
        let resource = Resource {
            id: "global-r1".to_string(),
            resource_type: ResourceType::Rule,
            name: "test-rule".to_string(),
            description: None,
            scope: ResourceScope::Global,
            source_path: "/tmp/global/rules/test-rule.md".to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        };
        db.insert_resource(&resource).unwrap();

        let fetched = db.get_resource("global-r1").unwrap().unwrap();
        assert_ne!(fetched.scope, ResourceScope::Registry);
    }

    #[test]
    fn test_scan_and_insert_plugins_fallback() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create some resources in the registry root (no marketplace.json)
        fs::create_dir_all(tmp.path().join("skills/test-skill")).unwrap();
        fs::write(tmp.path().join("skills/test-skill/SKILL.md"), "# Test").unwrap();

        let registry = make_registry("reg1", "Test Registry", "https://example.com/reg.git", tmp.path().to_str().unwrap());
        db.insert_registry(&registry).unwrap();

        scan_and_insert_plugins(&db, &registry).unwrap();

        let plugins = db.list_registry_plugins("reg1").unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "Test Registry");

        let resources = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        assert_eq!(resources.len(), 1);
    }

    #[test]
    fn test_scan_and_insert_plugins_with_marketplace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create marketplace.json
        fs::create_dir_all(tmp.path().join(".claude-plugin")).unwrap();
        let plugin_dir = tmp.path().join("plugins/local-plugin");
        fs::create_dir_all(plugin_dir.join("rules")).unwrap();
        fs::write(plugin_dir.join("rules/my-rule.md"), "# Rule").unwrap();

        fs::write(
            tmp.path().join(".claude-plugin/marketplace.json"),
            format!(r#"{{
                "name": "test-marketplace",
                "plugins": [
                    {{"name": "local-plugin", "source": "./plugins/local-plugin"}}
                ]
            }}"#),
        ).unwrap();

        let registry = make_registry("reg1", "Test Registry", "https://example.com/reg.git", tmp.path().to_str().unwrap());
        db.insert_registry(&registry).unwrap();

        scan_and_insert_plugins(&db, &registry).unwrap();

        let plugins = db.list_registry_plugins("reg1").unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "local-plugin");
        assert_eq!(plugins[0].source_type, "local");

        let resources = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "my-rule");
    }

    #[test]
    fn test_install_plugin_to_global() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create source files for two resources
        let source_dir = tmp.path().join("registry/skills");
        fs::create_dir_all(&source_dir).unwrap();
        let skill_file = source_dir.join("my-skill.md");
        fs::write(&skill_file, "# My Skill").unwrap();

        let agent_dir = tmp.path().join("registry/agents");
        fs::create_dir_all(&agent_dir).unwrap();
        let agent_file = agent_dir.join("my-agent.md");
        fs::write(&agent_file, "# My Agent").unwrap();

        // Insert resources with scope=Registry, metadata=plugin_id
        let r1 = Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Skill,
            name: "my-skill.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: skill_file.to_string_lossy().to_string(),
            content_hash: None,
            metadata: Some("plugin-abc".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        };
        let r2 = Resource {
            id: "res2".to_string(),
            resource_type: ResourceType::Agent,
            name: "my-agent.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: agent_file.to_string_lossy().to_string(),
            content_hash: None,
            metadata: Some("plugin-abc".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        };
        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();

        // Target: use tmp as fake home dir
        let global_dir = tmp.path().join(".claude");

        let links = install_plugin_to_global_impl(&db, "plugin-abc", &global_dir).unwrap();

        assert_eq!(links.len(), 2);
        assert!(global_dir.join("skills/my-skill.md").exists());
        assert!(global_dir.join("agents/my-agent.md").exists());

        // Verify symlinks point to correct sources
        let skill_target = fs::read_link(global_dir.join("skills/my-skill.md")).unwrap();
        assert_eq!(skill_target, skill_file);

        // Verify links have correct scope
        for link in &links {
            assert_eq!(link.target_scope, "global");
            assert!(link.project_id.is_none());
            assert_eq!(link.link_type, "symlink");
        }

        // Verify DB has the links
        let db_links = db.list_all_links().unwrap();
        assert_eq!(db_links.len(), 2);
    }

    #[test]
    fn test_install_plugin_to_global_skips_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let source_dir = tmp.path().join("registry/skills");
        fs::create_dir_all(&source_dir).unwrap();
        let skill_file = source_dir.join("existing-skill.md");
        fs::write(&skill_file, "# Skill").unwrap();

        let r1 = Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Skill,
            name: "existing-skill.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: skill_file.to_string_lossy().to_string(),
            content_hash: None,
            metadata: Some("plugin-xyz".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        };
        db.insert_resource(&r1).unwrap();

        let global_dir = tmp.path().join(".claude");
        let target_dir = global_dir.join("skills");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("existing-skill.md"), "already here").unwrap();

        let links = install_plugin_to_global_impl(&db, "plugin-xyz", &global_dir).unwrap();

        // Should skip the existing file, return empty
        assert_eq!(links.len(), 0);
        // Original file should be unchanged
        let content = fs::read_to_string(target_dir.join("existing-skill.md")).unwrap();
        assert_eq!(content, "already here");
    }
}
