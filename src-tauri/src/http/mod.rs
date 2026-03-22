pub mod auth;
pub mod models;
pub mod handlers;
pub mod routes;

use std::sync::Arc;
use crate::db::Database;
use tokio::sync::watch;

pub async fn start_server(
    db: Arc<Database>,
    port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), String> {
    let router = routes::create_router(db);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            while !*shutdown_rx.borrow_and_update() {
                if shutdown_rx.changed().await.is_err() {
                    break;
                }
            }
        })
        .await
        .map_err(|e| format!("HTTP server error: {}", e))
}
