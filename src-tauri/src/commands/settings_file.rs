use std::fs;
use std::path::PathBuf;
use serde::Serialize;

/// Expand ~ to home directory
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

/// Read a settings JSON file. Returns {} if file doesn't exist.
#[tauri::command]
pub fn read_settings_file(path: String) -> Result<serde_json::Value, String> {
    let p = expand_tilde(&path);
    if !p.is_file() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(&p)
        .map_err(|e| format!("Failed to read {}: {}", p.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in {}: {}", p.display(), e))
}

/// Write a JSON value to a settings file with 4-space pretty print.
#[tauri::command]
pub fn write_settings_file(path: String, content: serde_json::Value) -> Result<(), String> {
    let p = expand_tilde(&path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    content.serialize(&mut ser)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    fs::write(&p, buf)
        .map_err(|e| format!("Failed to write {}: {}", p.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_read_settings_file_not_found_returns_empty_object() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.json").to_string_lossy().to_string();
        let result = read_settings_file(path).unwrap();
        assert_eq!(result, serde_json::json!({}));
    }

    #[test]
    fn test_read_settings_file_valid_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(&path, r#"{"model":"claude-opus-4-6","permissions":{"allow":["Bash(git *)"]}}"#).unwrap();
        let result = read_settings_file(path.to_string_lossy().to_string()).unwrap();
        assert_eq!(result["model"], "claude-opus-4-6");
        assert_eq!(result["permissions"]["allow"][0], "Bash(git *)");
    }

    #[test]
    fn test_read_settings_file_invalid_json_returns_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.json");
        fs::write(&path, "not json{{{").unwrap();
        let result = read_settings_file(path.to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_write_settings_file_creates_parents_and_writes() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("a/b/.claude/settings.json").to_string_lossy().to_string();
        let content = serde_json::json!({"model": "claude-sonnet-4-6"});
        write_settings_file(path.clone(), content).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("claude-sonnet-4-6"));
        assert!(raw.contains("    ")); // 4-space indent
    }

    #[test]
    fn test_write_settings_file_overwrites_existing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(&path, r#"{"old": true}"#).unwrap();

        let content = serde_json::json!({"new": true});
        write_settings_file(path.to_string_lossy().to_string(), content).unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"new\""));
        assert!(!raw.contains("\"old\""));
    }

    #[test]
    fn test_read_settings_file_expands_tilde() {
        let result = read_settings_file("~/.claude/nonexistent-test-file-12345.json".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), serde_json::json!({}));
    }
}
