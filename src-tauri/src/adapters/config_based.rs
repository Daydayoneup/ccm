// Shared logic for config-based resource adapters (ConfigBased install strategy).
// Concrete implementations: HookAdapter, McpServerAdapter.

use serde_json::{json, Value};
use std::path::Path;

/// Read a JSON file, or return an empty object `{}` if the file does not exist.
/// Returns an error if the file exists but is not valid JSON.
pub fn read_or_create_json(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(json!({}));
    }

    serde_json::from_str(trimmed)
        .map_err(|e| format!("Failed to parse JSON from {}: {}", path.display(), e))
}

/// Write a JSON value to a file with pretty (4-space indent) formatting.
pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directories for {}: {}", path.display(), e))?;
    }

    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

/// Merge an entry into a top-level object key (used for MCP servers).
///
/// - If `root_key` doesn't exist, it is created as `{}`.
/// - If `entry_key` already exists under `root_key`, returns an error (conflict — not CCM managed).
/// - Otherwise inserts `value` under `config[root_key][entry_key]`.
pub fn config_merge_object(
    config: &mut Value,
    root_key: &str,
    entry_key: &str,
    value: Value,
) -> Result<(), String> {
    let root = config
        .as_object_mut()
        .ok_or_else(|| "config is not a JSON object".to_string())?;

    let section = root
        .entry(root_key)
        .or_insert_with(|| json!({}));

    let section_obj = section
        .as_object_mut()
        .ok_or_else(|| format!("config[\"{}\"] is not a JSON object", root_key))?;

    if section_obj.contains_key(entry_key) {
        return Err(format!(
            "Entry \"{}\" already exists in \"{}\": conflict — not CCM managed",
            entry_key, root_key
        ));
    }

    section_obj.insert(entry_key.to_string(), value);
    Ok(())
}

/// Remove an entry from a top-level object key (used for MCP servers).
///
/// If `root_key` or `entry_key` does not exist, this is a no-op (success).
pub fn config_unmerge_object(
    config: &mut Value,
    root_key: &str,
    entry_key: &str,
) -> Result<(), String> {
    let root = config
        .as_object_mut()
        .ok_or_else(|| "config is not a JSON object".to_string())?;

    if let Some(section) = root.get_mut(root_key) {
        if let Some(section_obj) = section.as_object_mut() {
            section_obj.remove(entry_key);
        }
    }

    Ok(())
}

/// Merge a hook entry into the hooks array for a given event.
///
/// - Ensures `config["hooks"][event]` array exists.
/// - Injects `_ccm_id` field into `hook_entry`.
/// - If an entry with the same `_ccm_id` already exists → replaces it (update).
/// - Otherwise → appends.
pub fn config_merge_hook(
    config: &mut Value,
    event: &str,
    ccm_id: &str,
    mut hook_entry: Value,
) -> Result<(), String> {
    // Inject _ccm_id into the hook entry
    let entry_obj = hook_entry
        .as_object_mut()
        .ok_or_else(|| "hook_entry must be a JSON object".to_string())?;
    entry_obj.insert("_ccm_id".to_string(), json!(ccm_id));

    // Re-bind as immutable borrow is gone
    let hook_entry = Value::Object(entry_obj.clone());

    // Ensure config is an object
    let root = config
        .as_object_mut()
        .ok_or_else(|| "config is not a JSON object".to_string())?;

    // Ensure hooks section exists
    let hooks_section = root
        .entry("hooks")
        .or_insert_with(|| json!({}));

    let hooks_obj = hooks_section
        .as_object_mut()
        .ok_or_else(|| "config[\"hooks\"] is not a JSON object".to_string())?;

    // Ensure event array exists
    let event_array = hooks_obj
        .entry(event)
        .or_insert_with(|| json!([]));

    let arr = event_array
        .as_array_mut()
        .ok_or_else(|| format!("config[\"hooks\"][\"{}\"] is not a JSON array", event))?;

    // Check if an entry with the same _ccm_id already exists
    let existing_pos = arr.iter().position(|item| {
        item.get("_ccm_id")
            .and_then(|v| v.as_str())
            .map(|id| id == ccm_id)
            .unwrap_or(false)
    });

    if let Some(pos) = existing_pos {
        arr[pos] = hook_entry;
    } else {
        arr.push(hook_entry);
    }

    Ok(())
}

/// Remove a hook entry identified by `_ccm_id` from `config["hooks"][event]`.
///
/// If the event array or the entry does not exist, this is a no-op (success).
pub fn config_unmerge_hook(
    config: &mut Value,
    event: &str,
    ccm_id: &str,
) -> Result<(), String> {
    let root = config
        .as_object_mut()
        .ok_or_else(|| "config is not a JSON object".to_string())?;

    if let Some(hooks_section) = root.get_mut("hooks") {
        if let Some(hooks_obj) = hooks_section.as_object_mut() {
            if let Some(event_array) = hooks_obj.get_mut(event) {
                if let Some(arr) = event_array.as_array_mut() {
                    arr.retain(|item| {
                        item.get("_ccm_id")
                            .and_then(|v| v.as_str())
                            .map(|id| id != ccm_id)
                            .unwrap_or(true)
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check if a hook with the given `_ccm_id` exists in `config["hooks"][event]`.
pub fn is_ccm_managed_hook(config: &Value, event: &str, ccm_id: &str) -> bool {
    config
        .get("hooks")
        .and_then(|h| h.get(event))
        .and_then(|arr| arr.as_array())
        .map(|arr| {
            arr.iter().any(|item| {
                item.get("_ccm_id")
                    .and_then(|v| v.as_str())
                    .map(|id| id == ccm_id)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Check if `config[root_key][entry_key]` exists.
pub fn has_entry(config: &Value, root_key: &str, entry_key: &str) -> bool {
    config
        .get(root_key)
        .and_then(|section| section.as_object())
        .map(|obj| obj.contains_key(entry_key))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -------------------------------------------------------------------------
    // read_or_create_json
    // -------------------------------------------------------------------------

    #[test]
    fn test_read_or_create_json_new_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        let result = read_or_create_json(&path).unwrap();
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_read_or_create_json_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"key": "value"}"#).unwrap();

        let result = read_or_create_json(&path).unwrap();
        assert_eq!(result, json!({"key": "value"}));
    }

    #[test]
    fn test_read_or_create_json_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.json");
        std::fs::write(&path, "   ").unwrap();

        let result = read_or_create_json(&path).unwrap();
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_read_or_create_json_malformed() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "{ not valid json }").unwrap();

        let result = read_or_create_json(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse JSON"));
    }

    // -------------------------------------------------------------------------
    // MCP server merge / unmerge
    // -------------------------------------------------------------------------

    #[test]
    fn test_merge_object_new_key() {
        let mut config = json!({});
        config_merge_object(&mut config, "mcpServers", "my-server", json!({"command": "node"}))
            .unwrap();

        assert_eq!(config["mcpServers"]["my-server"]["command"], "node");
    }

    #[test]
    fn test_merge_object_creates_root_key() {
        let mut config = json!({});
        config_merge_object(&mut config, "mcpServers", "srv", json!({"x": 1})).unwrap();
        assert!(config.get("mcpServers").is_some());
    }

    #[test]
    fn test_merge_object_conflict_error() {
        let mut config = json!({"mcpServers": {"existing": {"command": "python"}}});
        let result =
            config_merge_object(&mut config, "mcpServers", "existing", json!({"command": "node"}));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("conflict"), "Expected 'conflict' in: {}", err);
    }

    #[test]
    fn test_unmerge_object() {
        let mut config = json!({"mcpServers": {"my-server": {"command": "node"}}});
        config_unmerge_object(&mut config, "mcpServers", "my-server").unwrap();

        assert!(config["mcpServers"].as_object().unwrap().get("my-server").is_none());
    }

    #[test]
    fn test_unmerge_object_nonexistent_ok() {
        let mut config = json!({});
        // Should not error even if root_key or entry_key are absent
        let result = config_unmerge_object(&mut config, "mcpServers", "ghost");
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // Hook merge / unmerge with _ccm_id
    // -------------------------------------------------------------------------

    #[test]
    fn test_merge_hook_new_event() {
        let mut config = json!({});
        config_merge_hook(
            &mut config,
            "PreToolUse",
            "ccm-abc",
            json!({"matcher": "bash"}),
        )
        .unwrap();

        let arr = config["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["_ccm_id"], "ccm-abc");
        assert_eq!(arr[0]["matcher"], "bash");
    }

    #[test]
    fn test_merge_hook_existing_event_append() {
        let mut config = json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "existing", "_ccm_id": "ccm-old"}
                ]
            }
        });

        config_merge_hook(
            &mut config,
            "PreToolUse",
            "ccm-new",
            json!({"matcher": "new"}),
        )
        .unwrap();

        let arr = config["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[1]["_ccm_id"], "ccm-new");
    }

    #[test]
    fn test_merge_hook_update_existing_ccm_id() {
        let mut config = json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "old-matcher", "_ccm_id": "ccm-123"}
                ]
            }
        });

        config_merge_hook(
            &mut config,
            "PreToolUse",
            "ccm-123",
            json!({"matcher": "new-matcher"}),
        )
        .unwrap();

        let arr = config["hooks"]["PreToolUse"].as_array().unwrap();
        // Should replace, not append
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["matcher"], "new-matcher");
        assert_eq!(arr[0]["_ccm_id"], "ccm-123");
    }

    #[test]
    fn test_unmerge_hook() {
        let mut config = json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "bash", "_ccm_id": "ccm-abc"}
                ]
            }
        });

        config_unmerge_hook(&mut config, "PreToolUse", "ccm-abc").unwrap();

        let arr = config["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn test_unmerge_hook_preserves_others() {
        let mut config = json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "a", "_ccm_id": "ccm-1"},
                    {"matcher": "b", "_ccm_id": "ccm-2"},
                    {"matcher": "c", "_ccm_id": "ccm-3"}
                ]
            }
        });

        config_unmerge_hook(&mut config, "PreToolUse", "ccm-2").unwrap();

        let arr = config["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["_ccm_id"], "ccm-1");
        assert_eq!(arr[1]["_ccm_id"], "ccm-3");
    }

    #[test]
    fn test_unmerge_hook_nonexistent_ok() {
        let mut config = json!({});
        let result = config_unmerge_hook(&mut config, "PreToolUse", "ccm-ghost");
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // Helpers: is_ccm_managed_hook / has_entry
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_ccm_managed_hook() {
        let config = json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "bash", "_ccm_id": "ccm-abc"}
                ]
            }
        });

        assert!(is_ccm_managed_hook(&config, "PreToolUse", "ccm-abc"));
        assert!(!is_ccm_managed_hook(&config, "PreToolUse", "ccm-other"));
        assert!(!is_ccm_managed_hook(&config, "PostToolUse", "ccm-abc"));
    }

    #[test]
    fn test_is_ccm_managed_hook_empty_config() {
        let config = json!({});
        assert!(!is_ccm_managed_hook(&config, "PreToolUse", "ccm-abc"));
    }

    #[test]
    fn test_has_entry() {
        let config = json!({
            "mcpServers": {
                "my-server": {"command": "node"}
            }
        });

        assert!(has_entry(&config, "mcpServers", "my-server"));
        assert!(!has_entry(&config, "mcpServers", "other-server"));
        assert!(!has_entry(&config, "nonexistent", "my-server"));
    }

    #[test]
    fn test_has_entry_empty_config() {
        let config = json!({});
        assert!(!has_entry(&config, "mcpServers", "anything"));
    }

    // -------------------------------------------------------------------------
    // Integration: write and read back
    // -------------------------------------------------------------------------

    #[test]
    fn test_write_and_read_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        let original = json!({
            "mcpServers": {
                "my-server": {"command": "node", "args": ["index.js"]}
            },
            "hooks": {
                "PreToolUse": [
                    {"matcher": "bash", "_ccm_id": "ccm-123"}
                ]
            }
        });

        write_json(&path, &original).unwrap();
        let read_back = read_or_create_json(&path).unwrap();

        assert_eq!(original, read_back);
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("deep").join("config.json");

        write_json(&path, &json!({"ok": true})).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_roundtrip_merge_unmerge_object() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        let mut config = read_or_create_json(&path).unwrap();
        config_merge_object(&mut config, "mcpServers", "srv1", json!({"command": "python"}))
            .unwrap();
        write_json(&path, &config).unwrap();

        let mut config2 = read_or_create_json(&path).unwrap();
        assert!(has_entry(&config2, "mcpServers", "srv1"));

        config_unmerge_object(&mut config2, "mcpServers", "srv1").unwrap();
        write_json(&path, &config2).unwrap();

        let config3 = read_or_create_json(&path).unwrap();
        assert!(!has_entry(&config3, "mcpServers", "srv1"));
    }

    #[test]
    fn test_roundtrip_merge_unmerge_hook() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        let mut config = read_or_create_json(&path).unwrap();
        config_merge_hook(&mut config, "PreToolUse", "ccm-xyz", json!({"matcher": "bash"}))
            .unwrap();
        write_json(&path, &config).unwrap();

        let mut config2 = read_or_create_json(&path).unwrap();
        assert!(is_ccm_managed_hook(&config2, "PreToolUse", "ccm-xyz"));

        config_unmerge_hook(&mut config2, "PreToolUse", "ccm-xyz").unwrap();
        write_json(&path, &config2).unwrap();

        let config3 = read_or_create_json(&path).unwrap();
        assert!(!is_ccm_managed_hook(&config3, "PreToolUse", "ccm-xyz"));
    }
}
