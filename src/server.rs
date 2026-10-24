//! HTTP router and server lifecycle.

use anyhow::{Context, Result};
use axum::extract::State;
use axum::extract::rejection::FormRejection;
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, HeaderName};
use axum::response::Redirect;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router, middleware};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::signal::{self, unix::SignalKind};
use tower_http::compression::CompressionLayer;
use tracing::info;

use crate::assets::{evidence_file, static_asset};
use crate::auth::require_admin;
use crate::config::Config;
use crate::events::events;
use crate::evidence::Round;
use crate::pages;
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
        .route("/", get(guest_page))
        .route("/healthz", get(|| async { "ok" }))
        .route("/evidence/{slug}", get(evidence_file))
        .route("/events", get(events))
        .route("/assets/{file}", get(static_asset))
        .merge(admin)
        .with_state(state)
        // The default predicate skips `text/event-stream` (and images), so SSE is never buffered.
        .layer(CompressionLayer::new())
}

/// Both pages show the live round, so neither may be cached.
const NO_STORE: [(HeaderName, &str); 1] = [(CACHE_CONTROL, "no-store")];

/// `GET /`: the guest page for the current round.
async fn guest_page(State(state): State<AppState>) -> impl IntoResponse {
    (NO_STORE, pages::guest(state.current()))
}

/// `GET /admin`: the host's round controls.
async fn admin_page(State(state): State<AppState>) -> impl IntoResponse {
    (NO_STORE, pages::admin(state.current()))
}

/// Moves the game to the submitted round, or answers `400` if it is out of range.
async fn set_round(
    State(state): State<AppState>,
    form: Result<Form<RoundForm>, FormRejection>,
) -> Response {
    let Ok(Form(form)) = form else {
        return bad_round();
    };
    match Round::new(form.round) {
        Some(round) => {
            state.set(round);
            Redirect::to("/admin").into_response()
        }
        None => bad_round(),
    }
}

fn bad_round() -> Response {
    (StatusCode::BAD_REQUEST, "round must be 1 to 4").into_response()
}

/// Serves until ctrl-c or SIGTERM.
pub async fn run(config: Config) -> Result<()> {
    let round = Round::new(config.initial_round).context("initial round out of range")?;
    let state = AppState::with_round(&config.admin_password, round);
    let app = router(state.clone());
    let listener = TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;
    info!(address = %config.listen, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            state.shutdown();
        })
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
