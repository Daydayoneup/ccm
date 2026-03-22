use std::fs;
use std::path::Path;
use super::{scan_claude_dir, ScannedResource, ScannedPlugin, compute_file_hash, v1_to_v2_resource_type};

/// Scan installed plugins from ~/.claude/plugins/installed_plugins.json
pub fn scan_installed_plugins() -> Vec<ScannedPlugin> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let plugins_file = home
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    if !plugins_file.is_file() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&plugins_file) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let plugins_data: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut result = Vec::new();

    // Collect raw plugin entries from either v1 (flat array) or v2 (nested object) format
    let entries = parse_plugin_entries(&plugins_data);

    for (name, version, scope, install_path) in entries {
        let resources = scan_plugin_path(&install_path);
        result.push(ScannedPlugin {
            name,
            version,
            scope,
            install_path,
            resources,
        });
    }

    result
}

/// Parse plugin entries from installed_plugins.json, supporting both formats:
/// - v1: flat array `[{ name, version, scope, installPath }]`
/// - v2: `{ version: 2, plugins: { "name@source": [{ scope, installPath, version }] } }`
fn parse_plugin_entries(data: &serde_json::Value) -> Vec<(String, String, String, String)> {
    let mut entries = Vec::new();

    // v2 format: { "version": 2, "plugins": { "key@source": [ { ... } ] } }
    if let Some(plugins_obj) = data.get("plugins").and_then(|v| v.as_object()) {
        for (key, installs) in plugins_obj {
            // key is like "superpowers@claude-plugins-official"
            let plugin_name = key.split('@').next().unwrap_or(key).to_string();

            if let Some(arr) = installs.as_array() {
                for item in arr {
                    let version = item.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let scope = item.get("scope").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let install_path = item.get("installPath").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    entries.push((plugin_name.clone(), version, scope, install_path));
                }
            }
        }
        return entries;
    }

    // v1 format: flat array [{ name, version, scope, installPath }]
    if let Some(arr) = data.as_array() {
        for item in arr {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let version = item.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let scope = item.get("scope").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let install_path = item.get("installPath").and_then(|v| v.as_str()).unwrap_or("").to_string();
            entries.push((name, version, scope, install_path));
        }
    }

    entries
}

/// Scan a plugin's install path for resources (skills, agents, rules, hooks, commands)
///
/// Supports two directory layouts:
/// - Flat: `{install_path}/skills/`, `{install_path}/agents/`, etc.
/// - Nested: `{install_path}/.claude/skills/`, `{install_path}/.claude/agents/`, etc.
fn scan_plugin_path(install_path: &str) -> Vec<ScannedResource> {
    if install_path.is_empty() {
        return Vec::new();
    }
    let plugin_path = Path::new(install_path);
    if !plugin_path.is_dir() {
        return Vec::new();
    }

    // Try flat layout first (skills/ at root)
    let mut local = scan_claude_dir(plugin_path);

    // If flat layout found nothing, try nested .claude/ layout
    if local.is_empty() {
        let claude_dir = plugin_path.join(".claude");
        if claude_dir.is_dir() {
            local = scan_claude_dir(&claude_dir);
        }
    }

    local
        .into_iter()
        .map(|lr| {
            let hash = compute_file_hash(&lr.path);
            ScannedResource {
                resource_type: v1_to_v2_resource_type(&lr.resource_type),
                name: lr.name,
                source_path: lr.path,
                content_hash: hash,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scan_plugin_directory() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path();

        // Create plugin-like structure
        fs::create_dir_all(plugin_dir.join("skills/my-plugin-skill")).unwrap();
        fs::write(
            plugin_dir.join("skills/my-plugin-skill/SKILL.md"),
            "# Plugin Skill",
        )
        .unwrap();

        let local = scan_claude_dir(plugin_dir);
        let resources: Vec<ScannedResource> = local
            .into_iter()
            .map(|lr| {
                let hash = compute_file_hash(&lr.path);
                ScannedResource {
                    resource_type: v1_to_v2_resource_type(&lr.resource_type),
                    name: lr.name,
                    source_path: lr.path,
                    content_hash: hash,
                }
            })
            .collect();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "my-plugin-skill");
    }

    #[test]
    fn test_parse_v2_format() {
        let json = serde_json::json!({
            "version": 2,
            "plugins": {
                "superpowers@claude-plugins-official": [
                    {
                        "scope": "user",
                        "installPath": "/Users/test/.claude/plugins/cache/superpowers/4.3.1",
                        "version": "4.3.1",
                        "installedAt": "2026-01-21T03:35:56.388Z"
                    }
                ],
                "code-review@claude-plugins-official": [
                    {
                        "scope": "user",
                        "installPath": "/Users/test/.claude/plugins/cache/code-review/1.0.0",
                        "version": "1.0.0",
                        "installedAt": "2026-02-28T00:23:45.534Z"
                    }
                ]
            }
        });

        let entries = parse_plugin_entries(&json);
        assert_eq!(entries.len(), 2);

        let names: Vec<&str> = entries.iter().map(|(n, _, _, _)| n.as_str()).collect();
        assert!(names.contains(&"superpowers"));
        assert!(names.contains(&"code-review"));

        let sp = entries.iter().find(|(n, _, _, _)| n == "superpowers").unwrap();
        assert_eq!(sp.1, "4.3.1");
        assert_eq!(sp.2, "user");
        assert!(sp.3.contains("superpowers/4.3.1"));
    }

    #[test]
    fn test_scan_plugin_path_nested_claude_dir() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path();

        // Create nested .claude/ structure (like ui-ux-pro-max)
        fs::create_dir_all(plugin_dir.join(".claude/skills/design")).unwrap();
        fs::write(
            plugin_dir.join(".claude/skills/design/SKILL.md"),
            "# Design Skill",
        )
        .unwrap();

        fs::create_dir_all(plugin_dir.join(".claude/skills/ui-styling")).unwrap();
        fs::write(
            plugin_dir.join(".claude/skills/ui-styling/SKILL.md"),
            "# UI Styling",
        )
        .unwrap();

        let resources = scan_plugin_path(plugin_dir.to_str().unwrap());
        assert_eq!(resources.len(), 2);

        let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"design"));
        assert!(names.contains(&"ui-styling"));
    }

    #[test]
    fn test_scan_plugin_path_flat_layout_preferred() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path();

        // Create flat layout (like superpowers)
        fs::create_dir_all(plugin_dir.join("skills/my-skill")).unwrap();
        fs::write(
            plugin_dir.join("skills/my-skill/SKILL.md"),
            "# My Skill",
        )
        .unwrap();

        let resources = scan_plugin_path(plugin_dir.to_str().unwrap());
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "my-skill");
    }

    #[test]
    fn test_parse_v1_format() {
        let json = serde_json::json!([
            {
                "name": "my-plugin",
                "version": "1.0.0",
                "scope": "@claude",
                "installPath": "/tmp/plugins/my-plugin"
            }
        ]);

        let entries = parse_plugin_entries(&json);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "my-plugin");
        assert_eq!(entries[0].1, "1.0.0");
    }
}
