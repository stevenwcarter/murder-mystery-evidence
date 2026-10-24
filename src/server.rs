//! HTTP router and server lifecycle.

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router, middleware};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::signal::{self, unix::SignalKind};
use tracing::info;

use crate::assets::evidence_file;
use crate::auth::require_admin;
use crate::config::Config;
use crate::events::events;
use crate::evidence::Round;
use crate::state::AppState;

/// Body of `POST /admin/round`.
#[derive(Deserialize)]
struct RoundForm {
    round: u8,
}

/// Builds the application router.
pub fn router(state: AppState) -> Router {
    let admin = Router::new()
        .route("/admin", get(admin_page))
        .route("/admin/round", post(set_round))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_admin));
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/evidence/{slug}", get(evidence_file))
        .route("/events", get(events))
        .merge(admin)
        .with_state(state)
}

/// Placeholder admin page showing the current round.
async fn admin_page(State(state): State<AppState>) -> String {
    format!("round {}", state.current())
}

/// Moves the game to the submitted round, or answers `400` if it is out of range.
async fn set_round(State(state): State<AppState>, Form(form): Form<RoundForm>) -> Response {
    match Round::new(form.round) {
        Some(round) => {
            state.set(round);
            Redirect::to("/admin").into_response()
        }
        None => (StatusCode::BAD_REQUEST, "round must be 1 to 4").into_response(),
    }
}

/// Serves until ctrl-c or SIGTERM.
pub async fn run(config: Config) -> Result<()> {
    let round = Round::new(config.initial_round).context("initial round out of range")?;
    let app = router(AppState::with_round(&config.admin_password, round));
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
