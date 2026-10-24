//! Runtime configuration read from `MM_*` environment variables.

use std::env;
use std::net::SocketAddr;

use anyhow::{Context, Result, ensure};

/// Bind address used when `MM_LISTEN` is unset.
pub const DEFAULT_LISTEN: &str = "0.0.0.0:8080";
const DEFAULT_INITIAL_ROUND: u8 = 1;
const ROUNDS: std::ops::RangeInclusive<u8> = 1..=4;

/// Validated server configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP server binds to (`MM_LISTEN`).
    pub listen: SocketAddr,
    /// Password for the admin controls (`MM_ADMIN_PASSWORD`).
    pub admin_password: String,
    /// Round the game starts in (`MM_INITIAL_ROUND`, 1..=4).
    pub initial_round: u8,
}

impl Config {
    /// Reads the configuration from the process environment.
    pub fn from_env() -> Result<Self> {
        Self::from_vars(|name| env::var(name).ok())
    }

    /// Builds the configuration from an arbitrary variable lookup.
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let listen = get("MM_LISTEN")
            .unwrap_or_else(|| DEFAULT_LISTEN.to_owned())
            .parse()
            .context("MM_LISTEN must be a socket address like 0.0.0.0:8080")?;
        let admin_password = get("MM_ADMIN_PASSWORD").context("MM_ADMIN_PASSWORD must be set")?;
        let initial_round = get("MM_INITIAL_ROUND")
            .map(|raw| raw.parse::<u8>())
            .transpose()
            .context("MM_INITIAL_ROUND must be a number")?
            .unwrap_or(DEFAULT_INITIAL_ROUND);
        ensure!(
            ROUNDS.contains(&initial_round),
            "MM_INITIAL_ROUND must be between 1 and 4, got {initial_round}"
        );
        Ok(Self {
            listen,
            admin_password,
            initial_round,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn with_password(extra: Option<(&'static str, &'static str)>) -> Result<Config> {
        Config::from_vars(|k| match (k, extra) {
            ("MM_ADMIN_PASSWORD", _) => Some("pw".into()),
            (_, Some((name, value))) if k == name => Some(value.into()),
            _ => None,
        })
    }

    #[test]
    fn missing_password_is_an_error() {
        assert!(Config::from_vars(|_| None).is_err());
    }

    #[test]
    fn defaults_apply() {
        let c = with_password(None).unwrap();
        assert_eq!(c.listen, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(c.admin_password, "pw");
        assert_eq!(c.initial_round, 1);
    }

    #[test]
    fn overrides_apply() {
        let c = with_password(Some(("MM_LISTEN", "127.0.0.1:9000"))).unwrap();
        assert_eq!(c.listen, "127.0.0.1:9000".parse().unwrap());
        let c = with_password(Some(("MM_INITIAL_ROUND", "4"))).unwrap();
        assert_eq!(c.initial_round, 4);
    }

    #[test]
    fn initial_round_out_of_range_is_an_error() {
        for bad in ["0", "5", "x"] {
            assert!(
                with_password(Some(("MM_INITIAL_ROUND", bad))).is_err(),
                "{bad}"
            );
        }
    }
}
