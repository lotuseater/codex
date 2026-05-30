//! App-server-facing analytics: the protocol-aware reducer and the RPC
//! tracking extension that build on the protocol-free `codex-analytics` crate.
//!
//! `codex-app-server` depends on this crate (in addition to `codex-analytics`
//! and `codex-app-server-protocol`). It constructs the client with
//! [`AppServerReducer`] and brings [`AppServerAnalyticsExt`] into scope to use
//! the RPC `track_*` methods.

mod accepted_lines;
mod client_ext;
mod events;
mod reducer;
mod rpc_fact;

pub use client_ext::AppServerAnalyticsExt;
pub use reducer::AppServerReducer;

pub(crate) fn now_unix_seconds() -> u64 {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn serialize_enum_as_string<T: serde::Serialize>(value: &T) -> Option<String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
}

pub(crate) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(crate) fn option_i64_to_u64(value: Option<i64>) -> Option<u64> {
    value.and_then(|value| u64::try_from(value).ok())
}

#[cfg(test)]
mod analytics_client_tests;
#[cfg(test)]
mod client_tests;
