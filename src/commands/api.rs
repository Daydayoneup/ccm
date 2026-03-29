use crate::db::Database;
use crate::http::auth::{generate_token, hash_token};
use tauri::State;
use tokio::sync::watch;
use std::sync::Arc;

/// Generate a new API token. Returns the raw token (shown once to user).
/// Stores only the SHA-256 hash in the database.
#[tauri::command]
pub fn generate_api_token(db: State<'_, Database>) -> Result<String, String> {
    let token = generate_token();
    let hash = hash_token(&token);
    db.set_setting("api_token_hash", &hash).map_err(|e| e.to_string())?;
    let last4 = &token[token.len() - 4..];
    db.set_setting("api_token_last4", last4).map_err(|e| e.to_string())?;
    Ok(token)
}

/// Get token display status: returns last 4 chars if token exists, or null.
#[tauri::command]
pub fn get_api_token_status(db: State<'_, Database>) -> Result<Option<String>, String> {
    db.get_setting("api_token_last4").map_err(|e| e.to_string())
}

/// Toggle HTTP API server at runtime (no restart needed).
#[tauri::command]
pub async fn toggle_api_server(
    db: State<'_, Database>,
    shutdown_tx: State<'_, watch::Sender<bool>>,
    enabled: bool,
) -> Result<(), String> {
    db.set_setting("api_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;

    if enabled {
        let port: u16 = db.get_setting("api_port")
            .unwrap_or(None)
            .and_then(|p| p.parse().ok())
            .unwrap_or(23890);

        let _ = shutdown_tx.send(false);
        let db_arc = Arc::new((*db).clone());
        let rx = shutdown_tx.subscribe();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = crate::http::start_server(db_arc, port, rx).await {
                eprintln!("HTTP API server error: {}", e);
            }
        });
    } else {
        let _ = shutdown_tx.send(true);
    }
    Ok(())
}
