use super::*;
use super::file_based::{file_install, file_uninstall, resolve_file_target, scan_file_resources};
use std::path::{Path, PathBuf};

pub struct RuleAdapter;

impl ResourceAdapter for RuleAdapter {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Rule
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
        let file_name = if name.ends_with(".md") {
            name.to_string()
        } else {
            format!("{}.md", name)
        };
        resolve_file_target(scope, "rules", &file_name, project)
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
            _ => Err("RuleAdapter requires FilePath target".into()),
        }
    }

    fn uninstall(&self, link: &ResourceLink) -> Result<(), String> {
        file_uninstall(link)
    }

    fn validate_content(&self, content: &str) -> Result<(), String> {
        if content.trim().is_empty() {
            return Err("Rule file must not be empty".into());
        }
        Ok(())
    }

    fn scan(&self, scope: &ResourceScope, base_path: &Path) -> Result<Vec<Resource>, String> {
        scan_file_resources(scope, base_path, "rules", &ResourceType::Rule, false)
    }

    fn type_dir(&self) -> &'static str { "rules" }

    fn resolve_file_path(&self, base_dir: &Path, name: &str) -> PathBuf {
        base_dir.join("rules").join(format!("{}.md", name))
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
        let adapter = RuleAdapter;
        assert_eq!(adapter.resource_type(), ResourceType::Rule);
        assert_eq!(adapter.install_strategy(), InstallStrategy::FileBased);
    }

    #[test]
    fn test_resolve_target_global() {
        let adapter = RuleAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "no-todos", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                assert!(
                    s.contains("/.claude/rules/no-todos.md"),
                    "got: {}",
                    s
                );
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_target_global_with_extension() {
        let adapter = RuleAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "no-todos.md", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                assert!(s.ends_with("no-todos.md"), "got: {}", s);
                assert!(!s.ends_with("no-todos.md.md"), "double extension: {}", s);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_target_project() {
        let tmp = TempDir::new().unwrap();
        let adapter = RuleAdapter;
        let project = make_project(tmp.path().to_str().unwrap());
        let result = adapter
            .resolve_target(&TargetScope::Project, "style", Some(&project))
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let expected = tmp.path().join(".claude/rules/style.md");
                assert_eq!(p, expected);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_validate_content_valid() {
        let adapter = RuleAdapter;
        // Rules can optionally have frontmatter with `paths:`, but it's not required
        adapter
            .validate_content("Always write tests.")
            .expect("should be valid");
    }

    #[test]
    fn test_validate_content_valid_with_frontmatter() {
        let adapter = RuleAdapter;
        let content = "---\npaths:\n  - src/**/*.ts\n---\n\nAlways use TypeScript strict mode.";
        adapter.validate_content(content).expect("should be valid");
    }

    #[test]
    fn test_validate_content_invalid_empty() {
        let adapter = RuleAdapter;
        let err = adapter
            .validate_content("  \n  ")
            .expect_err("should fail for empty content");
        assert!(err.contains("empty"), "error: {}", err);
    }

    #[test]
    fn test_scan() {
        let tmp = TempDir::new().unwrap();
        let rules_dir = tmp.path().join(".claude/rules");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::write(rules_dir.join("no-todos.md"), "Do not leave TODO comments.").unwrap();
        fs::write(rules_dir.join("style.md"), "Follow the style guide.").unwrap();
        fs::write(rules_dir.join("notes.txt"), "ignored").unwrap();

        let adapter = RuleAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert_eq!(resources.len(), 2);
        let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"no-todos"));
        assert!(names.contains(&"style"));
        for r in &resources {
            assert_eq!(r.resource_type, ResourceType::Rule);
        }
    }

    #[test]
    fn test_scan_empty() {
        let tmp = TempDir::new().unwrap();
        let adapter = RuleAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should return empty vec");
        assert!(resources.is_empty());
    }
}
