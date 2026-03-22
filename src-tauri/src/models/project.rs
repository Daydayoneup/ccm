use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub language: String,
    pub linked_resources: Vec<LinkedResource>,
    pub local_resources: Vec<LocalResource>,
    pub last_scanned: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedResource {
    pub resource_type: ResourceType,
    pub library_id: String,
    pub project_path: String,
    pub symlink_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalResource {
    pub resource_type: ResourceType,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Skill,
    Agent,
    Rule,
    Hook,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectsRegistry {
    pub projects: Vec<Project>,
    pub scan_directories: Vec<String>,
}

impl ProjectsRegistry {
    pub fn new() -> Self {
        Self {
            projects: Vec::new(),
            scan_directories: Vec::new(),
        }
    }
}
