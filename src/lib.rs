//! Murder-mystery evidence viewer.

pub mod config;
pub mod healthcheck;
pub mod server;
pub mod state;

pub use config::Config;
pub use server::router;
pub use state::AppState;
