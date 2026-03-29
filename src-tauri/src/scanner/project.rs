use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use serde::{Deserialize, Serialize};
use super::detect_language;
use crate::models::v2::Project;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredProject {
    pub path: String,
    pub name: String,
    pub has_claude_config: bool,
}

/// Discover projects from `~/.claude/projects/` using the real home directory.
pub fn discover_from_claude_projects() -> Vec<DiscoveredProject> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let base = home.join(".claude").join("projects");
    discover_from_claude_projects_in(base.to_str().unwrap_or_default(), Some(home.to_str().unwrap_or_default()))
}

/// Discover projects from a given base path (for testing).
/// `home_dir_override` is used for filtering; if None, uses `dirs::home_dir()`.
pub fn discover_from_claude_projects_in(base_path: &str, home_dir_override: Option<&str>) -> Vec<DiscoveredProject> {
    let base = Path::new(base_path);
    if !base.is_dir() {
        return Vec::new();
    }

    let home_dir = home_dir_override
        .map(|s| s.to_string())
        .or_else(|| dirs::home_dir().map(|h| h.to_string_lossy().to_string()))
        .unwrap_or_default();

    let mut results = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        // Try to resolve the original project path
        let resolved = resolve_project_path(&entry_path);
        let project_path = match resolved {
            Some(p) => p,
            None => continue,
        };

        // Validate path exists on disk
        if !Path::new(&project_path).is_dir() {
            continue;
        }

        // Deduplicate
        if !seen_paths.insert(project_path.clone()) {
            continue;
        }

        // Filter out noise
        if should_filter_path(&project_path, &home_dir) {
            continue;
        }

        let has_claude_config = Path::new(&project_path).join(".claude").is_dir();

        let name = Path::new(&project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        results.push(DiscoveredProject {
            path: project_path,
            name,
            has_claude_config,
        });
    }

    results
}

/// Try to resolve the original project path from a Claude projects subdirectory.
/// Primary: read `sessions-index.json` for `originalPath`.
/// Fallback: scan first `.jsonl` file for a `cwd` field.
pub fn resolve_project_path(dir: &Path) -> Option<String> {
    // Primary: sessions-index.json
    let index_file = dir.join("sessions-index.json");
    if index_file.is_file() {
        if let Ok(content) = fs::read_to_string(&index_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(original_path) = val.get("originalPath").and_then(|v| v.as_str()) {
                    if !original_path.is_empty() {
                        return Some(original_path.to_string());
                    }
                }
            }
        }
    }

    // Fallback: first .jsonl file, scan for cwd field
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "jsonl" {
                    if let Some(cwd) = extract_cwd_from_jsonl(&path) {
                        return Some(cwd);
                    }
                }
            }
        }
    }

    None
}

/// Read a .jsonl file line by line and return the first `cwd` value found.
fn extract_cwd_from_jsonl(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(cwd) = val.get("cwd").and_then(|v| v.as_str()) {
                if !cwd.is_empty() {
                    return Some(cwd.to_string());
                }
            }
        }
    }

    None
}

/// Check if a path should be filtered out as noise.
fn should_filter_path(project_path: &str, home_dir: &str) -> bool {
    // Filter: path is the home directory itself
    if project_path == home_dir || project_path == format!("{}/", home_dir) {
        return true;
    }

    // Filter: common non-project directories under home
    let noise_dirs = ["Downloads", "Desktop", "Documents"];
    for dir_name in &noise_dirs {
        let noise_path = format!("{}/{}", home_dir, dir_name);
        if project_path == noise_path {
            return true;
        }
    }

    // Filter: .worktree paths
    if project_path.contains("/.worktree/") {
        return true;
    }

    // Filter: .paperclip paths
    if project_path.contains("/.paperclip/") {
        return true;
    }

    false
}

/// Scan a single project directory using adapter registry, returning a v2 Project + v2 Resources.
/// Unlike `scan_project_v2`, this delegates scanning to adapters for each resource type.
pub fn scan_project_v3(
    project_path: &str,
    adapter_registry: &crate::adapters::AdapterRegistry,
) -> Result<(Project, Vec<crate::models::v2::Resource>), String> {
    let path = Path::new(project_path);
    if !path.is_dir() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let language = detect_language(project_path);
    let now = chrono::Utc::now().to_rfc3339();

    let project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        path: project_path.to_string(),
        language: Some(language),
        last_scanned: Some(now),
        pinned: 0,
        launch_count: 0,
    };

    let resources = super::scan_claude_dir_v3(
        path,
        &crate::models::v2::ResourceScope::Project,
        adapter_registry,
    );

    Ok((project, resources))
}

/// Scan a directory for projects containing `.claude/`, returning v2 Projects.
/// Checks both the directory itself and its immediate subdirectories.
/// Uses adapter registry for resource scanning (v3 path).
pub fn scan_directory_v3(
    dir: &str,
    adapter_registry: &crate::adapters::AdapterRegistry,
) -> Vec<Project> {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return Vec::new();
    }

    let mut results = Vec::new();

    // Check if the directory itself is a project (has .claude/)
    if dir_path.join(".claude").is_dir() {
        if let Ok((project, _)) = scan_project_v3(dir, adapter_registry) {
            results.push(project);
        }
    }

    // Also check immediate subdirectories
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join(".claude").is_dir() {
                if let Ok((project, _)) = scan_project_v3(path.to_str().unwrap_or_default(), adapter_registry) {
                    results.push(project);
                }
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    // ── discover_from_claude_projects tests ──

    #[test]
    fn test_discover_with_sessions_index() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("claude_projects");
        let fake_home = tmp.path().join("home");

        // Create a real project directory
        let project_dir = fake_home.join("code").join("myapp");
        fs::create_dir_all(project_dir.join(".claude")).unwrap();

        // Create Claude projects entry with sessions-index.json
        let entry_dir = base.join("myapp-abc123");
        fs::create_dir_all(&entry_dir).unwrap();
        let index = serde_json::json!({
            "originalPath": project_dir.to_str().unwrap(),
            "entries": []
        });
        fs::write(entry_dir.join("sessions-index.json"), index.to_string()).unwrap();

        let results = discover_from_claude_projects_in(
            base.to_str().unwrap(),
            Some(fake_home.to_str().unwrap()),
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, project_dir.to_str().unwrap());
        assert_eq!(results[0].name, "myapp");
        assert!(results[0].has_claude_config);
    }

    #[test]
    fn test_discover_with_jsonl_fallback() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("claude_projects");
        let fake_home = tmp.path().join("home");

        // Create a real project directory (no .claude dir)
        let project_dir = fake_home.join("code").join("another");
        fs::create_dir_all(&project_dir).unwrap();

        // Create Claude projects entry with only .jsonl (no sessions-index.json)
        let entry_dir = base.join("another-def456");
        fs::create_dir_all(&entry_dir).unwrap();

        let jsonl_content = format!(
            "{}\n{}\n",
            r#"{"type":"init","timestamp":"2025-01-01"}"#,
            format!(r#"{{"type":"message","cwd":"{}"}}"#, project_dir.to_str().unwrap()),
        );
        fs::write(entry_dir.join("session.jsonl"), jsonl_content).unwrap();

        let results = discover_from_claude_projects_in(
            base.to_str().unwrap(),
            Some(fake_home.to_str().unwrap()),
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, project_dir.to_str().unwrap());
        assert_eq!(results[0].name, "another");
        assert!(!results[0].has_claude_config);
    }

    #[test]
    fn test_discover_filters_home_dir() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("claude_projects");
        let fake_home = tmp.path().join("home");
        fs::create_dir_all(&fake_home).unwrap();

        // Entry pointing to home dir itself
        let entry_dir = base.join("home-entry");
        fs::create_dir_all(&entry_dir).unwrap();
        let index = serde_json::json!({
            "originalPath": fake_home.to_str().unwrap(),
        });
        fs::write(entry_dir.join("sessions-index.json"), index.to_string()).unwrap();

        let results = discover_from_claude_projects_in(
            base.to_str().unwrap(),
            Some(fake_home.to_str().unwrap()),
        );
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_discover_filters_common_dirs() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("claude_projects");
        let fake_home = tmp.path().join("home");

        for dir_name in &["Downloads", "Desktop", "Documents"] {
            let dir_path = fake_home.join(dir_name);
            fs::create_dir_all(&dir_path).unwrap();

            let entry_dir = base.join(format!("{}-entry", dir_name));
            fs::create_dir_all(&entry_dir).unwrap();
            let index = serde_json::json!({
                "originalPath": dir_path.to_str().unwrap(),
            });
            fs::write(entry_dir.join("sessions-index.json"), index.to_string()).unwrap();
        }

        let results = discover_from_claude_projects_in(
            base.to_str().unwrap(),
            Some(fake_home.to_str().unwrap()),
        );
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_discover_filters_worktree_and_paperclip() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("claude_projects");
        let fake_home = tmp.path().join("home");

        // .worktree path
        let wt_path = fake_home.join("code").join(".worktree").join("branch1");
        fs::create_dir_all(&wt_path).unwrap();
        let entry1 = base.join("wt-entry");
        fs::create_dir_all(&entry1).unwrap();
        let index1 = serde_json::json!({ "originalPath": wt_path.to_str().unwrap() });
        fs::write(entry1.join("sessions-index.json"), index1.to_string()).unwrap();

        // .paperclip path
        let pp_path = fake_home.join("code").join(".paperclip").join("session1");
        fs::create_dir_all(&pp_path).unwrap();
        let entry2 = base.join("pp-entry");
        fs::create_dir_all(&entry2).unwrap();
        let index2 = serde_json::json!({ "originalPath": pp_path.to_str().unwrap() });
        fs::write(entry2.join("sessions-index.json"), index2.to_string()).unwrap();

        let results = discover_from_claude_projects_in(
            base.to_str().unwrap(),
            Some(fake_home.to_str().unwrap()),
        );
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_discover_skips_nonexistent_paths() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("claude_projects");
        let fake_home = tmp.path().join("home");

        let entry_dir = base.join("gone-entry");
        fs::create_dir_all(&entry_dir).unwrap();
        let index = serde_json::json!({
            "originalPath": "/nonexistent/path/that/does/not/exist",
        });
        fs::write(entry_dir.join("sessions-index.json"), index.to_string()).unwrap();

        let results = discover_from_claude_projects_in(
            base.to_str().unwrap(),
            Some(fake_home.to_str().unwrap()),
        );
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_discover_has_claude_config_detection() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("claude_projects");
        let fake_home = tmp.path().join("home");

        // Project WITH .claude dir
        let proj_with = fake_home.join("code").join("with-config");
        fs::create_dir_all(proj_with.join(".claude")).unwrap();
        let entry1 = base.join("with-config-entry");
        fs::create_dir_all(&entry1).unwrap();
        fs::write(
            entry1.join("sessions-index.json"),
            serde_json::json!({"originalPath": proj_with.to_str().unwrap()}).to_string(),
        ).unwrap();

        // Project WITHOUT .claude dir
        let proj_without = fake_home.join("code").join("without-config");
        fs::create_dir_all(&proj_without).unwrap();
        let entry2 = base.join("without-config-entry");
        fs::create_dir_all(&entry2).unwrap();
        fs::write(
            entry2.join("sessions-index.json"),
            serde_json::json!({"originalPath": proj_without.to_str().unwrap()}).to_string(),
        ).unwrap();

        let results = discover_from_claude_projects_in(
            base.to_str().unwrap(),
            Some(fake_home.to_str().unwrap()),
        );
        assert_eq!(results.len(), 2);

        let with = results.iter().find(|p| p.name == "with-config").unwrap();
        let without = results.iter().find(|p| p.name == "without-config").unwrap();
        assert!(with.has_claude_config);
        assert!(!without.has_claude_config);
    }

    #[test]
    fn test_discover_nonexistent_base_path() {
        let results = discover_from_claude_projects_in("/nonexistent/base/path", Some("/tmp"));
        assert_eq!(results.len(), 0);
    }

}
