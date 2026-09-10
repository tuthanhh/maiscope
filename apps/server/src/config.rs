//! Startup configuration, read once from the environment.
//!
//! Every value the server needs to boot lives here so that a misconfigured
//! deploy fails immediately with a message naming the variable, rather than a
//! bare `unwrap` panic somewhere in `main`.
//!
//! Config holds *data* only — no `tower` or `tracing` types. Ticket 06 turns
//! `cors_allowed_origins` into a `CorsLayer`; ticket 05 feeds `log_filter` to
//! `tracing_subscriber::EnvFilter`.

use std::fmt;

/// Bind port when `PORT` is unset — keeps `cargo run` unchanged locally.
/// Fly injects `PORT` itself, so production never takes this branch.
const DEFAULT_PORT: u16 = 3000;

/// Pool size when `DATABASE_MAX_CONNECTIONS` is unset. Matches the value
/// local docker Postgres was hardcoded to before this was configurable.
/// Neon's free tier caps total connections low enough that this needs to
/// shrink for production — see the ticket note.
const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 4;

/// Default log directives when `RUST_LOG` is unset.
const DEFAULT_LOG_FILTER: &str = "info,tower_http=info";

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub port: u16,
    /// Exact `Origin` values allowed to call the API cross-origin. Empty means
    /// "no allowlist configured" — see `from_env` for what that implies.
    pub cors_allowed_origins: Vec<String>,
    /// `tracing_subscriber::EnvFilter` directive string (ticket 05).
    pub log_filter: String,
}

#[derive(Debug, PartialEq)]
pub enum ConfigError {
    /// Required variable absent from the environment.
    Missing { var: &'static str },
    /// Variable present but unusable — bad number, non-UTF-8 bytes, etc.
    Invalid { var: &'static str, reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Missing { var } => {
                write!(f, "{var} is not set")
            }
            ConfigError::Invalid { var, reason } => {
                write!(f, "{var} is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Reads a required variable. Non-UTF-8 is reported as `Invalid`, not
/// `Missing`, so the operator knows the variable *is* set but unreadable.
fn required(var: &'static str) -> Result<String, ConfigError> {
    match std::env::var(var) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Err(ConfigError::Missing { var }),
        Err(std::env::VarError::NotUnicode(_)) => Err(ConfigError::Invalid {
            var,
            reason: "value is not valid UTF-8".to_string(),
        }),
    }
}

/// Reads an optional variable. Non-UTF-8 is still an error; a missing variable
/// yields `None` so the caller can apply its own default.
fn optional(var: &'static str) -> Result<Option<String>, ConfigError> {
    match std::env::var(var) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(ConfigError::Invalid {
            var,
            reason: "value is not valid UTF-8".to_string(),
        }),
    }
}

/// `None` (unset) defaults to [`DEFAULT_PORT`]; anything present must parse.
fn parse_port(raw: Option<String>) -> Result<u16, ConfigError> {
    match raw {
        Some(raw) => raw.parse::<u16>().map_err(|e| ConfigError::Invalid {
            var: "PORT",
            reason: format!("{raw:?} is not a valid port number ({e})"),
        }),
        None => Ok(DEFAULT_PORT),
    }
}

/// `None` (unset) defaults to [`DEFAULT_DATABASE_MAX_CONNECTIONS`]; anything
/// present must parse.
fn parse_database_max_connections(raw: Option<String>) -> Result<u32, ConfigError> {
    match raw {
        Some(raw) => raw.parse::<u32>().map_err(|e| ConfigError::Invalid {
            var: "DATABASE_MAX_CONNECTIONS",
            reason: format!("{raw:?} is not a valid connection count ({e})"),
        }),
        None => Ok(DEFAULT_DATABASE_MAX_CONNECTIONS),
    }
}

/// Comma-separated exact Origin values, e.g.
/// "https://maiscope.pages.dev,https://staging.maiscope.pages.dev". An empty
/// input (unset `CORS_ALLOWED_ORIGINS`) yields an empty list — ticket 06
/// decides what that implies (fail closed vs. permissive fallback for local
/// dev).
fn parse_cors_origins(raw: String) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

impl Config {
    /// Loads configuration from the process environment. Call once, in `main`,
    /// after `dotenvy::dotenv()`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = required("DATABASE_URL")?;
        let database_max_connections =
            parse_database_max_connections(optional("DATABASE_MAX_CONNECTIONS")?)?;
        let port = parse_port(optional("PORT")?)?;
        let log_filter = optional("RUST_LOG")?.unwrap_or_else(|| DEFAULT_LOG_FILTER.to_string());
        let cors_allowed_origins =
            parse_cors_origins(optional("CORS_ALLOWED_ORIGINS")?.unwrap_or_default());

        Ok(Config {
            database_url,
            database_max_connections,
            port,
            cors_allowed_origins,
            log_filter,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn missing_error_names_the_variable() {
        let err = ConfigError::Missing { var: "DATABASE_URL" };
        assert_eq!(err.to_string(), "DATABASE_URL is not set");
    }

    #[test]
    fn invalid_error_names_variable_and_reason() {
        let err = ConfigError::Invalid {
            var: "PORT",
            reason: "boom".to_string(),
        };
        assert_eq!(err.to_string(), "PORT is invalid: boom");
    }

    #[test]
    fn parse_port_defaults_when_unset() {
        assert_eq!(parse_port(None), Ok(DEFAULT_PORT));
    }

    #[test]
    fn parse_port_rejects_non_numeric_value() {
        let err = parse_port(Some("not-a-port".to_string())).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { var: "PORT", .. }));
        assert!(err.to_string().contains("not a valid port number"));
    }

    #[test]
    fn parse_database_max_connections_defaults_when_unset() {
        assert_eq!(
            parse_database_max_connections(None),
            Ok(DEFAULT_DATABASE_MAX_CONNECTIONS)
        );
    }

    #[test]
    fn parse_database_max_connections_rejects_non_numeric_value() {
        let err = parse_database_max_connections(Some("lots".to_string())).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Invalid {
                var: "DATABASE_MAX_CONNECTIONS",
                ..
            }
        ));
        assert!(err.to_string().contains("not a valid connection count"));
    }

    #[test]
    fn parse_cors_origins_trims_and_drops_empties() {
        let origins = parse_cors_origins(" https://a.example, https://b.example ,,".to_string());
        assert_eq!(origins, vec!["https://a.example", "https://b.example"]);
    }

    #[test]
    fn parse_cors_origins_empty_string_yields_empty_vec() {
        assert!(parse_cors_origins(String::new()).is_empty());
    }

    // `required`/`optional` read the real process environment, which is
    // shared with the `#[sqlx::test]` handler tests in main.rs running
    // concurrently in the same process. Using a var name nothing else reads,
    // plus a private lock so these two tests don't race each other, keeps
    // this from ever touching DATABASE_URL/PORT/etc.
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    const TEST_VAR: &str = "MAISCOPE_CONFIG_TEST_ONLY_VAR";

    #[test]
    fn required_returns_the_value_when_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var(TEST_VAR, "value") };
        assert_eq!(required(TEST_VAR).as_deref(), Ok("value"));
        unsafe { std::env::remove_var(TEST_VAR) };
    }

    #[test]
    fn optional_returns_none_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var(TEST_VAR) };
        assert_eq!(optional(TEST_VAR), Ok(None));
    }
}
