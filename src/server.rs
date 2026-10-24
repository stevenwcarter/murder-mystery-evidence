//! HTTP router and server lifecycle.

use anyhow::{Context, Result};
use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::signal::{self, unix::SignalKind};
use tracing::info;

use crate::config::Config;
use crate::state::AppState;

/// Builds the application router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state)
}

/// Serves until ctrl-c or SIGTERM.
pub async fn run(config: Config) -> Result<()> {
    let app = router(AppState::new(&config.admin_password));
    let listener = TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;
    info!(address = %config.listen, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };
    let sigterm = async {
        if let Ok(mut term) = signal::unix::signal(SignalKind::terminate()) {
            term.recv().await;
        }
    };
    tokio::select! {
        () = ctrl_c => {},
        () = sigterm => {},
    }
    info!("shutting down");
}
