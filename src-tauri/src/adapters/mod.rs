pub mod agent;
pub mod command;
pub mod config_based;
pub mod file_based;
pub mod hook;
pub mod mcp_server;
pub mod plugin_install;
pub mod rule;
pub mod skill;

use crate::models::v2::{Project, Resource, ResourceLink, ResourceScope, ResourceType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum TargetScope {
    Global,
    Project,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstallStrategy {
    FileBased,
    ConfigBased,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkType {
    Symlink,
    Copy,
    ConfigMerge,
    PluginInstall,
}

impl LinkType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Symlink => "symlink",
            Self::Copy => "copy",
            Self::ConfigMerge => "config_merge",
            Self::PluginInstall => "plugin_install",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "symlink" => Some(Self::Symlink),
            "copy" => Some(Self::Copy),
            "config_merge" => Some(Self::ConfigMerge),
            "plugin_install" => Some(Self::PluginInstall),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum InstallTarget {
    FilePath(PathBuf),
    ConfigEntry {
        config_file: PathBuf,
        key_path: String,
    },
}

pub trait ResourceAdapter: Send + Sync {
    fn resource_type(&self) -> ResourceType;
    fn install_strategy(&self) -> InstallStrategy;
    fn resolve_target(&self, scope: &TargetScope, resource_name: &str, project: Option<&Project>) -> Result<InstallTarget, String>;
    fn install(&self, resource: &Resource, target: &InstallTarget, link_type: &LinkType) -> Result<ResourceLink, String>;
    fn uninstall(&self, link: &ResourceLink) -> Result<(), String>;
    fn validate_content(&self, content: &str) -> Result<(), String>;
    fn scan(&self, scope: &ResourceScope, base_path: &Path) -> Result<Vec<Resource>, String>;
}

pub fn normalize_link_type(adapter: &dyn ResourceAdapter, requested: LinkType) -> LinkType {
    match adapter.install_strategy() {
        InstallStrategy::ConfigBased => LinkType::ConfigMerge,
        InstallStrategy::FileBased => requested,
    }
}

pub struct AdapterRegistry {
    adapters: HashMap<ResourceType, Box<dyn ResourceAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        let mut map: HashMap<ResourceType, Box<dyn ResourceAdapter>> = HashMap::new();
        map.insert(ResourceType::Agent, Box::new(agent::AgentAdapter));
        map.insert(ResourceType::Skill, Box::new(skill::SkillAdapter));
        map.insert(ResourceType::Rule, Box::new(rule::RuleAdapter));
        map.insert(ResourceType::Hook, Box::new(hook::HookAdapter));
        map.insert(ResourceType::McpServer, Box::new(mcp_server::McpServerAdapter));
        map.insert(ResourceType::Command, Box::new(command::CommandAdapter));
        Self { adapters: map }
    }

    pub fn get(&self, rt: &ResourceType) -> Option<&dyn ResourceAdapter> {
        self.adapters.get(rt).map(|a| a.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_all_types() {
        let registry = AdapterRegistry::new();
        assert!(registry.get(&ResourceType::Agent).is_some());
        assert!(registry.get(&ResourceType::Skill).is_some());
        assert!(registry.get(&ResourceType::Rule).is_some());
        assert!(registry.get(&ResourceType::Hook).is_some());
        assert!(registry.get(&ResourceType::McpServer).is_some());
        assert!(registry.get(&ResourceType::Command).is_some());
    }

    #[test]
    fn test_normalize_link_type_file_based() {
        let registry = AdapterRegistry::new();
        let agent = registry.get(&ResourceType::Agent).unwrap();
        assert_eq!(normalize_link_type(agent, LinkType::Symlink), LinkType::Symlink);
        assert_eq!(normalize_link_type(agent, LinkType::Copy), LinkType::Copy);
    }

    #[test]
    fn test_normalize_link_type_config_based() {
        let registry = AdapterRegistry::new();
        let hook = registry.get(&ResourceType::Hook).unwrap();
        assert_eq!(normalize_link_type(hook, LinkType::Symlink), LinkType::ConfigMerge);
        assert_eq!(normalize_link_type(hook, LinkType::Copy), LinkType::ConfigMerge);
    }

    #[test]
    fn test_link_type_round_trip() {
        assert_eq!(LinkType::from_str("symlink"), Some(LinkType::Symlink));
        assert_eq!(LinkType::from_str("copy"), Some(LinkType::Copy));
        assert_eq!(LinkType::from_str("config_merge"), Some(LinkType::ConfigMerge));
        assert_eq!(LinkType::from_str("invalid"), None);
    }
}
