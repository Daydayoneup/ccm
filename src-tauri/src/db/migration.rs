use std::fs;
use std::path::Path;

use crate::db::Database;
use crate::models::v2::{Resource, ResourceScope, ResourceType};
use crate::scanner;

/// Migrate legacy JSON data to SQLite. Called once during Tauri setup, before full_sync.
/// Safe to call multiple times — checks if migration already happened.
pub fn migrate_json_to_sqlite(db: &Database) -> Result<(), String> {
    // Check if we already have data (skip migration)
    let resource_count = db
        .count_resources_by_scope(&ResourceScope::Library)
        .map_err(|e| e.to_string())?;
    let project_count = db.count_projects().map_err(|e| e.to_string())?;

    if resource_count > 0 || project_count > 0 {
        return Ok(()); // Already migrated or has data
    }

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let manager_dir = home.join(".claude-manager");

    // Migrate projects from projects.json
    let projects_file = manager_dir.join("projects.json");
    if projects_file.is_file() {
        migrate_projects(db, &projects_file)?;
    }

    // Migrate library from library/index.json
    let library_index = manager_dir.join("library").join("index.json");
    if library_index.is_file() {
        migrate_library(db, &library_index)?;
    }

    Ok(())
}

fn migrate_projects(db: &Database, projects_file: &Path) -> Result<(), String> {
    let content = fs::read_to_string(projects_file).map_err(|e| e.to_string())?;
    let registry: crate::models::project::ProjectsRegistry =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse projects.json: {}", e))?;

    for old_project in &registry.projects {
        let project = crate::models::v2::Project {
            id: old_project.id.clone(),
            name: old_project.name.clone(),
            path: old_project.path.clone(),
            language: if old_project.language.is_empty() {
                None
            } else {
                Some(old_project.language.clone())
            },
            last_scanned: if old_project.last_scanned.is_empty() {
                None
            } else {
                Some(old_project.last_scanned.clone())
            },
            pinned: 0,
            launch_count: 0,
        };

        // Insert project (ignore duplicates)
        let _ = db.insert_project(&project);
    }

    Ok(())
}

fn migrate_library(db: &Database, index_file: &Path) -> Result<(), String> {
    let content = fs::read_to_string(index_file).map_err(|e| e.to_string())?;
    let index: crate::models::resource::LibraryIndex =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse index.json: {}", e))?;

    for old_resource in &index.resources {
        let resource_type = convert_resource_type(&old_resource.resource_type);
        let hash = scanner::compute_file_hash(&old_resource.path);

        let resource = Resource {
            id: old_resource.id.clone(),
            resource_type,
            name: old_resource.name.clone(),
            description: if old_resource.description.is_empty() {
                None
            } else {
                Some(old_resource.description.clone())
            },
            scope: ResourceScope::Library,
            source_path: old_resource.path.clone(),
            content_hash: hash,
            metadata: if old_resource.tags.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&old_resource.tags).unwrap_or_default())
            },
            created_at: old_resource.created_at.clone(),
            updated_at: old_resource.updated_at.clone(),
            version: None,
            is_draft: 1,
        };

        // Insert (ignore duplicates)
        let _ = db.insert_resource(&resource);
    }

    Ok(())
}

fn convert_resource_type(old: &crate::models::project::ResourceType) -> ResourceType {
    match old {
        crate::models::project::ResourceType::Skill => ResourceType::Skill,
        crate::models::project::ResourceType::Agent => ResourceType::Agent,
        crate::models::project::ResourceType::Rule => ResourceType::Rule,
        crate::models::project::ResourceType::Hook => ResourceType::Hook,
        crate::models::project::ResourceType::Command => ResourceType::Command,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::v2::ResourceScope;

    #[test]
    fn test_migrate_empty_db() {
        let db = Database::new_in_memory().unwrap();
        // No JSON files exist at home dir, migration should succeed silently
        let result = migrate_json_to_sqlite(&db);
        assert!(result.is_ok());
    }

    #[test]
    fn test_migrate_skips_when_data_exists() {
        let db = Database::new_in_memory().unwrap();
        // Insert a project so migration thinks it already ran
        let project = crate::models::v2::Project {
            id: "existing".to_string(),
            name: "existing".to_string(),
            path: "/tmp/existing".to_string(),
            language: None,
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        };
        db.insert_project(&project).unwrap();

        let result = migrate_json_to_sqlite(&db);
        assert!(result.is_ok());
        // Should still have just 1 project
        assert_eq!(db.count_projects().unwrap(), 1);
    }

    #[test]
    fn test_migrate_projects_json() {
        let db = Database::new_in_memory().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let projects_file = tmp.path().join("projects.json");

        let json = r#"{
            "projects": [
                {
                    "id": "p1",
                    "name": "my-project",
                    "path": "/home/user/projects/my-project",
                    "language": "Rust",
                    "linked_resources": [],
                    "local_resources": [],
                    "last_scanned": "2026-03-01T00:00:00Z"
                }
            ],
            "scan_directories": ["/home/user/projects"]
        }"#;
        std::fs::write(&projects_file, json).unwrap();

        migrate_projects(&db, &projects_file).unwrap();

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, "p1");
        assert_eq!(projects[0].name, "my-project");
        assert_eq!(projects[0].language, Some("Rust".to_string()));
    }

    #[test]
    fn test_migrate_projects_empty_language() {
        let db = Database::new_in_memory().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let projects_file = tmp.path().join("projects.json");

        let json = r#"{
            "projects": [
                {
                    "id": "p2",
                    "name": "bare-project",
                    "path": "/home/user/projects/bare",
                    "language": "",
                    "linked_resources": [],
                    "local_resources": [],
                    "last_scanned": ""
                }
            ],
            "scan_directories": []
        }"#;
        std::fs::write(&projects_file, json).unwrap();

        migrate_projects(&db, &projects_file).unwrap();

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, "p2");
        assert_eq!(projects[0].language, None);
        assert_eq!(projects[0].last_scanned, None);
    }

    #[test]
    fn test_migrate_library_index() {
        let db = Database::new_in_memory().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();

        // Create a file for the library resource to point to
        let resource_file = tmp.path().join("my-rule.md");
        std::fs::write(&resource_file, "# My Rule").unwrap();

        let index_file = tmp.path().join("index.json");
        let json = format!(
            r#"{{
            "resources": [
                {{
                    "id": "r1",
                    "resource_type": "rule",
                    "name": "my-rule",
                    "description": "A test rule",
                    "tags": ["test"],
                    "path": "{}",
                    "linked_projects": [],
                    "created_at": "2026-03-01T00:00:00Z",
                    "updated_at": "2026-03-01T00:00:00Z"
                }}
            ]
        }}"#,
            resource_file.to_string_lossy().replace('\\', "\\\\")
        );
        std::fs::write(&index_file, json).unwrap();

        migrate_library(&db, &index_file).unwrap();

        let resources = db.list_resources_by_scope(&ResourceScope::Library).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].id, "r1");
        assert_eq!(resources[0].name, "my-rule");
        assert_eq!(resources[0].scope, ResourceScope::Library);
        assert!(resources[0].description.is_some());
        assert_eq!(resources[0].description, Some("A test rule".to_string()));
        assert!(resources[0].content_hash.is_some());
    }

    #[test]
    fn test_migrate_library_empty_description_and_tags() {
        let db = Database::new_in_memory().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();

        let resource_file = tmp.path().join("my-agent.md");
        std::fs::write(&resource_file, "# My Agent").unwrap();

        let index_file = tmp.path().join("index.json");
        let json = format!(
            r#"{{
            "resources": [
                {{
                    "id": "r2",
                    "resource_type": "agent",
                    "name": "my-agent",
                    "description": "",
                    "tags": [],
                    "path": "{}",
                    "linked_projects": [],
                    "created_at": "2026-03-01T00:00:00Z",
                    "updated_at": "2026-03-01T00:00:00Z"
                }}
            ]
        }}"#,
            resource_file.to_string_lossy().replace('\\', "\\\\")
        );
        std::fs::write(&index_file, json).unwrap();

        migrate_library(&db, &index_file).unwrap();

        let resources = db.list_resources_by_scope(&ResourceScope::Library).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].description, None);
        assert_eq!(resources[0].metadata, None);
    }

    #[test]
    fn test_migrate_multiple_projects() {
        let db = Database::new_in_memory().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let projects_file = tmp.path().join("projects.json");

        let json = r#"{
            "projects": [
                {
                    "id": "p1",
                    "name": "project-a",
                    "path": "/home/user/projects/a",
                    "language": "Go",
                    "linked_resources": [],
                    "local_resources": [],
                    "last_scanned": "2026-03-01T00:00:00Z"
                },
                {
                    "id": "p2",
                    "name": "project-b",
                    "path": "/home/user/projects/b",
                    "language": "Rust",
                    "linked_resources": [
                        {
                            "resource_type": "skill",
                            "library_id": "lib1",
                            "project_path": "/home/user/projects/b/.claude/skills/shared",
                            "symlink_valid": true
                        }
                    ],
                    "local_resources": [
                        {
                            "resource_type": "rule",
                            "name": "local-rule",
                            "path": "/home/user/projects/b/.claude/rules/local-rule.md"
                        }
                    ],
                    "last_scanned": "2026-02-28T12:00:00Z"
                }
            ],
            "scan_directories": ["/home/user/projects"]
        }"#;
        std::fs::write(&projects_file, json).unwrap();

        migrate_projects(&db, &projects_file).unwrap();

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_convert_resource_type() {
        use crate::models::project::ResourceType as OldType;

        assert_eq!(convert_resource_type(&OldType::Skill).as_str(), "skill");
        assert_eq!(convert_resource_type(&OldType::Agent).as_str(), "agent");
        assert_eq!(convert_resource_type(&OldType::Rule).as_str(), "rule");
        assert_eq!(convert_resource_type(&OldType::Hook).as_str(), "hook");
        assert_eq!(convert_resource_type(&OldType::Command).as_str(), "command");
    }
}
