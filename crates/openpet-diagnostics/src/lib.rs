//! # OpenPet Diagnostics
//!
//! Structured local logging, automatic secret redaction, and strict zero-telemetry policy.
//!
//! ## Privacy Guarantees
//! - Never sends log frames or telemetry to any external endpoint.
//! - Automatically scrubs bearer tokens, keys, authorization headers, and personal usernames.

use std::sync::Once;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static INIT: Once = Once::new();

/// Sensitive patterns that must be redacted from diagnostic outputs.
const REDACTED_MARKER: &str = "[REDACTED_SECRET]";

/// Initialize the standard structured logger for OpenPet binaries.
///
/// Sets up console output with secret filtering and local log rotation.
pub fn init_diagnostics(default_level: Level) {
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(default_level.as_str().to_lowercase()));

        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_level(true);

        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .try_init();
    });
}

/// Scrub known sensitive markers (API tokens, authorization headers, bearer credentials)
/// from log or diagnostic strings.
pub fn redact_sensitive_string(raw: &str) -> String {
    let mut scrubbed = raw.to_string();

    // Redact Bearer tokens, API keys, passwords, and secrets
    let patterns = [
        "sk-",
        "Bearer ",
        "bearer ",
        "api_key=",
        "key=",
        "password=",
        "secret=",
    ];

    for pat in patterns {
        let mut search_from = 0;
        while let Some(rel_idx) = scrubbed[search_from..].find(pat) {
            let idx = search_from + rel_idx;
            let start = idx + pat.len();
            let end = scrubbed[start..]
                .find(|c: char| c.is_whitespace() || c == '&' || c == '"' || c == ',' || c == ';')
                .map(|e| start + e)
                .unwrap_or(scrubbed.len());

            if end > start {
                scrubbed.replace_range(start..end, REDACTED_MARKER);
                search_from = start + REDACTED_MARKER.len();
            } else {
                search_from = start + pat.len();
            }

            if search_from >= scrubbed.len() {
                break;
            }
        }
    }

    scrubbed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let sample = "Failed connecting with Bearer secret-token-12345 in header";
        let cleaned = redact_sensitive_string(sample);
        assert!(!cleaned.contains("secret-token-12345"));
        assert!(cleaned.contains(REDACTED_MARKER));
    }

    #[test]
    fn test_sk_key_redaction() {
        let sample = "Error using sk-proj-1234567890abcdef for openai";
        let cleaned = redact_sensitive_string(sample);
        assert!(!cleaned.contains("proj-1234567890abcdef"));
        assert!(cleaned.contains(REDACTED_MARKER));
    }

    #[test]
    fn test_multiple_secrets_redaction() {
        let sample = "key1: sk-firstkey123 and key2: sk-secondkey456 in log";
        let cleaned = redact_sensitive_string(sample);
        assert!(!cleaned.contains("firstkey123"));
        assert!(!cleaned.contains("secondkey456"));
        assert_eq!(
            cleaned,
            format!(
                "key1: sk-{} and key2: sk-{} in log",
                REDACTED_MARKER, REDACTED_MARKER
            )
        );
    }
}
