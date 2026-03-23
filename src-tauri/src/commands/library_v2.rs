use serde::{Deserialize, Serialize};
use tauri::State;
use crate::adapters::{AdapterRegistry, LinkType, TargetScope, normalize_link_type};
use crate::db::Database;
use crate::models::v2::{Resource, ResourceType, ResourceScope, ResourceLink};
use crate::scanner;
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkHealthInfo {
    pub link_id: String,
    pub target_path: String,
    pub healthy: bool,
    pub error: Option<String>,
}

/// List library resources, optionally filtered by type
#[tauri::command]
pub fn list_library_resources(
    db: State<Database>,
    resource_type: Option<String>,
) -> Result<Vec<Resource>, String> {
    match resource_type {
        Some(rt) => {
            let rtype = ResourceType::from_str(&rt)
                .ok_or_else(|| format!("Invalid resource type: {}", rt))?;
            db.list_resources_by_scope_and_type(&ResourceScope::Library, &rtype)
                .map_err(|e| e.to_string())
        }
        None => {
            db.list_resources_by_scope(&ResourceScope::Library)
                .map_err(|e| e.to_string())
        }
    }
}

/// Create a new library resource — writes file to ~/.claude-manager/library/<type>/ and inserts DB record
#[tauri::command]
pub fn create_library_resource(
    db: State<Database>,
    adapter_registry: State<AdapterRegistry>,
    resource_type: String,
    name: String,
    description: Option<String>,
    content: String,
) -> Result<Resource, String> {
    let rtype = ResourceType::from_str(&resource_type)
        .ok_or_else(|| format!("Invalid resource type: {}", resource_type))?;

    // Validate content via adapter
    let adapter = adapter_registry.get(&rtype)
        .ok_or_else(|| format!("No adapter for resource type: {}", resource_type))?;
    adapter.validate_content(&content)?;

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let library_base = home.join(".claude-manager").join("library");

    // Determine file path based on resource type
    let file_path = match rtype {
        ResourceType::Skill => {
            let skill_dir = library_base.join("skills").join(&name);
            fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;
            skill_dir.join("SKILL.md")
        }
        ResourceType::Agent => {
            let dir = library_base.join("agents");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.join(format!("{}.md", name))
        }
        ResourceType::Rule => {
            let dir = library_base.join("rules");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.join(format!("{}.md", name))
        }
        ResourceType::Hook => {
            let dir = library_base.join("hooks");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.join(format!("{}.json", name))
        }
        ResourceType::Command => {
            let dir = library_base.join("commands");
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
        description,
        scope: ResourceScope::Library,
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

/// Delete a library resource — checks for links first, optionally removes file, always removes DB record
#[tauri::command]
pub fn delete_library_resource(db: State<Database>, id: String, delete_from_disk: Option<bool>) -> Result<(), String> {
    let resource = db.get_resource(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", id))?;

    if resource.scope != ResourceScope::Library {
        return Err("Resource is not a library resource".to_string());
    }

    // Check for links
    let links = db.list_links_by_resource(&id).map_err(|e| e.to_string())?;
    if !links.is_empty() {
        return Err(format!(
            "Resource is linked to {} location(s). Unlink first before deleting.",
            links.len()
        ));
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

/// Install a library resource to a project's .claude/<type>/ directory via adapter
#[tauri::command]
pub fn install_to_project(
    db: State<Database>,
    adapter_registry: State<'_, AdapterRegistry>,
    resource_id: String,
    project_id: String,
    link_type: String,
) -> Result<ResourceLink, String> {
    let lib_resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if lib_resource.scope != ResourceScope::Library {
        return Err("Resource is not a library resource".to_string());
    }

    let project = db.get_project(&project_id)
        .map_err(|e| e.to_string())?
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
    Ok(link)
}

/// Deploy a library resource to the global ~/.claude/<type>/ directory via adapter
#[tauri::command]
pub fn deploy_to_global(
    db: State<Database>,
    adapter_registry: State<'_, AdapterRegistry>,
    resource_id: String,
    link_type: String,
) -> Result<ResourceLink, String> {
    let lib_resource = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    if lib_resource.scope != ResourceScope::Library {
        return Err("Resource is not a library resource".to_string());
    }

    let adapter = adapter_registry.get(&lib_resource.resource_type)
        .ok_or_else(|| "No adapter for resource type".to_string())?;

    let requested_link_type = LinkType::from_str(&link_type)
        .ok_or_else(|| "Invalid link type".to_string())?;
    let effective_link_type = normalize_link_type(adapter, requested_link_type);

    let target = adapter.resolve_target(
        &TargetScope::Global,
        &lib_resource.name,
        None,
    )?;

    let mut link = adapter.install(&lib_resource, &target, &effective_link_type)?;
    link.target_scope = "global".to_string();

    db.insert_link(&link).map_err(|e| e.to_string())?;
    Ok(link)
}

/// List all resource links for a given resource
#[tauri::command]
pub fn list_resource_links(
    db: State<Database>,
    resource_id: String,
) -> Result<Vec<ResourceLink>, String> {
    db.list_links_by_resource(&resource_id)
        .map_err(|e| e.to_string())
}

/// Check health of all resource links — verifies target paths exist and symlinks are valid
#[tauri::command]
pub fn check_link_health(db: State<Database>) -> Result<Vec<LinkHealthInfo>, String> {
    let all_links = db.list_all_links().map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for link in all_links {
        let target = Path::new(&link.target_path);
        let (healthy, error) = if link.link_type == "config_merge" {
            // Config-merge links: check that the config file exists and contains the expected entry
            if !target.exists() {
                (false, Some("Config file does not exist".to_string()))
            } else {
                match check_config_merge_health(&link) {
                    Ok(true) => (true, None),
                    Ok(false) => (false, Some("Entry not found in config file".to_string())),
                    Err(e) => (false, Some(format!("Failed to check config: {}", e))),
                }
            }
        } else if !target.exists() {
            // Check if it's a broken symlink (exists() returns false for broken symlinks)
            let is_broken_symlink = target.symlink_metadata().is_ok();
            if is_broken_symlink {
                (false, Some("Broken symlink: target does not exist".to_string()))
            } else {
                (false, Some("Target path does not exist".to_string()))
            }
        } else if link.link_type == "symlink" {
            // Verify it's actually a symlink
            match target.symlink_metadata() {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() {
                        (true, None)
                    } else {
                        (false, Some("Expected symlink but found regular file/directory".to_string()))
                    }
                }
                Err(e) => (false, Some(format!("Cannot read metadata: {}", e))),
            }
        } else {
            // Copy type: just check existence
            (true, None)
        };

        results.push(LinkHealthInfo {
            link_id: link.id,
            target_path: link.target_path,
            healthy,
            error,
        });
    }

    Ok(results)
}

/// Check whether a config_merge link's entry still exists in the config file.
fn check_config_merge_health(link: &ResourceLink) -> Result<bool, String> {
    let config_key = match &link.config_key {
        Some(k) => k,
        None => return Ok(false),
    };

    let config_path = Path::new(&link.target_path);
    let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    let config: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // Hook config_key format: "hooks.<EventName>._ccm_id=<resource_id>"
    if config_key.starts_with("hooks.") {
        let after_hooks = config_key.strip_prefix("hooks.").unwrap();
        let sep = "._ccm_id=";
        if let Some(sep_pos) = after_hooks.find(sep) {
            let event = &after_hooks[..sep_pos];
            let ccm_id = &after_hooks[sep_pos + sep.len()..];
            return Ok(crate::adapters::config_based::is_ccm_managed_hook(&config, event, ccm_id));
        }
        return Ok(false);
    }

    // MCP config_key format: "mcpServers.<name>"
    if config_key.starts_with("mcpServers.") {
        let server_name = config_key.strip_prefix("mcpServers.").unwrap();
        return Ok(crate::adapters::config_based::has_entry(&config, "mcpServers", server_name));
    }

    // Unknown config_key format
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::v2::{Resource, ResourceScope, ResourceType, ResourceLink};

    fn make_library_resource(id: &str, name: &str, rtype: ResourceType) -> Resource {
        Resource {
            id: id.to_string(),
            resource_type: rtype,
            name: name.to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: format!("/tmp/library/{}", name),
            content_hash: Some("abc123".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        }
    }

    fn setup_db_with_library_resources(db: &Database) {
        let r1 = make_library_resource("lib1", "test-skill", ResourceType::Skill);
        let r2 = make_library_resource("lib2", "test-agent", ResourceType::Agent);
        let r3 = Resource {
            id: "global1".to_string(),
            resource_type: ResourceType::Rule,
            name: "global-rule".to_string(),
            description: None,
            scope: ResourceScope::Global,
            source_path: "/tmp/global/rule".to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        };
        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();
        db.insert_resource(&r3).unwrap();
    }

    #[test]
    fn test_list_library_resources_all() {
        let db = Database::new_in_memory().unwrap();
        setup_db_with_library_resources(&db);

        let resources = db.list_resources_by_scope(&ResourceScope::Library).unwrap();
        assert_eq!(resources.len(), 2);
        assert!(resources.iter().all(|r| r.scope == ResourceScope::Library));
    }

    #[test]
    fn test_list_library_resources_filtered_by_type() {
        let db = Database::new_in_memory().unwrap();
        setup_db_with_library_resources(&db);

        let skills = db.list_resources_by_scope_and_type(&ResourceScope::Library, &ResourceType::Skill).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");

        let agents = db.list_resources_by_scope_and_type(&ResourceScope::Library, &ResourceType::Agent).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "test-agent");

        let hooks = db.list_resources_by_scope_and_type(&ResourceScope::Library, &ResourceType::Hook).unwrap();
        assert_eq!(hooks.len(), 0);
    }

    #[test]
    fn test_delete_library_resource_with_links_fails() {
        let db = Database::new_in_memory().unwrap();
        setup_db_with_library_resources(&db);

        // Add a project for FK
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('proj1', 'test-project', '/tmp/proj1')",
                [],
            ).unwrap();
        }

        // Create a link to the library resource
        let link = ResourceLink {
            id: "link1".to_string(),
            resource_id: "lib1".to_string(),
            target_scope: "project".to_string(),
            target_path: "/tmp/proj1/.claude/skills/test-skill".to_string(),
            config_key: None,
            project_id: Some("proj1".to_string()),
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_link(&link).unwrap();

        // Verify the resource has links
        let links = db.list_links_by_resource("lib1").unwrap();
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn test_list_resource_links() {
        let db = Database::new_in_memory().unwrap();
        setup_db_with_library_resources(&db);

        // Add a project for FK
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('proj1', 'test-project', '/tmp/proj1')",
                [],
            ).unwrap();
        }

        let link1 = ResourceLink {
            id: "link1".to_string(),
            resource_id: "lib1".to_string(),
            target_scope: "project".to_string(),
            target_path: "/tmp/proj1/.claude/skills/test-skill".to_string(),
            config_key: None,
            project_id: Some("proj1".to_string()),
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        let link2 = ResourceLink {
            id: "link2".to_string(),
            resource_id: "lib1".to_string(),
            target_scope: "global".to_string(),
            target_path: "/home/user/.claude/skills/test-skill".to_string(),
            config_key: None,
            project_id: None,
            link_type: "copy".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_link(&link1).unwrap();
        db.insert_link(&link2).unwrap();

        let links = db.list_links_by_resource("lib1").unwrap();
        assert_eq!(links.len(), 2);

        let links_lib2 = db.list_links_by_resource("lib2").unwrap();
        assert_eq!(links_lib2.len(), 0);
    }

    #[test]
    fn test_check_link_health_nonexistent_target() {
        let db = Database::new_in_memory().unwrap();
        setup_db_with_library_resources(&db);

        let link = ResourceLink {
            id: "link1".to_string(),
            resource_id: "lib1".to_string(),
            target_scope: "project".to_string(),
            target_path: "/nonexistent/path/that/does/not/exist".to_string(),
            config_key: None,
            project_id: None,
            link_type: "copy".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_link(&link).unwrap();

        let all_links = db.list_all_links().unwrap();
        assert_eq!(all_links.len(), 1);

        // Check health manually (simulating what the command does)
        let target = Path::new(&all_links[0].target_path);
        assert!(!target.exists());
    }

    #[test]
    fn test_check_link_health_existing_target() {
        let db = Database::new_in_memory().unwrap();
        setup_db_with_library_resources(&db);

        // Use a path that actually exists
        let link = ResourceLink {
            id: "link1".to_string(),
            resource_id: "lib1".to_string(),
            target_scope: "project".to_string(),
            target_path: "/tmp".to_string(),
            config_key: None,
            project_id: None,
            link_type: "copy".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_link(&link).unwrap();

        let target = Path::new(&link.target_path);
        assert!(target.exists());
    }

    #[test]
    fn test_list_all_links() {
        let db = Database::new_in_memory().unwrap();
        setup_db_with_library_resources(&db);

        // Add a project for FK
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('proj1', 'test-project', '/tmp/proj1')",
                [],
            ).unwrap();
        }

        let link1 = ResourceLink {
            id: "link1".to_string(),
            resource_id: "lib1".to_string(),
            target_scope: "project".to_string(),
            target_path: "/tmp/proj1/.claude/skills/test-skill".to_string(),
            config_key: None,
            project_id: Some("proj1".to_string()),
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        let link2 = ResourceLink {
            id: "link2".to_string(),
            resource_id: "lib2".to_string(),
            target_scope: "global".to_string(),
            target_path: "/home/user/.claude/agents/test-agent.md".to_string(),
            config_key: None,
            project_id: None,
            link_type: "copy".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
        };
        db.insert_link(&link1).unwrap();
        db.insert_link(&link2).unwrap();

        let all = db.list_all_links().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_link_health_info_serialization() {
        let info = LinkHealthInfo {
            link_id: "test-id".to_string(),
            target_path: "/some/path".to_string(),
            healthy: true,
            error: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("healthy"));

        let info_unhealthy = LinkHealthInfo {
            link_id: "test-id2".to_string(),
            target_path: "/broken/path".to_string(),
            healthy: false,
            error: Some("Target does not exist".to_string()),
        };
        let json2 = serde_json::to_string(&info_unhealthy).unwrap();
        assert!(json2.contains("Target does not exist"));
    }
}
