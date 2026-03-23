use super::*;
use super::config_based::*;
use crate::scanner::compute_content_hash;
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

const VALID_HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "SubagentStart",
    "SubagentStop",
    "Stop",
    "StopFailure",
    "TeammateIdle",
    "TaskCompleted",
    "InstructionsLoaded",
    "ConfigChange",
    "WorktreeCreate",
    "WorktreeRemove",
    "PreCompact",
    "PostCompact",
    "Elicitation",
    "ElicitationResult",
    "SessionEnd",
];

pub struct HookAdapter;

impl ResourceAdapter for HookAdapter {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Hook
    }

    fn install_strategy(&self) -> InstallStrategy {
        InstallStrategy::ConfigBased
    }

    fn resolve_target(
        &self,
        scope: &TargetScope,
        name: &str,
        project: Option<&Project>,
    ) -> Result<InstallTarget, String> {
        let config_file = match scope {
            TargetScope::Global => {
                let home = dirs::home_dir()
                    .ok_or_else(|| "Cannot determine home directory".to_string())?;
                home.join(".claude").join("settings.json")
            }
            TargetScope::Project => {
                let proj = project.ok_or_else(|| {
                    "Project scope requires a project to be provided".to_string()
                })?;
                PathBuf::from(&proj.path).join(".claude").join("settings.json")
            }
        };

        Ok(InstallTarget::ConfigEntry {
            config_file,
            key_path: format!("hooks.<event>._ccm_id={}", name),
        })
    }

    fn install(
        &self,
        resource: &Resource,
        target: &InstallTarget,
        _link_type: &LinkType,
    ) -> Result<ResourceLink, String> {
        let (config_file, _key_path) = match target {
            InstallTarget::ConfigEntry { config_file, key_path } => (config_file, key_path),
            InstallTarget::FilePath(_) => {
                return Err("HookAdapter requires ConfigEntry target".into())
            }
        };

        // Parse metadata
        let metadata_str = resource
            .metadata
            .as_deref()
            .ok_or_else(|| "Resource metadata is required for Hook install".to_string())?;

        let metadata: serde_json::Value = serde_json::from_str(metadata_str)
            .map_err(|e| format!("Failed to parse resource metadata as JSON: {}", e))?;

        // Extract fields from metadata
        let event = metadata
            .get("event")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Resource metadata must have 'event' field".to_string())?;

        let matcher = metadata.get("matcher").and_then(|v| v.as_str());

        let hook_config = metadata
            .get("hook_config")
            .ok_or_else(|| "Resource metadata must have 'hook_config' field".to_string())?;

        // Build hook entry:
        // { "matcher": "...", "hooks": [hook_config], "_ccm_id": resource.id }
        let mut hook_entry = serde_json::json!({
            "hooks": [hook_config]
        });

        if let Some(m) = matcher {
            hook_entry
                .as_object_mut()
                .unwrap()
                .insert("matcher".to_string(), serde_json::json!(m));
        }

        // Read (or create) config file
        let mut config = read_or_create_json(config_file)?;

        // config_merge_hook injects _ccm_id into the hook entry
        config_merge_hook(&mut config, event, &resource.id, hook_entry)?;

        // Write back
        write_json(config_file, &config)?;

        let config_key = format!("hooks.{}._ccm_id={}", event, resource.id);
        let now = Utc::now().to_rfc3339();

        Ok(ResourceLink {
            id: Uuid::new_v4().to_string(),
            resource_id: resource.id.clone(),
            target_scope: String::new(), // caller fills this in
            target_path: config_file.to_string_lossy().to_string(),
            config_key: Some(config_key),
            project_id: None, // caller fills this in
            link_type: LinkType::ConfigMerge.as_str().to_string(),
            created_at: now,
        })
    }

    fn uninstall(&self, link: &ResourceLink) -> Result<(), String> {
        let config_key = link
            .config_key
            .as_deref()
            .ok_or_else(|| "ResourceLink missing config_key for Hook uninstall".to_string())?;

        // Parse config_key format: "hooks.<EventName>._ccm_id=<resource_id>"
        // Strip leading "hooks." prefix
        let after_hooks = config_key
            .strip_prefix("hooks.")
            .ok_or_else(|| format!("Invalid config_key format (missing 'hooks.' prefix): {}", config_key))?;

        // Split on "._ccm_id=" to separate event name from resource id
        let sep = "._ccm_id=";
        let sep_pos = after_hooks
            .find(sep)
            .ok_or_else(|| format!("Invalid config_key format (missing '._ccm_id='): {}", config_key))?;

        let event = &after_hooks[..sep_pos];
        let ccm_id = &after_hooks[sep_pos + sep.len()..];

        let config_path = std::path::Path::new(&link.target_path);

        if !config_path.exists() {
            return Ok(());
        }

        let mut config = read_or_create_json(config_path)?;
        config_unmerge_hook(&mut config, event, ccm_id)?;
        write_json(config_path, &config)?;

        Ok(())
    }

    fn validate_content(&self, content: &str) -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        let event = value
            .get("event")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Hook metadata must have 'event' field".to_string())?;

        if !VALID_HOOK_EVENTS.contains(&event) {
            return Err(format!(
                "Invalid hook event '{}'. Must be one of: {}",
                event,
                VALID_HOOK_EVENTS.join(", ")
            ));
        }

        if value.get("hook_config").is_none() {
            return Err("Hook metadata must have 'hook_config' field".to_string());
        }

        Ok(())
    }

    fn scan(&self, scope: &ResourceScope, base_path: &Path) -> Result<Vec<Resource>, String> {
        let config_path = match scope {
            ResourceScope::Global => {
                let home = dirs::home_dir()
                    .ok_or_else(|| "Cannot determine home directory".to_string())?;
                home.join(".claude").join("settings.json")
            }
            ResourceScope::Project => base_path.join(".claude").join("settings.json"),
            other => {
                return Err(format!(
                    "HookAdapter.scan does not support scope: {}",
                    other.as_str()
                ))
            }
        };

        if !config_path.exists() {
            return Ok(Vec::new());
        }

        // On malformed JSON: log a warning and return empty vec
        let config = match read_or_create_json(&config_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "Warning: failed to parse settings at {}: {}",
                    config_path.display(),
                    e
                );
                return Ok(Vec::new());
            }
        };

        let hooks_obj = match config.get("hooks").and_then(|v| v.as_object()) {
            Some(obj) => obj,
            None => return Ok(Vec::new()),
        };

        let now = Utc::now().to_rfc3339();
        let source_path = config_path.to_string_lossy().to_string();
        let mut resources = Vec::new();

        for (event, event_value) in hooks_obj {
            let entries = match event_value.as_array() {
                Some(arr) => arr,
                None => continue,
            };

            for entry in entries {
                // Determine id: use _ccm_id if present, otherwise generate a new one
                let resource_id = entry
                    .get("_ccm_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| Uuid::new_v4().to_string());

                // Extract matcher and hooks array from entry to reconstruct metadata
                let matcher = entry.get("matcher").and_then(|v| v.as_str());
                let hook_actions = entry.get("hooks");

                // Build a representative hook_config from the entry's hooks array
                let hook_config = hook_actions
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.first())
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                // Reconstruct metadata in canonical format
                let mut meta = serde_json::json!({
                    "event": event,
                    "hook_config": hook_config
                });

                if let Some(m) = matcher {
                    meta.as_object_mut()
                        .unwrap()
                        .insert("matcher".to_string(), serde_json::json!(m));
                }

                let metadata_str = serde_json::to_string(&meta).unwrap_or_default();
                let content_hash = compute_content_hash(&metadata_str);

                // Use matcher as part of name, or fall back to event + id
                let name = if let Some(m) = matcher {
                    format!("{}/{}", event, m)
                } else {
                    format!("{}/{}", event, &resource_id[..8.min(resource_id.len())])
                };

                resources.push(Resource {
                    id: resource_id,
                    resource_type: ResourceType::Hook,
                    name,
                    description: None,
                    scope: scope.clone(),
                    source_path: source_path.clone(),
                    content_hash: Some(content_hash),
                    metadata: Some(metadata_str),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    version: None,
                    is_draft: 1,
                });
            }
        }

        Ok(resources)
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

    fn make_resource(id: &str, name: &str, metadata: Option<&str>) -> Resource {
        Resource {
            id: id.to_string(),
            resource_type: ResourceType::Hook,
            name: name.to_string(),
            description: None,
            scope: ResourceScope::Project,
            source_path: String::new(),
            content_hash: None,
            metadata: metadata.map(|s| s.to_string()),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            version: None,
            is_draft: 1,
        }
    }

    // -------------------------------------------------------------------------
    // Type and strategy
    // -------------------------------------------------------------------------

    #[test]
    fn test_hook_type_and_strategy() {
        let adapter = HookAdapter;
        assert_eq!(adapter.resource_type(), ResourceType::Hook);
        assert_eq!(adapter.install_strategy(), InstallStrategy::ConfigBased);
    }

    // -------------------------------------------------------------------------
    // resolve_target
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolve_target_global() {
        let adapter = HookAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "my-hook", None)
            .expect("should succeed");

        match result {
            InstallTarget::ConfigEntry { config_file, key_path } => {
                let s = config_file.to_string_lossy();
                assert!(
                    s.ends_with("/.claude/settings.json") || s.contains(".claude/settings.json"),
                    "got: {}",
                    s
                );
                assert_eq!(key_path, "hooks.<event>._ccm_id=my-hook");
            }
            _ => panic!("expected ConfigEntry"),
        }
    }

    #[test]
    fn test_resolve_target_project() {
        let tmp = TempDir::new().unwrap();
        let adapter = HookAdapter;
        let project = make_project(tmp.path().to_str().unwrap());

        let result = adapter
            .resolve_target(&TargetScope::Project, "my-hook", Some(&project))
            .expect("should succeed");

        match result {
            InstallTarget::ConfigEntry { config_file, key_path } => {
                let expected_file = tmp.path().join(".claude").join("settings.json");
                assert_eq!(config_file, expected_file);
                assert_eq!(key_path, "hooks.<event>._ccm_id=my-hook");
            }
            _ => panic!("expected ConfigEntry"),
        }
    }

    // -------------------------------------------------------------------------
    // install writes to settings.json
    // -------------------------------------------------------------------------

    #[test]
    fn test_install_writes_to_settings() {
        let tmp = TempDir::new().unwrap();
        let adapter = HookAdapter;
        let project = make_project(tmp.path().to_str().unwrap());

        let resource = make_resource(
            "res-1",
            "lint-hook",
            Some(r#"{"event":"PreToolUse","matcher":"Edit","hook_config":{"type":"command","command":"echo lint"}}"#),
        );

        let target = adapter
            .resolve_target(&TargetScope::Project, "lint-hook", Some(&project))
            .unwrap();

        let link = adapter.install(&resource, &target, &LinkType::ConfigMerge).unwrap();

        // Verify the link
        assert_eq!(link.resource_id, "res-1");
        assert_eq!(link.link_type, "config_merge");

        // Verify the config file was written
        let settings_path = tmp.path().join(".claude").join("settings.json");
        assert!(settings_path.exists());

        let content = fs::read_to_string(&settings_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        // hooks.PreToolUse should have one entry
        let arr = parsed["hooks"]["PreToolUse"].as_array().expect("should be array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["matcher"], "Edit");
        assert_eq!(arr[0]["hooks"][0]["command"], "echo lint");
    }

    // -------------------------------------------------------------------------
    // install injects _ccm_id
    // -------------------------------------------------------------------------

    #[test]
    fn test_install_injects_ccm_id() {
        let tmp = TempDir::new().unwrap();
        let adapter = HookAdapter;
        let project = make_project(tmp.path().to_str().unwrap());

        let resource = make_resource(
            "ccm-abc-123",
            "my-hook",
            Some(r#"{"event":"PostToolUse","matcher":"Bash","hook_config":{"type":"command","command":"echo done"}}"#),
        );

        let target = adapter
            .resolve_target(&TargetScope::Project, "my-hook", Some(&project))
            .unwrap();

        let link = adapter.install(&resource, &target, &LinkType::ConfigMerge).unwrap();

        // config_key should contain the resource id
        let config_key = link.config_key.expect("should have config_key");
        assert!(config_key.contains("ccm-abc-123"), "config_key: {}", config_key);
        assert!(config_key.starts_with("hooks.PostToolUse._ccm_id="), "config_key: {}", config_key);

        // Verify _ccm_id in file
        let settings_path = tmp.path().join(".claude").join("settings.json");
        let content = fs::read_to_string(&settings_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let arr = parsed["hooks"]["PostToolUse"].as_array().unwrap();
        assert_eq!(arr[0]["_ccm_id"], "ccm-abc-123");
    }

    // -------------------------------------------------------------------------
    // uninstall removes hook
    // -------------------------------------------------------------------------

    #[test]
    fn test_uninstall_removes_hook() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let settings_path = claude_dir.join("settings.json");

        // Write initial config with two hooks
        let initial = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Edit", "hooks": [{"type":"command","command":"echo a"}], "_ccm_id": "ccm-1"},
                    {"matcher": "Bash", "hooks": [{"type":"command","command":"echo b"}], "_ccm_id": "ccm-2"}
                ]
            }
        });
        fs::write(&settings_path, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let adapter = HookAdapter;
        let link = ResourceLink {
            id: Uuid::new_v4().to_string(),
            resource_id: "ccm-1".to_string(),
            target_scope: "project".to_string(),
            target_path: settings_path.to_string_lossy().to_string(),
            config_key: Some("hooks.PreToolUse._ccm_id=ccm-1".to_string()),
            project_id: None,
            link_type: "config_merge".to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        adapter.uninstall(&link).unwrap();

        let content = fs::read_to_string(&settings_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let arr = parsed["hooks"]["PreToolUse"].as_array().unwrap();

        // ccm-1 should be removed
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["_ccm_id"], "ccm-2");
    }

    // -------------------------------------------------------------------------
    // validate_content
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_valid() {
        let adapter = HookAdapter;
        let result = adapter.validate_content(
            r#"{"event":"PreToolUse","matcher":"Edit","hook_config":{"type":"command","command":"echo lint"}}"#,
        );
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    }

    #[test]
    fn test_validate_invalid_event() {
        let adapter = HookAdapter;
        let err = adapter
            .validate_content(r#"{"event":"InvalidEvent","hook_config":{"type":"command","command":"echo"}}"#)
            .expect_err("should fail");
        assert!(err.contains("Invalid hook event"), "error: {}", err);
        assert!(err.contains("InvalidEvent"), "error: {}", err);
    }

    #[test]
    fn test_validate_missing_event() {
        let adapter = HookAdapter;
        let err = adapter
            .validate_content(r#"{"hook_config":{"type":"command","command":"echo"}}"#)
            .expect_err("should fail");
        assert!(err.contains("event"), "error: {}", err);
    }

    #[test]
    fn test_validate_invalid_json() {
        let adapter = HookAdapter;
        let err = adapter
            .validate_content("{ not valid json }")
            .expect_err("should fail");
        assert!(err.contains("Invalid JSON") || err.contains("JSON"), "error: {}", err);
    }

    #[test]
    fn test_validate_missing_hook_config() {
        let adapter = HookAdapter;
        let err = adapter
            .validate_content(r#"{"event":"PreToolUse","matcher":"Edit"}"#)
            .expect_err("should fail");
        assert!(err.contains("hook_config"), "error: {}", err);
    }

    // -------------------------------------------------------------------------
    // scan
    // -------------------------------------------------------------------------

    #[test]
    fn test_scan_project_settings() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let settings_path = claude_dir.join("settings.json");

        let config = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Edit", "hooks": [{"type":"command","command":"echo lint"}], "_ccm_id": "ccm-aaa"},
                    {"matcher": "Bash", "hooks": [{"type":"command","command":"echo bash"}], "_ccm_id": "ccm-bbb"}
                ],
                "PostToolUse": [
                    {"matcher": "Edit", "hooks": [{"type":"command","command":"echo post"}], "_ccm_id": "ccm-ccc"}
                ]
            }
        });
        fs::write(&settings_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let adapter = HookAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert_eq!(resources.len(), 3);

        for r in &resources {
            assert_eq!(r.resource_type, ResourceType::Hook);
            assert_eq!(r.scope, ResourceScope::Project);
            assert!(r.content_hash.is_some());
            assert!(r.metadata.is_some());
            assert!(r.source_path.ends_with("settings.json"));
        }

        // Check that _ccm_id values are preserved as resource ids
        let ids: Vec<&str> = resources.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"ccm-aaa"), "ids: {:?}", ids);
        assert!(ids.contains(&"ccm-bbb"), "ids: {:?}", ids);
        assert!(ids.contains(&"ccm-ccc"), "ids: {:?}", ids);
    }

    #[test]
    fn test_scan_empty_settings() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let settings_path = claude_dir.join("settings.json");

        // Settings with no hooks key
        fs::write(&settings_path, r#"{"otherKey": 123}"#).unwrap();

        let adapter = HookAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert!(resources.is_empty());
    }

    #[test]
    fn test_scan_nonexistent_settings() {
        let tmp = TempDir::new().unwrap();

        let adapter = HookAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed on missing file");

        assert!(resources.is_empty());
    }

    #[test]
    fn test_scan_malformed_json_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let settings_path = claude_dir.join("settings.json");
        fs::write(&settings_path, "{ this is not valid json }").unwrap();

        let adapter = HookAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("malformed JSON should return empty vec, not error");

        assert!(resources.is_empty());
    }

    #[test]
    fn test_scan_generates_id_when_no_ccm_id() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let settings_path = claude_dir.join("settings.json");

        // Entry without _ccm_id
        let config = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Edit", "hooks": [{"type":"command","command":"echo lint"}]}
                ]
            }
        });
        fs::write(&settings_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let adapter = HookAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert_eq!(resources.len(), 1);
        // id should be a UUID (non-empty)
        assert!(!resources[0].id.is_empty());
    }

    #[test]
    fn test_scan_all_22_valid_events_accepted() {
        // Verify validate_content accepts all 22 valid events
        let adapter = HookAdapter;
        for event in VALID_HOOK_EVENTS {
            let content = format!(
                r#"{{"event":"{}","hook_config":{{"type":"command","command":"echo"}}}}"#,
                event
            );
            let result = adapter.validate_content(&content);
            assert!(result.is_ok(), "Event '{}' should be valid, got: {:?}", event, result);
        }
    }
}
