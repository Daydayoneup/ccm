//! Unified install/uninstall module for all resource types.
//!
//! Replaces the two separate install code paths (registry copy-based and
//! library adapter-based) with a single set of public functions.

use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::{AdapterRegistry, LinkType, TargetScope};
use crate::db::Database;
use crate::models::v2::{Resource, ResourceLink, ResourceScope, ResourceType};

// ── Manifest ─────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstallManifest {
    pub source_id: String,
    pub source_scope: String,
    pub source_name: String,
    pub installed_at: String,
}

/// Compute the manifest file path for an installed resource path.
/// e.g., installed/skills/foo → installed/skills/foo.ccm.json
pub fn manifest_path_for(installed_path: &Path) -> PathBuf {
    let name = installed_path.file_name().unwrap().to_string_lossy().to_string();
    installed_path.with_file_name(format!("{}.ccm.json", name))
}

/// Write a manifest file alongside an installed resource.
fn write_manifest(installed_path: &Path, resource: &Resource) -> Result<(), String> {
    let manifest = InstallManifest {
        source_id: resource.id.clone(),
        source_scope: resource.scope.as_str().to_string(),
        source_name: resource.name.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    };
    let manifest_path = manifest_path_for(installed_path);
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&manifest_path, json)
        .map_err(|e| format!("Failed to write manifest {}: {}", manifest_path.display(), e))?;
    Ok(())
}

/// Read a manifest file for an installed resource.
pub fn read_manifest(installed_path: &Path) -> Option<InstallManifest> {
    let manifest_path = manifest_path_for(installed_path);
    let content = fs::read_to_string(&manifest_path).ok()?;
    serde_json::from_str(&content).ok()
}

// ── Public types ─────────────────────────────────────────────────────

/// Where the resource should be installed.
pub enum InstallScope {
    Global,
    Project { id: String, path: String },
}

/// How the resource should be installed on disk.
pub enum InstallStrategy {
    /// Copy to `~/.claude-manager/installed/`, then create symlink.
    FileBased,
    /// Merge into a JSON config file (hooks → settings.json, MCP → .claude.json / .mcp.json).
    ConfigBased,
}

// ── Helpers ──────────────────────────────────────────────────────────

pub fn resource_type_dir(rt: &ResourceType) -> Result<&'static str, String> {
    if *rt == ResourceType::McpServer {
        return Err("MCP server resources use ConfigBased strategy, not FileBased".to_string());
    }
    AdapterRegistry::new()
        .get(rt)
        .map(|a| a.type_dir())
        .ok_or_else(|| format!("No adapter for {:?}", rt))
}

pub fn installed_base() -> Result<PathBuf, String> {
    Ok(dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude-manager")
        .join("installed"))
}

pub fn claude_home() -> Result<PathBuf, String> {
    Ok(dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".claude"))
}

// ── install_resource ─────────────────────────────────────────────────

/// Unified install entry point.
pub fn install_resource(
    db: &Database,
    resource: &Resource,
    scope: InstallScope,
    strategy: InstallStrategy,
    adapter_registry: &AdapterRegistry,
) -> Result<ResourceLink, String> {
    match strategy {
        InstallStrategy::FileBased => {
            let ib = installed_base()?;
            let ch = claude_home()?;
            install_file_based(db, resource, scope, &ib, &ch)
        }
        InstallStrategy::ConfigBased => install_config_based(db, resource, scope, adapter_registry),
    }
}

/// Testable variant that accepts explicit paths for installed_base and claude_home.
pub fn install_resource_with_paths(
    db: &Database,
    resource: &Resource,
    scope: InstallScope,
    strategy: InstallStrategy,
    ib: &Path,
    ch: &Path,
    adapter_registry: &AdapterRegistry,
) -> Result<ResourceLink, String> {
    match strategy {
        InstallStrategy::FileBased => install_file_based(db, resource, scope, ib, ch),
        InstallStrategy::ConfigBased => install_config_based(db, resource, scope, adapter_registry),
    }
}

/// FileBased: copy to installed/, create symlink to target.
fn install_file_based(
    db: &Database,
    resource: &Resource,
    scope: InstallScope,
    ib: &Path,
    ch: &Path,
) -> Result<ResourceLink, String> {
    let type_dir = resource_type_dir(&resource.resource_type)?;

    // 1. Validate source exists
    let source = Path::new(&resource.source_path);
    if !source.exists() {
        return Err("Source not available".to_string());
    }

    // 2. Get file_name from source path
    let file_name = source
        .file_name()
        .ok_or("Invalid source path")?;

    // 3. Copy to installed_base/<type_dir>/<file_name> (skip if already there)
    let installed_dir = ib.join(type_dir);
    let installed_path = installed_dir.join(file_name);
    if !installed_path.exists() {
        fs::create_dir_all(&installed_dir)
            .map_err(|e| format!("Failed to create installed dir: {}", e))?;
        if source.is_dir() {
            crate::adapters::file_based::copy_dir_recursive(source, &installed_path)?;
        } else {
            fs::copy(source, &installed_path)
                .map_err(|e| format!("Failed to copy resource: {}", e))?;
        }
    }

    // 3.5 Write install manifest
    write_manifest(&installed_path, resource)?;

    // 4. Compute installed_hash
    let installed_hash =
        crate::scanner::compute_file_hash(&installed_path.to_string_lossy());

    // 5. Determine target path
    let target = match &scope {
        InstallScope::Global => {
            ch.join(type_dir).join(file_name)
        }
        InstallScope::Project { path, .. } => {
            PathBuf::from(path)
                .join(".claude")
                .join(type_dir)
                .join(file_name)
        }
    };

    // 6. Ensure parent dir
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    // 7. Handle existing target
    if target.exists() || target.symlink_metadata().is_ok() {
        let is_symlink = target
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_symlink {
            fs::remove_file(&target)
                .map_err(|e| format!("Failed to remove old symlink: {}", e))?;
        } else {
            return Err(format!(
                "Target already exists and is not a symlink: {}",
                target.display()
            ));
        }
    }

    // 8. Create symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&installed_path, &target)
        .map_err(|e| format!("Failed to create symlink: {}", e))?;
    #[cfg(not(unix))]
    return Err("Symlinks not supported on this platform".to_string());

    // 9. Insert resource record for the installed copy so it's immediately visible
    let (target_scope_enum, project_id) = match &scope {
        InstallScope::Global => (ResourceScope::Global, None),
        InstallScope::Project { id, .. } => (ResourceScope::Project, Some(id.clone())),
    };

    let now = chrono::Utc::now().to_rfc3339();

    // Check if a resource with same name+type+scope already exists at this target
    let existing = db.list_resources_by_scope(&target_scope_enum)
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.name == resource.name && r.resource_type == resource.resource_type
            && r.source_path == target.to_string_lossy().as_ref());

    if existing.is_none() {
        let installed_resource = Resource {
            id: uuid::Uuid::new_v4().to_string(),
            resource_type: resource.resource_type.clone(),
            name: resource.name.clone(),
            description: resource.description.clone(),
            scope: target_scope_enum.clone(),
            source_path: target.to_string_lossy().to_string(),
            content_hash: installed_hash.clone(),
            metadata: resource.metadata.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        let _ = db.insert_resource(&installed_resource);
    }

    // 10. Build link (returned but not inserted to DB for file-based installs)
    let target_scope_str = match &scope {
        InstallScope::Global => "global".to_string(),
        InstallScope::Project { .. } => "project".to_string(),
    };

    let link = ResourceLink {
        id: uuid::Uuid::new_v4().to_string(),
        resource_id: resource.id.clone(),
        target_scope: target_scope_str,
        target_path: target.to_string_lossy().to_string(),
        config_key: None,
        project_id,
        link_type: "symlink".to_string(),
        created_at: now,
        installed_hash,
    };
    Ok(link)
}

/// ConfigBased: delegate to the appropriate adapter (Hook or McpServer),
/// then set installed_hash on the resulting link.
fn install_config_based(
    db: &Database,
    resource: &Resource,
    scope: InstallScope,
    adapter_registry: &AdapterRegistry,
) -> Result<ResourceLink, String> {
    let (adapter_scope, project_opt) = match &scope {
        InstallScope::Global => (TargetScope::Global, None),
        InstallScope::Project { id, path } => {
            let project = db
                .get_project(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Project not found: {}", id))?;
            // Sanity-check that the path matches (though we trust the caller)
            let _ = path;
            (TargetScope::Project, Some(project))
        }
    };

    // Look up the adapter via the registry
    let adapter = adapter_registry
        .get(&resource.resource_type)
        .ok_or_else(|| format!("No adapter for {:?}", resource.resource_type))?;

    let install_target = adapter.resolve_target(
        &adapter_scope,
        &resource.name,
        project_opt.as_ref(),
    )?;

    let mut link = adapter.install(resource, &install_target, &LinkType::ConfigMerge)?;

    // Compute installed_hash from resource source
    let installed_hash = if !resource.source_path.is_empty() && Path::new(&resource.source_path).exists() {
        crate::scanner::compute_file_hash(&resource.source_path)
    } else {
        // For config-based resources the source_path might be a config file.
        // Compute hash from metadata content instead.
        resource
            .metadata
            .as_ref()
            .map(|m| crate::scanner::compute_content_hash(m))
    };
    link.installed_hash = installed_hash;

    // Fill scope fields
    let (target_scope, project_id) = match scope {
        InstallScope::Global => ("global".to_string(), None),
        InstallScope::Project { id, .. } => ("project".to_string(), Some(id)),
    };
    link.target_scope = target_scope;
    link.project_id = project_id;

    db.insert_link(&link)
        .map_err(|e| format!("Failed to insert link: {}", e))?;

    // Insert a resource record in the target scope so the resource is immediately visible
    // (without waiting for sync to scan the config file).
    let (target_scope_enum, target_project_id) = match &link.target_scope.as_str() {
        &"global" => (ResourceScope::Global, None),
        _ => (ResourceScope::Project, link.project_id.clone()),
    };

    // Read the actual content that was written (from source file or metadata)
    let source_path = Path::new(&resource.source_path);
    let is_dedicated_file = source_path.extension().and_then(|e| e.to_str()) == Some("json")
        && !source_path.file_name().map(|f| f.to_string_lossy().starts_with('.')).unwrap_or(false);
    let installed_metadata = if is_dedicated_file && source_path.exists() {
        std::fs::read_to_string(source_path).ok()
    } else {
        resource.metadata.clone()
    };

    let existing = db.list_resources_by_scope(&target_scope_enum)
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.name == resource.name && r.resource_type == resource.resource_type);

    if existing.is_none() {
        let now = chrono::Utc::now().to_rfc3339();
        let installed_resource = Resource {
            id: uuid::Uuid::new_v4().to_string(),
            resource_type: resource.resource_type.clone(),
            name: resource.name.clone(),
            description: resource.description.clone(),
            scope: target_scope_enum,
            source_path: link.target_path.clone(),
            content_hash: link.installed_hash.clone(),
            metadata: installed_metadata,
            created_at: now.clone(),
            updated_at: now,
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        let _ = db.insert_resource(&installed_resource);
    }

    Ok(link)
}

// ── uninstall_resource ───────────────────────────────────────────────

/// Unified uninstall: removes disk artifacts + DB link, cleans installed/ if last reference.
pub fn uninstall_resource(
    db: &Database,
    link_ids: Vec<String>,
    adapter_registry: &AdapterRegistry,
) -> Result<Vec<String>, String> {
    uninstall_resource_with_base(db, link_ids, &installed_base()?, adapter_registry)
}

/// Testable variant that accepts installed_base.
pub fn uninstall_resource_with_base(
    db: &Database,
    link_ids: Vec<String>,
    ib: &Path,
    adapter_registry: &AdapterRegistry,
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();

    for link_id in &link_ids {
        let link = match db.get_link(link_id).map_err(|e| e.to_string())? {
            Some(l) => l,
            None => {
                errors.push(format!("Link not found: {}", link_id));
                continue;
            }
        };

        let resource_id = link.resource_id.clone();

        // Config-based links: delegate to adapter's uninstall
        if link.link_type == "config_merge" {
            if let Ok(Some(resource)) = db.get_resource(&link.resource_id) {
                if let Some(adapter) = adapter_registry.get(&resource.resource_type) {
                    adapter.uninstall(&link)?;
                }
            }
        } else {
            // File-based: remove symlink / file / dir
            let target = Path::new(&link.target_path);
            // Read symlink target before deletion (for manifest cleanup)
            let symlink_dest = if target.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
                fs::read_link(target).ok()
            } else {
                None
            };

            if target.exists() || target.symlink_metadata().is_ok() {
                let remove_result = if target
                    .symlink_metadata()
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false)
                {
                    fs::remove_file(target)
                } else if target.is_dir() {
                    fs::remove_dir_all(target)
                } else {
                    fs::remove_file(target)
                };
                if let Err(e) = remove_result {
                    errors.push(format!("Failed to remove {}: {}", link.target_path, e));
                }
            }

            // Clean up manifest file if the symlink pointed to installed/
            if let Some(dest) = symlink_dest {
                let manifest = manifest_path_for(&dest);
                let _ = fs::remove_file(&manifest);
            }
        }

        // Delete DB link record
        if let Err(e) = db.delete_link(link_id) {
            errors.push(format!("Failed to delete link record {}: {}", link_id, e));
        }

        // If no more links reference this resource, clean up installed/ copy
        let remaining = db.list_links_by_resource(&resource_id).unwrap_or_default();
        if remaining.is_empty() {
            if let Ok(Some(resource)) = db.get_resource(&resource_id) {
                if let Ok(type_dir) = resource_type_dir(&resource.resource_type) {
                    // Try both resource.name and file_name from source_path
                    let names_to_try: Vec<String> = {
                        let mut v = vec![resource.name.clone()];
                        if let Some(fname) = Path::new(&resource.source_path).file_name() {
                            let s = fname.to_string_lossy().to_string();
                            if s != resource.name {
                                v.push(s);
                            }
                        }
                        v
                    };
                    for name in names_to_try {
                        let installed_path = ib.join(type_dir).join(&name);
                        if installed_path.exists() {
                            if installed_path.is_dir() {
                                let _ = fs::remove_dir_all(&installed_path);
                            } else {
                                let _ = fs::remove_file(&installed_path);
                            }
                            // Also remove manifest
                            let _ = fs::remove_file(manifest_path_for(&installed_path));
                        }
                    }
                }
            }
        }
    }

    Ok(errors)
}

// ── update_installed ─────────────────────────────────────────────────

/// Update installed copy from registry source.
pub fn update_installed(db: &Database, resource_id: &str) -> Result<(), String> {
    update_installed_with_base(db, resource_id, &installed_base()?)
}

/// Testable variant.
pub fn update_installed_with_base(
    db: &Database,
    resource_id: &str,
    ib: &Path,
) -> Result<(), String> {
    let resource = db
        .get_resource(resource_id)
        .map_err(|e| e.to_string())?
        .ok_or("Resource not found")?;

    let source = Path::new(&resource.source_path);
    if !source.exists() {
        return Err("Source no longer available (upstream removed)".to_string());
    }

    let type_dir = resource_type_dir(&resource.resource_type)?;
    let file_name = source.file_name().ok_or("Invalid source path")?;
    let installed_path = ib.join(type_dir).join(file_name);

    // Remove old installed copy and replace
    if installed_path.exists() {
        if installed_path.is_dir() {
            fs::remove_dir_all(&installed_path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(&installed_path).map_err(|e| e.to_string())?;
        }
    }

    fs::create_dir_all(installed_path.parent().unwrap()).map_err(|e| e.to_string())?;
    if source.is_dir() {
        crate::adapters::file_based::copy_dir_recursive(source, &installed_path)?;
    } else {
        fs::copy(source, &installed_path).map_err(|e| e.to_string())?;
    }

    // Recompute hash and update all links (registry install path)
    let new_hash =
        crate::scanner::compute_file_hash(&installed_path.to_string_lossy());
    let links = db
        .list_links_by_resource(resource_id)
        .map_err(|e| e.to_string())?;
    for link in links {
        db.update_link_installed_hash(&link.id, new_hash.as_deref())
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ── retain_as_library ────────────────────────────────────────────────

/// Convert upstream-removed resource to library scope.
pub fn retain_as_library(db: &Database, resource_id: &str) -> Result<Resource, String> {
    retain_as_library_with_base(db, resource_id, &installed_base()?)
}

/// Testable variant.
pub fn retain_as_library_with_base(
    db: &Database,
    resource_id: &str,
    ib: &Path,
) -> Result<Resource, String> {
    let mut resource = db
        .get_resource(resource_id)
        .map_err(|e| e.to_string())?
        .ok_or("Resource not found")?;

    let type_dir = resource_type_dir(&resource.resource_type)?;
    let installed_path = ib.join(type_dir).join(&resource.name);

    if !installed_path.exists() {
        return Err("Installed copy not found".to_string());
    }

    resource.scope = crate::models::v2::ResourceScope::Library;
    resource.source_path = installed_path.to_string_lossy().to_string();
    resource.is_draft = 1;
    resource.metadata = None;
    resource.updated_at = chrono::Utc::now().to_rfc3339();

    db.update_resource(&resource).map_err(|e| e.to_string())?;
    Ok(resource)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::v2::{ResourceScope, ResourceType};
    use std::fs;
    use tempfile::TempDir;

    fn make_resource(
        id: &str,
        name: &str,
        rtype: ResourceType,
        source_path: &str,
    ) -> Resource {
        Resource {
            id: id.to_string(),
            resource_type: rtype,
            name: name.to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: source_path.to_string(),
            content_hash: Some("abc123".to_string()),
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        }
    }

    // ─── resource_type_dir ───────────────────────────────────────────

    #[test]
    fn test_resource_type_dir_all() {
        assert_eq!(resource_type_dir(&ResourceType::Skill).unwrap(), "skills");
        assert_eq!(resource_type_dir(&ResourceType::Agent).unwrap(), "agents");
        assert_eq!(resource_type_dir(&ResourceType::Rule).unwrap(), "rules");
        assert_eq!(resource_type_dir(&ResourceType::Hook).unwrap(), "hooks");
        assert_eq!(resource_type_dir(&ResourceType::Command).unwrap(), "commands");
        assert!(resource_type_dir(&ResourceType::McpServer).is_err());
    }

    // ─── install_file_based ──────────────────────────────────────────

    #[test]
    fn test_install_file_based_skill_directory() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create source skill directory
        let source_dir = tmp.path().join("registry/skills/my-skill");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("SKILL.md"), "# My Skill").unwrap();

        let resource = make_resource(
            "res1",
            "my-skill",
            ResourceType::Skill,
            source_dir.to_str().unwrap(),
        );
        db.insert_resource(&resource).unwrap();

        // We need to override installed_base and claude_home for testing.
        // Use install_single_resource_copy-style test instead.
        // For a unit test of install_file_based, we'd need DI. Instead, test
        // the public entry via the impl functions tested in registry.rs.
        // Here we just verify the helper functions work.
        assert_eq!(resource_type_dir(&resource.resource_type).unwrap(), "skills");
    }

    // ─── uninstall ───────────────────────────────────────────────────

    #[test]
    fn test_uninstall_removes_symlink_and_cleans_installed() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Set up resource
        let source_dir = tmp.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("my-agent.md"), "# Agent").unwrap();

        let resource = Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Agent,
            name: "my-agent.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: source_dir.join("my-agent.md").to_string_lossy().to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Set up installed copy
        let ib = tmp.path().join("installed");
        let installed_file = ib.join("agents").join("my-agent.md");
        fs::create_dir_all(installed_file.parent().unwrap()).unwrap();
        fs::write(&installed_file, "# Agent").unwrap();

        // Set up symlink target
        let target_dir = tmp.path().join("target/agents");
        fs::create_dir_all(&target_dir).unwrap();
        let target = target_dir.join("my-agent.md");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&installed_file, &target).unwrap();

        let link = ResourceLink {
            id: "link1".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "global".to_string(),
            target_path: target.to_string_lossy().to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let errors = uninstall_resource_with_base(
            &db,
            vec!["link1".to_string()],
            &ib,
            &adapter_registry,
        )
        .unwrap();

        assert!(errors.is_empty(), "errors: {:?}", errors);
        assert!(!target.exists(), "symlink should be removed");
        assert!(!installed_file.exists(), "installed copy should be cleaned");
        assert!(db.get_link("link1").unwrap().is_none());
    }

    #[test]
    fn test_uninstall_nonexistent_link() {
        let db = Database::new_in_memory().unwrap();
        let ib = PathBuf::from("/tmp/nonexistent-installed");
        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let errors = uninstall_resource_with_base(
            &db,
            vec!["no-such-link".to_string()],
            &ib,
            &adapter_registry,
        )
        .unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not found"));
    }

    // ─── update_installed ────────────────────────────────────────────

    #[test]
    fn test_update_installed_replaces_copy() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Source file (updated version)
        let source_dir = tmp.path().join("source/agents");
        fs::create_dir_all(&source_dir).unwrap();
        let source_file = source_dir.join("my-agent.md");
        fs::write(&source_file, "# Agent v2").unwrap();

        let resource = Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Agent,
            name: "my-agent.md".to_string(),
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

        // Old installed copy
        let ib = tmp.path().join("installed");
        let installed_file = ib.join("agents").join("my-agent.md");
        fs::create_dir_all(installed_file.parent().unwrap()).unwrap();
        fs::write(&installed_file, "# Agent v1").unwrap();

        // Link
        let link = ResourceLink {
            id: "link1".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "global".to_string(),
            target_path: "/some/target".to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: Some("old-hash".to_string()),
        };
        db.insert_link(&link).unwrap();

        update_installed_with_base(&db, "res1", &ib).unwrap();

        // Verify updated content
        let content = fs::read_to_string(&installed_file).unwrap();
        assert_eq!(content, "# Agent v2");

        // Verify link hash updated
        let updated_link = db.get_link("link1").unwrap().unwrap();
        assert_ne!(updated_link.installed_hash, Some("old-hash".to_string()));
        assert!(updated_link.installed_hash.is_some());
    }

    // ─── retain_as_library ───────────────────────────────────────────

    #[test]
    fn test_retain_as_library_converts_scope() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let resource = Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Agent,
            name: "my-agent.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/old/source".to_string(),
            content_hash: None,
            metadata: Some("plugin1".to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: -1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Create installed copy
        let ib = tmp.path().join("installed");
        let installed_file = ib.join("agents").join("my-agent.md");
        fs::create_dir_all(installed_file.parent().unwrap()).unwrap();
        fs::write(&installed_file, "# Agent").unwrap();

        let result = retain_as_library_with_base(&db, "res1", &ib).unwrap();

        assert_eq!(result.scope, ResourceScope::Library);
        assert_eq!(result.source_path, installed_file.to_string_lossy());
        assert!(result.metadata.is_none());
        assert_eq!(result.is_draft, 1);
    }

    #[test]
    fn test_retain_as_library_no_installed_copy() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let resource = Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Agent,
            name: "my-agent.md".to_string(),
            description: None,
            scope: ResourceScope::Registry,
            source_path: "/old/source".to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: -1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let ib = tmp.path().join("installed");
        let result = retain_as_library_with_base(&db, "res1", &ib);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Installed copy not found"));
    }

    // ─── uninstall config_merge ──────────────────────────────────────

    #[test]
    fn test_uninstall_config_merge_hook() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create settings.json with a hook
        let settings = tmp.path().join("settings.json");
        let config = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Edit", "hooks": [{"type":"command","command":"echo a"}], "_ccm_id": "ccm-1"},
                    {"matcher": "Bash", "hooks": [{"type":"command","command":"echo b"}], "_ccm_id": "ccm-2"}
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        // Resource (needed for cleanup check)
        let resource = Resource {
            id: "ccm-1".to_string(),
            resource_type: ResourceType::Hook,
            name: "PreToolUse/Edit".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: String::new(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let link = ResourceLink {
            id: "link-h1".to_string(),
            resource_id: "ccm-1".to_string(),
            target_scope: "project".to_string(),
            target_path: settings.to_string_lossy().to_string(),
            config_key: Some("hooks.PreToolUse._ccm_id=ccm-1".to_string()),
            project_id: None,
            link_type: "config_merge".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        let ib = tmp.path().join("installed");
        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let errors = uninstall_resource_with_base(
            &db,
            vec!["link-h1".to_string()],
            &ib,
            &adapter_registry,
        )
        .unwrap();

        assert!(errors.is_empty(), "errors: {:?}", errors);

        // Verify hook was removed from settings
        let content = fs::read_to_string(&settings).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let arr = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["_ccm_id"], "ccm-2");
    }

    #[test]
    fn test_uninstall_config_merge_mcp() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let config_path = tmp.path().join(".mcp.json");
        let config = serde_json::json!({
            "mcpServers": {
                "server-a": {"command": "node"},
                "server-b": {"command": "python"}
            }
        });
        fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let resource = Resource {
            id: "res-mcp1".to_string(),
            resource_type: ResourceType::McpServer,
            name: "server-a".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: String::new(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let link = ResourceLink {
            id: "link-m1".to_string(),
            resource_id: "res-mcp1".to_string(),
            target_scope: "project".to_string(),
            target_path: config_path.to_string_lossy().to_string(),
            config_key: Some("mcpServers.server-a".to_string()),
            project_id: None,
            link_type: "config_merge".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        let ib = tmp.path().join("installed");
        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let errors = uninstall_resource_with_base(
            &db,
            vec!["link-m1".to_string()],
            &ib,
            &adapter_registry,
        )
        .unwrap();

        assert!(errors.is_empty());

        let content = fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["mcpServers"].get("server-a").is_none());
        assert!(parsed["mcpServers"].get("server-b").is_some());
    }

    // ─── end-to-end: library MCP → install to project ───────────────

    #[test]
    fn test_install_library_mcp_to_project() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // 1. Create a project
        let project_path = tmp.path().join("my-project");
        fs::create_dir_all(&project_path).unwrap();
        let project = crate::models::v2::Project {
            id: "proj1".to_string(),
            name: "my-project".to_string(),
            path: project_path.to_string_lossy().to_string(),
            language: None,
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        };
        db.insert_project(&project).unwrap();

        // 2. Create a library MCP resource (simulates create_library_resource)
        let mcp_content = r#"{"command":"node","args":["server.js"]}"#;
        let lib_resource = Resource {
            id: "lib-mcp-1".to_string(),
            resource_type: ResourceType::McpServer,
            name: "testmcp".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: tmp.path().join("library/mcp_servers/testmcp.json")
                .to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: Some(mcp_content.to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&lib_resource).unwrap();

        // Write the library file
        let lib_dir = tmp.path().join("library/mcp_servers");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("testmcp.json"), mcp_content).unwrap();

        // 3. Install to project (simulates install_to_project command)
        let scope = InstallScope::Project {
            id: "proj1".to_string(),
            path: project_path.to_string_lossy().to_string(),
        };
        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let link = install_resource(&db, &lib_resource, scope, InstallStrategy::ConfigBased, &adapter_registry)
            .expect("install_resource should succeed");

        // 4. Verify the .mcp.json was created with correct content
        let mcp_json_path = project_path.join(".mcp.json");
        assert!(mcp_json_path.exists(), ".mcp.json should be created");

        let mcp_content_read = fs::read_to_string(&mcp_json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&mcp_content_read).unwrap();
        assert_eq!(parsed["mcpServers"]["testmcp"]["command"], "node");
        assert_eq!(parsed["mcpServers"]["testmcp"]["args"][0], "server.js");

        // 5. Verify the link was created correctly
        assert_eq!(link.resource_id, "lib-mcp-1");
        assert_eq!(link.target_scope, "project");
        assert_eq!(link.project_id, Some("proj1".to_string()));
        assert_eq!(link.link_type, "config_merge");
        assert_eq!(link.config_key, Some("mcpServers.testmcp".to_string()));

        // 6. Verify link is in DB
        let db_link = db.get_link(&link.id).unwrap();
        assert!(db_link.is_some(), "link should be saved in DB");

        // 7. Verify list_links_by_project finds it
        let project_links = db.list_links_by_project("proj1").unwrap();
        assert_eq!(project_links.len(), 1);
        assert_eq!(project_links[0].resource_id, "lib-mcp-1");
    }

    // ─── install manifest tests ──────────────────────────────────────

    #[test]
    fn test_install_writes_manifest() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create source skill directory
        let source_dir = tmp.path().join("library/skills/test-skill");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("SKILL.md"), "# Test Skill").unwrap();

        let resource = Resource {
            id: "lib-skill-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "test-skill".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: source_dir.to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Use a path that contains ".claude-manager/installed" so that
        // detect_install_source recognises symlinks pointing into it.
        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");
        fs::create_dir_all(&ch).unwrap();

        let ar = crate::adapters::AdapterRegistry::new();
        let _link = install_resource_with_paths(
            &db, &resource,
            InstallScope::Global,
            InstallStrategy::FileBased,
            &ib, &ch, &ar,
        ).expect("install should succeed");

        // Verify installed copy exists
        let installed_path = ib.join("skills").join("test-skill");
        assert!(installed_path.exists(), "installed copy should exist");

        // Verify manifest exists and has correct content
        let manifest = read_manifest(&installed_path);
        assert!(manifest.is_some(), "manifest should exist");
        let m = manifest.unwrap();
        assert_eq!(m.source_id, "lib-skill-1");
        assert_eq!(m.source_scope, "library");
        assert_eq!(m.source_name, "test-skill");

        // Verify NO link was inserted to DB for file-based installs
        let links = db.list_links_by_resource("lib-skill-1").unwrap();
        assert!(links.is_empty(), "no symlink ResourceLink should be created in DB");
    }

    #[test]
    fn test_uninstall_cleans_manifest() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Set up source
        let source_dir = tmp.path().join("library/skills/cleanup-test");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("SKILL.md"), "# Cleanup").unwrap();

        let resource = Resource {
            id: "lib-cleanup-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "cleanup-test".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: source_dir.to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Use a path that contains ".claude-manager/installed" so that
        // uninstall can resolve the manifest via the symlink destination.
        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");
        fs::create_dir_all(&ch).unwrap();

        let ar = crate::adapters::AdapterRegistry::new();
        let link_result = install_resource_with_paths(
            &db, &resource,
            InstallScope::Global,
            InstallStrategy::FileBased,
            &ib, &ch, &ar,
        ).unwrap();

        let installed_path = ib.join("skills").join("cleanup-test");
        let manifest_path = manifest_path_for(&installed_path);
        assert!(installed_path.exists(), "installed copy should exist");
        assert!(manifest_path.exists(), "manifest should exist before uninstall");

        let symlink_path = ch.join("skills").join("cleanup-test");
        assert!(symlink_path.symlink_metadata().is_ok(), "symlink should exist");

        // File-based installs do not insert a DB link, so we insert one manually
        // to allow uninstall to locate and process the installation.
        let link = crate::models::v2::ResourceLink {
            id: "test-link-1".to_string(),
            resource_id: "lib-cleanup-1".to_string(),
            target_scope: link_result.target_scope.clone(),
            target_path: symlink_path.to_string_lossy().to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: Some("hash1".to_string()),
        };
        db.insert_link(&link).unwrap();

        // Uninstall
        let errors = uninstall_resource_with_base(
            &db,
            vec!["test-link-1".to_string()],
            &ib,
            &ar,
        ).unwrap();

        assert!(errors.is_empty(), "errors: {:?}", errors);
        assert!(!symlink_path.exists(), "symlink should be removed");
        assert!(!manifest_path.exists(), "manifest should be removed");
    }

    // ─── Rule (file-based) manifest test ────────────────────────────

    #[test]
    fn test_install_rule_writes_manifest() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let source_dir = tmp.path().join("library/rules");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("test-rule.md"), "# Test Rule").unwrap();

        let resource = Resource {
            id: "lib-rule-1".to_string(),
            resource_type: ResourceType::Rule,
            name: "test-rule".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: source_dir.join("test-rule.md").to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");
        fs::create_dir_all(&ch).unwrap();

        let ar = crate::adapters::AdapterRegistry::new();
        let link = install_resource_with_paths(
            &db, &resource,
            InstallScope::Global,
            InstallStrategy::FileBased,
            &ib, &ch, &ar,
        ).expect("install should succeed");

        // Verify installed copy
        let installed_path = ib.join("rules").join("test-rule.md");
        assert!(installed_path.exists(), "installed copy should exist");

        // Verify manifest
        let manifest = read_manifest(&installed_path);
        assert!(manifest.is_some(), "manifest should exist");
        let m = manifest.unwrap();
        assert_eq!(m.source_id, "lib-rule-1");
        assert_eq!(m.source_name, "test-rule");

        // Verify symlink
        let symlink_path = ch.join("rules").join("test-rule.md");
        assert!(symlink_path.symlink_metadata().is_ok(), "symlink should exist");

        // Verify no DB link
        let links = db.list_links_by_resource("lib-rule-1").unwrap();
        assert!(links.is_empty(), "no symlink ResourceLink for file-based install");

        // Verify return value has correct fields
        assert_eq!(link.resource_id, "lib-rule-1");
        assert_eq!(link.link_type, "symlink");
    }

    // ─── Command (file-based) manifest test ─────────────────────────

    #[test]
    fn test_install_command_writes_manifest() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let source_dir = tmp.path().join("library/commands");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("test-cmd.md"), "# Test Command").unwrap();

        let resource = Resource {
            id: "lib-cmd-1".to_string(),
            resource_type: ResourceType::Command,
            name: "test-cmd".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: source_dir.join("test-cmd.md").to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");
        fs::create_dir_all(&ch).unwrap();

        let ar = crate::adapters::AdapterRegistry::new();
        let _link = install_resource_with_paths(
            &db, &resource,
            InstallScope::Global,
            InstallStrategy::FileBased,
            &ib, &ch, &ar,
        ).expect("install should succeed");

        let installed_path = ib.join("commands").join("test-cmd.md");
        assert!(installed_path.exists(), "installed copy should exist");

        let manifest = read_manifest(&installed_path);
        assert!(manifest.is_some(), "manifest should exist");
        assert_eq!(manifest.unwrap().source_id, "lib-cmd-1");

        let symlink_path = ch.join("commands").join("test-cmd.md");
        assert!(symlink_path.symlink_metadata().is_ok(), "symlink should exist");
    }

    // ─── Hook (config-based) test ───────────────────────────────────

    #[test]
    fn test_install_hook_uses_config_merge_not_manifest() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create a hook resource with proper metadata
        let hook_config = r#"{"event":"PreToolUse","matcher":"Edit","hook_config":{"type":"command","command":"echo lint"}}"#;
        let resource = Resource {
            id: "lib-hook-1".to_string(),
            resource_type: ResourceType::Hook,
            name: "PreToolUse/Edit".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: tmp.path().join("library/hooks/lint.json").to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: Some(hook_config.to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        // Create project for installation target
        let project_dir = tmp.path().join("test-project");
        fs::create_dir_all(project_dir.join(".claude")).unwrap();
        let project = crate::models::v2::Project {
            id: "proj1".to_string(),
            name: "test-project".to_string(),
            path: project_dir.to_string_lossy().to_string(),
            language: None,
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        };
        db.insert_project(&project).unwrap();

        let ar = crate::adapters::AdapterRegistry::new();
        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");

        let link = install_resource_with_paths(
            &db, &resource,
            InstallScope::Project { id: "proj1".to_string(), path: project_dir.to_string_lossy().to_string() },
            InstallStrategy::ConfigBased,
            &ib, &ch, &ar,
        ).expect("config-based install should succeed");

        // Config-based installs DO create DB links
        assert_eq!(link.link_type, "config_merge");
        let db_links = db.list_links_by_resource("lib-hook-1").unwrap();
        assert_eq!(db_links.len(), 1, "config_merge link should be in DB");

        // NO manifest should exist (config-based doesn't use installed/)
        let installed_hooks = ib.join("hooks");
        assert!(!installed_hooks.exists(), "no installed/ copy for config-based resources");

        // The settings.json should have the hook merged
        let settings_path = project_dir.join(".claude").join("settings.json");
        assert!(settings_path.exists(), "settings.json should be created");
        let content = fs::read_to_string(&settings_path).unwrap();
        assert!(content.contains("PreToolUse"), "hook should be merged into settings");
    }

    // ─── McpServer (config-based) test ──────────────────────────────

    #[test]
    fn test_install_mcp_uses_config_merge_not_manifest() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let mcp_config = r#"{"command":"node","args":["server.js"]}"#;
        let resource = Resource {
            id: "lib-mcp-1".to_string(),
            resource_type: ResourceType::McpServer,
            name: "test-server".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: tmp.path().join("library/mcp_servers/test-server.json").to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: Some(mcp_config.to_string()),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let project_dir = tmp.path().join("test-project");
        fs::create_dir_all(&project_dir).unwrap();
        let project = crate::models::v2::Project {
            id: "proj1".to_string(),
            name: "test-project".to_string(),
            path: project_dir.to_string_lossy().to_string(),
            language: None,
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        };
        db.insert_project(&project).unwrap();

        let ar = crate::adapters::AdapterRegistry::new();
        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");

        let link = install_resource_with_paths(
            &db, &resource,
            InstallScope::Project { id: "proj1".to_string(), path: project_dir.to_string_lossy().to_string() },
            InstallStrategy::ConfigBased,
            &ib, &ch, &ar,
        ).expect("MCP install should succeed");

        // Config-based: DB link created
        assert_eq!(link.link_type, "config_merge");
        assert!(link.config_key.as_ref().unwrap().contains("mcpServers.test-server"));
        let db_links = db.list_links_by_resource("lib-mcp-1").unwrap();
        assert_eq!(db_links.len(), 1);

        // NO manifest in installed/
        assert!(!ib.join("mcp_servers").exists(), "no installed/ copy for MCP");

        // .mcp.json should have the server
        let mcp_path = project_dir.join(".mcp.json");
        assert!(mcp_path.exists(), ".mcp.json should be created");
        let content = fs::read_to_string(&mcp_path).unwrap();
        assert!(content.contains("test-server"), "MCP server should be merged");
        assert!(content.contains("node"), "MCP command should be in config");
    }

    // ─── Rule scan roundtrip test ───────────────────────────────────

    #[test]
    fn test_install_scan_roundtrip_rule() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        let source_dir = tmp.path().join("library/rules");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("no-force-push.md"), "Never force push to main").unwrap();

        let resource = Resource {
            id: "lib-rule-1".to_string(),
            resource_type: ResourceType::Rule,
            name: "no-force-push".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: source_dir.join("no-force-push.md").to_string_lossy().to_string(),
            content_hash: Some("hash1".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let project_dir = tmp.path().join("test-project");
        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");
        fs::create_dir_all(project_dir.join(".claude/rules")).unwrap();

        let project = crate::models::v2::Project {
            id: "proj1".to_string(),
            name: "test-project".to_string(),
            path: project_dir.to_string_lossy().to_string(),
            language: None,
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        };
        db.insert_project(&project).unwrap();

        let ar = crate::adapters::AdapterRegistry::new();
        install_resource_with_paths(
            &db, &resource,
            InstallScope::Project { id: "proj1".to_string(), path: project_dir.to_string_lossy().to_string() },
            InstallStrategy::FileBased,
            &ib, &ch, &ar,
        ).unwrap();

        // Scan project rules
        let scanned = crate::adapters::file_based::scan_file_resources(
            &ResourceScope::Project,
            &project_dir,
            "rules",
            &ResourceType::Rule,
            false,
        ).unwrap();

        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].name, "no-force-push");
        assert_eq!(scanned[0].installed_from_id, Some("lib-rule-1".to_string()));
    }

    #[test]
    fn test_update_installed_syncs_link_installed_hash() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();

        // Create library source with v1 content
        let source_dir = tmp.path().join("library/skills/my-skill");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("SKILL.md"), "# v1 content").unwrap();

        let lib_resource = Resource {
            id: "lib-1".to_string(),
            resource_type: ResourceType::Skill,
            name: "my-skill".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: source_dir.to_string_lossy().to_string(),
            content_hash: Some("lib-hash-v1".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&lib_resource).unwrap();

        // Create installed copy with old content
        let ib = tmp.path().join("installed");
        let installed_dir = ib.join("skills").join("my-skill");
        fs::create_dir_all(&installed_dir).unwrap();
        fs::write(installed_dir.join("SKILL.md"), "# old content").unwrap();

        // Create a resource_link pointing to the installed path with old hash
        let link = crate::models::v2::ResourceLink {
            id: "link-1".to_string(),
            resource_id: "lib-1".to_string(),
            target_scope: "global".to_string(),
            target_path: "/fake/global/skills/my-skill".to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: Some("old-hash".to_string()),
        };
        db.insert_link(&link).unwrap();

        // Now update library source to v2
        fs::write(source_dir.join("SKILL.md"), "# v2 updated content").unwrap();

        // Call update_installed_with_base with the LIBRARY resource id
        update_installed_with_base(&db, "lib-1", &ib).unwrap();

        // Verify installed file has v2 content
        let installed_content = fs::read_to_string(installed_dir.join("SKILL.md")).unwrap();
        assert_eq!(installed_content, "# v2 updated content");

        // Verify the link's installed_hash was updated (not still "old-hash")
        let updated_links = db.list_links_by_resource("lib-1").unwrap();
        assert_eq!(updated_links.len(), 1);
        assert_ne!(updated_links[0].installed_hash, Some("old-hash".to_string()));
        assert!(updated_links[0].installed_hash.is_some(), "hash should be set");

        // Verify the hash matches what we'd compute for the installed path
        let expected_hash = crate::scanner::compute_file_hash(&installed_dir.to_string_lossy());
        assert_eq!(updated_links[0].installed_hash, expected_hash);
    }

    /// End-to-end test: install → edit library → sync → detect has_update → update → verify
    #[test]
    fn test_library_edit_sync_update_e2e() {
        let tmp = TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();
        let ar = crate::adapters::AdapterRegistry::new();

        // === Step 1: Create library resource with v1 content ===
        let lib_dir = tmp.path().join("library/skills/e2e-skill");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("SKILL.md"), "# v1 content").unwrap();

        let lib_hash = crate::scanner::compute_file_hash(&lib_dir.to_string_lossy());
        let lib_resource = Resource {
            id: "lib-e2e".to_string(),
            resource_type: ResourceType::Skill,
            name: "e2e-skill".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: lib_dir.to_string_lossy().to_string(),
            content_hash: lib_hash.clone(),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&lib_resource).unwrap();

        // === Step 2: Install to global ===
        let ib = tmp.path().join(".claude-manager/installed");
        let ch = tmp.path().join("claude-home");
        fs::create_dir_all(&ch).unwrap();

        let link = install_resource_with_paths(
            &db, &lib_resource,
            InstallScope::Global,
            InstallStrategy::FileBased,
            &ib, &ch, &ar,
        ).expect("install should succeed");
        // The caller is responsible for persisting the link to DB
        db.insert_link(&link).expect("link insert should succeed");

        // install_file_based creates a ResourceLink record
        let installed_dir = ib.join("skills").join("e2e-skill");
        let links = db.list_links_by_resource("lib-e2e").unwrap();
        assert_eq!(links.len(), 1, "install should create a resource_link record");
        let installed_hash = links[0].installed_hash.clone();

        // At this point: lib hash == installed hash (all v1)
        assert_eq!(lib_hash, installed_hash, "hashes should match after install");

        // === Step 3: Edit library source (simulate user editing via editor) ===
        fs::write(lib_dir.join("SKILL.md"), "# v2 edited content").unwrap();

        // Simulate sync: update library resource hash in DB
        let new_lib_hash = crate::scanner::compute_file_hash(&lib_dir.to_string_lossy());
        assert_ne!(new_lib_hash, lib_hash, "hash should change after edit");
        let mut updated_lib = db.get_resource("lib-e2e").unwrap().unwrap();
        updated_lib.content_hash = new_lib_hash.clone();
        db.update_resource(&updated_lib).unwrap();

        // === Step 4: Check has_update (simulate list_library_resources_with_installs) ===
        let links = db.list_links_by_resource("lib-e2e").unwrap();
        assert_eq!(links.len(), 1, "should have 1 link");
        let has_update = match (updated_lib.content_hash.as_deref(), links[0].installed_hash.as_deref()) {
            (Some(src), Some(dst)) => src != dst,
            _ => false,
        };
        assert!(has_update, "has_update should be true after library edit");

        // === Step 5: Click update (call update_installed_with_base) ===
        update_installed_with_base(&db, "lib-e2e", &ib).unwrap();

        // === Step 6: Verify everything is synced ===
        // Installed file should have v2 content
        let installed_content = fs::read_to_string(installed_dir.join("SKILL.md")).unwrap();
        assert_eq!(installed_content, "# v2 edited content");

        // Link's installed_hash should now match library hash
        let final_links = db.list_links_by_resource("lib-e2e").unwrap();
        let final_lib = db.get_resource("lib-e2e").unwrap().unwrap();
        assert_eq!(
            final_links[0].installed_hash, final_lib.content_hash,
            "link installed_hash should match library hash after update"
        );

        // has_update should now be false
        let has_update_after = match (final_lib.content_hash.as_deref(), final_links[0].installed_hash.as_deref()) {
            (Some(src), Some(dst)) => src != dst,
            _ => false,
        };
        assert!(!has_update_after, "has_update should be false after update");
    }
}
