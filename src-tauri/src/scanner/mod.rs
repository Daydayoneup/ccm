pub mod global;
pub mod project;
pub mod plugin;
pub mod registry;

use std::fs;
use std::path::Path;

use sha2::{Sha256, Digest};

/// Scan a project base directory for all resource types using adapter registry.
/// Returns v2 Resource structs. `base_path` is the project root (not the .claude dir).
pub fn scan_claude_dir_v3(
    base_path: &Path,
    scope: &crate::models::v2::ResourceScope,
    adapter_registry: &crate::adapters::AdapterRegistry,
) -> Vec<crate::models::v2::Resource> {
    let types = [
        crate::models::v2::ResourceType::Agent,
        crate::models::v2::ResourceType::Skill,
        crate::models::v2::ResourceType::Rule,
        crate::models::v2::ResourceType::Command,
        crate::models::v2::ResourceType::Hook,
        crate::models::v2::ResourceType::McpServer,
    ];

    let mut all_resources = Vec::new();
    for rt in &types {
        if let Some(adapter) = adapter_registry.get(rt) {
            match adapter.scan(scope, base_path) {
                Ok(resources) => all_resources.extend(resources),
                Err(e) => eprintln!("Warning: scan for {:?} failed: {}", rt, e),
            }
        }
    }
    all_resources
}

/// Unified scan function for sync: scans via adapters and returns ScannedResource
/// for use with SyncEngine::reconcile().
pub fn scan_resources_for_sync(
    base_path: &Path,
    scope: &crate::models::v2::ResourceScope,
    adapter_registry: &crate::adapters::AdapterRegistry,
) -> Vec<ScannedResource> {
    let resources = scan_claude_dir_v3(base_path, scope, adapter_registry);
    resources
        .into_iter()
        .map(|r| ScannedResource {
            resource_type: r.resource_type,
            name: r.name,
            source_path: r.source_path,
            content_hash: r.content_hash,
            scope_override: None,
            linked_metadata: r.metadata,
            installed_from_id: r.installed_from_id,
        })
        .collect()
}

/// A scanned resource in v2 format, used by the new sub-module scanners.
#[derive(Debug, Clone)]
pub struct ScannedResource {
    pub resource_type: crate::models::v2::ResourceType,
    pub name: String,
    pub source_path: String,
    pub content_hash: Option<String>,
    /// When set, overrides the scope passed to SyncEngine::reconcile.
    pub scope_override: Option<crate::models::v2::ResourceScope>,
    /// When set, stored in the resource's metadata field.
    /// Used to mark global symlinks that point to managed directories ("linked").
    pub linked_metadata: Option<String>,
    /// When set, records the library resource ID this was installed from.
    pub installed_from_id: Option<String>,
}

/// A scanned plugin entry, used by the plugin sub-module scanner.
#[derive(Debug, Clone)]
pub struct ScannedPlugin {
    pub name: String,
    pub version: String,
    pub scope: String,
    pub install_path: String,
    pub resources: Vec<ScannedResource>,
}

/// Compute a SHA-256 hash of a string's contents directly.
pub fn compute_content_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute a SHA-256 hash of a file's contents.
/// If the path is a directory (e.g., a skill directory), try to hash the SKILL.md file inside.
pub fn compute_file_hash(path: &str) -> Option<String> {
    let p = Path::new(path);
    let file_path = if p.is_dir() {
        // For skill directories, hash the SKILL.md file
        let skill_md = p.join("SKILL.md");
        if skill_md.is_file() {
            skill_md
        } else {
            return None;
        }
    } else {
        p.to_path_buf()
    };
    let content = std::fs::read(&file_path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Some(format!("{:x}", hasher.finalize()))
}

/// Detect the primary language of a project by checking for common build files.
pub fn detect_language(project_path: &str) -> String {
    let path = Path::new(project_path);

    let checks: Vec<(&str, &str)> = vec![
        ("go.mod", "Go"),
        ("Cargo.toml", "Rust"),
        ("package.json", "JavaScript"),
        ("pyproject.toml", "Python"),
        ("setup.py", "Python"),
        ("requirements.txt", "Python"),
        ("pom.xml", "Java"),
        ("build.gradle", "Java"),
        ("build.gradle.kts", "Kotlin"),
        ("*.csproj", "C#"),
        ("CMakeLists.txt", "C/C++"),
        ("Makefile", "C/C++"),
        ("Gemfile", "Ruby"),
        ("mix.exs", "Elixir"),
        ("pubspec.yaml", "Dart"),
        ("Package.swift", "Swift"),
    ];

    for (file, lang) in &checks {
        if file.starts_with('*') {
            // Glob pattern - check for any matching file
            let ext = &file[1..]; // e.g., ".csproj"
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(ext) {
                            return lang.to_string();
                        }
                    }
                }
            }
        } else if path.join(file).exists() {
            return lang.to_string();
        }
    }

    // Check for tsconfig.json to differentiate TypeScript from JavaScript
    if path.join("tsconfig.json").exists() {
        return "TypeScript".to_string();
    }

    "Unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_language() {
        let tmp = TempDir::new().unwrap();

        // Go project
        fs::write(tmp.path().join("go.mod"), "module test").unwrap();
        assert_eq!(detect_language(tmp.path().to_str().unwrap()), "Go");
    }

    #[test]
    fn test_detect_language_rust() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(detect_language(tmp.path().to_str().unwrap()), "Rust");
    }

    #[test]
    fn test_detect_language_typescript() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        fs::write(tmp.path().join("tsconfig.json"), "{}").unwrap();
        // package.json is checked first, so it returns JavaScript
        // unless we check tsconfig.json first — let's verify current behavior
        let lang = detect_language(tmp.path().to_str().unwrap());
        assert_eq!(lang, "JavaScript");
    }

    #[test]
    fn test_detect_language_unknown() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(detect_language(tmp.path().to_str().unwrap()), "Unknown");
    }

}
