use tauri::State;
use crate::db::Database;
use crate::models::v2::{Resource, ResourceVersion};
use crate::frontmatter;
use crate::scanner;
use std::path::Path;
use std::fs;

/// Recursively copy directory contents from src to dst, skipping any `.versions/` subdirectory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Skip the .versions directory
        if name_str == ".versions" {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {:?} to {:?}: {}", src_path, dst_path, e))?;
        }
    }
    Ok(())
}

/// Publish a new version snapshot of a library resource.
///
/// Steps:
/// 1. Validate `version` is valid semver.
/// 2. Fetch the resource from the DB.
/// 3. Compute the current content hash.
/// 4. Copy all files (excluding `.versions/`) into `<source_path>/.versions/<version>/`.
/// 5. Insert a `ResourceVersion` record.
/// 6. Update the resource's `version` and `is_draft` fields.
#[tauri::command]
pub fn publish_resource_version(
    db: State<Database>,
    resource_id: String,
    version: String,
    changelog: Option<String>,
) -> Result<ResourceVersion, String> {
    // 1. Validate semver
    frontmatter::validate_semver(&version)?;

    // 2. Fetch resource
    let resource: Resource = db
        .get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    // 3. Compute content hash
    let content_hash = scanner::compute_file_hash(&resource.source_path)
        .unwrap_or_default();

    // 4. Create version snapshot directory and copy files
    let source_path = Path::new(&resource.source_path);
    let version_dir = source_path.join(".versions").join(&version);
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

    // Copy all contents (skip .versions itself) into the version snapshot
    copy_dir_recursive(source_path, &version_dir)?;

    // 5. Build and insert ResourceVersion record
    let now = chrono::Utc::now().to_rfc3339();
    let rv = ResourceVersion {
        id: uuid::Uuid::new_v4().to_string(),
        resource_id: resource_id.clone(),
        version: version.clone(),
        changelog,
        content_hash,
        created_at: now.clone(),
    };
    db.insert_resource_version(&rv)?;

    // 6. Update resource: mark published version and clear draft flag
    let mut updated = resource.clone();
    updated.version = Some(version.clone());
    updated.is_draft = 0;
    updated.updated_at = now;
    db.update_resource(&updated).map_err(|e| e.to_string())?;

    Ok(rv)
}

/// List all versions recorded for a given resource, ordered newest first.
#[tauri::command]
pub fn list_resource_versions(
    db: State<Database>,
    resource_id: String,
) -> Result<Vec<ResourceVersion>, String> {
    db.list_resource_versions(&resource_id)
}

/// Roll back a resource's content to a previously published version snapshot.
///
/// Steps:
/// 1. Fetch the resource.
/// 2. Verify the version snapshot directory exists.
/// 3. Delete all files/dirs in source_path except `.versions/`.
/// 4. Copy the snapshot back into source_path.
/// 5. Update the resource record (version, is_draft, content_hash).
#[tauri::command]
pub fn rollback_resource_version(
    db: State<Database>,
    resource_id: String,
    version: String,
) -> Result<(), String> {
    // 1. Fetch resource
    let resource: Resource = db
        .get_resource(&resource_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    let source_path = Path::new(&resource.source_path);

    // 2. Verify version snapshot exists
    let version_path = source_path.join(".versions").join(&version);
    if !version_path.exists() {
        return Err(format!(
            "Version '{}' snapshot not found at {:?}",
            version, version_path
        ));
    }

    // 3. Delete everything in source_path except .versions/
    if source_path.exists() {
        let entries = fs::read_dir(source_path).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            if name.to_string_lossy() == ".versions" {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
            } else {
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }

    // 4. Copy snapshot back into source_path (copy_dir_recursive skips .versions)
    copy_dir_recursive(&version_path, source_path)?;

    // 5. Recompute hash and update resource record
    let new_hash = scanner::compute_file_hash(&resource.source_path)
        .unwrap_or_default();

    let now = chrono::Utc::now().to_rfc3339();
    let mut updated = resource.clone();
    updated.version = Some(version.clone());
    updated.is_draft = 0;
    updated.content_hash = Some(new_hash);
    updated.updated_at = now;
    db.update_resource(&updated).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::v2::{ResourceType, ResourceScope};
    use tempfile::TempDir;

    fn make_resource_in_dir(dir: &Path) -> Resource {
        // Write a test file inside the dir
        fs::write(dir.join("SKILL.md"), "# Test Skill\nSome content").unwrap();
        Resource {
            id: "res1".to_string(),
            resource_type: ResourceType::Skill,
            name: "test-skill".to_string(),
            description: None,
            scope: ResourceScope::Library,
            source_path: dir.to_string_lossy().to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
        }
    }

    fn setup_db_with_resource(dir: &Path) -> Database {
        let db = Database::new_in_memory().unwrap();
        let resource = make_resource_in_dir(dir);
        db.insert_resource(&resource).unwrap();
        db
    }

    #[test]
    fn test_copy_dir_recursive_skips_versions() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file.md"), "content").unwrap();
        // Create a .versions dir that should be skipped
        let versions_dir = src.join(".versions").join("1.0.0");
        fs::create_dir_all(&versions_dir).unwrap();
        fs::write(versions_dir.join("file.md"), "old content").unwrap();

        let dst = tmp.path().join("dst");
        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("file.md").exists());
        assert!(!dst.join(".versions").exists(), ".versions should not be copied");
    }

    #[test]
    fn test_publish_resource_version_creates_snapshot() {
        let tmp = TempDir::new().unwrap();
        let resource_dir = tmp.path().join("test-skill");
        fs::create_dir_all(&resource_dir).unwrap();
        let db = setup_db_with_resource(&resource_dir);

        // Directly call the DB-level logic (command requires State<Database>)
        let resource = db.get_resource("res1").unwrap().unwrap();
        frontmatter::validate_semver("1.0.0").unwrap();
        let content_hash = scanner::compute_file_hash(&resource.source_path).unwrap_or_default();
        let version_dir = Path::new(&resource.source_path).join(".versions").join("1.0.0");
        fs::create_dir_all(&version_dir).unwrap();
        copy_dir_recursive(Path::new(&resource.source_path), &version_dir).unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        let rv = ResourceVersion {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: "res1".to_string(),
            version: "1.0.0".to_string(),
            changelog: Some("Initial".to_string()),
            content_hash,
            created_at: now.clone(),
        };
        db.insert_resource_version(&rv).unwrap();

        let mut updated = resource.clone();
        updated.version = Some("1.0.0".to_string());
        updated.is_draft = 0;
        updated.updated_at = now;
        db.update_resource(&updated).unwrap();

        // Verify snapshot created
        assert!(version_dir.exists());
        assert!(version_dir.join("SKILL.md").exists());

        // Verify DB record
        let versions = db.list_resource_versions("res1").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "1.0.0");

        // Verify resource updated
        let r = db.get_resource("res1").unwrap().unwrap();
        assert_eq!(r.version, Some("1.0.0".to_string()));
        assert_eq!(r.is_draft, 0);
    }

    #[test]
    fn test_invalid_semver_rejected() {
        assert!(frontmatter::validate_semver("1.0").is_err());
        assert!(frontmatter::validate_semver("bad").is_err());
        assert!(frontmatter::validate_semver("1.0.0").is_ok());
    }

    #[test]
    fn test_rollback_logic() {
        let tmp = TempDir::new().unwrap();
        let resource_dir = tmp.path().join("test-skill");
        fs::create_dir_all(&resource_dir).unwrap();
        let db = setup_db_with_resource(&resource_dir);

        // Overwrite file with "# Version 1" BEFORE snapshotting
        fs::write(resource_dir.join("SKILL.md"), "# Version 1").unwrap();

        // Simulate a publish: create snapshot
        let version_dir = resource_dir.join(".versions").join("1.0.0");
        fs::create_dir_all(&version_dir).unwrap();
        copy_dir_recursive(&resource_dir, &version_dir).unwrap();

        let rv = ResourceVersion {
            id: "v1".to_string(),
            resource_id: "res1".to_string(),
            version: "1.0.0".to_string(),
            changelog: None,
            content_hash: "h1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_resource_version(&rv).unwrap();

        // Modify the working file (simulating changes after publishing)
        fs::write(resource_dir.join("SKILL.md"), "# Modified content").unwrap();
        let content_before = fs::read_to_string(resource_dir.join("SKILL.md")).unwrap();
        assert!(content_before.contains("Modified"));

        // Simulate rollback
        let resource = db.get_resource("res1").unwrap().unwrap();
        assert!(version_dir.exists());

        // Delete everything except .versions
        for entry in fs::read_dir(&resource_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name().to_string_lossy() == ".versions" {
                continue;
            }
            let p = entry.path();
            if p.is_dir() { fs::remove_dir_all(&p).unwrap(); } else { fs::remove_file(&p).unwrap(); }
        }

        // Copy snapshot back
        copy_dir_recursive(&version_dir, &resource_dir).unwrap();

        // Verify content restored
        let content_after = fs::read_to_string(resource_dir.join("SKILL.md")).unwrap();
        assert!(content_after.contains("Version 1"));
        assert!(!content_after.contains("Modified"));

        // Update resource record
        let mut updated = resource.clone();
        updated.version = Some("1.0.0".to_string());
        updated.is_draft = 0;
        db.update_resource(&updated).unwrap();

        let r = db.get_resource("res1").unwrap().unwrap();
        assert_eq!(r.version, Some("1.0.0".to_string()));
        assert_eq!(r.is_draft, 0);
    }
}
