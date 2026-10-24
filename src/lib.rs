//! Murder-mystery evidence viewer.

mod assets;
mod auth;
pub mod config;
mod events;
pub mod evidence;
pub mod healthcheck;
pub mod pages;
pub mod server;
pub mod state;

pub use config::Config;
pub use server::router;
pub use state::AppState;
