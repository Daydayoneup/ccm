use tauri::State;
use crate::adapters::{AdapterRegistry, file_based::copy_dir_recursive, plugin_install};
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
            installed_from_id: None,
                    };
                    let _ = db.insert_resource(&resource);
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
            installed_from_id: None,
            };
            let _ = db.insert_resource(&resource);
        }

    }
    Ok(())
}

/// Upsert registry plugins and resources: preserves existing resource IDs (and their links)
/// by matching on source_path instead of deleting and re-creating.
pub fn upsert_registry_plugins(db: &crate::db::Database, registry: &Registry) -> Result<(), String> {
    use std::collections::{HashMap, HashSet};

    // 1. Collect all existing registry resources under this registry path, keyed by source_path
    let existing_resources = db
        .list_registry_resources_by_path_prefix(&registry.local_path)
        .map_err(|e| format!("Failed to list existing resources: {}", e))?;
    let mut existing_by_path: HashMap<String, crate::models::v2::Resource> = existing_resources
        .into_iter()
        .map(|r| (r.source_path.clone(), r))
        .collect();

    // 2. Delete old registry_plugins (we'll re-create them; plugins have no links to preserve)
    let _ = db.delete_registry_plugins_by_registry(&registry.id);

    // 3. Scan and upsert
    let marketplace = read_marketplace_json(&registry.local_path);
    let mut seen_paths: HashSet<String> = HashSet::new();

    let plugin_scans: Vec<(RegistryPlugin, Vec<crate::scanner::ScannedResource>)> =
        if let Some(mp) = marketplace {
            if let Some(plugins) = mp.plugins {
                let mut result = Vec::new();
                for mp_plugin in &plugins {
                    let clone_path = resolve_plugin_source_path(&registry.local_path, mp_plugin);
                    if mp_plugin.is_external() {
                        if let Some(url) = mp_plugin.external_url() {
                            if !std::path::Path::new(&clone_path).exists() {
                                match crate::git::clone(&url, &clone_path, None) {
                                    Ok(r) if r.success => {}
                                    Ok(r) => {
                                        eprintln!("Warning: Failed to clone external plugin {}: {}", mp_plugin.name, r.stderr);
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
                    let scanned = scan_plugin_dir(&source_path);
                    result.push((reg_plugin, scanned));
                }
                result
            } else {
                vec![]
            }
        } else {
            // Fallback: no marketplace.json
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
            let scanned = scan_registry(&registry.local_path);
            vec![(fallback_plugin, scanned)]
        };

    let now = chrono::Utc::now().to_rfc3339();
    for (reg_plugin, scanned) in plugin_scans {
        db.insert_registry_plugin(&reg_plugin)
            .map_err(|e| format!("Failed to insert registry plugin: {}", e))?;

        for sr in scanned {
            seen_paths.insert(sr.source_path.clone());

            if let Some(mut existing) = existing_by_path.remove(&sr.source_path) {
                // Update existing resource: preserve ID, update metadata and content_hash
                existing.metadata = Some(reg_plugin.id.clone());
                existing.content_hash = sr.content_hash;
                existing.resource_type = sr.resource_type;
                existing.updated_at = now.clone();
                let _ = db.update_resource(&existing);
            } else {
                // New resource
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
            installed_from_id: None,
                };
                let _ = db.insert_resource(&resource);
            }
        }
    }

    // 4. Handle resources that no longer exist in the registry
    for (_path, old_resource) in &existing_by_path {
        let links = db.list_links_by_resource(&old_resource.id).unwrap_or_default();
        if links.is_empty() {
            // No active installations — safe to delete
            let _ = db.delete_resource(&old_resource.id);
        } else {
            // Has active installations — mark as upstream-removed instead of deleting
            let mut marked = old_resource.clone();
            marked.is_draft = -1;
            marked.updated_at = chrono::Utc::now().to_rfc3339();
            let _ = db.update_resource(&marked);
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

        // Re-scan plugins (upsert preserves resource IDs and links)
        upsert_registry_plugins(&db, &registry)?;

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

            let mut updated = reg;
            updated.last_synced = Some(now);
            updated.has_remote_changes = false;
            let _ = upsert_registry_plugins(&db, &updated);
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

    let adapter_registry = AdapterRegistry::new();
    let type_dir = adapter_registry
        .get(&source_resource.resource_type)
        .map(|a| a.type_dir())
        .ok_or_else(|| format!("No adapter for {:?}", source_resource.resource_type))?;

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
            installed_from_id: None,
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
    let adapter_registry_inst = AdapterRegistry::new();
    let type_dir = adapter_registry_inst
        .get(&resource.resource_type)
        .map(|a| a.type_dir())
        .ok_or_else(|| format!("No adapter for {:?}", resource.resource_type))?;
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
        installed_hash: None,
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
    let adapter_registry_deploy = AdapterRegistry::new();
    let type_dir = adapter_registry_deploy
        .get(&resource.resource_type)
        .map(|a| a.type_dir())
        .ok_or_else(|| format!("No adapter for {:?}", resource.resource_type))?;
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
        installed_hash: None,
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

/// Get MCP servers for a specific registry plugin (from resources table)
#[tauri::command]
pub fn get_registry_plugin_mcp_servers(
    db: State<Database>,
    plugin_id: String,
) -> Result<Vec<crate::models::v2::Resource>, String> {
    let resources = db
        .list_resources_by_scope(&crate::models::v2::ResourceScope::Registry)
        .map_err(|e| format!("Failed to list resources: {}", e))?;
    Ok(resources
        .into_iter()
        .filter(|r| {
            r.resource_type == crate::models::v2::ResourceType::McpServer
                && r.metadata.as_deref() == Some(&plugin_id)
        })
        .collect())
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
                    installed_hash: None,
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

    let loop_adapter_registry = AdapterRegistry::new();
    let mut links = Vec::new();
    for resource in &plugin_resources {
        let type_dir = loop_adapter_registry
            .get(&resource.resource_type)
            .map(|a| a.type_dir())
            .ok_or_else(|| format!("No adapter for {:?}", resource.resource_type))?;
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
            installed_hash: None,
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
    installed_base: &Path,
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
        // McpServer resources use plugin_install mode, skip for symlink fallback
        if matches!(resource.resource_type, ResourceType::McpServer) {
            continue;
        }
        let scope = crate::install::InstallScope::Global;
        let strategy = crate::install::InstallStrategy::FileBased;
        let ar = crate::adapters::AdapterRegistry::new();
        match crate::install::install_resource_with_paths(db, resource, scope, strategy, installed_base, claude_dir, &ar) {
            Ok(link) => links.push(link),
            Err(e) if e.starts_with("Target already exists") => continue,
            Err(e) => return Err(e),
        }
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

/// Shared helper: resolve resource type to directory name, reject McpServer.
fn resource_type_dir(rt: &ResourceType) -> Result<&'static str, String> {
    if *rt == ResourceType::McpServer {
        return Err("MCP server resources cannot be installed individually; use plugin-level install instead".to_string());
    }
    AdapterRegistry::new()
        .get(rt)
        .map(|a| a.type_dir())
        .ok_or_else(|| format!("No adapter for {:?}", rt))
}

/// Shared helper: create symlink and ResourceLink record for a single resource.
#[cfg(test)]
fn install_single_resource_symlink(
    db: &Database,
    resource: &crate::models::v2::Resource,
    target_base: &Path,
    target_scope: &str,
    project_id: Option<String>,
) -> Result<crate::models::v2::ResourceLink, String> {
    let type_dir = resource_type_dir(&resource.resource_type)?;
    let target_dir = target_base.join(type_dir);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create dir: {}", e))?;

    let source = std::path::Path::new(&resource.source_path);
    let file_name = source.file_name().ok_or("Invalid source path")?;
    let target = target_dir.join(file_name);

    if target.exists() {
        return Err(format!("Target already exists: {}", target.display()));
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
        target_scope: target_scope.to_string(),
        target_path: target.to_string_lossy().to_string(),
        config_key: None,
        project_id,
        link_type: "symlink".to_string(),
        created_at: now,
        installed_hash: None,
    };
    db.insert_link(&link)
        .map_err(|e| format!("Failed to insert link: {}", e))?;
    Ok(link)
}

/// New install function: copies resource to installed/ directory, then creates symlink pointing to the installed copy.
#[cfg(test)]
fn install_single_resource_copy(
    db: &Database,
    resource: &crate::models::v2::Resource,
    target_base: &Path,
    target_scope: &str,
    project_id: Option<String>,
    installed_base: &Path,
) -> Result<crate::models::v2::ResourceLink, String> {
    let type_dir = resource_type_dir(&resource.resource_type)?;

    // 1. Validate source exists
    let source = std::path::Path::new(&resource.source_path);
    if !source.exists() {
        return Err("Source not available".to_string());
    }

    // 2. Get file_name from source path
    let file_name = source.file_name().ok_or("Invalid source path")?;

    // 3. Copy to installed_base/<type_dir>/<file_name>/
    let installed_dir = installed_base.join(type_dir);
    let installed_path = installed_dir.join(file_name);
    if !installed_path.exists() {
        fs::create_dir_all(&installed_dir)
            .map_err(|e| format!("Failed to create installed dir: {}", e))?;
        if source.is_dir() {
            copy_dir_recursive(source, &installed_path)?;
        } else {
            fs::copy(source, &installed_path)
                .map_err(|e| format!("Failed to copy resource: {}", e))?;
        }
    }

    // 4. Compute hash of installed copy
    let installed_hash = crate::scanner::compute_file_hash(&installed_path.to_string_lossy());

    // 5. Create symlink from target_base/<type_dir>/<file_name> → installed copy
    let target_dir = target_base.join(type_dir);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create dir: {}", e))?;
    let target = target_dir.join(file_name);
    if target.exists() || target.symlink_metadata().is_ok() {
        // If it's an existing symlink (e.g., from old install mechanism), replace it
        let is_symlink = target
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_symlink {
            std::fs::remove_file(&target)
                .map_err(|e| format!("Failed to remove old symlink: {}", e))?;
        } else {
            return Err(format!("Target already exists and is not a symlink: {}", target.display()));
        }
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(&installed_path, &target)
        .map_err(|e| format!("Failed to create symlink: {}", e))?;
    #[cfg(not(unix))]
    return Err("Symlinks not supported on this platform".to_string());

    // 6. Insert ResourceLink with installed_hash
    let now = chrono::Utc::now().to_rfc3339();
    let link = crate::models::v2::ResourceLink {
        id: uuid::Uuid::new_v4().to_string(),
        resource_id: resource.id.clone(),
        target_scope: target_scope.to_string(),
        target_path: target.to_string_lossy().to_string(),
        config_key: None,
        project_id,
        link_type: "symlink".to_string(),
        created_at: now,
        installed_hash,
    };
    db.insert_link(&link)
        .map_err(|e| format!("Failed to insert link: {}", e))?;
    Ok(link)
}

/// Internal implementation for single-resource project install, accepts project_path for testability.
fn install_resource_to_project_impl(
    db: &Database,
    resource_id: &str,
    project_path: &Path,
    project_id: Option<String>,
    installed_base: &Path,
) -> Result<Vec<crate::models::v2::ResourceLink>, String> {
    let resource = db
        .get_resource(resource_id)
        .map_err(|e| format!("Failed to get resource: {}", e))?
        .ok_or("Resource not found")?;
    let scope = crate::install::InstallScope::Project {
        id: project_id.unwrap_or_default(),
        path: project_path.to_string_lossy().to_string(),
    };
    let ar = crate::adapters::AdapterRegistry::new();
    let link = crate::install_service::install_with_paths(
        db, &resource, scope, &ar, installed_base, &std::path::PathBuf::new(),
    )?;
    Ok(vec![link])
}

/// Install a single resource to a project via symlink (no CLI plugin mode).
#[tauri::command]
pub fn install_resource_to_project(
    db: State<Database>,
    resource_id: String,
    project_id: String,
) -> Result<Vec<crate::models::v2::ResourceLink>, String> {
    let project = db
        .get_project(&project_id)
        .map_err(|e| format!("Failed to get project: {}", e))?
        .ok_or("Project not found")?;
    let installed_base = dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude-manager")
        .join("installed");
    install_resource_to_project_impl(&db, &resource_id, std::path::Path::new(&project.path), Some(project_id), &installed_base)
}

fn install_resource_to_global_impl(
    db: &Database,
    resource_id: &str,
    claude_dir: &Path,
    installed_base: &Path,
) -> Result<Vec<crate::models::v2::ResourceLink>, String> {
    let resource = db
        .get_resource(resource_id)
        .map_err(|e| format!("Failed to get resource: {}", e))?
        .ok_or("Resource not found")?;
    let scope = crate::install::InstallScope::Global;
    let ar = crate::adapters::AdapterRegistry::new();
    let link = crate::install_service::install_with_paths(
        db, &resource, scope, &ar, installed_base, claude_dir,
    )?;
    Ok(vec![link])
}

/// Install a single resource to global ~/.claude/<type>/ via symlink.
#[tauri::command]
pub fn install_resource_to_global(
    db: State<Database>,
    resource_id: String,
) -> Result<Vec<crate::models::v2::ResourceLink>, String> {
    let claude_dir = dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude");
    let installed_base = dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude-manager")
        .join("installed");
    install_resource_to_global_impl(&db, &resource_id, &claude_dir, &installed_base)
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
                    installed_hash: None,
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
    let installed_base = home.join(".claude-manager").join("installed");
    install_plugin_to_global_impl(&db, &plugin_id, &claude_dir, &installed_base)
}

/// Internal implementation for uninstalling resources by link IDs (best-effort).
fn uninstall_resource_impl(
    db: &Database,
    link_ids: Vec<String>,
    installed_base: &Path,
    adapter_registry: &crate::adapters::AdapterRegistry,
) -> Result<Vec<String>, String> {
    crate::install::uninstall_resource_with_base(db, link_ids, installed_base, adapter_registry)
}

/// Uninstall specific resource links by their IDs (best-effort).
#[tauri::command]
pub fn uninstall_resource(
    db: State<Database>,
    adapter_registry: State<crate::adapters::AdapterRegistry>,
    link_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    let installed_base = dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude-manager").join("installed");
    uninstall_resource_impl(&db, link_ids, &installed_base, &adapter_registry)
}

fn get_plugin_resources_install_status_impl(
    db: &Database,
    plugin_id: &str,
) -> Result<std::collections::HashMap<String, Vec<crate::models::v2::ResourceLink>>, String> {
    let resources = db
        .list_resources_by_scope(&ResourceScope::Registry)
        .map_err(|e| format!("Failed to list resources: {}", e))?;
    let plugin_resources: Vec<_> = resources
        .into_iter()
        .filter(|r| r.metadata.as_deref() == Some(plugin_id))
        .collect();

    let mut status_map = std::collections::HashMap::new();
    for resource in &plugin_resources {
        let statuses = crate::install_service::query_install_status(db, &resource.id)
            .unwrap_or_default();
        if !statuses.is_empty() {
            let links: Vec<_> = statuses.into_iter().map(|s| s.link).collect();
            status_map.insert(resource.id.clone(), links);
        }
    }
    Ok(status_map)
}

/// Get install status for all resources in a plugin.
#[tauri::command]
pub fn get_plugin_resources_install_status(
    db: State<Database>,
    plugin_id: String,
) -> Result<std::collections::HashMap<String, Vec<crate::models::v2::ResourceLink>>, String> {
    get_plugin_resources_install_status_impl(&db, &plugin_id)
}

// ── Update / Retain / Hash commands ──────────────────────────────────

#[tauri::command]
pub fn update_installed_resource(db: State<Database>, resource_id: String) -> Result<(), String> {
    crate::install_service::update_installed(&db, &resource_id)
}

#[tauri::command]
pub fn retain_as_library(db: State<Database>, resource_id: String) -> Result<crate::models::v2::Resource, String> {
    crate::install_service::retain_as_library(&db, &resource_id)
}

#[tauri::command]
pub fn compute_installed_hash(
    resource_type: String,
    resource_name: String,
) -> Result<Option<String>, String> {
    let installed_base = dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude-manager").join("installed");
    let type_dir = match resource_type.as_str() {
        "skill" => "skills",
        "agent" => "agents",
        "rule" => "rules",
        "hook" => "hooks",
        "command" => "commands",
        _ => return Ok(None),
    };
    let installed_path = installed_base.join(type_dir).join(&resource_name);
    if !installed_path.exists() {
        return Ok(None);
    }
    Ok(crate::scanner::compute_file_hash(&installed_path.to_string_lossy()))
}

/// One-time migration: convert symlinks pointing to registry into installed/ copies.
pub fn migrate_symlinks_to_installed(db: &Database, installed_base: &Path) -> Result<(), String> {
    let links = db.list_all_links().map_err(|e| e.to_string())?;

    for link in links {
        // Skip if already migrated (has installed_hash)
        if link.installed_hash.is_some() {
            continue;
        }
        // Skip non-symlink types
        if link.link_type != "symlink" {
            continue;
        }

        let target = Path::new(&link.target_path);
        let is_symlink = target
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if !is_symlink {
            continue;
        }

        // Read where the symlink currently points
        let current_dest = match std::fs::read_link(target) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Skip if already pointing to installed/
        if current_dest.starts_with(installed_base) {
            continue;
        }

        // Get the resource to determine type
        let resource = match db.get_resource(&link.resource_id) {
            Ok(Some(r)) => r,
            _ => continue,
        };

        let type_dir = match resource_type_dir(&resource.resource_type) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let name = match current_dest.file_name() {
            Some(n) => n.to_os_string(),
            None => continue,
        };

        let installed_path = installed_base.join(type_dir).join(&name);

        // Copy source to installed/ if not already there
        if !installed_path.exists() && current_dest.exists() {
            if current_dest.is_dir() {
                let _ = crate::adapters::file_based::copy_dir_recursive(&current_dest, &installed_path);
            } else {
                let _ = fs::create_dir_all(installed_path.parent().unwrap());
                let _ = fs::copy(&current_dest, &installed_path);
            }
        }

        if !installed_path.exists() {
            continue;
        }

        // Re-create symlink pointing to installed/
        let _ = std::fs::remove_file(target);
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&installed_path, target);

        // Update link with installed_hash
        let hash = crate::scanner::compute_file_hash(&installed_path.to_string_lossy());
        let _ = db.update_link_installed_hash(&link.id, hash.as_deref());
    }

    Ok(())
}

// --- list_installed_resources ---

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstalledResourceInfo {
    pub resource: crate::models::v2::Resource,
    pub links: Vec<crate::models::v2::ResourceLink>,
    pub registry_name: Option<String>,
}

fn list_installed_resources_impl(db: &Database) -> Result<Vec<InstalledResourceInfo>, String> {
    let all_links = db.list_all_links().map_err(|e| e.to_string())?;

    // Group all links by resource_id
    let mut resource_links: std::collections::HashMap<String, Vec<crate::models::v2::ResourceLink>> =
        std::collections::HashMap::new();
    for link in all_links {
        resource_links.entry(link.resource_id.clone()).or_default().push(link);
    }

    // Build registry name cache
    let registries = db.list_registries().map_err(|e| e.to_string())?;
    let registry_name_map: std::collections::HashMap<String, String> = registries
        .iter()
        .map(|r| (r.id.clone(), r.name.clone()))
        .collect();

    let mut result = Vec::new();
    for (resource_id, links) in resource_links {
        if let Ok(Some(resource)) = db.get_resource(&resource_id) {
            let registry_name = resource.metadata.as_ref().and_then(|plugin_id| {
                db.get_registry_plugin(plugin_id)
                    .ok()
                    .flatten()
                    .and_then(|rp| registry_name_map.get(&rp.registry_id).cloned())
            });
            result.push(InstalledResourceInfo { resource, links, registry_name });
        }
    }
    result.sort_by(|a, b| a.resource.name.cmp(&b.resource.name));
    Ok(result)
}

#[tauri::command]
pub fn list_installed_resources(db: State<Database>) -> Result<Vec<InstalledResourceInfo>, String> {
    list_installed_resources_impl(&db)
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
            installed_from_id: None,
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
            installed_hash: None,
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
            installed_from_id: None,
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
            installed_from_id: None,
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
            installed_from_id: None,
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
            installed_from_id: None,
        };
        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();

        // Target: use tmp as fake home dir
        let global_dir = tmp.path().join(".claude");
        let installed_base = tmp.path().join("installed");

        let links = install_plugin_to_global_impl(&db, "plugin-abc", &global_dir, &installed_base).unwrap();

        assert_eq!(links.len(), 2);
        assert!(global_dir.join("skills/my-skill.md").exists());
        assert!(global_dir.join("agents/my-agent.md").exists());

        // Verify symlinks point to installed copies (not registry)
        let skill_target = fs::read_link(global_dir.join("skills/my-skill.md")).unwrap();
        assert_eq!(skill_target, installed_base.join("skills").join("my-skill.md"));

        // Verify links have correct scope
        for link in &links {
            assert_eq!(link.target_scope, "global");
            assert!(link.project_id.is_none());
            assert_eq!(link.link_type, "symlink");
        }

        // File-based installs no longer insert DB links (tracking via .ccm.json manifest)
        let db_links = db.list_all_links().unwrap();
        assert_eq!(db_links.len(), 0);
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
            installed_from_id: None,
        };
        db.insert_resource(&r1).unwrap();

        let global_dir = tmp.path().join(".claude");
        let installed_base = tmp.path().join("installed");
        let target_dir = global_dir.join("skills");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("existing-skill.md"), "already here").unwrap();

        let links = install_plugin_to_global_impl(&db, "plugin-xyz", &global_dir, &installed_base).unwrap();

        // Should skip the existing file, return empty
        assert_eq!(links.len(), 0);
        // Original file should be unchanged
        let content = fs::read_to_string(target_dir.join("existing-skill.md")).unwrap();
        assert_eq!(content, "already here");
    }

    #[test]
    fn test_install_single_resource_to_global() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let registry_id = "reg1";
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO registries (id, name, url, local_path, readonly, created_at) VALUES (?1, 'test-reg', 'https://example.com', ?2, 0, '2026-03-01')",
                rusqlite::params![registry_id, tmp.path().to_string_lossy().to_string()],
            ).unwrap();
        }

        let source_dir = tmp.path().join("skills").join("my-skill");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("skill.md"), "# test").unwrap();
        let resource_id = "res-single-1";
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO resources (id, resource_type, name, scope, source_path, metadata, created_at, updated_at) VALUES (?1, 'skill', 'my-skill', 'registry', ?2, ?3, '2026-03-01', '2026-03-01')",
                rusqlite::params![resource_id, source_dir.to_string_lossy().to_string(), registry_id],
            ).unwrap();
        }

        let claude_dir = tmp.path().join("claude-home");
        fs::create_dir_all(&claude_dir).unwrap();
        let installed_base = tmp.path().join("installed");

        let links = install_resource_to_global_impl(&db, resource_id, &claude_dir, &installed_base).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_scope, "global");
        assert_eq!(links[0].link_type, "symlink");

        let target = claude_dir.join("skills").join("my-skill");
        assert!(target.symlink_metadata().is_ok());
    }

    #[test]
    fn test_install_single_resource_to_project() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let project_dir = tmp.path().join("my-project");
        fs::create_dir_all(&project_dir).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('proj1', 'my-project', ?1)",
                rusqlite::params![project_dir.to_string_lossy().to_string()],
            ).unwrap();
        }

        let source_dir = tmp.path().join("skills").join("my-skill");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("skill.md"), "# test").unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO resources (id, resource_type, name, scope, source_path, created_at, updated_at) VALUES ('res1', 'skill', 'my-skill', 'registry', ?1, '2026-03-01', '2026-03-01')",
                rusqlite::params![source_dir.to_string_lossy().to_string()],
            ).unwrap();
        }

        let installed_base = tmp.path().join("installed");
        let links = install_resource_to_project_impl(&db, "res1", &project_dir, Some("proj1".to_string()), &installed_base).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_scope, "project");
        assert_eq!(links[0].project_id, Some("proj1".to_string()));

        let target = project_dir.join(".claude").join("skills").join("my-skill");
        assert!(target.symlink_metadata().is_ok());
    }

    #[test]
    fn test_install_mcp_server_resource_uses_config_based() {
        // With unified install_service, MCP servers are installed via ConfigBased strategy
        // (not rejected outright). This test verifies the adapter routes correctly.
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let source_dir = tmp.path().join("mcp").join("my-server");
        fs::create_dir_all(&source_dir).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO resources (id, resource_type, name, scope, source_path, created_at, updated_at) VALUES ('res-mcp', 'mcp_server', 'my-server', 'registry', ?1, '2026-03-01', '2026-03-01')",
                rusqlite::params![source_dir.to_string_lossy().to_string()],
            ).unwrap();
        }

        let claude_dir = tmp.path().join("claude-home");
        let installed_base = tmp.path().join("installed");
        // MCP servers go through ConfigBased path — may fail due to missing config,
        // but should NOT fail with a "MCP server" rejection message.
        let result = install_resource_to_global_impl(&db, "res-mcp", &claude_dir, &installed_base);
        if let Err(ref e) = result {
            assert!(!e.contains("MCP server resources cannot"), "Should not reject MCP servers outright; got: {}", e);
        }
    }

    #[test]
    fn test_uninstall_resource_best_effort() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO resources (id, resource_type, name, scope, source_path, created_at, updated_at) VALUES ('res1', 'skill', 'test', 'registry', '/tmp/src', '2026-03-01', '2026-03-01')",
                [],
            ).unwrap();
        }

        let target_path = tmp.path().join("skills").join("test-skill");
        fs::create_dir_all(tmp.path().join("skills")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/tmp/src", &target_path).unwrap();

        let link = crate::models::v2::ResourceLink {
            id: "link-u1".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "global".to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        let installed_base = tmp.path().join("installed");
        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let result = uninstall_resource_impl(&db, vec!["link-u1".to_string()], &installed_base, &adapter_registry);
        assert!(result.is_ok());

        assert!(!target_path.exists());
        assert!(db.get_link("link-u1").unwrap().is_none());
    }

    #[test]
    fn test_get_plugin_resources_install_status() {
        let db = Database::new_in_memory().unwrap();

        let plugin_id = "plugin1";
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO resources (id, resource_type, name, scope, source_path, metadata, created_at, updated_at) VALUES ('res1', 'skill', 'skill-a', 'registry', '/tmp/a', ?1, '2026-03-01', '2026-03-01')",
                rusqlite::params![plugin_id],
            ).unwrap();
            conn.execute(
                "INSERT INTO resources (id, resource_type, name, scope, source_path, metadata, created_at, updated_at) VALUES ('res2', 'agent', 'agent-b', 'registry', '/tmp/b', ?1, '2026-03-01', '2026-03-01')",
                rusqlite::params![plugin_id],
            ).unwrap();
        }

        let link = crate::models::v2::ResourceLink {
            id: "link1".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "global".to_string(),
            target_path: "/target/link1".to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        let status = get_plugin_resources_install_status_impl(&db, plugin_id).unwrap();
        assert_eq!(status.len(), 1);
        assert!(status.contains_key("res1"));
        assert!(!status.contains_key("res2"));
        assert_eq!(status["res1"].len(), 1);
    }

    #[test]
    fn test_install_single_resource_copies_to_installed_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create a fake registry skill directory with SKILL.md
        let source_dir = tmp.path().join("registry").join("skills").join("my-skill");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("SKILL.md"), "# My Skill\nSome content").unwrap();

        // Insert resource into DB
        let resource = Resource {
            id: "res-copy-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "my-skill".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: source_dir.to_string_lossy().to_string(),
            content_hash: Some("abc123".to_string()),
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let target_base = tmp.path().join("target");
        let installed_base = tmp.path().join("installed");

        let link = install_single_resource_copy(
            &db,
            &resource,
            &target_base,
            "global",
            None,
            &installed_base,
        ).unwrap();

        // 1. Verify file exists in installed/ dir
        let installed_skill = installed_base.join("skills").join("my-skill");
        assert!(installed_skill.exists(), "Installed copy should exist");
        assert!(installed_skill.join("SKILL.md").exists(), "SKILL.md should be copied");
        let content = fs::read_to_string(installed_skill.join("SKILL.md")).unwrap();
        assert_eq!(content, "# My Skill\nSome content");

        // 2. Verify symlink points to installed/ (not registry)
        let symlink_path = target_base.join("skills").join("my-skill");
        assert!(symlink_path.symlink_metadata().is_ok(), "Symlink should exist");
        let symlink_target = fs::read_link(&symlink_path).unwrap();
        assert_eq!(symlink_target, installed_skill, "Symlink should point to installed copy");

        // 3. Verify link has installed_hash
        assert!(link.installed_hash.is_some(), "Link should have installed_hash");
        assert_eq!(link.link_type, "symlink");
        assert_eq!(link.target_scope, "global");

        // 4. Verify DB has the link
        let db_link = db.get_link(&link.id).unwrap().unwrap();
        assert!(db_link.installed_hash.is_some());
    }

    #[test]
    fn test_install_single_resource_copy_file_resource() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create a fake agent .md file
        let source_dir = tmp.path().join("registry").join("agents");
        fs::create_dir_all(&source_dir).unwrap();
        let source_file = source_dir.join("my-agent.md");
        fs::write(&source_file, "# My Agent").unwrap();

        let resource = Resource {
            id: "res-copy-2".to_string(),
            resource_type: ResourceType::Agent,
            name: "my-agent.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: source_file.to_string_lossy().to_string(),
            content_hash: Some("def456".to_string()),
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let target_base = tmp.path().join("target");
        let installed_base = tmp.path().join("installed");

        // Insert a project for the FK constraint
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('proj1', 'test-project', '/tmp/proj1')",
                [],
            ).unwrap();
        }

        let link = install_single_resource_copy(
            &db,
            &resource,
            &target_base,
            "project",
            Some("proj1".to_string()),
            &installed_base,
        ).unwrap();

        // Verify file copied to installed/
        let installed_file = installed_base.join("agents").join("my-agent.md");
        assert!(installed_file.exists());
        assert_eq!(fs::read_to_string(&installed_file).unwrap(), "# My Agent");

        // Verify symlink
        let symlink_path = target_base.join("agents").join("my-agent.md");
        let symlink_target = fs::read_link(&symlink_path).unwrap();
        assert_eq!(symlink_target, installed_file);

        assert!(link.installed_hash.is_some());
        assert_eq!(link.target_scope, "project");
        assert_eq!(link.project_id, Some("proj1".to_string()));
    }

    #[test]
    fn test_install_single_resource_copy_source_not_available() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let resource = Resource {
            id: "res-missing".to_string(),
            resource_type: ResourceType::Skill,
            name: "missing-skill".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/nonexistent/path/skill".to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let target_base = tmp.path().join("target");
        let installed_base = tmp.path().join("installed");

        let result = install_single_resource_copy(
            &db,
            &resource,
            &target_base,
            "global",
            None,
            &installed_base,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Source not available"));
    }

    #[test]
    fn test_uninstall_cleans_installed_dir_when_last_link() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create resource in DB
        let resource = Resource {
            id: "res-clean-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "my-skill".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/tmp/src/my-skill".to_string(),
            content_hash: Some("abc".to_string()),
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Create installed copy
        let installed_base = tmp.path().join("installed");
        let installed_skill = installed_base.join("skills").join("my-skill");
        fs::create_dir_all(&installed_skill).unwrap();
        fs::write(installed_skill.join("SKILL.md"), "# Skill").unwrap();

        // Create symlink target
        let target_path = tmp.path().join("target").join("skills").join("my-skill");
        fs::create_dir_all(tmp.path().join("target").join("skills")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&installed_skill, &target_path).unwrap();

        // Create link in DB
        let link = crate::models::v2::ResourceLink {
            id: "link-clean-1".to_string(),
            resource_id: "res-clean-1".to_string(),
            target_scope: "global".to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        // Uninstall
        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let result = uninstall_resource_impl(&db, vec!["link-clean-1".to_string()], &installed_base, &adapter_registry);
        assert!(result.is_ok());

        // Symlink should be removed
        assert!(!target_path.exists());
        // Link record should be gone
        assert!(db.get_link("link-clean-1").unwrap().is_none());
        // Installed dir should be cleaned up (no more links)
        assert!(!installed_skill.exists());
    }

    #[test]
    fn test_uninstall_keeps_installed_dir_when_other_links_exist() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create resource in DB
        let resource = Resource {
            id: "res-keep-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "shared-skill".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/tmp/src/shared-skill".to_string(),
            content_hash: Some("abc".to_string()),
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Create installed copy
        let installed_base = tmp.path().join("installed");
        let installed_skill = installed_base.join("skills").join("shared-skill");
        fs::create_dir_all(&installed_skill).unwrap();
        fs::write(installed_skill.join("SKILL.md"), "# Skill").unwrap();

        // Create two symlink targets
        let target1 = tmp.path().join("target1").join("skills").join("shared-skill");
        let target2 = tmp.path().join("target2").join("skills").join("shared-skill");
        fs::create_dir_all(tmp.path().join("target1").join("skills")).unwrap();
        fs::create_dir_all(tmp.path().join("target2").join("skills")).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&installed_skill, &target1).unwrap();
            std::os::unix::fs::symlink(&installed_skill, &target2).unwrap();
        }

        // Create two links in DB
        let link1 = crate::models::v2::ResourceLink {
            id: "link-keep-1".to_string(),
            resource_id: "res-keep-1".to_string(),
            target_scope: "global".to_string(),
            target_path: target1.to_string_lossy().to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        let link2 = crate::models::v2::ResourceLink {
            id: "link-keep-2".to_string(),
            resource_id: "res-keep-1".to_string(),
            target_scope: "global".to_string(),
            target_path: target2.to_string_lossy().to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link1).unwrap();
        db.insert_link(&link2).unwrap();

        // Uninstall only the first link
        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let result = uninstall_resource_impl(&db, vec!["link-keep-1".to_string()], &installed_base, &adapter_registry);
        assert!(result.is_ok());

        // First symlink should be removed
        assert!(!target1.exists());
        // First link record should be gone
        assert!(db.get_link("link-keep-1").unwrap().is_none());
        // Second link should still exist
        assert!(db.get_link("link-keep-2").unwrap().is_some());
        // Installed dir should still exist (other link remains)
        assert!(installed_skill.exists());
    }

    #[test]
    fn test_upsert_marks_removed_resource_with_links() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create a registry
        let registry = crate::models::v2::Registry {
            id: "reg-mark-1".to_string(),
            name: "test-registry".to_string(),
            url: "https://example.com/reg.git".to_string(),
            local_path: tmp.path().to_string_lossy().to_string(),
            readonly: true,
            last_synced: None,
            has_remote_changes: false,
            has_local_changes: false,
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_registry(&registry).unwrap();

        // Create a skill on disk
        let skill_dir = tmp.path().join("skills").join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Test Skill").unwrap();

        // Run upsert to populate DB
        upsert_registry_plugins(&db, &registry).unwrap();

        // Find the resource that was created
        let resources = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        assert_eq!(resources.len(), 1);
        let resource = &resources[0];
        assert_eq!(resource.name, "test-skill");
        assert_eq!(resource.is_draft, 1);

        // Create a link (simulating installation)
        let link = crate::models::v2::ResourceLink {
            id: "link-mark-1".to_string(),
            resource_id: resource.id.clone(),
            target_scope: "global".to_string(),
            target_path: "/tmp/target/skills/test-skill".to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        // Remove the skill from disk (simulating upstream removal)
        fs::remove_dir_all(&skill_dir).unwrap();

        // Run upsert again
        upsert_registry_plugins(&db, &registry).unwrap();

        // Resource should still exist with is_draft == -1
        let updated_resource = db.get_resource(&resource.id).unwrap().expect("Resource should still exist");
        assert_eq!(updated_resource.is_draft, -1);
        // Link should still exist
        assert!(db.get_link("link-mark-1").unwrap().is_some());
    }

    #[test]
    fn test_upsert_deletes_removed_resource_without_links() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create a registry
        let registry = crate::models::v2::Registry {
            id: "reg-del-1".to_string(),
            name: "test-registry-del".to_string(),
            url: "https://example.com/reg-del.git".to_string(),
            local_path: tmp.path().to_string_lossy().to_string(),
            readonly: true,
            last_synced: None,
            has_remote_changes: false,
            has_local_changes: false,
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_registry(&registry).unwrap();

        // Create a skill on disk
        let skill_dir = tmp.path().join("skills").join("del-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Del Skill").unwrap();

        // Run upsert to populate DB
        upsert_registry_plugins(&db, &registry).unwrap();

        // Find the resource
        let resources = db.list_resources_by_scope(&ResourceScope::Registry).unwrap();
        assert_eq!(resources.len(), 1);
        let resource_id = resources[0].id.clone();

        // No link — don't create one

        // Remove the skill from disk
        fs::remove_dir_all(&skill_dir).unwrap();

        // Run upsert again
        upsert_registry_plugins(&db, &registry).unwrap();

        // Resource should be deleted (no links)
        assert!(db.get_resource(&resource_id).unwrap().is_none());
    }

    #[test]
    fn test_update_installed_resource() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create source file with "v2" content
        let source_dir = tmp.path().join("source").join("skills");
        fs::create_dir_all(&source_dir).unwrap();
        let source_file = source_dir.join("my-skill.md");
        fs::write(&source_file, "# Skill v2 - updated content").unwrap();

        // Create installed copy with "v1" content
        let installed_base = tmp.path().join("installed");
        let installed_skill_dir = installed_base.join("skills");
        fs::create_dir_all(&installed_skill_dir).unwrap();
        let installed_file = installed_skill_dir.join("my-skill.md");
        fs::write(&installed_file, "# Skill v1 - old content").unwrap();

        // Insert resource in DB
        let resource = crate::models::v2::Resource {
            id: "res-upd-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "my-skill.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: source_file.to_string_lossy().to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Insert a link
        let link = crate::models::v2::ResourceLink {
            id: "link-upd-1".to_string(),
            resource_id: "res-upd-1".to_string(),
            target_scope: "global".to_string(),
            target_path: "/tmp/target/skills/my-skill.md".to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: Some("old-hash".to_string()),
        };
        db.insert_link(&link).unwrap();

        // Call update
        crate::install_service::update_installed_with_base(&db, "res-upd-1", &installed_base).unwrap();

        // Verify installed file has new content
        let content = fs::read_to_string(&installed_file).unwrap();
        assert_eq!(content, "# Skill v2 - updated content");

        // Verify link's installed_hash was updated (not the old value)
        let updated_link = db.get_link("link-upd-1").unwrap().unwrap();
        assert_ne!(updated_link.installed_hash, Some("old-hash".to_string()));
        assert!(updated_link.installed_hash.is_some());
    }

    #[test]
    fn test_retain_as_library() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create installed copy
        let installed_base = tmp.path().join("installed");
        let installed_skill_dir = installed_base.join("skills");
        fs::create_dir_all(&installed_skill_dir).unwrap();
        fs::write(installed_skill_dir.join("orphan-skill"), "# Orphan Skill").unwrap();

        // Insert resource with is_draft=-1 (marked as removed)
        let resource = crate::models::v2::Resource {
            id: "res-retain-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "orphan-skill".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/old/registry/path/skills/orphan-skill".to_string(),
            content_hash: None,
            metadata: Some("plugin-id-123".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: -1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Call retain
        let result = crate::install::retain_as_library_with_base(&db, "res-retain-1", &installed_base).unwrap();

        // Verify scope changed to Library
        assert_eq!(result.scope, ResourceScope::Library);
        // Verify source_path points to installed copy
        assert!(result.source_path.contains("installed/skills/orphan-skill"));
        // Verify is_draft reset to 1
        assert_eq!(result.is_draft, 1);
        // Verify metadata cleared
        assert!(result.metadata.is_none());

        // Verify DB was updated
        let db_resource = db.get_resource("res-retain-1").unwrap().unwrap();
        assert_eq!(db_resource.scope, ResourceScope::Library);
        assert!(db_resource.metadata.is_none());
    }

    #[test]
    fn test_migrate_existing_symlinks_to_installed() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v7_to_v8().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();

        // Create registry skill
        let registry_skill = tmp.path().join("registry/skills/my-skill");
        fs::create_dir_all(&registry_skill).unwrap();
        fs::write(registry_skill.join("SKILL.md"), "# Skill").unwrap();

        // Create symlink pointing directly to registry (old behavior)
        let target_dir = tmp.path().join("claude/skills");
        fs::create_dir_all(&target_dir).unwrap();
        let target = target_dir.join("my-skill");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&registry_skill, &target).unwrap();

        // Insert resource and link (without installed_hash — old schema)
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO resources (id, resource_type, name, scope, source_path, created_at, updated_at)
             VALUES ('res1', 'skill', 'my-skill', 'registry', ?1, '2026-03-01', '2026-03-01')",
            [registry_skill.to_string_lossy().to_string()],
        ).unwrap();
        conn.execute(
            "INSERT INTO resource_links (id, resource_id, target_scope, target_path, link_type, created_at)
             VALUES ('link1', 'res1', 'global', ?1, 'symlink', '2026-03-01')",
            [target.to_string_lossy().to_string()],
        ).unwrap();
        drop(conn);

        let installed_base = tmp.path().join("installed");
        migrate_symlinks_to_installed(&db, &installed_base).unwrap();

        // File copied to installed/
        assert!(installed_base.join("skills/my-skill/SKILL.md").exists());

        // Symlink now points to installed/
        let resolved = fs::read_link(&target).unwrap();
        assert!(resolved.starts_with(&installed_base));

        // Link has installed_hash
        let link = db.get_link("link1").unwrap().unwrap();
        assert!(link.installed_hash.is_some());
    }

    #[test]
    fn test_list_installed_resources() {
        let db = Database::new_in_memory().unwrap();

        // Create a registry
        let registry = make_registry("reg1", "my-registry", "https://github.com/user/reg1.git", "/tmp/reg1");
        db.insert_registry(&registry).unwrap();

        // Create a registry plugin
        let plugin = RegistryPlugin {
            id: "plugin1".to_string(),
            registry_id: "reg1".to_string(),
            name: "my-plugin".to_string(),
            description: None,
            category: None,
            source_path: "/tmp/reg1/plugins/my-plugin".to_string(),
            source_type: "local".to_string(),
            source_url: None,
            homepage: None,
        };
        db.insert_registry_plugin(&plugin).unwrap();

        // Create a resource with metadata pointing to the plugin, and a link WITH installed_hash
        let res1 = Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Skill,
            name: "installed-skill".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/tmp/reg1/skills/installed-skill".to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 0,
            installed_from_id: None,
        };
        db.insert_resource(&res1).unwrap();

        let link1 = ResourceLink {
            id: "link1".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "global".to_string(),
            target_path: "/home/user/.claude/skills/installed-skill".to_string(),
            config_key: None,
            project_id: None,
            link_type: "copy".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: Some("installed_hash_1".to_string()),
        };
        db.insert_link(&link1).unwrap();

        // Create another resource with a link WITHOUT installed_hash (should be excluded)
        let res2 = Resource {
            id: "res2".to_string(),
            resource_type: ResourceType::Agent,
            name: "not-installed-agent".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/tmp/reg1/agents/not-installed-agent.md".to_string(),
            content_hash: Some("hash2".to_string()),
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 0,
            installed_from_id: None,
        };
        db.insert_resource(&res2).unwrap();

        let link2 = ResourceLink {
            id: "link2".to_string(),
            resource_id: "res2".to_string(),
            target_scope: "global".to_string(),
            target_path: "/home/user/.claude/agents/not-installed-agent.md".to_string(),
            config_key: None,
            project_id: None,
            link_type: "copy".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link2).unwrap();

        // Call list_installed_resources_impl
        let results = list_installed_resources_impl(&db).unwrap();

        // Should return both resources (all with active links)
        assert_eq!(results.len(), 2);
        // Results sorted by name: "installed-skill" < "not-installed-agent"
        assert_eq!(results[0].resource.name, "installed-skill");
        assert_eq!(results[0].links.len(), 1);
        assert_eq!(results[0].registry_name, Some("my-registry".to_string()));
        assert_eq!(results[1].resource.name, "not-installed-agent");
        assert_eq!(results[1].links[0].installed_hash, None);
    }
}
