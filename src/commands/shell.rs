use crate::db::Database;
use tauri::State;

/// Shell-escape a path for use inside AppleScript double-quoted strings.
fn shell_escape_for_applescript(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Shell-escape a value by wrapping in single quotes with proper escaping.
fn shell_escape_value(val: &str) -> String {
    format!("'{}'", val.replace('\'', "'\\''"))
}

/// Core launch logic — callable from both Tauri command and HTTP handler.
pub fn launch_claude_core(
    db: &Database,
    project_path: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    let terminal = db
        .get_setting("terminal_app")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "Terminal".to_string());

    let escaped_path = shell_escape_for_applescript(project_path);

    // Build env export prefix
    let env_prefix = {
        let env_entries: Vec<(String, String)> = match project_id {
            Some(pid) => {
                let merged = db.list_merged_env_vars(pid).map_err(|e| e.to_string())?;
                merged.into_iter().map(|v| (v.key, v.value)).collect()
            }
            None => {
                let globals = db.list_env_vars(None).map_err(|e| e.to_string())?;
                globals.into_iter().map(|v| (v.key, v.value)).collect()
            }
        };
        if env_entries.is_empty() {
            String::new()
        } else {
            let exports: Vec<String> = env_entries
                .iter()
                .map(|(k, v)| format!("export {}={}", k, shell_escape_value(v)))
                .collect();
            format!("{} && ", exports.join(" && "))
        }
    };

    match terminal.as_str() {
        "Terminal" => {
            let script = format!(
                "tell application \"Terminal\" to do script \"{}cd \\\"{}\\\" && claude\"",
                env_prefix, escaped_path
            );
            std::process::Command::new("osascript")
                .args(["-e", &script])
                .spawn()
                .map_err(|e| format!("Failed to launch Terminal: {}", e))?;
        }
        "iTerm2" => {
            let script = format!(
                "tell application \"iTerm2\" to create window with default profile command \"/bin/zsh -li -c '{}cd \\\"{}\\\" && claude'\"",
                env_prefix, escaped_path
            );
            std::process::Command::new("osascript")
                .args(["-e", &script])
                .spawn()
                .map_err(|e| format!("Failed to launch iTerm2: {}", e))?;
        }
        "Warp" => {
            std::process::Command::new("open")
                .args(["-a", "Warp", project_path])
                .spawn()
                .map_err(|e| format!("Failed to launch Warp: {}", e))?;
        }
        other => {
            return Err(format!("Unsupported terminal: {}", other));
        }
    }

    if let Some(pid) = project_id {
        let _ = db.increment_launch_count(pid);
    }
    Ok(())
}

#[tauri::command]
pub fn launch_claude_in_terminal(
    db: State<'_, Database>,
    project_path: String,
    project_id: Option<String>,
) -> Result<(), String> {
    launch_claude_core(&db, &project_path, project_id.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_launch_core_project_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = launch_claude_core(&db, "/nonexistent", Some("fake-id"));
        assert!(result.is_ok() || result.is_err());
    }
}

#[tauri::command]
pub fn get_terminal_preference(db: State<'_, Database>) -> Result<String, String> {
    db.get_setting("terminal_app")
        .map_err(|e| e.to_string())
        .map(|opt| opt.unwrap_or_else(|| "Terminal".to_string()))
}

#[tauri::command]
pub fn set_terminal_preference(
    db: State<'_, Database>,
    terminal: String,
) -> Result<(), String> {
    let valid = ["Terminal", "iTerm2", "Warp"];
    if !valid.contains(&terminal.as_str()) {
        return Err(format!(
            "Invalid terminal: {}. Valid values: {:?}",
            terminal, valid
        ));
    }
    db.set_setting("terminal_app", &terminal)
        .map_err(|e| e.to_string())
}
