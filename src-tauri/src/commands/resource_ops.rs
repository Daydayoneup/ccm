use std::fs;
use std::path::Path;

use crate::adapters::AdapterRegistry;
use crate::db::Database;
use crate::models::v2::{Resource, ResourceLink, ResourceScope, ResourceType};
use crate::scanner;

/// Shared resource creation logic used by library, project, and global create commands.
pub(crate) fn create_resource_at(
    db: &Database,
    adapter_registry: &AdapterRegistry,
    resource_type: &str,
    name: &str,
    description: Option<String>,
    content: &str,
    base_dir: &Path,
    scope: ResourceScope,
) -> Result<Resource, String> {
    let rtype = ResourceType::from_str(resource_type)
        .ok_or_else(|| format!("Invalid resource type: {}", resource_type))?;

    let adapter = adapter_registry
        .get(&rtype)
        .ok_or_else(|| format!("No adapter for resource type: {}", resource_type))?;

    adapter.validate_content(content)?;

    let file_path = adapter.resolve_file_path(base_dir, name);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&file_path, content).map_err(|e| e.to_string())?;

    let source_path = adapter.source_path_from_file(&file_path);
    let hash = scanner::compute_file_hash(&source_path);
    let metadata = adapter.metadata_from_content(content);
    let now = chrono::Utc::now().to_rfc3339();

    let resource = Resource {
        id: uuid::Uuid::new_v4().to_string(),
        resource_type: rtype,
        name: name.to_string(),
        description,
        scope,
        source_path,
        content_hash: hash,
        metadata,
        created_at: now.clone(),
        updated_at: now,
        version: None,
        is_draft: 1,
            installed_from_id: None,
    };

    db.insert_resource(&resource).map_err(|e| e.to_string())?;
    Ok(resource)
}

/// Shared install logic — delegates to install_service for unified tracking.
pub(crate) fn install_resource_to(
    db: &Database,
    resource: &Resource,
    scope: crate::install::InstallScope,
    adapter_registry: &AdapterRegistry,
) -> Result<ResourceLink, String> {
    crate::install_service::install(db, resource, scope, adapter_registry)
}
