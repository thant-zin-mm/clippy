//! Server configuration from environment variables.
//!
//! See `docs/coding-guide.md` §11 for the authoritative env-var reference.

use std::net::SocketAddr;

use http::HeaderValue;
use tracing::warn;

pub const DEFAULT_ADDR: &str = "127.0.0.1:7817";
pub const DEFAULT_ORIGINS: &str =
    "http://localhost:5173,http://127.0.0.1:5173,http://localhost:4173,http://127.0.0.1:4173";
pub const DEFAULT_TMP_TTL_SECS: u64 = 3600;
pub const DEFAULT_MAX_UPLOAD_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,
    pub allowed_origins: Vec<String>,
    pub tmp_ttl_secs: u64,
    pub max_upload_bytes: u64,
}

impl Config {
    /// Read configuration from the process environment.
    pub fn from_env() -> Self {
        Self::parse(
            std::env::var("SILENCE_CUTTER_ADDR").ok().as_deref(),
            std::env::var("SILENCE_CUTTER_ORIGINS").ok().as_deref(),
            std::env::var("SILENCE_CUTTER_TMP_TTL_SECS").ok().as_deref(),
            std::env::var("SILENCE_CUTTER_MAX_UPLOAD_BYTES")
                .ok()
                .as_deref(),
        )
    }

    /// Pure parser so tests never mutate process env (see module tests).
    pub fn parse(
        addr: Option<&str>,
        origins: Option<&str>,
        tmp_ttl_secs: Option<&str>,
        max_upload_bytes: Option<&str>,
    ) -> Self {
        let addr = match addr.map(str::trim).filter(|s| !s.is_empty()) {
            Some(raw) => raw.parse::<SocketAddr>().unwrap_or_else(|err| {
                warn!(
                    raw,
                    error = %err,
                    "invalid SILENCE_CUTTER_ADDR; falling back to default"
                );
                DEFAULT_ADDR.parse().expect("default addr is valid")
            }),
            None => DEFAULT_ADDR.parse().expect("default addr is valid"),
        };

        let allowed_origins = parse_origins(origins.unwrap_or(DEFAULT_ORIGINS));

        let tmp_ttl_secs = parse_u64(
            tmp_ttl_secs,
            DEFAULT_TMP_TTL_SECS,
            "SILENCE_CUTTER_TMP_TTL_SECS",
        );
        let max_upload_bytes = parse_u64(
            max_upload_bytes,
            DEFAULT_MAX_UPLOAD_BYTES,
            "SILENCE_CUTTER_MAX_UPLOAD_BYTES",
        );

        Self {
            addr,
            allowed_origins,
            tmp_ttl_secs,
            max_upload_bytes,
        }
    }
}

fn parse_origins(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| {
            let valid = HeaderValue::from_str(s).is_ok();
            if !valid {
                warn!(origin = s, "ignoring invalid CORS origin");
            }
            valid
        })
        .map(str::to_owned)
        .collect()
}

fn parse_u64(raw: Option<&str>, default: u64, var_name: &str) -> u64 {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return default;
    };
    raw.parse::<u64>().unwrap_or_else(|err| {
        warn!(
            raw,
            error = %err,
            var = var_name,
            "invalid value; falling back to default"
        );
        default
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_when_nothing_set() {
        let config = Config::parse(None, None, None, None);
        assert_eq!(config.addr, DEFAULT_ADDR.parse::<SocketAddr>().unwrap());
        assert_eq!(
            config.allowed_origins,
            vec![
                "http://localhost:5173",
                "http://127.0.0.1:5173",
                "http://localhost:4173",
                "http://127.0.0.1:4173"
            ]
        );
        assert_eq!(config.tmp_ttl_secs, DEFAULT_TMP_TTL_SECS);
        assert_eq!(config.max_upload_bytes, DEFAULT_MAX_UPLOAD_BYTES);
    }

    #[test]
    fn parse_overrides() {
        let config = Config::parse(
            Some("127.0.0.1:9999"),
            Some("http://localhost:5173, http://example.test"),
            Some("600"),
            Some("1024"),
        );
        assert_eq!(config.addr.port(), 9999);
        assert_eq!(
            config.allowed_origins,
            vec!["http://localhost:5173", "http://example.test"]
        );
        assert_eq!(config.tmp_ttl_secs, 600);
        assert_eq!(config.max_upload_bytes, 1024);
    }

    #[test]
    fn parse_falls_back_on_invalid_values() {
        // `HeaderValue` allows spaces and Latin-1 obs-text, so the invalid
        // origin uses a DEL control character (0x7F), which it rejects.
        let config = Config::parse(
            Some("not-an-addr"),
            Some("http://ok.test,http://bad\u{7f}.test"),
            Some("not-a-number"),
            Some("-5"),
        );
        assert_eq!(config.addr.port(), 7817);
        assert_eq!(config.allowed_origins, vec!["http://ok.test"]);
        assert_eq!(config.tmp_ttl_secs, DEFAULT_TMP_TTL_SECS);
        assert_eq!(config.max_upload_bytes, DEFAULT_MAX_UPLOAD_BYTES);
    }

    #[test]
    fn parse_ignores_blank_entries() {
        let config = Config::parse(None, Some(" ,http://a.test, ,"), None, None);
        assert_eq!(config.allowed_origins, vec!["http://a.test"]);
    }
}
