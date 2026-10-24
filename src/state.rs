//! Shared application state.

use std::sync::Arc;

/// State cloned into every request handler.
#[derive(Clone)]
pub struct AppState {
    /// Password required by the admin controls.
    pub admin_password: Arc<str>,
}

impl AppState {
    /// Creates the state around the admin password.
    pub fn new(admin_password: &str) -> Self {
        Self {
            admin_password: admin_password.into(),
        }
    }
}
