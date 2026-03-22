use super::*;
use super::file_based::{file_install, file_uninstall, resolve_file_target, scan_file_resources};
use std::path::Path;

pub struct CommandAdapter;

impl ResourceAdapter for CommandAdapter {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Command
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
        resolve_file_target(scope, "commands", &file_name, project)
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
            _ => Err("CommandAdapter requires FilePath target".into()),
        }
    }

    fn uninstall(&self, link: &ResourceLink) -> Result<(), String> {
        file_uninstall(link)
    }

    fn validate_content(&self, content: &str) -> Result<(), String> {
        if content.trim().is_empty() {
            return Err("Command file must not be empty".into());
        }
        Ok(())
    }

    fn scan(&self, scope: &ResourceScope, base_path: &Path) -> Result<Vec<Resource>, String> {
        scan_file_resources(scope, base_path, "commands", &ResourceType::Command, false)
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
        let adapter = CommandAdapter;
        assert_eq!(adapter.resource_type(), ResourceType::Command);
        assert_eq!(adapter.install_strategy(), InstallStrategy::FileBased);
    }

    #[test]
    fn test_resolve_target_global() {
        let adapter = CommandAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "build", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                assert!(
                    s.contains("/.claude/commands/build.md"),
                    "got: {}",
                    s
                );
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_target_global_with_extension() {
        let adapter = CommandAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "build.md", None)
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let s = p.to_string_lossy();
                assert!(s.ends_with("build.md"), "got: {}", s);
                assert!(!s.ends_with("build.md.md"), "double extension: {}", s);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_resolve_target_project() {
        let tmp = TempDir::new().unwrap();
        let adapter = CommandAdapter;
        let project = make_project(tmp.path().to_str().unwrap());
        let result = adapter
            .resolve_target(&TargetScope::Project, "deploy", Some(&project))
            .expect("should succeed");
        match result {
            InstallTarget::FilePath(p) => {
                let expected = tmp.path().join(".claude/commands/deploy.md");
                assert_eq!(p, expected);
            }
            _ => panic!("expected FilePath"),
        }
    }

    #[test]
    fn test_validate_content_valid() {
        let adapter = CommandAdapter;
        adapter
            .validate_content("Run the build pipeline.")
            .expect("should be valid");
    }

    #[test]
    fn test_validate_content_invalid_empty() {
        let adapter = CommandAdapter;
        let err = adapter
            .validate_content("\n\n")
            .expect_err("should fail for empty content");
        assert!(err.contains("empty"), "error: {}", err);
    }

    #[test]
    fn test_scan() {
        let tmp = TempDir::new().unwrap();
        let commands_dir = tmp.path().join(".claude/commands");
        fs::create_dir_all(&commands_dir).unwrap();
        fs::write(commands_dir.join("build.md"), "Run the build.").unwrap();
        fs::write(commands_dir.join("deploy.md"), "Run the deploy.").unwrap();
        fs::write(commands_dir.join("notes.txt"), "ignored").unwrap();

        let adapter = CommandAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert_eq!(resources.len(), 2);
        let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"build"));
        assert!(names.contains(&"deploy"));
        for r in &resources {
            assert_eq!(r.resource_type, ResourceType::Command);
        }
    }

    #[test]
    fn test_scan_empty() {
        let tmp = TempDir::new().unwrap();
        let adapter = CommandAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should return empty vec");
        assert!(resources.is_empty());
    }
}
