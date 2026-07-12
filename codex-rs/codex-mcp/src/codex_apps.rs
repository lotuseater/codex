//! Codex Apps support for the host-owned apps MCP server.
//!
//! This module owns the normalization that turns ChatGPT-hosted app
//! connector/tool metadata into model-visible MCP callable names.

use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use codex_utils_plugins::mcp_connector::sanitize_name;
use serde::Deserialize;
use sha1::Digest;
use sha1::Sha1;

use crate::codex_apps_cache::CodexAppsToolsCacheKey;
use crate::tools::ToolInfo;

mod file_params;

pub use file_params::declared_openai_file_input_param_names;
pub(crate) use file_params::prepare_openai_file_params_for_model;

pub(crate) fn normalize_codex_apps_tool_title(connector_name: Option<&str>, value: &str) -> String {
    let Some(connector_name) = connector_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
    else {
        return value.to_string();
    };

    let prefix = format!("{connector_name}_");
    if let Some(stripped) = value.strip_prefix(&prefix)
        && !stripped.is_empty()
    {
        return stripped.to_string();
    }

    value.to_string()
}

pub(crate) fn normalize_codex_apps_callable_name(
    tool_name: &str,
    connector_id: Option<&str>,
    connector_name: Option<&str>,
) -> String {
    let tool_name = sanitize_name(tool_name);

    if let Some(connector_name) = connector_name
        .map(str::trim)
        .map(sanitize_name)
        .filter(|name| !name.is_empty())
        && let Some(stripped) = tool_name.strip_prefix(&connector_name)
        && !stripped.is_empty()
    {
        return stripped.to_string();
    }

    if let Some(connector_id) = connector_id
        .map(str::trim)
        .map(sanitize_name)
        .filter(|name| !name.is_empty())
        && let Some(stripped) = tool_name.strip_prefix(&connector_id)
        && !stripped.is_empty()
    {
        return stripped.to_string();
    }

    tool_name
}

pub(crate) fn normalize_codex_apps_callable_namespace(
    server_name: &str,
    connector_name: Option<&str>,
) -> String {
    if let Some(connector_name) = connector_name {
        format!("{}__{}", server_name, sanitize_name(connector_name))
    } else {
        server_name.to_string()
    }
}

// fork-local: Codex Apps tools cache status inspector backing the fork's
// `McpCacheStatus` app-server request. Upstream's 7th-cycle refactor moved the
// live tool cache into `codex_apps_cache`; this read-only on-disk probe mirrors
// that module's disk layout (same cache dir, schema version, and auth-key hash)
// so it reports the exact file `codex_apps_cache` persists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexAppsToolsCacheState {
    Hit,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexAppsToolsCacheStatus {
    pub cache_path: PathBuf,
    pub state: CodexAppsToolsCacheState,
    pub schema_version: Option<u8>,
    pub byte_size: Option<u64>,
    pub modified_at: Option<i64>,
    pub tool_count: Option<usize>,
}

pub fn codex_apps_tools_cache_status(
    codex_home: &Path,
    user_key: CodexAppsToolsCacheKey,
) -> CodexAppsToolsCacheStatus {
    let cache_path = codex_apps_tools_cache_path(codex_home, &user_key);
    let metadata = std::fs::metadata(&cache_path).ok();
    let byte_size = metadata.as_ref().map(std::fs::Metadata::len);
    let modified_at = metadata
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_to_epoch_seconds);

    let bytes = match std::fs::read(&cache_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return CodexAppsToolsCacheStatus {
                cache_path,
                state: CodexAppsToolsCacheState::Missing,
                schema_version: None,
                byte_size,
                modified_at,
                tool_count: None,
            };
        }
        Err(_) => {
            return CodexAppsToolsCacheStatus {
                cache_path,
                state: CodexAppsToolsCacheState::Invalid,
                schema_version: None,
                byte_size,
                modified_at,
                tool_count: None,
            };
        }
    };

    let cache: CodexAppsToolsDiskCache = match serde_json::from_slice(&bytes) {
        Ok(cache) => cache,
        Err(_) => {
            return CodexAppsToolsCacheStatus {
                cache_path,
                state: CodexAppsToolsCacheState::Invalid,
                schema_version: None,
                byte_size,
                modified_at,
                tool_count: None,
            };
        }
    };
    let schema_version = Some(cache.schema_version);
    if cache.schema_version != CODEX_APPS_TOOLS_CACHE_SCHEMA_VERSION {
        return CodexAppsToolsCacheStatus {
            cache_path,
            state: CodexAppsToolsCacheState::Invalid,
            schema_version,
            byte_size,
            modified_at,
            tool_count: None,
        };
    }

    CodexAppsToolsCacheStatus {
        cache_path,
        state: CodexAppsToolsCacheState::Hit,
        schema_version,
        byte_size,
        modified_at,
        tool_count: Some(cache.tools.len()),
    }
}

#[derive(Deserialize)]
struct CodexAppsToolsDiskCache {
    schema_version: u8,
    tools: Vec<ToolInfo>,
}

const CODEX_APPS_TOOLS_CACHE_DIR: &str = "cache/codex_apps_tools";
const CODEX_APPS_TOOLS_CACHE_SCHEMA_VERSION: u8 = 4;

fn codex_apps_tools_cache_path(codex_home: &Path, user_key: &CodexAppsToolsCacheKey) -> PathBuf {
    let user_key_json = serde_json::to_string(user_key).unwrap_or_default();
    let user_key_hash = sha1_hex(&user_key_json);
    codex_home
        .join(CODEX_APPS_TOOLS_CACHE_DIR)
        .join(format!("{user_key_hash}.json"))
}

fn sha1_hex(s: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(s.as_bytes());
    let sha1 = hasher.finalize();
    hex::encode(sha1)
}

fn system_time_to_epoch_seconds(time: SystemTime) -> Option<i64> {
    let duration = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    i64::try_from(duration.as_secs()).ok()
}
