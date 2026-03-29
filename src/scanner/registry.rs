use std::path::Path;
use serde::Deserialize;

use super::ScannedResource;

#[derive(Debug, Deserialize)]
pub struct MarketplaceJson {
    pub name: Option<String>,
    pub description: Option<String>,
    pub plugins: Option<Vec<MarketplacePlugin>>,
}

#[derive(Debug, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub source: serde_json::Value, // String or { source: "url", url: "..." }
    pub homepage: Option<String>,
}

impl MarketplacePlugin {
    /// Returns true if source is a URL object (external plugin)
    pub fn is_external(&self) -> bool {
        self.source.is_object()
    }

    /// Extract the relative path for local plugins (e.g., "./plugins/superpowers")
    pub fn local_source_path(&self) -> Option<String> {
        self.source.as_str().map(|s| s.to_string())
    }

    /// Extract the URL for external plugins.
    /// Handles both full URLs and GitHub shorthand (e.g., "stripe/ai" → "https://github.com/stripe/ai.git").
    pub fn external_url(&self) -> Option<String> {
        let raw = self.source.get("url").and_then(|v| v.as_str())?;
        if raw.starts_with("http://") || raw.starts_with("https://") || raw.starts_with("git@") {
            Some(raw.to_string())
        } else {
            // GitHub shorthand: "owner/repo" → full URL
            Some(format!("https://github.com/{}.git", raw))
        }
    }

    /// Extract the subdirectory path for git-subdir sources (e.g., "providers/claude/plugin").
    pub fn external_subdir_path(&self) -> Option<String> {
        self.source.get("path").and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// Extract the git ref (branch/tag) for external plugins.
    pub fn external_ref(&self) -> Option<String> {
        self.source.get("ref").and_then(|v| v.as_str()).map(|s| s.to_string())
    }
}

/// Read and parse .claude-plugin/marketplace.json
pub fn read_marketplace_json(registry_path: &str) -> Option<MarketplaceJson> {
    let path = Path::new(registry_path).join(".claude-plugin").join("marketplace.json");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Read registry.json (legacy/fallback metadata)
pub fn read_registry_metadata(registry_path: &str) -> Option<(String, Option<String>)> {
    let path = Path::new(registry_path).join("registry.json");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let name = json.get("name")?.as_str()?.to_string();
    let description = json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
    Some((name, description))
}

/// Scan a plugin directory for resources (skills/, agents/, rules/, commands/, hooks/)
///
/// Supports two directory layouts:
/// - Flat: `{plugin_path}/skills/`, etc.
/// - Nested: `{plugin_path}/.claude/skills/`, etc.
pub fn scan_plugin_dir(plugin_path: &str) -> Vec<ScannedResource> {
    let path = Path::new(plugin_path);
    if !path.exists() {
        return vec![];
    }
    let adapter_registry = crate::adapters::AdapterRegistry::new();
    // Try flat layout first (skills/ at root)
    let resources = super::scan_resources_for_sync(
        path,
        &crate::models::v2::ResourceScope::Registry,
        &adapter_registry,
    );
    if !resources.is_empty() {
        return resources;
    }
    // Try nested .claude/ layout
    let nested = path.join(".claude");
    if nested.is_dir() {
        return super::scan_resources_for_sync(
            &nested,
            &crate::models::v2::ResourceScope::Registry,
            &adapter_registry,
        );
    }
    vec![]
}

/// Scan a registry using the old flat structure (fallback when no marketplace.json)
pub fn scan_registry(registry_path: &str) -> Vec<ScannedResource> {
    scan_plugin_dir(registry_path)
}

/// Resolve the local path for a plugin source
pub fn resolve_plugin_source_path(registry_path: &str, plugin: &MarketplacePlugin) -> String {
    if let Some(local_source) = plugin.local_source_path() {
        // "./plugins/superpowers" -> "/path/to/registry/plugins/superpowers"
        let relative = local_source.strip_prefix("./").unwrap_or(&local_source);
        Path::new(registry_path).join(relative).to_string_lossy().to_string()
    } else {
        // External plugin: stored in external_plugins/<name>/
        Path::new(registry_path)
            .join("external_plugins")
            .join(&plugin.name)
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_registry_finds_all_resource_types() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("skills/my-skill")).unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md"), "# My Skill").unwrap();

        fs::create_dir_all(root.join("agents")).unwrap();
        fs::write(root.join("agents/my-agent.md"), "# Agent").unwrap();

        fs::create_dir_all(root.join("rules")).unwrap();
        fs::write(root.join("rules/my-rule.md"), "# Rule").unwrap();

        fs::create_dir_all(root.join("commands")).unwrap();
        fs::write(root.join("commands/my-cmd.md"), "# Command").unwrap();

        // Note: hooks are config-based (settings.json) and not scanned as files
        // in registry scope, so we don't include them here.

        let resources = scan_registry(root.to_str().unwrap());
        assert_eq!(resources.len(), 4);
    }

    #[test]
    fn test_scan_registry_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let resources = scan_registry(tmp.path().to_str().unwrap());
        assert!(resources.is_empty());
    }

    #[test]
    fn test_scan_registry_nonexistent_dir() {
        let resources = scan_registry("/nonexistent/path");
        assert!(resources.is_empty());
    }

    #[test]
    fn test_scan_plugin_dir_nested_claude_layout() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create nested .claude/ layout (no flat skills/ at root)
        fs::create_dir_all(root.join(".claude/skills/my-skill")).unwrap();
        fs::write(root.join(".claude/skills/my-skill/SKILL.md"), "# My Skill").unwrap();

        fs::create_dir_all(root.join(".claude/agents")).unwrap();
        fs::write(root.join(".claude/agents/my-agent.md"), "# Agent").unwrap();

        let resources = scan_plugin_dir(root.to_str().unwrap());
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_read_registry_metadata() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("registry.json"),
            r#"{"name": "My Registry", "description": "Shared resources", "version": "1.0.0"}"#,
        ).unwrap();

        let (name, desc) = read_registry_metadata(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(name, "My Registry");
        assert_eq!(desc, Some("Shared resources".to_string()));
    }

    #[test]
    fn test_read_registry_metadata_missing_file() {
        let tmp = TempDir::new().unwrap();
        assert!(read_registry_metadata(tmp.path().to_str().unwrap()).is_none());
    }

    #[test]
    fn test_read_marketplace_json() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".claude-plugin")).unwrap();
        fs::write(
            tmp.path().join(".claude-plugin/marketplace.json"),
            r#"{
                "name": "test-marketplace",
                "plugins": [
                    {"name": "local-plugin", "source": "./plugins/local-plugin"},
                    {"name": "ext-plugin", "source": {"source": "url", "url": "https://github.com/test/ext.git"}}
                ]
            }"#,
        ).unwrap();

        let mp = read_marketplace_json(tmp.path().to_str().unwrap()).unwrap();
        let plugins = mp.plugins.unwrap();
        assert_eq!(plugins.len(), 2);
        assert!(!plugins[0].is_external());
        assert!(plugins[1].is_external());
        assert_eq!(plugins[1].external_url(), Some("https://github.com/test/ext.git".to_string()));
    }

    #[test]
    fn test_external_url_github_shorthand() {
        let plugin = MarketplacePlugin {
            name: "stripe".to_string(),
            description: None,
            category: None,
            source: serde_json::json!({"source": "git-subdir", "url": "stripe/ai", "path": "providers/claude/plugin", "ref": "main"}),
            homepage: None,
        };
        assert!(plugin.is_external());
        assert_eq!(plugin.external_url(), Some("https://github.com/stripe/ai.git".to_string()));
        assert_eq!(plugin.external_subdir_path(), Some("providers/claude/plugin".to_string()));
        assert_eq!(plugin.external_ref(), Some("main".to_string()));
    }

    #[test]
    fn test_external_url_full_url_unchanged() {
        let plugin = MarketplacePlugin {
            name: "ext-plugin".to_string(),
            description: None,
            category: None,
            source: serde_json::json!({"source": "url", "url": "https://github.com/test/ext.git"}),
            homepage: None,
        };
        assert_eq!(plugin.external_url(), Some("https://github.com/test/ext.git".to_string()));
        assert_eq!(plugin.external_subdir_path(), None);
    }

    #[test]
    fn test_resolve_plugin_source_path_local() {
        let plugin = MarketplacePlugin {
            name: "my-plugin".to_string(),
            description: None,
            category: None,
            source: serde_json::Value::String("./plugins/my-plugin".to_string()),
            homepage: None,
        };
        let result = resolve_plugin_source_path("/registry/path", &plugin);
        assert_eq!(result, "/registry/path/plugins/my-plugin");
    }

    #[test]
    fn test_resolve_plugin_source_path_external() {
        let plugin = MarketplacePlugin {
            name: "ext-plugin".to_string(),
            description: None,
            category: None,
            source: serde_json::json!({"source": "url", "url": "https://github.com/test/ext.git"}),
            homepage: None,
        };
        let result = resolve_plugin_source_path("/registry/path", &plugin);
        assert_eq!(result, "/registry/path/external_plugins/ext-plugin");
    }
}
