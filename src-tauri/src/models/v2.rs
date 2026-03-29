use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Skill,
    Agent,
    Rule,
    Hook,
    Command,
    McpServer,
}

impl ResourceType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Skill => "skill",
            Self::Agent => "agent",
            Self::Rule => "rule",
            Self::Hook => "hook",
            Self::Command => "command",
            Self::McpServer => "mcp_server",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "skill" => Some(Self::Skill),
            "agent" => Some(Self::Agent),
            "rule" => Some(Self::Rule),
            "hook" => Some(Self::Hook),
            "command" => Some(Self::Command),
            "mcp_server" => Some(Self::McpServer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResourceScope {
    Global,
    Library,
    Project,
    Plugin,
    Registry,
}

impl ResourceScope {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Global => "global",
            Self::Library => "library",
            Self::Project => "project",
            Self::Plugin => "plugin",
            Self::Registry => "registry",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "global" => Some(Self::Global),
            "library" => Some(Self::Library),
            "project" => Some(Self::Project),
            "plugin" => Some(Self::Plugin),
            "registry" => Some(Self::Registry),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub resource_type: ResourceType,
    pub name: String,
    pub description: Option<String>,
    pub scope: ResourceScope,
    pub source_path: String,
    pub content_hash: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: Option<String>,
    pub is_draft: i32,
    pub installed_from_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceVersion {
    pub id: String,
    pub resource_id: String,
    pub version: String,
    pub changelog: Option<String>,
    pub content_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub language: Option<String>,
    pub last_scanned: Option<String>,
    pub pinned: i32,
    pub launch_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub scope: Option<String>,
    pub install_path: Option<String>,
    pub status: String,
    pub last_checked: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLink {
    pub id: String,
    pub resource_id: String,
    pub target_scope: String,
    pub target_path: String,
    pub config_key: Option<String>,  // for ConfigBased installs
    pub project_id: Option<String>,
    pub link_type: String,
    pub created_at: String,
    pub installed_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub id: String,
    pub watched_path: String,
    pub last_hash: Option<String>,
    pub last_synced: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub global_count: i64,
    pub project_count: i64,
    pub plugin_count: i64,
    pub library_count: i64,
    pub registry_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub id: String,
    pub project_id: Option<String>,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedEnvVar {
    pub id: String,
    pub key: String,
    pub value: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub id: String,
    pub name: String,
    pub url: String,
    pub local_path: String,
    pub readonly: bool,
    pub last_synced: Option<String>,
    pub has_remote_changes: bool,
    pub has_local_changes: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegistryPlugin {
    pub id: String,
    pub registry_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub source_path: String,
    pub source_type: String, // "local" or "external"
    pub source_url: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibraryPlugin {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibraryPluginResource {
    pub id: String,
    pub plugin_id: String,
    pub resource_id: String,
    pub created_at: String,
}
