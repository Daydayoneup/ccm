use super::*;
use super::file_based::{file_install, file_uninstall, resolve_file_target, scan_file_resources};
use std::path::{Path, PathBuf};

pub struct AgentAdapter;

impl ResourceAdapter for AgentAdapter {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Agent
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
        resolve_file_target(scope, "agents", &file_name, project)
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
            _ => Err("AgentAdapter requires FilePath target".into()),
        }
    }

    fn uninstall(&self, link: &ResourceLink) -> Result<(), String> {
        file_uninstall(link)
    }

    fn validate_content(&self, content: &str) -> Result<(), String> {
        if !content.starts_with("---") {
            return Err("Agent must have YAML frontmatter (starting with ---)".into());
        }
        let end = content[3..]
            .find("---")
            .ok_or("Missing closing --- for frontmatter")?;
        let frontmatter = &content[3..3 + end];
        if !frontmatter.contains("name:") {
            return Err("Agent frontmatter must contain 'name' field".into());
        }
        if !frontmatter.contains("description:") {
            return Err("Agent frontmatter must contain 'description' field".into());
        }
        Ok(())
    }

    fn scan(&self, scope: &ResourceScope, base_path: &Path) -> Result<Vec<Resource>, String> {
        scan_file_resources(scope, base_path, "agents", &ResourceType::Agent, false)
    }

    fn type_dir(&self) -> &'static str { "agents" }

    fn resolve_file_path(&self, base_dir: &Path, name: &str) -> PathBuf {
        base_dir.join("agents").join(format!("{}.md", name))
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
        let adapter = AgentAdapter;
        assert_eq!(adapter.resource_type(), ResourceType::Agent);
        assert_eq!(adapter.install_strategy(), InstallStrategy::FileBased);
    }

    #[test]
    fn test_resolve_target_global() {
        let adapter = AgentAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "my-agent", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                assert!(
                    s.contains("/.claude/agents/my-agent.md"),
                    "got: {}",
                    s
                );
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_target_global_with_extension() {
        let adapter = AgentAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "my-agent.md", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                // Should not double the .md extension
                assert!(s.ends_with("my-agent.md"), "got: {}", s);
                assert!(!s.ends_with("my-agent.md.md"), "got double extension: {}", s);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_target_project() {
        let tmp = TempDir::new().unwrap();
        let adapter = AgentAdapter;
        let project = make_project(tmp.path().to_str().unwrap());
        let result = adapter
            .resolve_target(&TargetScope::Project, "reviewer", Some(&project))
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let expected = tmp.path().join(".claude/agents/reviewer.md");
                assert_eq!(p, expected);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_validate_content_valid() {
        let adapter = AgentAdapter;
        let content = "---\nname: my-agent\ndescription: Does something useful\n---\n\n# Body";
        adapter.validate_content(content).expect("should be valid");
    }

    #[test]
    fn test_validate_content_missing_frontmatter() {
        let adapter = AgentAdapter;
        let err = adapter
            .validate_content("# No frontmatter here")
            .expect_err("should fail");
        assert!(err.contains("frontmatter"), "error: {}", err);
    }

    #[test]
    fn test_validate_content_missing_name() {
        let adapter = AgentAdapter;
        let content = "---\ndescription: something\n---\n\nbody";
        let err = adapter.validate_content(content).expect_err("should fail");
        assert!(err.contains("name"), "error: {}", err);
    }

    #[test]
    fn test_validate_content_missing_description() {
        let adapter = AgentAdapter;
        let content = "---\nname: my-agent\n---\n\nbody";
        let err = adapter.validate_content(content).expect_err("should fail");
        assert!(err.contains("description"), "error: {}", err);
    }

    #[test]
    fn test_scan() {
        let tmp = TempDir::new().unwrap();
        let agents_dir = tmp.path().join(".claude/agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("reviewer.md"), "# Reviewer").unwrap();
        fs::write(agents_dir.join("builder.md"), "# Builder").unwrap();
        fs::write(agents_dir.join("notes.txt"), "ignored").unwrap();

        let adapter = AgentAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert_eq!(resources.len(), 2);
        let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"reviewer"));
        assert!(names.contains(&"builder"));
        for r in &resources {
            assert_eq!(r.resource_type, ResourceType::Agent);
        }
    }

    #[test]
    fn test_scan_empty() {
        let tmp = TempDir::new().unwrap();
        let adapter = AgentAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should return empty vec");
        assert!(resources.is_empty());
    }
}
