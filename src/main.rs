use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use murder_mystery_evidence::config::DEFAULT_LISTEN;
use murder_mystery_evidence::{Config, healthcheck, server};
use tracing_subscriber::{EnvFilter, fmt};

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// Murder-mystery evidence viewer.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the web server (default).
    Serve,
    /// Probe the running server's /healthz and exit 0 or 1.
    Healthcheck,
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();

    match Cli::parse().command.unwrap_or(Command::Serve) {
        Command::Serve => {
            server::run(Config::from_env()?).await?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Healthcheck => Ok(healthcheck::run(healthcheck_addr()?)),
    }
}

/// Healthcheck target: the configured listen address, which needs no password.
fn healthcheck_addr() -> Result<SocketAddr> {
    let raw = env::var("MM_LISTEN").unwrap_or_else(|_| DEFAULT_LISTEN.to_owned());
    Ok(raw.parse()?)
}
