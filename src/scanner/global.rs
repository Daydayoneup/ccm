use super::ScannedResource;

/// Scan ~/.claude/ for global resources, returning v2 ScannedResource list.
/// Adapters handle symlink detection for linked_metadata automatically.
pub fn scan_global_resources() -> Vec<ScannedResource> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let claude_dir = home.join(".claude");
    if !claude_dir.is_dir() {
        return Vec::new();
    }
    let adapter_registry = crate::adapters::AdapterRegistry::new();
    super::scan_resources_for_sync(
        &claude_dir,
        &crate::models::v2::ResourceScope::Global,
        &adapter_registry,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_claude_dir_as_scanned_resources() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("skills/my-skill")).unwrap();
        fs::write(claude_dir.join("skills/my-skill/SKILL.md"), "# Test Skill").unwrap();
        fs::create_dir_all(claude_dir.join("rules")).unwrap();
        fs::write(claude_dir.join("rules/style.md"), "# Style Rule").unwrap();

        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let resources = crate::scanner::scan_resources_for_sync(
            &claude_dir,
            &crate::models::v2::ResourceScope::Library,
            &adapter_registry,
        );

        assert_eq!(resources.len(), 2);
        assert!(resources.iter().any(|r| r.name == "my-skill" && r.content_hash.is_some()));
        assert!(resources.iter().any(|r| r.name == "style" && r.content_hash.is_some()));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlinks_to_registry_no_installed_from_id() {
        let tmp = TempDir::new().unwrap();

        let registry_dir = tmp.path().join(".claude-manager/registries/skills/agent-browser");
        fs::create_dir_all(&registry_dir).unwrap();
        fs::write(registry_dir.join("SKILL.md"), "# Agent Browser").unwrap();

        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("skills")).unwrap();
        std::os::unix::fs::symlink(&registry_dir, claude_dir.join("skills/agent-browser")).unwrap();

        fs::create_dir_all(claude_dir.join("skills/local-skill")).unwrap();
        fs::write(claude_dir.join("skills/local-skill/SKILL.md"), "# Local").unwrap();

        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let resources = crate::scanner::scan_resources_for_sync(
            &claude_dir,
            &crate::models::v2::ResourceScope::Library,
            &adapter_registry,
        );

        assert_eq!(resources.len(), 2);
        // Registry symlinks don't point to installed/ so installed_from_id is None
        let registry_res = resources.iter().find(|r| r.name == "agent-browser").unwrap();
        assert_eq!(registry_res.installed_from_id, None);
        assert_eq!(registry_res.linked_metadata, None);
        let local_res = resources.iter().find(|r| r.name == "local-skill").unwrap();
        assert_eq!(local_res.installed_from_id, None);
    }

    #[cfg(unix)]
    #[test]
    fn test_symlinks_to_library_no_installed_from_id() {
        let tmp = TempDir::new().unwrap();

        let library_dir = tmp.path().join(".claude-manager/library/rules");
        fs::create_dir_all(&library_dir).unwrap();
        fs::write(library_dir.join("lib-rule.md"), "# Library Rule").unwrap();

        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("rules")).unwrap();
        std::os::unix::fs::symlink(library_dir.join("lib-rule.md"), claude_dir.join("rules/lib-rule.md")).unwrap();
        fs::write(claude_dir.join("rules/local-rule.md"), "# Local Rule").unwrap();

        let adapter_registry = crate::adapters::AdapterRegistry::new();
        let resources = crate::scanner::scan_resources_for_sync(
            &claude_dir,
            &crate::models::v2::ResourceScope::Library,
            &adapter_registry,
        );

        assert_eq!(resources.len(), 2);
        // Library symlinks don't point to installed/ so installed_from_id is None
        let lib_res = resources.iter().find(|r| r.name == "lib-rule").unwrap();
        assert_eq!(lib_res.installed_from_id, None);
        assert_eq!(lib_res.linked_metadata, None);
        let local_res = resources.iter().find(|r| r.name == "local-rule").unwrap();
        assert_eq!(local_res.installed_from_id, None);
    }
}
