use axum::{
    Router,
    routing::{get, post},
    middleware::{self, Next},
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use super::handlers::*;
use super::models::{ApiResponse, AppState};
use super::auth::verify_token;
use std::sync::Arc;
use crate::db::Database;

async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let api_enabled = state.db.get_setting("api_enabled")
        .unwrap_or(None)
        .unwrap_or_default();
    if api_enabled != "true" {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<()>::error("API is not enabled")),
        ).into_response();
    }

    let stored_hash = match state.db.get_setting("api_token_hash") {
        Ok(Some(h)) => h,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::<()>::error("No API token configured. Generate one in CCM Settings.")),
            ).into_response();
        }
    };

    let auth_header = request.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let token = if let Some(t) = auth_header.strip_prefix("Bearer ") {
        t
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("Missing or invalid Authorization header")),
        ).into_response();
    };

    if !verify_token(token, &stored_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("Invalid token")),
        ).into_response();
    }

    next.run(request).await.into_response()
}

pub fn create_router(db: Arc<Database>) -> Router {
    let state = AppState { db: db.clone() };

    let protected = Router::new()
        .route("/api/projects", get(list_projects_handler))
        .route("/api/projects/{id}", get(get_project_handler))
        .route("/api/projects/{id}/launch", post(launch_project_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state.clone());

    let public = Router::new()
        .route("/api/health", get(health_handler));

    public.merge(protected)
}
