//! Shared application state.

use std::sync::Arc;

use tokio::sync::watch;

use crate::evidence::Round;

/// State cloned into every request handler.
#[derive(Clone)]
pub struct AppState {
    /// Current round; receivers learn about every change.
    round: Arc<watch::Sender<Round>>,
    /// Password required by the admin controls.
    pub admin_password: Arc<str>,
}

impl AppState {
    /// Creates the state in the first round around the admin password.
    pub fn new(admin_password: &str) -> Self {
        Self::with_round(admin_password, Round::MIN)
    }

    /// Creates the state starting in `round`.
    pub fn with_round(admin_password: &str, round: Round) -> Self {
        Self {
            round: Arc::new(watch::Sender::new(round)),
            admin_password: admin_password.into(),
        }
    }

    /// The round currently in play.
    pub fn current(&self) -> Round {
        *self.round.borrow()
    }

    /// Moves the game to `round` and notifies subscribers.
    pub fn set(&self, round: Round) {
        self.round.send_replace(round);
    }

    /// A receiver that sees the current round and every later change.
    pub fn subscribe(&self) -> watch::Receiver<Round> {
        self.round.subscribe()
    }
}
