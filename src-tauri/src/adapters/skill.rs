use super::*;
use super::file_based::{file_install, file_uninstall, resolve_file_target, scan_file_resources};
use std::path::{Path, PathBuf};

pub struct SkillAdapter;

impl ResourceAdapter for SkillAdapter {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Skill
    }

    fn install_strategy(&self) -> InstallStrategy {
        InstallStrategy::FileBased
    }

    fn resolve_target(
        &self,
        scope: &TargetScope,
        name: &str,
        project: Option<&Project>,
    ) -> Result<InstallTarget, String> {
        // Skills are directories — name is used as-is (no .md extension)
        resolve_file_target(scope, "skills", name, project)
    }

    fn install(
        &self,
        resource: &Resource,
        target: &InstallTarget,
        link_type: &LinkType,
    ) -> Result<ResourceLink, String> {
        match target {
            InstallTarget::FilePath(target_path) => {
                let source = Path::new(&resource.source_path);
                file_install(source, target_path, link_type, &resource.id)
            }
            _ => Err("SkillAdapter requires FilePath target".into()),
        }
    }

    fn uninstall(&self, link: &ResourceLink) -> Result<(), String> {
        file_uninstall(link)
    }

    fn validate_content(&self, content: &str) -> Result<(), String> {
        if content.trim().is_empty() {
            return Err("Skill SKILL.md must not be empty".into());
        }
        Ok(())
    }

    fn scan(&self, scope: &ResourceScope, base_path: &Path) -> Result<Vec<Resource>, String> {
        scan_file_resources(scope, base_path, "skills", &ResourceType::Skill, true)
    }

    fn type_dir(&self) -> &'static str { "skills" }

    fn resolve_file_path(&self, base_dir: &Path, name: &str) -> PathBuf {
        base_dir.join("skills").join(name).join("SKILL.md")
    }

    fn source_path_from_file(&self, file_path: &Path) -> String {
        file_path.parent().unwrap().to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::v2::Project;
    use std::fs;
    use tempfile::TempDir;

    fn make_project(path: &str) -> Project {
        Project {
            id: "p1".to_string(),
            name: "test-project".to_string(),
            path: path.to_string(),
            language: None,
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        }
    }

    #[test]
    fn test_resource_type_and_strategy() {
        let adapter = SkillAdapter;
        assert_eq!(adapter.resource_type(), ResourceType::Skill);
        assert_eq!(adapter.install_strategy(), InstallStrategy::FileBased);
    }

    #[test]
    fn test_resolve_target_global() {
        let adapter = SkillAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "my-skill", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                assert!(
                    s.contains("/.claude/skills/my-skill"),
                    "got: {}",
                    s
                );
                // Skills should NOT have .md extension appended
                assert!(!s.ends_with(".md"), "skill target should not end with .md: {}", s);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_target_project() {
        let tmp = TempDir::new().unwrap();
        let adapter = SkillAdapter;
        let project = make_project(tmp.path().to_str().unwrap());
        let result = adapter
            .resolve_target(&TargetScope::Project, "deploy", Some(&project))
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let expected = tmp.path().join(".claude/skills/deploy");
                assert_eq!(p, expected);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_validate_content_valid() {
        let adapter = SkillAdapter;
        adapter
            .validate_content("# Deploy Skill\n\nSome content here.")
            .expect("should be valid");
    }

    #[test]
    fn test_validate_content_invalid_empty() {
        let adapter = SkillAdapter;
        let err = adapter
            .validate_content("   \n  ")
            .expect_err("should fail for empty content");
        assert!(err.contains("empty"), "error: {}", err);
    }

    #[test]
    fn test_scan() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join(".claude/skills");
        fs::create_dir_all(skills_dir.join("deploy")).unwrap();
        fs::write(skills_dir.join("deploy/SKILL.md"), "# Deploy").unwrap();
        // A directory without SKILL.md should be ignored
        fs::create_dir_all(skills_dir.join("empty-dir")).unwrap();

        let adapter = SkillAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "deploy");
        assert_eq!(resources[0].resource_type, ResourceType::Skill);
    }

    #[test]
    fn test_scan_empty() {
        let tmp = TempDir::new().unwrap();
        let adapter = SkillAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should return empty vec");
        assert!(resources.is_empty());
    }
}
