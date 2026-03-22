// Shared logic for file-based resource adapters (FileBased install strategy).
// Concrete implementations: AgentAdapter, SkillAdapter, RuleAdapter, CommandAdapter.

use super::{InstallTarget, LinkType, TargetScope};
use crate::models::v2::{Project, Resource, ResourceLink, ResourceScope, ResourceType};
use crate::scanner::compute_file_hash;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use uuid::Uuid;

/// Compute target path for a file-based install.
///
/// - Global:  `~/.claude/<type_dir>/<file_name>`
/// - Project: `<project.path>/.claude/<type_dir>/<file_name>`
pub fn resolve_file_target(
    scope: &TargetScope,
    type_dir: &str,
    file_name: &str,
    project: Option<&Project>,
) -> Result<InstallTarget, String> {
    let target_path = match scope {
        TargetScope::Global => {
            let home = dirs::home_dir()
                .ok_or_else(|| "Cannot determine home directory".to_string())?;
            home.join(".claude").join(type_dir).join(file_name)
        }
        TargetScope::Project => {
            let proj = project.ok_or_else(|| {
                "Project scope requires a project to be provided".to_string()
            })?;
            PathBuf::from(&proj.path)
                .join(".claude")
                .join(type_dir)
                .join(file_name)
        }
    };
    Ok(InstallTarget::FilePath(target_path))
}

/// Execute a symlink or copy install.
///
/// Returns a `ResourceLink` with `target_scope` and `project_id` left empty —
/// the command layer is responsible for filling those in.
pub fn file_install(
    source: &Path,
    target: &Path,
    link_type: &LinkType,
    resource_id: &str,
) -> Result<ResourceLink, String> {
    if target.exists() {
        return Err(format!(
            "Target already exists: {}",
            target.display()
        ));
    }

    // Ensure parent directories exist.
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent dirs for {}: {}", target.display(), e))?;
    }

    match link_type {
        LinkType::Symlink => {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(source, target).map_err(|e| {
                    format!(
                        "Failed to create symlink {} -> {}: {}",
                        target.display(),
                        source.display(),
                        e
                    )
                })?;
            }
            #[cfg(not(unix))]
            {
                return Err("Symlinks are only supported on Unix platforms".to_string());
            }
        }
        LinkType::Copy => {
            if source.is_dir() {
                copy_dir_recursive(source, target)?;
            } else {
                fs::copy(source, target).map_err(|e| {
                    format!(
                        "Failed to copy {} to {}: {}",
                        source.display(),
                        target.display(),
                        e
                    )
                })?;
            }
        }
        LinkType::ConfigMerge => {
            return Err("ConfigMerge is not supported for file-based installs".to_string());
        }
        LinkType::PluginInstall => {
            return Err("PluginInstall is not supported for file-based installs".to_string());
        }
    }

    let now = Utc::now().to_rfc3339();
    Ok(ResourceLink {
        id: Uuid::new_v4().to_string(),
        resource_id: resource_id.to_string(),
        target_scope: String::new(),   // caller fills this in
        target_path: target.to_string_lossy().to_string(),
        config_key: None,              // file-based links have no config key
        project_id: None,              // caller fills this in
        link_type: link_type.as_str().to_string(),
        created_at: now,
    })
}

/// Remove a symlink or copied file/directory described by `link`.
///
/// If the target no longer exists this is treated as success (idempotent).
pub fn file_uninstall(link: &ResourceLink) -> Result<(), String> {
    let target = Path::new(&link.target_path);

    if !target.exists() && !target.symlink_metadata().is_ok() {
        // Already gone — that's fine.
        return Ok(());
    }

    if target.is_symlink() || target.is_file() {
        fs::remove_file(target).map_err(|e| {
            format!("Failed to remove {}: {}", target.display(), e)
        })?;
    } else if target.is_dir() {
        fs::remove_dir_all(target).map_err(|e| {
            format!("Failed to remove directory {}: {}", target.display(), e)
        })?;
    }

    Ok(())
}

/// Detect if a path is a symlink pointing to a registry or library location.
/// Returns the effective scope and optional metadata with origin info.
fn detect_symlink_origin(path: &Path, default_scope: &ResourceScope) -> (ResourceScope, Option<String>) {
    // Check if this is a symlink
    let is_symlink = path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if !is_symlink {
        return (default_scope.clone(), None);
    }

    // Resolve the symlink target
    if let Ok(target) = fs::read_link(path) {
        let target_str = target.to_string_lossy();

        // Check if target is under a registry path
        if target_str.contains(".claude-manager/registries/") {
            return (
                ResourceScope::Registry,
                Some(format!(r#"{{"origin":"registry","symlink_target":"{}"}}"#, target_str)),
            );
        }

        // Check if target is under the library path
        if target_str.contains(".claude-manager/library/") {
            return (
                ResourceScope::Library,
                Some(format!(r#"{{"origin":"library","symlink_target":"{}"}}"#, target_str)),
            );
        }
    }

    (default_scope.clone(), None)
}

/// Discover resources in a directory.
///
/// - `is_directory = true`  → skills (subdirectories containing SKILL.md)
/// - `is_directory = false` → agents / rules / commands (.md files)
///
/// The `base_path` meaning depends on `scope`:
/// - Global:  ignored; scan dir is `~/.claude/<type_dir>`
/// - Project: scan dir is `<base_path>/.claude/<type_dir>`
/// - Library: scan dir is `<base_path>/<type_dir>`
pub fn scan_file_resources(
    scope: &ResourceScope,
    base_path: &Path,
    type_dir: &str,
    resource_type: &ResourceType,
    is_directory: bool,
) -> Result<Vec<Resource>, String> {
    let scan_dir = match scope {
        ResourceScope::Global => {
            let home = dirs::home_dir()
                .ok_or_else(|| "Cannot determine home directory".to_string())?;
            home.join(".claude").join(type_dir)
        }
        ResourceScope::Project => base_path.join(".claude").join(type_dir),
        ResourceScope::Library => base_path.join(type_dir),
        // Plugin and Registry scopes are not file-scanned by this helper.
        ResourceScope::Plugin | ResourceScope::Registry => {
            return Err(format!(
                "scan_file_resources does not support scope: {}",
                scope.as_str()
            ));
        }
    };

    if !scan_dir.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&scan_dir)
        .map_err(|e| format!("Failed to read directory {}: {}", scan_dir.display(), e))?;

    let now = Utc::now().to_rfc3339();
    let mut resources = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // Detect symlink origin: if the entry is a symlink pointing to a
        // registry path, record the origin so the UI can show a badge.
        let (effective_scope, metadata) = detect_symlink_origin(&path, scope);

        if is_directory {
            // Skills: look for subdirectories containing SKILL.md
            if !path.is_dir() {
                continue;
            }
            if !path.join("SKILL.md").is_file() {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let source_path = path.to_string_lossy().to_string();
            let content_hash = compute_file_hash(&source_path);
            resources.push(Resource {
                id: Uuid::new_v4().to_string(),
                resource_type: resource_type.clone(),
                name,
                description: None,
                scope: effective_scope.clone(),
                source_path,
                content_hash,
                metadata: metadata.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        } else {
            // Agents / Rules / Commands: look for .md files
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let name = path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let source_path = path.to_string_lossy().to_string();
            let content_hash = compute_file_hash(&source_path);
            resources.push(Resource {
                id: Uuid::new_v4().to_string(),
                resource_type: resource_type.clone(),
                name,
                description: None,
                scope: effective_scope.clone(),
                source_path,
                content_hash,
                metadata: metadata.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        }
    }

    Ok(resources)
}

/// Recursively copy a directory from `src` to `dst`.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // resolve_file_target
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_file_target_global() {
        let result = resolve_file_target(&TargetScope::Global, "agents", "my-agent.md", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                assert!(s.contains("/.claude/agents/my-agent.md"), "got: {}", s);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_file_target_project() {
        let tmp = TempDir::new().unwrap();
        let project = Project {
            id: "p1".to_string(),
            name: "myproject".to_string(),
            path: tmp.path().to_string_lossy().to_string(),
            language: None,
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        };
        let result = resolve_file_target(
            &TargetScope::Project,
            "skills",
            "my-skill",
            Some(&project),
        )
        .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let expected = tmp.path().join(".claude/skills/my-skill");
                assert_eq!(p, expected);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_file_target_project_missing_error() {
        let err = resolve_file_target(&TargetScope::Project, "agents", "my-agent.md", None)
            .expect_err("should fail without project");
        assert!(err.contains("project"), "error should mention 'project': {}", err);
    }

    // -----------------------------------------------------------------------
    // file_install
    // -----------------------------------------------------------------------

    #[test]
    fn test_file_install_copy_file() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("agent.md");
        fs::write(&source, "# Agent").unwrap();

        let target = tmp.path().join("dest/agent.md");
        let link = file_install(&source, &target, &LinkType::Copy, "res-1")
            .expect("copy should succeed");

        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "# Agent");
        assert_eq!(link.resource_id, "res-1");
        assert_eq!(link.link_type, "copy");
        assert_eq!(link.target_path, target.to_string_lossy());
        assert!(link.config_key.is_none());
        assert!(link.project_id.is_none());
        assert!(link.target_scope.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn test_file_install_symlink() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("rule.md");
        fs::write(&source, "# Rule").unwrap();

        let target = tmp.path().join("dest/rule.md");
        let link = file_install(&source, &target, &LinkType::Symlink, "res-2")
            .expect("symlink should succeed");

        assert!(target.exists());
        assert!(target.is_symlink());
        assert_eq!(link.link_type, "symlink");
    }

    #[test]
    fn test_file_install_target_exists_error() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("agent.md");
        let target = tmp.path().join("agent.md");
        fs::write(&source, "# Agent").unwrap();
        // source == target, target already exists
        let err = file_install(&source, &target, &LinkType::Copy, "res-3")
            .expect_err("should fail because target exists");
        assert!(err.contains("already exists"), "error: {}", err);
    }

    #[test]
    fn test_file_install_copy_directory() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("my-skill");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("SKILL.md"), "# Skill").unwrap();
        fs::write(src_dir.join("helper.md"), "helper").unwrap();

        let target = tmp.path().join("dest/my-skill");
        let link = file_install(&src_dir, &target, &LinkType::Copy, "res-4")
            .expect("copy dir should succeed");

        assert!(target.is_dir());
        assert!(target.join("SKILL.md").exists());
        assert!(target.join("helper.md").exists());
        assert_eq!(link.link_type, "copy");
    }

    // -----------------------------------------------------------------------
    // file_uninstall
    // -----------------------------------------------------------------------

    fn make_link(target_path: &str) -> ResourceLink {
        ResourceLink {
            id: Uuid::new_v4().to_string(),
            resource_id: "r1".to_string(),
            target_scope: String::new(),
            target_path: target_path.to_string(),
            config_key: None,
            project_id: None,
            link_type: "copy".to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_file_uninstall_symlink() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("rule.md");
        fs::write(&source, "# Rule").unwrap();
        let target = tmp.path().join("link.md");
        std::os::unix::fs::symlink(&source, &target).unwrap();

        let link = make_link(target.to_str().unwrap());
        file_uninstall(&link).expect("should remove symlink");
        assert!(!target.exists());
        // source should still be there
        assert!(source.exists());
    }

    #[test]
    fn test_file_uninstall_copy() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("agent.md");
        fs::write(&target, "# Agent").unwrap();

        let link = make_link(target.to_str().unwrap());
        file_uninstall(&link).expect("should remove file");
        assert!(!target.exists());
    }

    #[test]
    fn test_file_uninstall_nonexistent_ok() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("nonexistent.md");
        // File does not exist — should succeed silently.
        let link = make_link(target.to_str().unwrap());
        file_uninstall(&link).expect("should be ok for nonexistent target");
    }

    // -----------------------------------------------------------------------
    // scan_file_resources
    // -----------------------------------------------------------------------

    #[test]
    fn test_scan_file_resources_md_files() {
        let tmp = TempDir::new().unwrap();
        let agents_dir = tmp.path().join(".claude/agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("reviewer.md"), "# Reviewer").unwrap();
        fs::write(agents_dir.join("builder.md"), "# Builder").unwrap();
        // Non-md file should be ignored.
        fs::write(agents_dir.join("notes.txt"), "ignored").unwrap();

        let resources = scan_file_resources(
            &ResourceScope::Project,
            tmp.path(),
            "agents",
            &ResourceType::Agent,
            false,
        )
        .expect("scan should succeed");

        assert_eq!(resources.len(), 2);
        let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"reviewer"));
        assert!(names.contains(&"builder"));
        for r in &resources {
            assert_eq!(r.resource_type, ResourceType::Agent);
            assert_eq!(r.scope, ResourceScope::Project);
            assert!(r.content_hash.is_some());
        }
    }

    #[test]
    fn test_scan_file_resources_directories() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join(".claude/skills");
        fs::create_dir_all(skills_dir.join("deploy")).unwrap();
        fs::write(skills_dir.join("deploy/SKILL.md"), "# Deploy").unwrap();

        // A directory without SKILL.md should be ignored.
        fs::create_dir_all(skills_dir.join("empty-dir")).unwrap();

        // A plain file should be ignored.
        fs::write(skills_dir.join("loose.md"), "ignored").unwrap();

        let resources = scan_file_resources(
            &ResourceScope::Project,
            tmp.path(),
            "skills",
            &ResourceType::Skill,
            true,
        )
        .expect("scan should succeed");

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "deploy");
        assert_eq!(resources[0].resource_type, ResourceType::Skill);
        assert!(resources[0].content_hash.is_some());
    }

    #[test]
    fn test_scan_file_resources_empty_dir() {
        let tmp = TempDir::new().unwrap();
        // The .claude/rules directory does not exist at all.
        let resources = scan_file_resources(
            &ResourceScope::Project,
            tmp.path(),
            "rules",
            &ResourceType::Rule,
            false,
        )
        .expect("scan should return empty vec for missing dir");

        assert!(resources.is_empty());
    }

    #[test]
    fn test_scan_file_resources_library_scope() {
        let tmp = TempDir::new().unwrap();
        let commands_dir = tmp.path().join("commands");
        fs::create_dir_all(&commands_dir).unwrap();
        fs::write(commands_dir.join("build.md"), "# Build").unwrap();

        let resources = scan_file_resources(
            &ResourceScope::Library,
            tmp.path(),
            "commands",
            &ResourceType::Command,
            false,
        )
        .expect("scan should succeed for Library scope");

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "build");
        assert_eq!(resources[0].scope, ResourceScope::Library);
    }

    // -----------------------------------------------------------------------
    // copy_dir_recursive
    // -----------------------------------------------------------------------

    #[test]
    fn test_copy_dir_recursive() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("root.md"), "root").unwrap();
        fs::write(src.join("sub/nested.md"), "nested").unwrap();

        let dst = tmp.path().join("dst");
        copy_dir_recursive(&src, &dst).expect("copy should succeed");

        assert!(dst.join("root.md").exists());
        assert!(dst.join("sub/nested.md").exists());
        assert_eq!(fs::read_to_string(dst.join("sub/nested.md")).unwrap(), "nested");
    }
}
