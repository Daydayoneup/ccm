use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::adapters::file_based::copy_dir_recursive;
use crate::commands::symlinks::create_resource_link;
use crate::models::project::{LocalResource, ResourceType};
use crate::models::resource::{AppConfig, GlobalLink, GlobalLinksIndex, LibraryIndex, LibraryResource, McpFileContent, McpServerConfig, PluginInfo};
use crate::scanner;

/// Import a resource from a project into the central library.
///
/// 1. Copies the resource from `source_path` to `library_dir`
/// 2. Creates a backup of the original in `backup_dir`
/// 3. If `replace_with_symlink` is true, removes the original and creates a symlink back to the library copy
///
/// Returns the path to the resource in the library.
pub fn import_resource(
    source_path: &str,
    library_dir: &str,
    backup_dir: &str,
    replace_with_symlink: bool,
) -> Result<String, String> {
    let source = Path::new(source_path);
    if !source.exists() {
        return Err(format!("Source does not exist: {}", source_path));
    }

    let resource_name = source
        .file_name()
        .ok_or_else(|| "Invalid source path".to_string())?
        .to_string_lossy()
        .to_string();

    // Destination in library
    let lib_dest = Path::new(library_dir).join(&resource_name);
    if lib_dest.exists() {
        return Err(format!(
            "Resource already exists in library: {}",
            lib_dest.display()
        ));
    }

    // Create library directory if needed
    fs::create_dir_all(library_dir)
        .map_err(|e| format!("Failed to create library directory: {}", e))?;

    // Copy resource to library
    if source.is_dir() {
        copy_dir_recursive(source, &lib_dest)?;
    } else {
        fs::copy(source, &lib_dest)
            .map_err(|e| format!("Failed to copy file to library: {}", e))?;
    }

    // Create backup
    fs::create_dir_all(backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("{}_{}", resource_name, timestamp);
    let backup_dest = Path::new(backup_dir).join(&backup_name);

    if source.is_dir() {
        copy_dir_recursive(source, &backup_dest)?;
    } else {
        fs::copy(source, &backup_dest)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }

    // Replace with symlink if requested
    if replace_with_symlink {
        if source.is_dir() {
            fs::remove_dir_all(source)
                .map_err(|e| format!("Failed to remove original directory: {}", e))?;
        } else {
            fs::remove_file(source)
                .map_err(|e| format!("Failed to remove original file: {}", e))?;
        }

        create_resource_link(
            lib_dest.to_str().unwrap_or_default(),
            source_path,
        )?;
    }

    Ok(lib_dest.to_string_lossy().to_string())
}


/// Map a ResourceType to its subdirectory name in the library.
fn resource_type_dir(rt: &ResourceType) -> &str {
    match rt {
        ResourceType::Skill => "skills",
        ResourceType::Agent => "agents",
        ResourceType::Rule => "rules",
        ResourceType::Hook => "hooks",
        ResourceType::Command => "commands",
    }
}

/// Tauri command: Import resources from a project into the central library.
///
/// Scans the project, finds resources matching the given types, and imports them.
/// Returns a list of library paths where resources were imported.
#[tauri::command]
pub fn import_resources_from_project(
    project_path: String,
    library_base: String,
    resource_types: Vec<ResourceType>,
    replace_with_symlinks: bool,
) -> Result<Vec<String>, String> {
    let project = scanner::scan_project(&project_path)?;

    let mut imported_paths = Vec::new();

    for local_resource in &project.local_resources {
        if !resource_types.contains(&local_resource.resource_type) {
            continue;
        }

        let type_dir = resource_type_dir(&local_resource.resource_type);
        let library_dir = Path::new(&library_base)
            .join("library")
            .join(type_dir);
        let backup_dir = Path::new(&library_base).join("backups");

        let lib_path = import_resource(
            &local_resource.path,
            library_dir.to_str().unwrap_or_default(),
            backup_dir.to_str().unwrap_or_default(),
            replace_with_symlinks,
        )?;

        imported_paths.push(lib_path);
    }

    Ok(imported_paths)
}

/// Check if a path is a symlink pointing into the library directory.
fn is_symlink_to_library(path: &Path, library_base: &str) -> bool {
    match fs::read_link(path) {
        Ok(target) => target.to_string_lossy().starts_with(library_base),
        Err(_) => false,
    }
}

/// Scan ~/.claude/ for global resources, filtering out symlinks that already point to the library.
pub fn scan_global_resources_at(
    home_dir: &str,
    library_base: &str,
) -> Result<Vec<LocalResource>, String> {
    let claude_dir = Path::new(home_dir).join(".claude");
    if !claude_dir.is_dir() {
        return Ok(Vec::new());
    }

    let all_resources = scanner::scan_claude_dir(&claude_dir);

    let filtered: Vec<LocalResource> = all_resources
        .into_iter()
        .filter(|r| !is_symlink_to_library(Path::new(&r.path), library_base))
        .collect();

    Ok(filtered)
}

/// Tauri command: Scan global resources in ~/.claude/.
#[tauri::command]
pub fn scan_global_resources() -> Result<Vec<LocalResource>, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?;
    let config = AppConfig::default();

    scan_global_resources_at(
        home.to_str().unwrap_or_default(),
        &config.central_library_path,
    )
}

/// Import global resources into the central library, replace originals with symlinks,
/// update library index.json and global-links.json.
pub fn import_global_resources_at(
    _home_dir: &str,
    library_base: &str,
    resources: Vec<LocalResource>,
) -> Result<Vec<String>, String> {
    let lib_base = Path::new(library_base);
    let backup_dir = lib_base.join("backups");
    let mut imported_paths = Vec::new();
    let mut new_links = Vec::new();
    let mut new_lib_resources = Vec::new();

    for resource in &resources {
        let type_dir = resource_type_dir(&resource.resource_type);
        let library_dir = lib_base.join("library").join(type_dir);

        let lib_path = import_resource(
            &resource.path,
            library_dir.to_str().unwrap_or_default(),
            backup_dir.to_str().unwrap_or_default(),
            true,
        )?;

        let resource_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        new_links.push(GlobalLink {
            library_resource_id: resource_id.clone(),
            library_path: lib_path.clone(),
            original_path: resource.path.clone(),
            resource_type: resource.resource_type.clone(),
            imported_at: now.clone(),
        });

        new_lib_resources.push(LibraryResource {
            id: resource_id,
            resource_type: resource.resource_type.clone(),
            name: resource.name.clone(),
            description: String::new(),
            tags: vec!["global".to_string()],
            path: lib_path.clone(),
            linked_projects: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        });

        imported_paths.push(lib_path);
    }

    // Update global-links.json
    let links_path = lib_base.join("global-links.json");
    let mut links_index = if links_path.exists() {
        let content = fs::read_to_string(&links_path)
            .map_err(|e| format!("Failed to read global-links.json: {}", e))?;
        serde_json::from_str::<GlobalLinksIndex>(&content)
            .map_err(|e| format!("Failed to parse global-links.json: {}", e))?
    } else {
        GlobalLinksIndex::new()
    };
    links_index.links.extend(new_links);
    let links_json = serde_json::to_string_pretty(&links_index)
        .map_err(|e| format!("Failed to serialize global-links.json: {}", e))?;
    fs::write(&links_path, links_json)
        .map_err(|e| format!("Failed to write global-links.json: {}", e))?;

    // Update library/index.json
    let index_path = lib_base.join("library").join("index.json");
    let mut lib_index = if index_path.exists() {
        let content = fs::read_to_string(&index_path)
            .map_err(|e| format!("Failed to read index.json: {}", e))?;
        serde_json::from_str::<LibraryIndex>(&content)
            .map_err(|e| format!("Failed to parse index.json: {}", e))?
    } else {
        LibraryIndex::new()
    };
    lib_index.resources.extend(new_lib_resources);
    let index_json = serde_json::to_string_pretty(&lib_index)
        .map_err(|e| format!("Failed to serialize index.json: {}", e))?;
    fs::write(&index_path, index_json)
        .map_err(|e| format!("Failed to write index.json: {}", e))?;

    Ok(imported_paths)
}

/// Tauri command: Import global resources from ~/.claude/ into the central library.
#[tauri::command]
pub fn import_global_resources(resources: Vec<LocalResource>) -> Result<Vec<String>, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?;
    let config = AppConfig::default();

    import_global_resources_at(
        home.to_str().unwrap_or_default(),
        &config.central_library_path,
        resources,
    )
}

/// Restore global resources from the central library back to ~/.claude/.
/// Removes symlinks and copies library files back to their original locations.
/// If `resource_ids` is None, restores all. If Some, restores only those IDs.
pub fn restore_global_resources_at(
    library_base: &str,
    resource_ids: Option<Vec<String>>,
) -> Result<usize, String> {
    let lib_base = Path::new(library_base);
    let links_path = lib_base.join("global-links.json");

    if !links_path.exists() {
        return Ok(0);
    }

    let content = fs::read_to_string(&links_path)
        .map_err(|e| format!("Failed to read global-links.json: {}", e))?;
    let links_index: GlobalLinksIndex = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse global-links.json: {}", e))?;

    let (to_restore, to_keep): (Vec<GlobalLink>, Vec<GlobalLink>) =
        if let Some(ref ids) = resource_ids {
            links_index
                .links
                .into_iter()
                .partition(|l| ids.contains(&l.library_resource_id))
        } else {
            (links_index.links, Vec::new())
        };

    let mut restored_count = 0;

    for link in &to_restore {
        let original = Path::new(&link.original_path);
        let library_src = Path::new(&link.library_path);

        // Remove symlink if it exists
        if original.symlink_metadata().is_ok() {
            fs::remove_file(original)
                .map_err(|e| format!("Failed to remove symlink at {}: {}", link.original_path, e))?;
        }

        // Ensure parent directory exists
        if let Some(parent) = original.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent dir: {}", e))?;
        }

        // Copy from library back to original location
        if library_src.is_dir() {
            copy_dir_recursive(library_src, original)?;
        } else if library_src.is_file() {
            fs::copy(library_src, original).map_err(|e| {
                format!(
                    "Failed to copy {} to {}: {}",
                    link.library_path, link.original_path, e
                )
            })?;
        }

        restored_count += 1;
    }

    // Update global-links.json (remove restored entries)
    let updated_index = GlobalLinksIndex { links: to_keep };
    let links_json = serde_json::to_string_pretty(&updated_index)
        .map_err(|e| format!("Failed to serialize global-links.json: {}", e))?;
    fs::write(&links_path, links_json)
        .map_err(|e| format!("Failed to write global-links.json: {}", e))?;

    Ok(restored_count)
}

/// Tauri command: Restore global resources back to ~/.claude/.
#[tauri::command]
pub fn restore_global_resources(resource_ids: Option<Vec<String>>) -> Result<usize, String> {
    let config = AppConfig::default();
    restore_global_resources_at(&config.central_library_path, resource_ids)
}

/// Internal structs for parsing installed_plugins.json
#[derive(Debug, Deserialize)]
struct InstalledPluginsFile {
    plugins: std::collections::HashMap<String, Vec<PluginEntry>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginEntry {
    scope: String,
    install_path: String,
    version: String,
}

/// Scan installed Claude Code plugins and their resources.
pub fn scan_installed_plugins_at(home_dir: &str) -> Result<Vec<PluginInfo>, String> {
    let plugins_json = Path::new(home_dir)
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");

    if !plugins_json.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&plugins_json)
        .map_err(|e| format!("Failed to read installed_plugins.json: {}", e))?;
    let file: InstalledPluginsFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse installed_plugins.json: {}", e))?;

    let mut plugins = Vec::new();

    for (key, entries) in &file.plugins {
        // key format: "plugin-name@marketplace"
        let plugin_name = key.split('@').next().unwrap_or(key).to_string();

        for entry in entries {
            let install_path = Path::new(&entry.install_path);
            if !install_path.is_dir() {
                continue;
            }

            // scan_claude_dir expects a .claude/-like directory; plugin roots have the same structure
            let resources = scanner::scan_claude_dir(install_path);

            plugins.push(PluginInfo {
                name: plugin_name.clone(),
                version: entry.version.clone(),
                scope: entry.scope.clone(),
                install_path: entry.install_path.clone(),
                resources,
            });
        }
    }

    // Sort by name for consistent ordering
    plugins.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(plugins)
}

/// Tauri command: Scan installed Claude Code plugins.
#[tauri::command]
pub fn scan_installed_plugins() -> Result<Vec<PluginInfo>, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?;
    scan_installed_plugins_at(home.to_str().unwrap_or_default())
}

/// Scan registered projects for .mcp.json files and extract MCP server configs.
pub fn scan_mcp_configs_at(registry_path: &str) -> Result<Vec<McpServerConfig>, String> {
    let content = fs::read_to_string(registry_path)
        .map_err(|e| format!("Failed to read projects registry: {}", e))?;
    let registry: crate::models::project::ProjectsRegistry = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse projects registry: {}", e))?;

    let mut configs = Vec::new();

    for project in &registry.projects {
        let mcp_path = Path::new(&project.path).join(".mcp.json");
        if !mcp_path.is_file() {
            continue;
        }

        let mcp_content = match fs::read_to_string(&mcp_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mcp_file: McpFileContent = match serde_json::from_str(&mcp_content) {
            Ok(f) => f,
            Err(_) => continue,
        };

        for (name, value) in &mcp_file.mcp_servers {
            let command = value.get("command").and_then(|v| v.as_str()).map(|s| s.to_string());
            let args = value.get("args").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|a| a.as_str().map(|s| s.to_string())).collect()
            });
            let url = value.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
            let server_type = value.get("type").and_then(|v| v.as_str()).map(|s| s.to_string());

            configs.push(McpServerConfig {
                name: name.clone(),
                project_name: project.name.clone(),
                project_path: project.path.clone(),
                command,
                args,
                url,
                server_type,
            });
        }
    }

    Ok(configs)
}

/// Tauri command: Scan all registered projects for MCP server configurations.
#[tauri::command]
pub fn scan_mcp_configs() -> Result<Vec<McpServerConfig>, String> {
    let config = AppConfig::default();
    let registry_path = Path::new(&config.central_library_path)
        .join("projects.json");

    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    scan_mcp_configs_at(registry_path.to_str().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::library::init_library_at;
    use crate::commands::symlinks::check_symlink_valid;
    use crate::models::resource::LibraryIndex;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_import_resource_file() {
        let tmp = TempDir::new().unwrap();

        // Create source file
        let source = tmp.path().join("my-rule.md");
        fs::write(&source, "# My Rule\nContent here").unwrap();

        let lib_dir = tmp.path().join("lib/rules");
        let backup_dir = tmp.path().join("backups");

        let result = import_resource(
            source.to_str().unwrap(),
            lib_dir.to_str().unwrap(),
            backup_dir.to_str().unwrap(),
            false,
        );
        assert!(result.is_ok(), "import_resource failed: {:?}", result.err());

        let lib_path = result.unwrap();

        // Verify file exists in library
        assert!(Path::new(&lib_path).exists());
        let content = fs::read_to_string(&lib_path).unwrap();
        assert_eq!(content, "# My Rule\nContent here");

        // Verify backup exists
        let backups: Vec<_> = fs::read_dir(&backup_dir).unwrap().collect();
        assert_eq!(backups.len(), 1);

        // Original should still exist (no symlink replacement)
        assert!(source.exists());
    }

    #[test]
    fn test_import_resource_directory() {
        let tmp = TempDir::new().unwrap();

        // Create a skill directory
        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Skill").unwrap();
        fs::write(skill_dir.join("helper.sh"), "#!/bin/bash").unwrap();

        let lib_dir = tmp.path().join("lib/skills");
        let backup_dir = tmp.path().join("backups");

        let result = import_resource(
            skill_dir.to_str().unwrap(),
            lib_dir.to_str().unwrap(),
            backup_dir.to_str().unwrap(),
            false,
        );
        assert!(result.is_ok());

        let lib_path = result.unwrap();
        assert!(Path::new(&lib_path).join("SKILL.md").exists());
        assert!(Path::new(&lib_path).join("helper.sh").exists());
    }

    #[test]
    fn test_import_resource_with_symlink_replacement() {
        let tmp = TempDir::new().unwrap();

        // Create source file
        let source = tmp.path().join("my-agent.md");
        fs::write(&source, "# Agent").unwrap();

        let lib_dir = tmp.path().join("lib/agents");
        let backup_dir = tmp.path().join("backups");

        let result = import_resource(
            source.to_str().unwrap(),
            lib_dir.to_str().unwrap(),
            backup_dir.to_str().unwrap(),
            true, // replace with symlink
        );
        assert!(result.is_ok());

        // The original path should now be a symlink
        assert!(check_symlink_valid(source.to_str().unwrap()));

        // Reading through the symlink should give the same content
        let content = fs::read_to_string(&source).unwrap();
        assert_eq!(content, "# Agent");
    }

    #[test]
    fn test_import_resource_dir_with_symlink_replacement() {
        let tmp = TempDir::new().unwrap();

        // Create skill directory
        let skill_dir = tmp.path().join("deploy-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Deploy Skill").unwrap();

        let lib_dir = tmp.path().join("lib/skills");
        let backup_dir = tmp.path().join("backups");

        let result = import_resource(
            skill_dir.to_str().unwrap(),
            lib_dir.to_str().unwrap(),
            backup_dir.to_str().unwrap(),
            true,
        );
        assert!(result.is_ok());

        // The original path should now be a symlink to the library copy
        assert!(check_symlink_valid(skill_dir.to_str().unwrap()));
        assert!(skill_dir.join("SKILL.md").exists());
    }

    #[test]
    fn test_import_resources_from_project() {
        let tmp = TempDir::new().unwrap();

        // Setup library
        let lib_base = tmp.path().join("library-base");
        init_library_at(lib_base.to_str().unwrap()).unwrap();

        // Create a project with resources
        let proj = tmp.path().join("test-project");
        fs::create_dir_all(proj.join(".claude/skills/my-skill")).unwrap();
        fs::write(proj.join(".claude/skills/my-skill/SKILL.md"), "# Skill").unwrap();
        fs::create_dir_all(proj.join(".claude/rules")).unwrap();
        fs::write(proj.join(".claude/rules/style.md"), "# Style").unwrap();

        // Import only skills
        let result = import_resources_from_project(
            proj.to_string_lossy().to_string(),
            lib_base.to_string_lossy().to_string(),
            vec![ResourceType::Skill],
            false,
        );
        assert!(result.is_ok(), "import failed: {:?}", result.err());

        let imported = result.unwrap();
        assert_eq!(imported.len(), 1);
        assert!(imported[0].contains("my-skill"));

        // Verify skill was copied to library
        assert!(lib_base.join("library/skills/my-skill/SKILL.md").exists());

        // Rule should NOT have been imported
        assert!(!lib_base.join("library/rules/style.md").exists());
    }

    #[test]
    fn test_import_resource_source_not_exists() {
        let tmp = TempDir::new().unwrap();
        let result = import_resource(
            tmp.path().join("nonexistent").to_str().unwrap(),
            tmp.path().join("lib").to_str().unwrap(),
            tmp.path().join("backup").to_str().unwrap(),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_import_resource_already_exists_in_library() {
        let tmp = TempDir::new().unwrap();

        let source = tmp.path().join("resource.md");
        fs::write(&source, "content").unwrap();

        let lib_dir = tmp.path().join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("resource.md"), "existing").unwrap();

        let result = import_resource(
            source.to_str().unwrap(),
            lib_dir.to_str().unwrap(),
            tmp.path().join("backup").to_str().unwrap(),
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_copy_dir_recursive() {
        let tmp = TempDir::new().unwrap();

        let src = tmp.path().join("source");
        fs::create_dir_all(src.join("sub/deep")).unwrap();
        fs::write(src.join("root.txt"), "root").unwrap();
        fs::write(src.join("sub/mid.txt"), "mid").unwrap();
        fs::write(src.join("sub/deep/leaf.txt"), "leaf").unwrap();

        let dst = tmp.path().join("dest");
        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("root.txt").exists());
        assert!(dst.join("sub/mid.txt").exists());
        assert!(dst.join("sub/deep/leaf.txt").exists());

        assert_eq!(fs::read_to_string(dst.join("root.txt")).unwrap(), "root");
        assert_eq!(fs::read_to_string(dst.join("sub/mid.txt")).unwrap(), "mid");
        assert_eq!(
            fs::read_to_string(dst.join("sub/deep/leaf.txt")).unwrap(),
            "leaf"
        );
    }

    #[test]
    fn test_scan_global_resources() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");

        // Create global resources
        fs::create_dir_all(claude_dir.join("skills/my-global-skill")).unwrap();
        fs::write(claude_dir.join("skills/my-global-skill/SKILL.md"), "# Global Skill").unwrap();
        fs::create_dir_all(claude_dir.join("agents")).unwrap();
        fs::write(claude_dir.join("agents/global-agent.md"), "# Global Agent").unwrap();
        fs::create_dir_all(claude_dir.join("rules")).unwrap();
        fs::write(claude_dir.join("rules/global-rule.md"), "# Global Rule").unwrap();

        let home_dir = tmp.path().to_string_lossy().to_string();
        let library_base = tmp.path().join("lib-base").to_string_lossy().to_string();

        let result = scan_global_resources_at(&home_dir, &library_base);
        assert!(result.is_ok());

        let resources = result.unwrap();
        assert_eq!(resources.len(), 3);

        let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"my-global-skill"));
        assert!(names.contains(&"global-agent"));
        assert!(names.contains(&"global-rule"));
    }

    #[test]
    fn test_scan_global_resources_skips_symlinks_to_library() {
        let tmp = TempDir::new().unwrap();

        // Setup library
        let lib_base = tmp.path().join("lib-base");
        init_library_at(lib_base.to_str().unwrap()).unwrap();

        // Create a real agent in ~/.claude/agents/
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("agents")).unwrap();
        fs::write(claude_dir.join("agents/real-agent.md"), "# Real Agent").unwrap();

        // Create a library resource and symlink it into ~/.claude/agents/
        let lib_agent = lib_base.join("library/agents/linked-agent.md");
        fs::write(&lib_agent, "# Linked Agent").unwrap();
        std::os::unix::fs::symlink(&lib_agent, claude_dir.join("agents/linked-agent.md")).unwrap();

        let home_dir = tmp.path().to_string_lossy().to_string();
        let library_base = lib_base.to_string_lossy().to_string();

        let result = scan_global_resources_at(&home_dir, &library_base);
        assert!(result.is_ok());

        let resources = result.unwrap();
        // Should only find the real agent, not the symlinked one
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "real-agent");
    }

    #[test]
    fn test_import_global_resources() {
        let tmp = TempDir::new().unwrap();

        // Setup library
        let lib_base = tmp.path().join("lib-base");
        init_library_at(lib_base.to_str().unwrap()).unwrap();

        // Create global resources in ~/.claude/
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("agents")).unwrap();
        fs::write(claude_dir.join("agents/my-agent.md"), "# My Agent").unwrap();
        fs::create_dir_all(claude_dir.join("rules")).unwrap();
        fs::write(claude_dir.join("rules/my-rule.md"), "# My Rule").unwrap();

        let home_dir = tmp.path().to_string_lossy().to_string();
        let library_base = lib_base.to_string_lossy().to_string();

        // Scan first
        let resources = scan_global_resources_at(&home_dir, &library_base).unwrap();
        assert_eq!(resources.len(), 2);

        // Import
        let result = import_global_resources_at(&home_dir, &library_base, resources);
        assert!(result.is_ok(), "import failed: {:?}", result.err());

        let imported = result.unwrap();
        assert_eq!(imported.len(), 2);

        // Verify files copied to library
        assert!(lib_base.join("library/agents/my-agent.md").exists());
        assert!(lib_base.join("library/rules/my-rule.md").exists());

        // Verify originals replaced with symlinks
        assert!(check_symlink_valid(claude_dir.join("agents/my-agent.md").to_str().unwrap()));
        assert!(check_symlink_valid(claude_dir.join("rules/my-rule.md").to_str().unwrap()));

        // Verify global-links.json written
        let links_path = lib_base.join("global-links.json");
        assert!(links_path.exists());
        let links_content = fs::read_to_string(&links_path).unwrap();
        let links_index: GlobalLinksIndex = serde_json::from_str(&links_content).unwrap();
        assert_eq!(links_index.links.len(), 2);

        // Verify library index.json updated
        let index_content = fs::read_to_string(lib_base.join("library/index.json")).unwrap();
        let index: LibraryIndex = serde_json::from_str(&index_content).unwrap();
        assert_eq!(index.resources.len(), 2);
    }

    #[test]
    fn test_restore_global_resources() {
        let tmp = TempDir::new().unwrap();

        // Setup library
        let lib_base = tmp.path().join("lib-base");
        init_library_at(lib_base.to_str().unwrap()).unwrap();

        // Create global resource
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("agents")).unwrap();
        fs::write(claude_dir.join("agents/my-agent.md"), "# My Agent").unwrap();

        let home_dir = tmp.path().to_string_lossy().to_string();
        let library_base = lib_base.to_string_lossy().to_string();

        // Import it
        let resources = scan_global_resources_at(&home_dir, &library_base).unwrap();
        import_global_resources_at(&home_dir, &library_base, resources).unwrap();

        // Verify it's now a symlink
        let agent_path = claude_dir.join("agents/my-agent.md");
        assert!(check_symlink_valid(agent_path.to_str().unwrap()));

        // Restore
        let result = restore_global_resources_at(&library_base, None);
        assert!(result.is_ok(), "restore failed: {:?}", result.err());
        assert_eq!(result.unwrap(), 1);

        // Verify it's now a regular file again (not a symlink)
        assert!(agent_path.exists());
        let meta = agent_path.symlink_metadata().unwrap();
        assert!(!meta.file_type().is_symlink());
        let content = fs::read_to_string(&agent_path).unwrap();
        assert_eq!(content, "# My Agent");

        // Verify global-links.json is empty
        let links_content = fs::read_to_string(lib_base.join("global-links.json")).unwrap();
        let links_index: GlobalLinksIndex = serde_json::from_str(&links_content).unwrap();
        assert!(links_index.links.is_empty());
    }

    #[test]
    fn test_scan_installed_plugins() {
        let tmp = TempDir::new().unwrap();

        // Create plugins directory structure
        let plugins_dir = tmp.path().join(".claude/plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        // Create a plugin with skills and commands
        let plugin_path = tmp.path().join(".claude/plugins/cache/test-marketplace/my-plugin/1.0.0");
        fs::create_dir_all(plugin_path.join("skills/brainstorming")).unwrap();
        fs::write(plugin_path.join("skills/brainstorming/SKILL.md"), "# Brainstorming").unwrap();
        fs::create_dir_all(plugin_path.join("commands")).unwrap();
        fs::write(plugin_path.join("commands/deploy.md"), "# Deploy").unwrap();
        fs::create_dir_all(plugin_path.join("agents")).unwrap();
        fs::write(plugin_path.join("agents/reviewer.md"), "# Reviewer").unwrap();

        // Create installed_plugins.json
        let installed = serde_json::json!({
            "version": 2,
            "plugins": {
                "my-plugin@test-marketplace": [{
                    "scope": "user",
                    "installPath": plugin_path.to_string_lossy(),
                    "version": "1.0.0",
                    "installedAt": "2026-01-01T00:00:00Z",
                    "lastUpdated": "2026-01-01T00:00:00Z"
                }]
            }
        });
        fs::write(
            plugins_dir.join("installed_plugins.json"),
            serde_json::to_string_pretty(&installed).unwrap(),
        ).unwrap();

        let result = scan_installed_plugins_at(tmp.path().to_str().unwrap());
        assert!(result.is_ok(), "scan failed: {:?}", result.err());

        let plugins = result.unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "my-plugin");
        assert_eq!(plugins[0].version, "1.0.0");
        assert_eq!(plugins[0].scope, "user");
        assert_eq!(plugins[0].resources.len(), 3); // 1 skill + 1 command + 1 agent
    }

    #[test]
    fn test_scan_mcp_configs() {
        let tmp = TempDir::new().unwrap();

        // Setup library and registry
        let lib_base = tmp.path().join("lib-base");
        init_library_at(lib_base.to_str().unwrap()).unwrap();

        // Create a project with .mcp.json
        let proj = tmp.path().join("my-project");
        fs::create_dir_all(proj.join(".claude")).unwrap();
        fs::write(proj.join(".mcp.json"), r#"{
            "mcpServers": {
                "mcp-ssh": {
                    "command": "npx",
                    "args": ["@aiondadotcom/mcp-ssh"]
                },
                "my-api": {
                    "url": "http://localhost:3000",
                    "type": "sse"
                }
            }
        }"#).unwrap();

        // Register the project
        let project = crate::models::project::Project {
            id: "test-id".to_string(),
            name: "my-project".to_string(),
            path: proj.to_string_lossy().to_string(),
            language: "Unknown".to_string(),
            linked_resources: Vec::new(),
            local_resources: Vec::new(),
            last_scanned: chrono::Utc::now().to_rfc3339(),
        };

        let registry = crate::models::project::ProjectsRegistry {
            projects: vec![project],
            scan_directories: Vec::new(),
        };
        let registry_json = serde_json::to_string_pretty(&registry).unwrap();
        fs::write(lib_base.join("projects.json"), registry_json).unwrap();

        let result = scan_mcp_configs_at(lib_base.join("projects.json").to_str().unwrap());
        assert!(result.is_ok(), "scan failed: {:?}", result.err());

        let configs = result.unwrap();
        assert_eq!(configs.len(), 2);

        let ssh = configs.iter().find(|c| c.name == "mcp-ssh").unwrap();
        assert_eq!(ssh.command.as_deref(), Some("npx"));
        assert_eq!(ssh.project_name, "my-project");

        let api = configs.iter().find(|c| c.name == "my-api").unwrap();
        assert_eq!(api.url.as_deref(), Some("http://localhost:3000"));
        assert_eq!(api.server_type.as_deref(), Some("sse"));
    }
}
