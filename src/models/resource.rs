use serde::{Deserialize, Serialize};

use super::project::ResourceType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryResource {
    pub id: String,
    pub resource_type: ResourceType,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub path: String,
    pub linked_projects: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryIndex {
    pub resources: Vec<LibraryResource>,
}

impl LibraryIndex {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalLink {
    pub library_resource_id: String,
    pub library_path: String,
    pub original_path: String,
    pub resource_type: ResourceType,
    pub imported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalLinksIndex {
    pub links: Vec<GlobalLink>,
}

impl GlobalLinksIndex {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub scope: String,
    pub install_path: String,
    pub resources: Vec<super::project::LocalResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub project_name: String,
    pub project_path: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpFileContent {
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub central_library_path: String,
    pub scan_directories: Vec<String>,
    pub version: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            central_library_path: home
                .join(".claude-manager")
                .to_string_lossy()
                .to_string(),
            scan_directories: vec![home.join("program").to_string_lossy().to_string()],
            version: "0.1.0".to_string(),
        }
    }
}
