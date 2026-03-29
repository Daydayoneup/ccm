use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::db::Database;
use super::models::*;

#[derive(serde::Deserialize)]
pub struct ProjectsQuery {
    pub q: Option<String>,
}

/// Query projects with optional search filter. Used by handler and tests.
pub fn query_projects(db: &Database, search: Option<&str>) -> Result<Vec<ProjectListItem>, String> {
    let projects = db.list_projects_ranked().map_err(|e| e.to_string())?;
    let filtered: Vec<_> = match search {
        Some(q) if !q.is_empty() => {
            let q_lower = q.to_lowercase();
            projects.into_iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&q_lower)
                        || p.path.to_lowercase().contains(&q_lower)
                })
                .collect()
        }
        _ => projects,
    };
    Ok(filtered.into_iter().map(|p| ProjectListItem {
        id: p.id,
        name: p.name,
        path: p.path,
        language: p.language,
        pinned: p.pinned != 0,
        launch_count: p.launch_count,
    }).collect())
}

/// Get a single project's full detail. Used by handler and tests.
pub fn get_project_detail(db: &Database, id: &str) -> Result<Option<ProjectDetail>, String> {
    let project = db.get_project(id).map_err(|e| e.to_string())?;
    Ok(project.map(|p| ProjectDetail {
        id: p.id,
        name: p.name,
        path: p.path,
        language: p.language,
        last_scanned: p.last_scanned,
        pinned: p.pinned != 0,
        launch_count: p.launch_count,
    }))
}

// --- Axum handlers ---

pub async fn health_handler() -> impl IntoResponse {
    Json(ApiResponse::success(HealthData {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

pub async fn list_projects_handler(
    State(state): State<AppState>,
    Query(params): Query<ProjectsQuery>,
) -> impl IntoResponse {
    match query_projects(&state.db, params.q.as_deref()) {
        Ok(items) => (StatusCode::OK, Json(ApiResponse::success(items))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e)),
        ).into_response(),
    }
}

pub async fn get_project_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match get_project_detail(&state.db, &id) {
        Ok(Some(detail)) => (StatusCode::OK, Json(ApiResponse::success(detail))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("Project not found")),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e)),
        ).into_response(),
    }
}

pub async fn launch_project_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let project = match state.db.get_project(&id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("Project not found")),
            ).into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(e.to_string())),
            ).into_response();
        }
    };

    let db = state.db.clone();
    let path = project.path.clone();
    let pid = project.id.clone();
    match tokio::task::spawn_blocking(move || {
        crate::commands::shell::launch_claude_core(&db, &path, Some(&pid))
    }).await {
        Ok(Ok(())) => (StatusCode::OK, Json(ApiResponse::success(()))).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e)),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        ).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::v2::Project;
    use crate::http::models::AppState;
    use std::sync::Arc;

    fn test_state() -> AppState {
        let db = Database::new_in_memory().unwrap();
        AppState { db: Arc::new(db) }
    }

    fn insert_project(state: &AppState, id: &str, name: &str, path: &str) {
        let db: &Database = &state.db;
        db.insert_project(&Project {
            id: id.to_string(),
            name: name.to_string(),
            path: path.to_string(),
            language: Some("Rust".to_string()),
            last_scanned: Some("2026-03-22T00:00:00Z".to_string()),
            pinned: 0,
            launch_count: 3,
        }).unwrap();
    }

    #[test]
    fn test_list_projects_empty() {
        let state = test_state();
        let items = query_projects(&state.db, None).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_list_projects_returns_all() {
        let state = test_state();
        insert_project(&state, "p1", "alpha", "/tmp/alpha");
        insert_project(&state, "p2", "bravo", "/tmp/bravo");
        let items = query_projects(&state.db, None).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_list_projects_search_by_name() {
        let state = test_state();
        insert_project(&state, "p1", "alpha", "/tmp/alpha");
        insert_project(&state, "p2", "bravo", "/tmp/bravo");
        let items = query_projects(&state.db, Some("alph")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "alpha");
    }

    #[test]
    fn test_list_projects_search_by_path() {
        let state = test_state();
        insert_project(&state, "p1", "alpha", "/home/user/alpha");
        insert_project(&state, "p2", "bravo", "/tmp/bravo");
        let items = query_projects(&state.db, Some("/home/")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "alpha");
    }

    #[test]
    fn test_list_projects_search_case_insensitive() {
        let state = test_state();
        insert_project(&state, "p1", "Alpha", "/tmp/alpha");
        let items = query_projects(&state.db, Some("alpha")).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_list_projects_pinned_as_bool() {
        let state = test_state();
        insert_project(&state, "p1", "alpha", "/tmp/alpha");
        let items = query_projects(&state.db, None).unwrap();
        assert!(!items[0].pinned);
    }

    #[test]
    fn test_get_project_detail_found() {
        let state = test_state();
        insert_project(&state, "p1", "alpha", "/tmp/alpha");
        let detail = get_project_detail(&state.db, "p1").unwrap();
        assert!(detail.is_some());
        let d = detail.unwrap();
        assert_eq!(d.name, "alpha");
        assert!(!d.pinned);
    }

    #[test]
    fn test_get_project_detail_not_found() {
        let state = test_state();
        let detail = get_project_detail(&state.db, "nope").unwrap();
        assert!(detail.is_none());
    }
}
