use serde::Serialize;
use std::sync::Arc;
use crate::db::Database;

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { ok: true, data: Some(data), error: None }
    }
}

impl ApiResponse<()> {
    pub fn error(msg: impl Into<String>) -> ApiResponse<()> {
        ApiResponse { ok: false, data: None, error: Some(msg.into()) }
    }
}

#[derive(Serialize)]
pub struct HealthData {
    pub version: String,
}

#[derive(Serialize)]
pub struct ProjectListItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub language: Option<String>,
    pub pinned: bool,
    pub launch_count: i32,
}

#[derive(Serialize)]
pub struct ProjectDetail {
    pub id: String,
    pub name: String,
    pub path: String,
    pub language: Option<String>,
    pub last_scanned: Option<String>,
    pub pinned: bool,
    pub launch_count: i32,
}

/// Shared state for axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}
