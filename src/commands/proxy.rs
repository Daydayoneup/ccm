use tauri::State;
use crate::db::Database;
use crate::proxy::ProxyConfig;

#[derive(serde::Deserialize)]
pub struct SaveProxyInput {
    pub enabled: bool,
    pub proxy_type: String,
    pub host: String,
    pub port: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[tauri::command]
pub fn get_proxy_config(db: State<'_, Database>) -> Result<Option<ProxyConfig>, String> {
    // Return config even if disabled (so UI can show saved values)
    let enabled = db.get_setting("proxy_enabled")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let proxy_type = db.get_setting("proxy_type")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let host = db.get_setting("proxy_host")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let port = db.get_setting("proxy_port")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    if host.is_empty() && port.is_empty() && proxy_type.is_empty() {
        return Ok(None);
    }

    let username = db.get_setting("proxy_username")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.is_empty());
    let password = db.get_setting("proxy_password")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.is_empty());

    Ok(Some(ProxyConfig {
        enabled: enabled == "true",
        proxy_type: if proxy_type.is_empty() { "http".to_string() } else { proxy_type },
        host,
        port,
        username,
        password,
    }))
}

#[tauri::command]
pub fn save_proxy_config(db: State<'_, Database>, config: SaveProxyInput) -> Result<(), String> {
    let enabled_str = if config.enabled { "true" } else { "false" };
    db.set_setting("proxy_enabled", enabled_str).map_err(|e| e.to_string())?;
    db.set_setting("proxy_type", &config.proxy_type).map_err(|e| e.to_string())?;
    db.set_setting("proxy_host", &config.host).map_err(|e| e.to_string())?;
    db.set_setting("proxy_port", &config.port).map_err(|e| e.to_string())?;
    db.set_setting("proxy_username", config.username.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;
    db.set_setting("proxy_password", config.password.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn test_proxy(db: State<'_, Database>) -> Result<String, String> {
    let proxy = ProxyConfig::load(&db)
        .ok_or("Proxy is not configured or not enabled")?;

    let url = proxy.to_url();
    tauri::async_runtime::spawn_blocking(move || {
        let output = std::process::Command::new("curl")
            .args([
                "--proxy", &url,
                "-s", "-o", "/dev/null",
                "-w", "%{http_code}",
                "--connect-timeout", "10",
                "--max-time", "15",
                "https://github.com",
            ])
            .output()
            .map_err(|e| format!("Failed to execute curl: {}", e))?;

        let status_code = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if output.status.success() && (status_code.starts_with('2') || status_code.starts_with('3')) {
            Ok(format!("Connection successful (HTTP {})", status_code))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "Connection failed (HTTP {}){}",
                status_code,
                if stderr.is_empty() { String::new() } else { format!(": {}", stderr) }
            ))
        }
    }).await.map_err(|e| e.to_string())?
}
