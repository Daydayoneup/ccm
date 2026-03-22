use std::fs;
use super::ScannedMcpServer;

/// Parse a .mcp.json file and return ScannedMcpServer entries
pub fn parse_mcp_file(mcp_path: &str, project_id: Option<String>) -> Vec<ScannedMcpServer> {
    let content = match fs::read_to_string(mcp_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let servers = match parsed.get("mcpServers").and_then(|v| v.as_object()) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut result = Vec::new();
    for (name, config) in servers {
        let command = config
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let args = config.get("args").map(|v| v.to_string());
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let env = config.get("env").map(|v| v.to_string());

        let server_type = if url.is_some() {
            Some("sse".to_string())
        } else if command.is_some() {
            Some("stdio".to_string())
        } else {
            None
        };

        result.push(ScannedMcpServer {
            name: name.clone(),
            project_id: project_id.clone(),
            server_type,
            command,
            args,
            url,
            env,
            source_path: mcp_path.to_string(),
        });
    }

    result
}

/// Parse a project's .claude/settings.local.json and return MCP server names
/// from the `enabledMcpjsonServers` field.
pub fn parse_enabled_mcp_from_settings(settings_path: &str) -> Vec<String> {
    let content = match fs::read_to_string(settings_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    match parsed.get("enabledMcpjsonServers").and_then(|v| v.as_array()) {
        Some(arr) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
        None => Vec::new(),
    }
}

/// Parse a plugin-style .mcp.json file that may use either:
/// - `{"mcpServers": {"name": {...}}}` (standard format)
/// - `{"name": {"command": ..., ...}}` (plugin shorthand format)
pub fn parse_plugin_mcp_file(mcp_path: &str) -> Vec<ScannedMcpServer> {
    let content = match fs::read_to_string(mcp_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    // Try standard format first
    if let Some(servers) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
        return build_mcp_entries(servers, mcp_path);
    }

    // Plugin shorthand: top-level object where each key is a server name
    if let Some(obj) = parsed.as_object() {
        return build_mcp_entries(obj, mcp_path);
    }

    Vec::new()
}

fn build_mcp_entries(servers: &serde_json::Map<String, serde_json::Value>, source_path: &str) -> Vec<ScannedMcpServer> {
    let mut result = Vec::new();
    for (name, config) in servers {
        let command = config.get("command").and_then(|v| v.as_str()).map(|s| s.to_string());
        let args = config.get("args").map(|v| v.to_string());
        let url = config.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
        let env = config.get("env").map(|v| v.to_string());

        let server_type = if config.get("type").and_then(|v| v.as_str()) == Some("http") {
            Some("http".to_string())
        } else if url.is_some() {
            Some("sse".to_string())
        } else if command.is_some() {
            Some("stdio".to_string())
        } else {
            None
        };

        result.push(ScannedMcpServer {
            name: name.clone(),
            project_id: None,
            server_type,
            command,
            args,
            url,
            env,
            source_path: source_path.to_string(),
        });
    }
    result
}

/// Scan enabled plugins for MCP server definitions.
/// Reads ~/.claude/settings.json -> enabledPlugins, then checks each plugin's .mcp.json.
pub fn scan_plugin_mcp_servers() -> Vec<ScannedMcpServer> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let settings_path = home.join(".claude").join("settings.json");
    let content = match fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let enabled = match parsed.get("enabledPlugins").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return Vec::new(),
    };

    let mut result = Vec::new();
    for (plugin_key, enabled_val) in enabled {
        if enabled_val.as_bool() != Some(true) {
            continue;
        }
        // Parse "plugin-name@marketplace-name"
        let parts: Vec<&str> = plugin_key.splitn(2, '@').collect();
        if parts.len() != 2 {
            continue;
        }
        let plugin_name = parts[0];
        let marketplace = parts[1];

        let mcp_path = home
            .join(".claude")
            .join("plugins")
            .join("marketplaces")
            .join(marketplace)
            .join("external_plugins")
            .join(plugin_name)
            .join(".mcp.json");

        if mcp_path.is_file() {
            let servers = parse_plugin_mcp_file(mcp_path.to_str().unwrap_or_default());
            result.extend(servers);
        }
    }

    result
}

/// Scan global MCP config at ~/.claude/.mcp.json
pub fn scan_global_mcp() -> Vec<ScannedMcpServer> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let mcp_path = home.join(".claude").join(".mcp.json");
    if !mcp_path.is_file() {
        return Vec::new();
    }
    parse_mcp_file(mcp_path.to_str().unwrap_or_default(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_mcp_file() {
        let tmp = TempDir::new().unwrap();
        let mcp_path = tmp.path().join(".mcp.json");
        fs::write(
            &mcp_path,
            r#"{
            "mcpServers": {
                "my-server": {
                    "command": "node",
                    "args": ["server.js"],
                    "env": {"API_KEY": "test"}
                },
                "remote-server": {
                    "url": "https://example.com/mcp"
                }
            }
        }"#,
        )
        .unwrap();

        let servers = parse_mcp_file(mcp_path.to_str().unwrap(), Some("proj1".to_string()));
        assert_eq!(servers.len(), 2);

        let stdio = servers.iter().find(|s| s.name == "my-server").unwrap();
        assert_eq!(stdio.command.as_deref(), Some("node"));
        assert_eq!(stdio.server_type.as_deref(), Some("stdio"));
        assert!(stdio.args.is_some());
        assert!(stdio.env.is_some());
        assert_eq!(stdio.project_id.as_deref(), Some("proj1"));

        let sse = servers.iter().find(|s| s.name == "remote-server").unwrap();
        assert_eq!(sse.url.as_deref(), Some("https://example.com/mcp"));
        assert_eq!(sse.server_type.as_deref(), Some("sse"));
        assert!(sse.command.is_none());
    }

    #[test]
    fn test_parse_mcp_file_not_found() {
        let servers = parse_mcp_file("/nonexistent/.mcp.json", None);
        assert!(servers.is_empty());
    }

    #[test]
    fn test_parse_mcp_file_invalid_json() {
        let tmp = TempDir::new().unwrap();
        let mcp_path = tmp.path().join(".mcp.json");
        fs::write(&mcp_path, "not json").unwrap();
        let servers = parse_mcp_file(mcp_path.to_str().unwrap(), None);
        assert!(servers.is_empty());
    }

    #[test]
    fn test_parse_mcp_file_no_servers() {
        let tmp = TempDir::new().unwrap();
        let mcp_path = tmp.path().join(".mcp.json");
        fs::write(&mcp_path, r#"{"mcpServers": {}}"#).unwrap();
        let servers = parse_mcp_file(mcp_path.to_str().unwrap(), None);
        assert!(servers.is_empty());
    }

    #[test]
    fn test_parse_plugin_mcp_file_standard_format() {
        let tmp = TempDir::new().unwrap();
        let mcp_path = tmp.path().join(".mcp.json");
        fs::write(&mcp_path, r#"{"mcpServers": {"stripe": {"type": "http", "url": "https://mcp.stripe.com"}}}"#).unwrap();
        let servers = parse_plugin_mcp_file(mcp_path.to_str().unwrap());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "stripe");
        assert_eq!(servers[0].server_type.as_deref(), Some("http"));
    }

    #[test]
    fn test_parse_plugin_mcp_file_shorthand_format() {
        let tmp = TempDir::new().unwrap();
        let mcp_path = tmp.path().join(".mcp.json");
        fs::write(&mcp_path, r#"{"playwright": {"command": "npx", "args": ["@playwright/mcp@latest"]}}"#).unwrap();
        let servers = parse_plugin_mcp_file(mcp_path.to_str().unwrap());
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "playwright");
        assert_eq!(servers[0].server_type.as_deref(), Some("stdio"));
    }

    #[test]
    fn test_parse_enabled_mcp_from_settings() {
        let tmp = TempDir::new().unwrap();
        let settings_path = tmp.path().join("settings.local.json");
        fs::write(
            &settings_path,
            r#"{
                "permissions": {"allow": []},
                "enableAllProjectMcpServers": true,
                "enabledMcpjsonServers": ["mcp-ssh", "mcp-github"]
            }"#,
        )
        .unwrap();
        let names = parse_enabled_mcp_from_settings(settings_path.to_str().unwrap());
        assert_eq!(names, vec!["mcp-ssh", "mcp-github"]);
    }

    #[test]
    fn test_parse_enabled_mcp_from_settings_missing_field() {
        let tmp = TempDir::new().unwrap();
        let settings_path = tmp.path().join("settings.local.json");
        fs::write(&settings_path, r#"{"permissions": {"allow": []}}"#).unwrap();
        let names = parse_enabled_mcp_from_settings(settings_path.to_str().unwrap());
        assert!(names.is_empty());
    }

    #[test]
    fn test_parse_enabled_mcp_from_settings_not_found() {
        let names = parse_enabled_mcp_from_settings("/nonexistent/settings.local.json");
        assert!(names.is_empty());
    }
}
