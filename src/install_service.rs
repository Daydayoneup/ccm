//! Unified install service layer.
//!
//! Single entry point for all install-related operations:
//! install, uninstall, query_install_status, update_installed.
//!
//! All command files should delegate to this module instead of
//! calling install.rs directly.

use crate::adapters::AdapterRegistry;
use crate::db::Database;
use crate::install::{InstallScope, InstallStrategy};
use crate::models::v2::{Resource, ResourceLink};

/// Unified install status for a single installation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallStatus {
    pub link: ResourceLink,
    pub has_update: bool,
}

/// Query all installations of a source resource.
///
/// Returns one InstallStatus per installation (each link = one installation).
/// Works for both FileBased (link_type="symlink") and ConfigBased (link_type="config_merge").
pub fn query_install_status(
    db: &Database,
    source_id: &str,
) -> Result<Vec<InstallStatus>, String> {
    let source = db
        .get_resource(source_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Source resource not found: {}", source_id))?;

    let links = db
        .list_links_by_resource(source_id)
        .map_err(|e| e.to_string())?;

    let statuses = links
        .into_iter()
        .map(|link| {
            let has_update = match (source.content_hash.as_deref(), link.installed_hash.as_deref()) {
                (Some(src), Some(dst)) => src != dst,
                _ => false,
            };
            InstallStatus { link, has_update }
        })
        .collect();

    Ok(statuses)
}

/// Install a source resource to a target scope.
///
/// Handles both FileBased and ConfigBased strategies.
/// Always inserts a resource_links record as the single source of truth.
pub fn install(
    db: &Database,
    source: &Resource,
    scope: InstallScope,
    adapter_registry: &AdapterRegistry,
) -> Result<ResourceLink, String> {
    let adapter = adapter_registry
        .get(&source.resource_type)
        .ok_or_else(|| format!("No adapter for {:?}", source.resource_type))?;

    let strategy = adapter.install_strategy();
    match strategy {
        crate::adapters::InstallStrategy::FileBased => {
            let ib = crate::install::installed_base()?;
            let ch = crate::install::claude_home()?;
            install_file_based_with_link(db, source, scope, &ib, &ch)
        }
        crate::adapters::InstallStrategy::ConfigBased => {
            // ConfigBased already inserts link in install.rs
            crate::install::install_resource(db, source, scope, InstallStrategy::ConfigBased, adapter_registry)
        }
    }
}

/// Testable variant that accepts explicit paths.
pub fn install_with_paths(
    db: &Database,
    source: &Resource,
    scope: InstallScope,
    adapter_registry: &AdapterRegistry,
    ib: &std::path::Path,
    ch: &std::path::Path,
) -> Result<ResourceLink, String> {
    let adapter = adapter_registry
        .get(&source.resource_type)
        .ok_or_else(|| format!("No adapter for {:?}", source.resource_type))?;

    let strategy = adapter.install_strategy();
    match strategy {
        crate::adapters::InstallStrategy::FileBased => {
            install_file_based_with_link(db, source, scope, ib, ch)
        }
        crate::adapters::InstallStrategy::ConfigBased => {
            crate::install::install_resource_with_paths(db, source, scope, InstallStrategy::ConfigBased, ib, ch, adapter_registry)
        }
    }
}

/// FileBased install that also inserts a resource_links record.
/// This is the key unification: FileBased installs now create link records
/// just like ConfigBased installs do.
fn install_file_based_with_link(
    db: &Database,
    source: &Resource,
    scope: InstallScope,
    ib: &std::path::Path,
    ch: &std::path::Path,
) -> Result<ResourceLink, String> {
    // Call the existing install_file_based which returns a link but doesn't insert it
    let link = crate::install::install_resource_with_paths(
        db, source, scope, InstallStrategy::FileBased, ib, ch,
        &AdapterRegistry::new(),
    )?;

    // Insert the link record — this is the key change
    db.insert_link(&link)
        .map_err(|e| format!("Failed to insert link: {}", e))?;

    Ok(link)
}

/// Uninstall resources by link IDs.
pub fn uninstall(
    db: &Database,
    link_ids: Vec<String>,
    adapter_registry: &AdapterRegistry,
) -> Result<Vec<String>, String> {
    crate::install::uninstall_resource(db, link_ids, adapter_registry)
}

/// Testable variant.
pub fn uninstall_with_base(
    db: &Database,
    link_ids: Vec<String>,
    ib: &std::path::Path,
    adapter_registry: &AdapterRegistry,
) -> Result<Vec<String>, String> {
    crate::install::uninstall_resource_with_base(db, link_ids, ib, adapter_registry)
}

/// Update all installations of a source resource.
pub fn update_installed(
    db: &Database,
    source_id: &str,
) -> Result<(), String> {
    crate::install::update_installed(db, source_id)
}

/// Testable variant.
pub fn update_installed_with_base(
    db: &Database,
    source_id: &str,
    ib: &std::path::Path,
) -> Result<(), String> {
    crate::install::update_installed_with_base(db, source_id, ib)
}

/// Convert upstream-removed resource to library scope.
pub fn retain_as_library(
    db: &Database,
    resource_id: &str,
) -> Result<Resource, String> {
    crate::install::retain_as_library(db, resource_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::v2::{Resource, ResourceScope, ResourceType};

    fn make_source_at(db: &Database, id: &str, hash: &str, source_path: &str) {
        let resource = Resource {
            id: id.to_string(),
            resource_type: ResourceType::Skill,
            name: "test-skill".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: source_path.to_string(),
            content_hash: Some(hash.to_string()),
            metadata: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();
    }

    fn make_source(db: &Database, id: &str, hash: &str) {
        let resource = Resource {
            id: id.to_string(),
            resource_type: ResourceType::Skill,
            name: "test-skill".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: "/fake/source".to_string(),
            content_hash: Some(hash.to_string()),
            metadata: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();
    }

    fn make_link(db: &Database, id: &str, resource_id: &str, installed_hash: &str, link_type: &str) {
        let link = ResourceLink {
            id: id.to_string(),
            resource_id: resource_id.to_string(),
            target_scope: "global".to_string(),
            target_path: format!("/target/{}", id),
            config_key: None,
            project_id: None,
            link_type: link_type.to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            installed_hash: Some(installed_hash.to_string()),
        };
        db.insert_link(&link).unwrap();
    }

    #[test]
    fn test_query_no_installations() {
        let db = Database::new_in_memory().unwrap();
        make_source(&db, "src-1", "hash-v1");
        let status = query_install_status(&db, "src-1").unwrap();
        assert!(status.is_empty());
    }

    #[test]
    fn test_query_up_to_date() {
        let db = Database::new_in_memory().unwrap();
        make_source(&db, "src-1", "hash-v1");
        make_link(&db, "link-1", "src-1", "hash-v1", "symlink");
        let status = query_install_status(&db, "src-1").unwrap();
        assert_eq!(status.len(), 1);
        assert!(!status[0].has_update);
    }

    #[test]
    fn test_query_has_update() {
        let db = Database::new_in_memory().unwrap();
        make_source(&db, "src-1", "hash-v2");
        make_link(&db, "link-1", "src-1", "hash-v1", "symlink");
        let status = query_install_status(&db, "src-1").unwrap();
        assert_eq!(status.len(), 1);
        assert!(status[0].has_update);
    }

    #[test]
    fn test_query_multiple_installations() {
        let db = Database::new_in_memory().unwrap();
        make_source(&db, "src-1", "hash-v2");
        make_link(&db, "link-1", "src-1", "hash-v1", "symlink");
        make_link(&db, "link-2", "src-1", "hash-v2", "config_merge");
        let status = query_install_status(&db, "src-1").unwrap();
        assert_eq!(status.len(), 2);
        assert!(status.iter().any(|s| s.has_update));
        assert!(status.iter().any(|s| !s.has_update));
    }

    #[test]
    fn test_install_file_based_creates_link() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = Database::new_in_memory().unwrap();
        let ar = AdapterRegistry::new();

        // Create source skill
        let source_dir = tmp.path().join("library/skills/test-skill");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("SKILL.md"), "# Test").unwrap();

        make_source_at(&db, "src-1", "hash1", &source_dir.to_string_lossy());

        let ib = tmp.path().join("installed");
        let ch = tmp.path().join("claude-home");
        std::fs::create_dir_all(&ch).unwrap();

        let link = install_with_paths(
            &db, &db.get_resource("src-1").unwrap().unwrap(),
            InstallScope::Global, &ar, &ib, &ch,
        ).unwrap();

        // Link should be in DB
        assert_eq!(link.link_type, "symlink");
        assert_eq!(link.resource_id, "src-1");
        let db_link = db.get_link(&link.id).unwrap();
        assert!(db_link.is_some(), "link should be persisted to DB");

        // Query should find the installation
        let status = query_install_status(&db, "src-1").unwrap();
        assert_eq!(status.len(), 1);
    }
}
