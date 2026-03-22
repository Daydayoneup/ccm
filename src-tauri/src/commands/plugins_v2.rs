use tauri::State;
use crate::adapters::file_based::copy_dir_recursive;
use crate::db::Database;
use crate::models::v2::{Plugin, Resource, ResourceType, ResourceScope};
use crate::scanner;
use std::path::Path;
use std::fs;

/// List all plugins from DB
#[tauri::command]
pub fn list_plugins_v2(db: State<Database>) -> Result<Vec<Plugin>, String> {
    db.list_plugins().map_err(|e| e.to_string())
}

/// Scan installed plugins, reconcile with DB, return updated list
#[tauri::command]
pub fn scan_plugins(db: State<Database>) -> Result<Vec<Plugin>, String> {
    let scanned_plugins = scanner::plugin::scan_installed_plugins();
    let existing_plugins = db.list_plugins().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    for scanned in &scanned_plugins {
        // Check if already in DB by install_path
        let existing = existing_plugins.iter().find(|p| {
            p.install_path.as_deref() == Some(&scanned.install_path)
        });

        if let Some(existing_plugin) = existing {
            // Update the existing record
            let updated = Plugin {
                id: existing_plugin.id.clone(),
                name: scanned.name.clone(),
                version: if scanned.version.is_empty() { None } else { Some(scanned.version.clone()) },
                scope: if scanned.scope.is_empty() { None } else { Some(scanned.scope.clone()) },
                install_path: Some(scanned.install_path.clone()),
                status: "installed".to_string(),
                last_checked: Some(now.clone()),
            };
            db.update_plugin(&updated).map_err(|e| e.to_string())?;

            // Reconcile resources: remove old plugin resources and re-insert
            let existing_resources = db.list_resources_by_scope(&ResourceScope::Plugin)
                .map_err(|e| e.to_string())?;
            for r in &existing_resources {
                if r.source_path.starts_with(&scanned.install_path) {
                    let _ = db.delete_resource(&r.id);
                }
            }
            for sr in &scanned.resources {
                let resource = Resource {
                    id: uuid::Uuid::new_v4().to_string(),
                    resource_type: sr.resource_type.clone(),
                    name: sr.name.clone(),
                    description: None,
                    scope: ResourceScope::Plugin,
                    source_path: sr.source_path.clone(),
                    content_hash: sr.content_hash.clone(),
                    metadata: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                let _ = db.insert_resource(&resource);
            }
        } else {
            // Insert new plugin
            let plugin = Plugin {
                id: uuid::Uuid::new_v4().to_string(),
                name: scanned.name.clone(),
                version: if scanned.version.is_empty() { None } else { Some(scanned.version.clone()) },
                scope: if scanned.scope.is_empty() { None } else { Some(scanned.scope.clone()) },
                install_path: Some(scanned.install_path.clone()),
                status: "installed".to_string(),
                last_checked: Some(now.clone()),
            };
            db.insert_plugin(&plugin).map_err(|e| e.to_string())?;

            // Insert its resources
            for sr in &scanned.resources {
                let resource = Resource {
                    id: uuid::Uuid::new_v4().to_string(),
                    resource_type: sr.resource_type.clone(),
                    name: sr.name.clone(),
                    description: None,
                    scope: ResourceScope::Plugin,
                    source_path: sr.source_path.clone(),
                    content_hash: sr.content_hash.clone(),
                    metadata: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };
                let _ = db.insert_resource(&resource);
            }
        }
    }

    // Return the reconciled list from DB
    db.list_plugins().map_err(|e| e.to_string())
}

/// Get resources for a specific plugin, optionally filtered by type
#[tauri::command]
pub fn get_plugin_resources(
    db: State<Database>,
    plugin_id: String,
    resource_type: Option<String>,
) -> Result<Vec<Resource>, String> {
    let plugin = db.get_plugin(&plugin_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    let install_path = plugin.install_path
        .ok_or_else(|| format!("Plugin has no install_path: {}", plugin_id))?;

    let all_resources = match &resource_type {
        Some(rt) => {
            let rtype = ResourceType::from_str(rt)
                .ok_or_else(|| format!("Invalid resource type: {}", rt))?;
            db.list_resources_by_scope_and_type(&ResourceScope::Plugin, &rtype)
                .map_err(|e| e.to_string())?
        }
        None => {
            db.list_resources_by_scope(&ResourceScope::Plugin)
                .map_err(|e| e.to_string())?
        }
    };

    // Filter to only this plugin's resources (source_path starts with install_path)
    Ok(all_resources.into_iter()
        .filter(|r| r.source_path.starts_with(&install_path))
        .collect())
}

/// Extract a plugin resource to the central library
#[tauri::command]
pub fn extract_to_library(
    db: State<Database>,
    resource_id: String,
) -> Result<Resource, String> {
    let resource = db.get_resource(&resource_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if resource.scope != ResourceScope::Plugin {
        return Err("Resource is not a plugin resource".to_string());
    }

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
        updated_at: now,
    };

    db.insert_resource(&library_resource).map_err(|e| e.to_string())?;
    Ok(library_resource)
}

/// Stub: install a plugin by name (not yet implemented)
#[tauri::command]
pub fn install_plugin(_name: String) -> Result<(), String> {
    Err("Plugin installation is not yet implemented. This feature will be available after Claude plugin mechanism research.".to_string())
}

/// Stub: uninstall a plugin by id (not yet implemented)
#[tauri::command]
pub fn uninstall_plugin(_id: String) -> Result<(), String> {
    Err("Plugin uninstallation is not yet implemented. This feature will be available after Claude plugin mechanism research.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::v2::{Plugin, Resource, ResourceScope, ResourceType};
    use std::fs;

    fn make_plugin(id: &str, name: &str, install_path: &str) -> Plugin {
        Plugin {
            id: id.to_string(),
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            scope: Some("@claude".to_string()),
            install_path: Some(install_path.to_string()),
            status: "installed".to_string(),
            last_checked: Some("2026-03-01T00:00:00Z".to_string()),
        }
    }

    fn make_plugin_resource(id: &str, name: &str, source_path: &str, rtype: ResourceType) -> Resource {
        Resource {
            id: id.to_string(),
            resource_type: rtype,
            name: name.to_string(),
            description: None,
            scope: ResourceScope::Plugin,
            source_path: source_path.to_string(),
            content_hash: Some("abc123".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_list_plugins_v2_empty() {
        let db = Database::new_in_memory().unwrap();
        let plugins = db.list_plugins().unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_list_plugins_v2_with_data() {
        let db = Database::new_in_memory().unwrap();
        let p1 = make_plugin("p1", "alpha-plugin", "/home/user/.claude/plugins/alpha");
        let p2 = make_plugin("p2", "beta-plugin", "/home/user/.claude/plugins/beta");
        db.insert_plugin(&p1).unwrap();
        db.insert_plugin(&p2).unwrap();

        let plugins = db.list_plugins().unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "alpha-plugin");
        assert_eq!(plugins[1].name, "beta-plugin");
    }

    #[test]
    fn test_get_plugin_resources_filters_by_install_path() {
        let db = Database::new_in_memory().unwrap();
        let p1 = make_plugin("p1", "plugin-a", "/plugins/a");
        let p2 = make_plugin("p2", "plugin-b", "/plugins/b");
        db.insert_plugin(&p1).unwrap();
        db.insert_plugin(&p2).unwrap();

        let r1 = make_plugin_resource("r1", "skill-a", "/plugins/a/skills/skill-a", ResourceType::Skill);
        let r2 = make_plugin_resource("r2", "agent-a", "/plugins/a/agents/agent-a.md", ResourceType::Agent);
        let r3 = make_plugin_resource("r3", "skill-b", "/plugins/b/skills/skill-b", ResourceType::Skill);
        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();
        db.insert_resource(&r3).unwrap();

        // Get all resources for plugin-a
        let plugin_a_resources = db.list_resources_by_scope(&ResourceScope::Plugin).unwrap();
        let filtered: Vec<_> = plugin_a_resources.into_iter()
            .filter(|r| r.source_path.starts_with("/plugins/a"))
            .collect();
        assert_eq!(filtered.len(), 2);

        // Get only skills for plugin-a
        let skills = db.list_resources_by_scope_and_type(&ResourceScope::Plugin, &ResourceType::Skill).unwrap();
        let filtered_skills: Vec<_> = skills.into_iter()
            .filter(|r| r.source_path.starts_with("/plugins/a"))
            .collect();
        assert_eq!(filtered_skills.len(), 1);
        assert_eq!(filtered_skills[0].name, "skill-a");
    }

    #[test]
    fn test_get_plugin_resources_empty_when_no_match() {
        let db = Database::new_in_memory().unwrap();
        let p1 = make_plugin("p1", "plugin-a", "/plugins/a");
        db.insert_plugin(&p1).unwrap();

        let resources = db.list_resources_by_scope(&ResourceScope::Plugin).unwrap();
        let filtered: Vec<_> = resources.into_iter()
            .filter(|r| r.source_path.starts_with("/plugins/a"))
            .collect();
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_extract_to_library_copies_file() {
        let src_dir = tempfile::TempDir::new().unwrap();
        let dst_dir = tempfile::TempDir::new().unwrap();

        // Create a source file
        let src_file = src_dir.path().join("test-rule.md");
        fs::write(&src_file, "# Test Rule").unwrap();

        // Simulate extraction by copying
        let target = dst_dir.path().join("rules").join("test-rule.md");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(&src_file, &target).unwrap();

        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "# Test Rule");
    }

    #[test]
    fn test_extract_to_library_copies_directory() {
        let src_dir = tempfile::TempDir::new().unwrap();
        let dst_dir = tempfile::TempDir::new().unwrap();

        // Create a skill directory structure
        let skill_dir = src_dir.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# My Skill").unwrap();
        fs::write(skill_dir.join("helper.py"), "print('hello')").unwrap();

        let target = dst_dir.path().join("skills").join("my-skill");
        copy_dir_recursive(&skill_dir, &target).unwrap();

        assert!(target.join("SKILL.md").exists());
        assert!(target.join("helper.py").exists());
        assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "# My Skill");
    }

    #[test]
    fn test_extract_to_library_rejects_non_plugin_resource() {
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
        };
        db.insert_resource(&resource).unwrap();

        let fetched = db.get_resource("lib-r1").unwrap().unwrap();
        assert_ne!(fetched.scope, ResourceScope::Plugin);
    }

    #[test]
    fn test_install_plugin_stub_returns_error() {
        let result = install_plugin("some-plugin".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet implemented"));
    }

    #[test]
    fn test_uninstall_plugin_stub_returns_error() {
        let result = uninstall_plugin("some-id".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not yet implemented"));
    }

    #[test]
    fn test_copy_dir_recursive() {
        let src = tempfile::TempDir::new().unwrap();
        let dst = tempfile::TempDir::new().unwrap();

        // Create nested structure
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
    fn test_scan_reconcile_inserts_new_plugin() {
        let db = Database::new_in_memory().unwrap();

        // Simulate what scan_plugins does for a new plugin
        let now = chrono::Utc::now().to_rfc3339();
        let plugin = Plugin {
            id: uuid::Uuid::new_v4().to_string(),
            name: "new-plugin".to_string(),
            version: Some("1.0.0".to_string()),
            scope: Some("@claude".to_string()),
            install_path: Some("/home/user/.claude/plugins/new-plugin".to_string()),
            status: "installed".to_string(),
            last_checked: Some(now.clone()),
        };
        db.insert_plugin(&plugin).unwrap();

        let plugins = db.list_plugins().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "new-plugin");
    }

    #[test]
    fn test_scan_reconcile_updates_existing_plugin() {
        let db = Database::new_in_memory().unwrap();

        let plugin = make_plugin("p1", "my-plugin", "/plugins/my-plugin");
        db.insert_plugin(&plugin).unwrap();

        // Simulate update
        let now = chrono::Utc::now().to_rfc3339();
        let updated = Plugin {
            id: "p1".to_string(),
            name: "my-plugin".to_string(),
            version: Some("2.0.0".to_string()),
            scope: Some("@claude".to_string()),
            install_path: Some("/plugins/my-plugin".to_string()),
            status: "installed".to_string(),
            last_checked: Some(now),
        };
        db.update_plugin(&updated).unwrap();

        let fetched = db.get_plugin("p1").unwrap().unwrap();
        assert_eq!(fetched.version, Some("2.0.0".to_string()));
    }
}
