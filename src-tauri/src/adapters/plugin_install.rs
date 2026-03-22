use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ─────────────────────────────────────────────
// Detection helpers
// ─────────────────────────────────────────────

/// Returns true if the registry path contains a `.claude-plugin/marketplace.json` file,
/// indicating it is a Claude Code marketplace registry.
pub fn is_claude_marketplace(registry_path: &Path) -> bool {
    registry_path
        .join(".claude-plugin")
        .join("marketplace.json")
        .is_file()
}

/// Returns true if the plugin path is a valid Claude Code plugin — it must be a directory
/// that contains either a `skills/` subdirectory (flat layout) or a manifest file
/// (e.g. `package.json`, `plugin.json`, or `.claude-plugin/manifest.json`).
pub fn is_valid_plugin(plugin_path: &Path) -> bool {
    if !plugin_path.is_dir() {
        return false;
    }
    // Flat layout: skills/ or agents/ or commands/ etc. at root
    let has_resource_dir = ["skills", "agents", "rules", "commands", "hooks"]
        .iter()
        .any(|d| plugin_path.join(d).is_dir());
    if has_resource_dir {
        return true;
    }
    // Nested layout: .claude/skills/ etc.
    let claude_dir = plugin_path.join(".claude");
    if claude_dir.is_dir() {
        let has_nested = ["skills", "agents", "rules", "commands", "hooks"]
            .iter()
            .any(|d| claude_dir.join(d).is_dir());
        if has_nested {
            return true;
        }
    }
    // Manifest files
    if plugin_path.join("package.json").is_file()
        || plugin_path.join("plugin.json").is_file()
        || plugin_path.join(".claude-plugin").join("manifest.json").is_file()
    {
        return true;
    }
    false
}

/// Returns the canonical CCM wrapper name for a registry: `ccm-<name>`.
pub fn ccm_marketplace_name(registry_name: &str) -> String {
    format!("ccm-{}", registry_name)
}

/// Returns true if the `claude` CLI binary is available on `$PATH`.
pub fn is_claude_cli_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns the directory used to store CCM wrapper marketplaces:
/// `~/.claude-manager/ccm-marketplaces/`.
pub fn ccm_marketplaces_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    Ok(home.join(".claude-manager").join("ccm-marketplaces"))
}

/// Returns true if a CCM wrapper already exists for the given registry name.
pub fn has_wrapper(registry_name: &str) -> Result<bool, String> {
    let dir = ccm_marketplaces_dir()?;
    let wrapper_dir = dir.join(ccm_marketplace_name(registry_name));
    Ok(wrapper_dir.join(".claude-plugin").join("marketplace.json").is_file())
}

// ─────────────────────────────────────────────
// Relative path calculation (no external deps)
// ─────────────────────────────────────────────

/// Compute a relative path from `from` to `to` (both treated as directories).
fn relative_path(from: &Path, to: &Path) -> String {
    // Canonicalize both paths where possible
    let from = from.canonicalize().unwrap_or_else(|_| from.to_path_buf());
    let to = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());

    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    let common_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let ups = from_components.len() - common_len;
    let mut rel = String::new();
    for _ in 0..ups {
        rel.push_str("../");
    }
    for component in &to_components[common_len..] {
        rel.push_str(&component.as_os_str().to_string_lossy());
        rel.push('/');
    }
    if rel.ends_with('/') {
        rel.pop();
    }
    if rel.is_empty() {
        ".".to_string()
    } else {
        rel
    }
}

// ─────────────────────────────────────────────
// Wrapper generation
// ─────────────────────────────────────────────

/// Generate (or regenerate) a CCM wrapper `marketplace.json` for the given registry.
///
/// * Reads `<registry_path>/.claude-plugin/marketplace.json`
/// * Creates `<wrapper_base>/ccm-<name>/.claude-plugin/marketplace.json`
/// * Sets `name` to `ccm-<name>`
/// * Rewrites local source paths (`./…`) to relative paths pointing back to the
///   original registry directory
/// * Keeps remote sources (objects with `source`/`url` fields) unchanged
///
/// Returns the path to the generated `marketplace.json`.
pub fn generate_wrapper_marketplace(
    registry_path: &Path,
    registry_name: &str,
    wrapper_base: &Path,
) -> Result<PathBuf, String> {
    let original_marketplace = registry_path
        .join(".claude-plugin")
        .join("marketplace.json");

    let content = fs::read_to_string(&original_marketplace).map_err(|e| {
        format!(
            "Failed to read marketplace.json at {}: {}",
            original_marketplace.display(),
            e
        )
    })?;

    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid marketplace.json: {}", e))?;

    let wrapper_name = ccm_marketplace_name(registry_name);

    // Set the wrapper name
    if let Some(obj) = json.as_object_mut() {
        obj.insert("name".to_string(), serde_json::Value::String(wrapper_name.clone()));
    }

    // The wrapper .claude-plugin/ directory (the "from" directory for relative paths)
    let wrapper_plugin_dir = wrapper_base.join(&wrapper_name).join(".claude-plugin");

    // Create the wrapper directory early so canonicalize works in relative_path
    fs::create_dir_all(&wrapper_plugin_dir).map_err(|e| {
        format!(
            "Failed to create wrapper directory {}: {}",
            wrapper_plugin_dir.display(),
            e
        )
    })?;

    // Rewrite local source paths in plugins array
    if let Some(plugins) = json.get_mut("plugins").and_then(|v| v.as_array_mut()) {
        for plugin in plugins.iter_mut() {
            let source = plugin.get("source").cloned();
            if let Some(serde_json::Value::String(src)) = source {
                if src.starts_with("./") {
                    // Resolve the local plugin directory (absolute)
                    let relative_src = src.strip_prefix("./").unwrap_or(&src);
                    let abs_plugin_dir = registry_path.join(relative_src);

                    // Compute relative path from wrapper's .claude-plugin/ to abs_plugin_dir
                    let rel = relative_path(&wrapper_plugin_dir, &abs_plugin_dir);
                    // Prefix with "./" to match marketplace.json convention
                    let new_source = if rel.starts_with("../") || rel == "." {
                        rel
                    } else {
                        format!("./{}", rel)
                    };
                    plugin["source"] = serde_json::Value::String(new_source);
                }
                // Non-./ strings (bare paths, names) are left unchanged
            }
            // Objects (remote sources) are left unchanged
        }
    }

    let output_path = wrapper_plugin_dir.join("marketplace.json");
    let output_content = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize wrapper marketplace.json: {}", e))?;

    fs::write(&output_path, output_content).map_err(|e| {
        format!(
            "Failed to write wrapper marketplace.json at {}: {}",
            output_path.display(),
            e
        )
    })?;

    Ok(output_path)
}

// ─────────────────────────────────────────────
// CLI invocation
// ─────────────────────────────────────────────

/// Register a marketplace with the Claude Code CLI.
///
/// Runs: `claude marketplace add <wrapper_path>`
pub fn register_marketplace_cli(wrapper_path: &Path) -> Result<(), String> {
    let output = Command::new("claude")
        .args(["marketplace", "add", &wrapper_path.to_string_lossy()])
        .output()
        .map_err(|e| format!("Failed to run 'claude marketplace add': {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "'claude marketplace add' failed (exit {}): {} {}",
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim()
        ))
    }
}

/// Install a plugin via the Claude Code CLI.
///
/// Runs: `claude plugin install <plugin_name> --marketplace <marketplace_name> --scope <scope>`
/// When `scope` is "project", also passes `--project <project_path>`.
pub fn install_plugin_cli(
    project_path: &str,
    plugin_name: &str,
    marketplace_name: &str,
    scope: &str,
) -> Result<(), String> {
    let mut cmd = Command::new("claude");
    cmd.args([
        "plugin",
        "install",
        plugin_name,
        "--marketplace",
        marketplace_name,
        "--scope",
        scope,
    ]);
    if scope == "project" && !project_path.is_empty() {
        cmd.args(["--project", project_path]);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run 'claude plugin install': {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "'claude plugin install' failed (exit {}): {} {}",
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim()
        ))
    }
}

/// Uninstall a plugin via the Claude Code CLI.
///
/// Runs: `claude plugin uninstall <plugin_name> --marketplace <marketplace_name> --scope <scope>`
/// When `scope` is "project", also passes `--project <project_path>`.
pub fn uninstall_plugin_cli(
    project_path: &str,
    plugin_name: &str,
    marketplace_name: &str,
    scope: &str,
) -> Result<(), String> {
    let mut cmd = Command::new("claude");
    cmd.args([
        "plugin",
        "uninstall",
        plugin_name,
        "--marketplace",
        marketplace_name,
        "--scope",
        scope,
    ]);
    if scope == "project" && !project_path.is_empty() {
        cmd.args(["--project", project_path]);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run 'claude plugin uninstall': {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "'claude plugin uninstall' failed (exit {}): {} {}",
            output.status.code().unwrap_or(-1),
            stdout.trim(),
            stderr.trim()
        ))
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── Detection ──────────────────────────────────────────────────────────────

    #[test]
    fn test_is_claude_marketplace_true() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".claude-plugin")).unwrap();
        fs::write(
            tmp.path().join(".claude-plugin/marketplace.json"),
            r#"{"name":"test","plugins":[]}"#,
        )
        .unwrap();
        assert!(is_claude_marketplace(tmp.path()));
    }

    #[test]
    fn test_is_claude_marketplace_false() {
        let tmp = TempDir::new().unwrap();
        // No .claude-plugin directory at all
        assert!(!is_claude_marketplace(tmp.path()));
        // Directory exists but no marketplace.json
        fs::create_dir_all(tmp.path().join(".claude-plugin")).unwrap();
        assert!(!is_claude_marketplace(tmp.path()));
    }

    #[test]
    fn test_ccm_marketplace_name() {
        assert_eq!(ccm_marketplace_name("my-registry"), "ccm-my-registry");
        assert_eq!(ccm_marketplace_name("official"), "ccm-official");
        assert_eq!(ccm_marketplace_name(""), "ccm-");
    }

    #[test]
    fn test_is_valid_plugin_with_skills() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("skills/my-skill")).unwrap();
        fs::write(tmp.path().join("skills/my-skill/SKILL.md"), "# Skill").unwrap();
        assert!(is_valid_plugin(tmp.path()));
    }

    #[test]
    fn test_is_valid_plugin_with_manifest() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"name":"plugin"}"#).unwrap();
        assert!(is_valid_plugin(tmp.path()));
    }

    #[test]
    fn test_is_valid_plugin_empty() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_valid_plugin(tmp.path()));
    }

    #[test]
    fn test_is_valid_plugin_not_a_dir() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("not-a-dir.txt");
        fs::write(&file_path, "content").unwrap();
        assert!(!is_valid_plugin(&file_path));
    }

    #[test]
    fn test_is_valid_plugin_nested_claude_layout() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".claude/agents")).unwrap();
        fs::write(tmp.path().join(".claude/agents/my-agent.md"), "# Agent").unwrap();
        assert!(is_valid_plugin(tmp.path()));
    }

    // ── Wrapper generation ──────────────────────────────────────────────────────

    #[test]
    fn test_generate_wrapper_marketplace() {
        let registry_tmp = TempDir::new().unwrap();
        let wrapper_tmp = TempDir::new().unwrap();

        let registry_path = registry_tmp.path();
        let wrapper_base = wrapper_tmp.path();

        // Create a minimal registry with a local plugin
        fs::create_dir_all(registry_path.join(".claude-plugin")).unwrap();
        fs::create_dir_all(registry_path.join("plugins/my-plugin")).unwrap();
        fs::write(registry_path.join("plugins/my-plugin/package.json"), "{}").unwrap();

        let marketplace_content = r#"{
            "name": "test-registry",
            "description": "A test registry",
            "plugins": [
                {
                    "name": "my-plugin",
                    "description": "A local plugin",
                    "source": "./plugins/my-plugin"
                }
            ]
        }"#;
        fs::write(
            registry_path.join(".claude-plugin/marketplace.json"),
            marketplace_content,
        )
        .unwrap();

        let result = generate_wrapper_marketplace(registry_path, "test-registry", wrapper_base);
        assert!(result.is_ok(), "generate_wrapper_marketplace failed: {:?}", result);

        let output_path = result.unwrap();
        assert!(output_path.exists(), "output file does not exist");
        assert!(output_path.ends_with(".claude-plugin/marketplace.json"));

        let written = fs::read_to_string(&output_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&written).unwrap();

        // Name should be rewritten to ccm-<name>
        assert_eq!(json["name"].as_str().unwrap(), "ccm-test-registry");

        // Local source should have been rewritten to a relative path (not starting with ./)
        let plugins = json["plugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 1);
        let src = plugins[0]["source"].as_str().unwrap();
        // Should point back to the original registry path via relative path
        assert!(
            src.contains(".."),
            "Expected relative path with '..', got: {}",
            src
        );
        // Original bare ./ prefix should be gone and path should resolve to registry plugins dir
        // Verify the relative path resolves correctly by walking the components manually.
        // We can't use fs::canonicalize on the joined path because the wrapper dir itself
        // is a temp dir whose canonical prefix differs from a naive join with "..".
        // Instead, verify via relative_path round-trip: compute relative path from
        // wrapper_plugin_dir to the original plugin dir and confirm it equals src.
        let wrapper_plugin_dir = wrapper_base
            .join("ccm-test-registry")
            .join(".claude-plugin");
        let original_plugin_dir = registry_path.join("plugins/my-plugin");
        let expected_rel = relative_path(&wrapper_plugin_dir, &original_plugin_dir);
        assert_eq!(
            src, expected_rel,
            "Rewritten source path '{}' does not match expected relative path '{}'",
            src, expected_rel
        );
    }

    #[test]
    fn test_generate_wrapper_preserves_remote_sources() {
        let registry_tmp = TempDir::new().unwrap();
        let wrapper_tmp = TempDir::new().unwrap();

        let registry_path = registry_tmp.path();
        let wrapper_base = wrapper_tmp.path();

        fs::create_dir_all(registry_path.join(".claude-plugin")).unwrap();

        let marketplace_content = r#"{
            "name": "mixed-registry",
            "plugins": [
                {
                    "name": "remote-plugin",
                    "source": {"source": "url", "url": "https://github.com/test/remote.git"}
                },
                {
                    "name": "git-subdir-plugin",
                    "source": {"source": "git-subdir", "url": "stripe/ai", "path": "providers/claude/plugin", "ref": "main"}
                }
            ]
        }"#;
        fs::write(
            registry_path.join(".claude-plugin/marketplace.json"),
            marketplace_content,
        )
        .unwrap();

        let result =
            generate_wrapper_marketplace(registry_path, "mixed-registry", wrapper_base);
        assert!(result.is_ok(), "failed: {:?}", result);

        let written = fs::read_to_string(result.unwrap()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&written).unwrap();

        let plugins = json["plugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 2);

        // Remote sources must remain as objects, unchanged
        assert!(
            plugins[0]["source"].is_object(),
            "remote source should remain an object"
        );
        assert_eq!(
            plugins[0]["source"]["url"].as_str().unwrap(),
            "https://github.com/test/remote.git"
        );
        assert!(
            plugins[1]["source"].is_object(),
            "git-subdir source should remain an object"
        );
        assert_eq!(plugins[1]["source"]["ref"].as_str().unwrap(), "main");
    }

    #[test]
    fn test_generate_wrapper_idempotent() {
        let registry_tmp = TempDir::new().unwrap();
        let wrapper_tmp = TempDir::new().unwrap();

        let registry_path = registry_tmp.path();
        let wrapper_base = wrapper_tmp.path();

        fs::create_dir_all(registry_path.join(".claude-plugin")).unwrap();
        fs::create_dir_all(registry_path.join("plugins/some-plugin")).unwrap();
        fs::write(registry_path.join("plugins/some-plugin/package.json"), "{}").unwrap();

        let marketplace_content = r#"{
            "name": "idempotent-registry",
            "plugins": [
                {"name": "some-plugin", "source": "./plugins/some-plugin"}
            ]
        }"#;
        fs::write(
            registry_path.join(".claude-plugin/marketplace.json"),
            marketplace_content,
        )
        .unwrap();

        // Generate twice
        let r1 = generate_wrapper_marketplace(registry_path, "idempotent-registry", wrapper_base);
        let r2 = generate_wrapper_marketplace(registry_path, "idempotent-registry", wrapper_base);

        assert!(r1.is_ok(), "first call failed: {:?}", r1);
        assert!(r2.is_ok(), "second call failed: {:?}", r2);

        let content1 = fs::read_to_string(r1.unwrap()).unwrap();
        let content2 = fs::read_to_string(r2.unwrap()).unwrap();

        let json1: serde_json::Value = serde_json::from_str(&content1).unwrap();
        let json2: serde_json::Value = serde_json::from_str(&content2).unwrap();

        assert_eq!(json1, json2, "idempotency check failed");
        assert_eq!(json1["name"].as_str().unwrap(), "ccm-idempotent-registry");
    }

    // ── Relative path ───────────────────────────────────────────────────────────

    #[test]
    fn test_relative_path() {
        let tmp = TempDir::new().unwrap();

        // Create real directories so canonicalize works
        let from_dir = tmp.path().join("a/b/c");
        let to_dir = tmp.path().join("a/d/e");
        fs::create_dir_all(&from_dir).unwrap();
        fs::create_dir_all(&to_dir).unwrap();

        let rel = relative_path(&from_dir, &to_dir);
        // from a/b/c -> a/d/e should be ../../d/e
        assert_eq!(rel, "../../d/e", "got: {}", rel);
    }

    #[test]
    fn test_relative_path_same_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("x");
        fs::create_dir_all(&dir).unwrap();
        let rel = relative_path(&dir, &dir);
        assert_eq!(rel, ".");
    }

    #[test]
    fn test_relative_path_child() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        let rel = relative_path(&parent, &child);
        assert_eq!(rel, "child", "got: {}", rel);
    }

    #[test]
    fn test_relative_path_parent() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        let rel = relative_path(&child, &parent);
        assert_eq!(rel, "..", "got: {}", rel);
    }
}
