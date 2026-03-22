use std::fs;
use std::path::Path;

use crate::models::project::ProjectsRegistry;
use crate::models::resource::{AppConfig, LibraryIndex};

/// Initialize the central library at the given base path.
/// Creates the directory structure: library/{skills,agents,rules,hooks,commands}, backups
/// Writes default config.json, projects.json, and library/index.json.
pub fn init_library_at(base_path: &str) -> Result<(), String> {
    let base = Path::new(base_path);

    // Create subdirectories
    let subdirs = [
        "library/skills",
        "library/agents",
        "library/rules",
        "library/hooks",
        "library/commands",
        "backups",
    ];

    for subdir in &subdirs {
        let dir_path = base.join(subdir);
        fs::create_dir_all(&dir_path)
            .map_err(|e| format!("Failed to create directory {}: {}", dir_path.display(), e))?;
    }

    // Write default config.json
    let config = AppConfig {
        central_library_path: base_path.to_string(),
        scan_directories: AppConfig::default().scan_directories,
        version: "0.1.0".to_string(),
    };
    let config_path = base.join("config.json");
    if !config_path.exists() {
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&config_path, config_json)
            .map_err(|e| format!("Failed to write config.json: {}", e))?;
    }

    // Write default projects.json
    let projects_path = base.join("projects.json");
    if !projects_path.exists() {
        let registry = ProjectsRegistry::new();
        let projects_json = serde_json::to_string_pretty(&registry)
            .map_err(|e| format!("Failed to serialize projects registry: {}", e))?;
        fs::write(&projects_path, projects_json)
            .map_err(|e| format!("Failed to write projects.json: {}", e))?;
    }

    // Write default library/index.json
    let index_path = base.join("library").join("index.json");
    if !index_path.exists() {
        let index = LibraryIndex::new();
        let index_json = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize library index: {}", e))?;
        fs::write(&index_path, index_json)
            .map_err(|e| format!("Failed to write library/index.json: {}", e))?;
    }

    Ok(())
}

/// Tauri command: Initialize the central library at the default path (~/.claude-manager).
#[tauri::command]
pub fn init_library() -> Result<String, String> {
    let config = AppConfig::default();
    let path = config.central_library_path.clone();
    init_library_at(&path)?;
    Ok(path)
}

/// Tauri command: Get the library path if it exists.
#[tauri::command]
pub fn get_library_path() -> Result<String, String> {
    let config = AppConfig::default();
    let path = &config.central_library_path;
    let base = Path::new(path);

    if base.exists() && base.join("config.json").exists() {
        Ok(path.to_string())
    } else {
        Err("Library not initialized".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_library_at_creates_structure() {
        let tmp = TempDir::new().unwrap();
        let base_path = tmp.path().to_str().unwrap();

        let result = init_library_at(base_path);
        assert!(result.is_ok(), "init_library_at failed: {:?}", result.err());

        // Verify directories
        assert!(tmp.path().join("library/skills").is_dir());
        assert!(tmp.path().join("library/agents").is_dir());
        assert!(tmp.path().join("library/rules").is_dir());
        assert!(tmp.path().join("library/hooks").is_dir());
        assert!(tmp.path().join("library/commands").is_dir());
        assert!(tmp.path().join("backups").is_dir());

        // Verify files
        assert!(tmp.path().join("config.json").is_file());
        assert!(tmp.path().join("projects.json").is_file());
        assert!(tmp.path().join("library/index.json").is_file());

        // Verify config.json content
        let config_content = fs::read_to_string(tmp.path().join("config.json")).unwrap();
        let config: AppConfig = serde_json::from_str(&config_content).unwrap();
        assert_eq!(config.central_library_path, base_path);
        assert_eq!(config.version, "0.1.0");

        // Verify projects.json content
        let projects_content = fs::read_to_string(tmp.path().join("projects.json")).unwrap();
        let registry: ProjectsRegistry = serde_json::from_str(&projects_content).unwrap();
        assert!(registry.projects.is_empty());

        // Verify library/index.json content
        let index_content =
            fs::read_to_string(tmp.path().join("library/index.json")).unwrap();
        let index: LibraryIndex = serde_json::from_str(&index_content).unwrap();
        assert!(index.resources.is_empty());
    }

    #[test]
    fn test_init_library_at_idempotent() {
        let tmp = TempDir::new().unwrap();
        let base_path = tmp.path().to_str().unwrap();

        // Initialize twice should not fail
        init_library_at(base_path).unwrap();
        init_library_at(base_path).unwrap();

        // Files should still be valid
        assert!(tmp.path().join("config.json").is_file());
    }
}
