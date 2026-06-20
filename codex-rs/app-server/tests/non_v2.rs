// Independent integration test binary for non-v2 app-server domains.
#[path = "suite/auth.rs"]
mod auth;
#[path = "suite/conversation_summary.rs"]
mod conversation_summary;
#[path = "suite/fuzzy_file_search.rs"]
mod fuzzy_file_search;
#[path = "suite/strict_config.rs"]
mod strict_config;
