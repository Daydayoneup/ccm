use super::*;
use super::config_based::*;
use crate::scanner::compute_content_hash;
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

pub struct McpServerAdapter;

impl ResourceAdapter for McpServerAdapter {
    fn resource_type(&self) -> ResourceType {
        ResourceType::McpServer
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
                home.join(".claude.json")
            }
            TargetScope::Project => {
                let proj = project.ok_or_else(|| {
                    "Project scope requires a project to be provided".to_string()
                })?;
                PathBuf::from(&proj.path).join(".mcp.json")
            }
        };

        Ok(InstallTarget::ConfigEntry {
            config_file,
            key_path: format!("mcpServers.{}", name),
        })
    }

    fn install(
        &self,
        resource: &Resource,
        target: &InstallTarget,
        _link_type: &LinkType,
    ) -> Result<ResourceLink, String> {
        let (config_file, key_path) = match target {
            InstallTarget::ConfigEntry { config_file, key_path } => (config_file, key_path),
            InstallTarget::FilePath(_) => {
                return Err("McpServerAdapter requires ConfigEntry target".into())
            }
        };

        // Extract server name from key_path ("mcpServers.<name>")
        let server_name = key_path
            .strip_prefix("mcpServers.")
            .ok_or_else(|| format!("Invalid key_path format: {}", key_path))?;

        // Parse the metadata as JSON fragment
        let metadata_str = resource
            .metadata
            .as_deref()
            .ok_or_else(|| "Resource metadata is required for McpServer install".to_string())?;

        let server_value: serde_json::Value = serde_json::from_str(metadata_str)
            .map_err(|e| format!("Failed to parse resource metadata as JSON: {}", e))?;

        // Read (or create) the config file
        let mut config = read_or_create_json(config_file)?;

        // Merge the server entry
        config_merge_object(&mut config, "mcpServers", server_name, server_value)?;

        // Write back
        write_json(config_file, &config)?;

        let now = Utc::now().to_rfc3339();
        Ok(ResourceLink {
            id: Uuid::new_v4().to_string(),
            resource_id: resource.id.clone(),
            target_scope: String::new(), // caller fills this in
            target_path: config_file.to_string_lossy().to_string(),
            config_key: Some(key_path.clone()),
            project_id: None, // caller fills this in
            link_type: LinkType::ConfigMerge.as_str().to_string(),
            created_at: now,
        })
    }

    fn uninstall(&self, link: &ResourceLink) -> Result<(), String> {
        let config_key = link
            .config_key
            .as_deref()
            .ok_or_else(|| "ResourceLink missing config_key for McpServer uninstall".to_string())?;

        let server_name = config_key
            .strip_prefix("mcpServers.")
            .ok_or_else(|| format!("Invalid config_key format: {}", config_key))?;

        let config_path = std::path::Path::new(&link.target_path);

        // If the file doesn't exist, nothing to do.
        if !config_path.exists() {
            return Ok(());
        }

        let mut config = read_or_create_json(config_path)?;
        config_unmerge_object(&mut config, "mcpServers", server_name)?;
        write_json(config_path, &config)?;

        Ok(())
    }

    fn validate_content(&self, content: &str) -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        let has_command = value.get("command").is_some();
        let has_url = value.get("url").is_some();

        if !has_command && !has_url {
            return Err(
                "MCP server definition must contain either 'command' or 'url' field".to_string(),
            );
        }

        Ok(())
    }

    fn scan(&self, scope: &ResourceScope, base_path: &Path) -> Result<Vec<Resource>, String> {
        let config_path = match scope {
            ResourceScope::Global => {
                let home = dirs::home_dir()
                    .ok_or_else(|| "Cannot determine home directory".to_string())?;
                home.join(".claude.json")
            }
            ResourceScope::Project => base_path.join(".mcp.json"),
            other => {
                return Err(format!(
                    "McpServerAdapter.scan does not support scope: {}",
                    other.as_str()
                ))
            }
        };

        if !config_path.exists() {
            return Ok(Vec::new());
        }

        // On malformed JSON: log a warning and return empty vec (don't error).
        let config = match read_or_create_json(&config_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "Warning: failed to parse MCP config at {}: {}",
                    config_path.display(),
                    e
                );
                return Ok(Vec::new());
            }
        };

        let mcp_servers = match config.get("mcpServers").and_then(|v| v.as_object()) {
            Some(obj) => obj,
            None => return Ok(Vec::new()),
        };

        let now = Utc::now().to_rfc3339();
        let source_path = config_path.to_string_lossy().to_string();
        let mut resources = Vec::new();

        for (name, server_def) in mcp_servers {
            let metadata_str = serde_json::to_string(server_def).unwrap_or_default();
            let content_hash = compute_content_hash(&metadata_str);

            resources.push(Resource {
                id: Uuid::new_v4().to_string(),
                resource_type: ResourceType::McpServer,
                name: name.clone(),
                description: None,
                scope: scope.clone(),
                source_path: source_path.clone(),
                content_hash: Some(content_hash),
                metadata: Some(metadata_str),
                created_at: now.clone(),
                updated_at: now.clone(),
            });
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
            resource_type: ResourceType::McpServer,
            name: name.to_string(),
            description: None,
            scope: ResourceScope::Project,
            source_path: String::new(),
            content_hash: None,
            metadata: metadata.map(|s| s.to_string()),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    // -------------------------------------------------------------------------
    // Type and strategy
    // -------------------------------------------------------------------------

    #[test]
    fn test_mcp_type_and_strategy() {
        let adapter = McpServerAdapter;
        assert_eq!(adapter.resource_type(), ResourceType::McpServer);
        assert_eq!(adapter.install_strategy(), InstallStrategy::ConfigBased);
    }

    // -------------------------------------------------------------------------
    // resolve_target
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolve_target_global() {
        let adapter = McpServerAdapter;
        let result = adapter
            .resolve_target(&TargetScope::Global, "my-server", None)
            .expect("should succeed");

        match result {
            InstallTarget::ConfigEntry { config_file, key_path } => {
                let s = config_file.to_string_lossy();
                assert!(s.ends_with("/.claude.json") || s.ends_with(".claude.json"), "got: {}", s);
                assert_eq!(key_path, "mcpServers.my-server");
            }
            _ => panic!("expected ConfigEntry"),
        }
    }

    #[test]
    fn test_resolve_target_project() {
        let tmp = TempDir::new().unwrap();
        let adapter = McpServerAdapter;
        let project = make_project(tmp.path().to_str().unwrap());

        let result = adapter
            .resolve_target(&TargetScope::Project, "my-server", Some(&project))
            .expect("should succeed");

        match result {
            InstallTarget::ConfigEntry { config_file, key_path } => {
                let expected_file = tmp.path().join(".mcp.json");
                assert_eq!(config_file, expected_file);
                assert_eq!(key_path, "mcpServers.my-server");
            }
            _ => panic!("expected ConfigEntry"),
        }
    }

    // -------------------------------------------------------------------------
    // install and verify config
    // -------------------------------------------------------------------------

    #[test]
    fn test_install_and_verify_config() {
        let tmp = TempDir::new().unwrap();
        let adapter = McpServerAdapter;
        let project = make_project(tmp.path().to_str().unwrap());

        let resource = make_resource(
            "res-1",
            "my-server",
            Some(r#"{"server_type":"stdio","command":"node","args":["server.js"],"env":{}}"#),
        );

        let target = adapter
            .resolve_target(&TargetScope::Project, "my-server", Some(&project))
            .unwrap();

        let link = adapter.install(&resource, &target, &LinkType::ConfigMerge).unwrap();

        // Verify the link
        assert_eq!(link.resource_id, "res-1");
        assert_eq!(link.link_type, "config_merge");
        assert_eq!(link.config_key, Some("mcpServers.my-server".to_string()));

        // Verify the config file was written
        let config_path = tmp.path().join(".mcp.json");
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["mcpServers"]["my-server"]["command"], "node");
    }

    // -------------------------------------------------------------------------
    // uninstall removes entry
    // -------------------------------------------------------------------------

    #[test]
    fn test_uninstall_removes_entry() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".mcp.json");

        // Write initial config with two servers
        let initial = serde_json::json!({
            "mcpServers": {
                "server-a": {"command": "node", "args": ["a.js"]},
                "server-b": {"command": "python", "args": ["b.py"]}
            }
        });
        fs::write(&config_path, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let adapter = McpServerAdapter;
        let link = ResourceLink {
            id: Uuid::new_v4().to_string(),
            resource_id: "res-1".to_string(),
            target_scope: "project".to_string(),
            target_path: config_path.to_string_lossy().to_string(),
            config_key: Some("mcpServers.server-a".to_string()),
            project_id: None,
            link_type: "config_merge".to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        adapter.uninstall(&link).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["mcpServers"].get("server-a").is_none());
        // server-b should remain
        assert!(parsed["mcpServers"].get("server-b").is_some());
    }

    // -------------------------------------------------------------------------
    // install conflict error
    // -------------------------------------------------------------------------

    #[test]
    fn test_install_conflict_error() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".mcp.json");

        // Pre-populate with an existing server
        let initial = serde_json::json!({
            "mcpServers": {
                "my-server": {"command": "existing"}
            }
        });
        fs::write(&config_path, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let adapter = McpServerAdapter;
        let project = make_project(tmp.path().to_str().unwrap());
        let resource = make_resource("res-1", "my-server", Some(r#"{"command":"new"}"#));
        let target = adapter
            .resolve_target(&TargetScope::Project, "my-server", Some(&project))
            .unwrap();

        let err = adapter
            .install(&resource, &target, &LinkType::ConfigMerge)
            .expect_err("should fail due to conflict");
        assert!(err.contains("conflict"), "expected 'conflict' in: {}", err);
    }

    // -------------------------------------------------------------------------
    // validate_content
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_content_with_command() {
        let adapter = McpServerAdapter;
        let result =
            adapter.validate_content(r#"{"command":"node","args":["server.js"]}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_content_with_url() {
        let adapter = McpServerAdapter;
        let result = adapter.validate_content(r#"{"url":"http://localhost:8080/mcp"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_content_missing_both() {
        let adapter = McpServerAdapter;
        let err = adapter
            .validate_content(r#"{"env":{"FOO":"bar"}}"#)
            .expect_err("should fail");
        assert!(
            err.contains("command") || err.contains("url"),
            "error: {}",
            err
        );
    }

    #[test]
    fn test_validate_content_invalid_json() {
        let adapter = McpServerAdapter;
        let err = adapter
            .validate_content("{ not valid json }")
            .expect_err("should fail");
        assert!(err.contains("Invalid JSON") || err.contains("JSON"), "error: {}", err);
    }

    // -------------------------------------------------------------------------
    // scan
    // -------------------------------------------------------------------------

    #[test]
    fn test_scan_project() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".mcp.json");

        let config = serde_json::json!({
            "mcpServers": {
                "server-a": {"command": "node", "args": ["a.js"]},
                "server-b": {"url": "http://localhost:8080/mcp"}
            }
        });
        fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let adapter = McpServerAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert_eq!(resources.len(), 2);
        let names: Vec<&str> = resources.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"server-a"));
        assert!(names.contains(&"server-b"));

        for r in &resources {
            assert_eq!(r.resource_type, ResourceType::McpServer);
            assert_eq!(r.scope, ResourceScope::Project);
            assert!(r.content_hash.is_some());
            assert!(r.metadata.is_some());
            // source_path should point to the .mcp.json file
            assert!(r.source_path.ends_with(".mcp.json"));
        }
    }

    #[test]
    fn test_scan_empty_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".mcp.json");
        // Config with no mcpServers key
        fs::write(&config_path, r#"{"otherKey": 123}"#).unwrap();

        let adapter = McpServerAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("scan should succeed");

        assert!(resources.is_empty());
    }

    #[test]
    fn test_scan_malformed_json_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".mcp.json");
        fs::write(&config_path, "{ this is not valid json }").unwrap();

        let adapter = McpServerAdapter;
        let resources = adapter
            .scan(&ResourceScope::Project, tmp.path())
            .expect("malformed JSON should return empty vec, not error");

        assert!(resources.is_empty());
    }
}
