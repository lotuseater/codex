use std::path::Path;

use crate::original_image_detail::can_request_original_image_detail;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::McpToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::registry::AnyToolResult;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;
pub(crate) use codex_operation_cache::OperationCacheHit;
use codex_protocol::mcp::CallToolResult;
use codex_utils_absolute_path::AbsolutePathBuf;
use serde_json::Value;

pub(crate) struct CachedToolResult {
    pub(crate) duration: std::time::Duration,
    pub(crate) result: AnyToolResult,
}

pub(crate) async fn lookup(
    payload: &PreToolUsePayload,
    cwd: &Path,
    supports_parallel_tool_calls: bool,
) -> Option<OperationCacheHit> {
    if !tool_is_cacheable(
        payload.tool_name.name(),
        &payload.tool_input,
        supports_parallel_tool_calls,
    ) {
        return None;
    }

    codex_operation_cache::lookup(payload.tool_name.name(), &payload.tool_input, cwd).await
}

pub(crate) async fn store(
    payload: &PostToolUsePayload,
    cwd: &Path,
    supports_parallel_tool_calls: bool,
) {
    if !tool_is_cacheable(
        payload.tool_name.name(),
        &payload.tool_input,
        supports_parallel_tool_calls,
    ) {
        return;
    }

    codex_operation_cache::store(
        payload.tool_name.name(),
        &payload.tool_input,
        &payload.tool_response,
        cwd,
    )
    .await;
}

pub(crate) fn cwd(invocation: &ToolInvocation) -> AbsolutePathBuf {
    match &invocation.payload {
        ToolPayload::Function { arguments } => serde_json::from_str::<Value>(arguments)
            .ok()
            .and_then(|value| {
                value
                    .get("workdir")
                    .and_then(Value::as_str)
                    .filter(|workdir| !workdir.is_empty())
                    .map(str::to_string)
            })
            .map_or_else(
                || invocation.turn.cwd.clone(),
                #[allow(deprecated)]
                |workdir| invocation.turn.cwd.join(workdir),
            ),
        ToolPayload::ToolSearch { .. } | ToolPayload::Custom { .. } => invocation.turn.cwd.clone(),
    }
}

pub(crate) fn result_from_hit(
    invocation: &ToolInvocation,
    pre_tool_use_payload: &PreToolUsePayload,
    cache_hit: OperationCacheHit,
) -> CachedToolResult {
    let cache_text = cache_hit.text;
    let mcp_call_result = if pre_tool_use_payload.tool_name.name().starts_with("mcp__") {
        serde_json::from_str::<CallToolResult>(&cache_text).ok()
    } else {
        None
    };
    let hook_tool_response = match &mcp_call_result {
        Some(call_result) => {
            serde_json::to_value(call_result).unwrap_or_else(|_| Value::String(cache_text.clone()))
        }
        None => Value::String(cache_text.clone()),
    };
    let result: Box<dyn ToolOutput> = match mcp_call_result {
        Some(call_result) => Box::new(McpToolOutput {
            result: call_result,
            tool_input: pre_tool_use_payload.tool_input.clone(),
            wall_time: cache_hit.duration,
            original_image_detail_supported: can_request_original_image_detail(
                &invocation.turn.model_info,
            ),
            truncation_policy: invocation.turn.truncation_policy,
        }),
        None => Box::new(FunctionToolOutput::from_text(cache_text, Some(true))),
    };

    CachedToolResult {
        duration: cache_hit.duration,
        result: AnyToolResult {
            call_id: invocation.call_id.clone(),
            payload: invocation.payload.clone(),
            result,
            post_tool_use_payload: Some(PostToolUsePayload {
                tool_name: pre_tool_use_payload.tool_name.clone(),
                tool_use_id: invocation.call_id.clone(),
                tool_input: pre_tool_use_payload.tool_input.clone(),
                tool_response: hook_tool_response,
            }),
        },
    }
}

pub(crate) async fn try_serve_from_cache(
    invocation: &ToolInvocation,
    pre_tool_use_payload: Option<&PreToolUsePayload>,
    cwd: &Path,
    supports_parallel_tool_calls: bool,
) -> Option<CachedToolResult> {
    let pre_tool_use_payload = pre_tool_use_payload?;
    let cache_hit = lookup(pre_tool_use_payload, cwd, supports_parallel_tool_calls).await?;
    Some(result_from_hit(invocation, pre_tool_use_payload, cache_hit))
}

/// fork-local: skip the operation-cache store when the result was served from
/// cache or replaced by a PostToolUse hook; a PostToolUse block rejects the
/// result, not the execution.
pub(crate) fn should_store(
    served_from_operation_cache: bool,
    replaced_by_post_tool_use: bool,
) -> bool {
    !served_from_operation_cache && !replaced_by_post_tool_use
}

pub(crate) fn tool_is_cacheable(
    tool_name: &str,
    tool_input: &Value,
    supports_parallel_tool_calls: bool,
) -> bool {
    if tool_name.starts_with("dab_") {
        return false;
    }

    if tool_name.starts_with("mcp__") {
        return supports_parallel_tool_calls;
    }

    if !matches!(
        tool_name,
        "Bash" | "bash" | "Shell" | "shell" | "exec_command"
    ) {
        return false;
    }

    let Some(command) = tool_input.get("command").and_then(Value::as_str) else {
        return false;
    };
    command_looks_read_only(command)
}

fn command_looks_read_only(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }

    let lower = command.to_ascii_lowercase();
    if contains_mutating_shell_signal(&lower) {
        return false;
    }

    const ALLOWED_PREFIXES: &[&str] = &[
        "dir",
        "gc ",
        "get-childitem",
        "get-content",
        "get-item",
        "git diff",
        "git log",
        "git ls-files",
        "git show",
        "git status",
        "gci ",
        "ls",
        "pwd",
        "rg ",
        "rg.exe ",
        "select-string",
        "test-path",
        "type ",
    ];

    ALLOWED_PREFIXES
        .iter()
        .any(|prefix| lower == *prefix || lower.starts_with(prefix))
}

fn contains_mutating_shell_signal(command: &str) -> bool {
    const MUTATING_SIGNALS: &[&str] = &[
        ">",
        ";",
        "&&",
        "||",
        " set-content",
        " out-file",
        " remove-item",
        " move-item",
        " copy-item",
        " new-item",
        " mkdir",
        " rm ",
        " del ",
        " erase ",
        " git add",
        " git commit",
        " git checkout",
        " git clean",
        " git merge",
        " git mv",
        " git rebase",
        " git reset",
        " git rm",
    ];

    MUTATING_SIGNALS
        .iter()
        .any(|signal| command.contains(signal))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn shell_cacheability_allows_simple_read_only_probes() {
        for command in [
            "rg -n operation_cache codex-rs/core/src",
            "Get-Content -Raw codex-rs/core/src/tools/mod.rs",
            "git status --short --branch",
            "git show HEAD:README.md",
            "Select-String -Path file.rs -Pattern needle",
        ] {
            assert!(
                tool_is_cacheable("Bash", &json!({ "command": command }), false),
                "{command}"
            );
        }
    }

    #[test]
    fn shell_cacheability_rejects_mutating_or_compound_commands() {
        for command in [
            "Set-Content file.txt value",
            "Get-Content file.txt > copy.txt",
            "rg -n TODO .; Remove-Item file.txt",
            "git reset --hard HEAD",
            "cargo test --release",
        ] {
            assert!(
                !tool_is_cacheable("Bash", &json!({ "command": command }), false),
                "{command}"
            );
        }
    }

    #[test]
    fn mcp_cacheability_requires_read_only_parallel_signal() {
        let input = json!({ "path": "README.md" });

        assert!(tool_is_cacheable(
            "mcp__filesystem__read_file",
            &input,
            /*supports_parallel_tool_calls*/ true
        ));
        assert!(!tool_is_cacheable(
            "mcp__filesystem__write_file",
            &input,
            /*supports_parallel_tool_calls*/ false
        ));
    }

    #[test]
    fn generic_and_live_tools_are_not_cacheable_by_default() {
        assert_eq!(
            tool_is_cacheable("workflow_batch", &json!({ "spec": {} }), false),
            false
        );
        assert_eq!(tool_is_cacheable("dab_screenshot", &json!({}), true), false);
    }

    #[test]
    fn should_store_only_when_not_served_and_not_replaced() {
        assert!(should_store(
            /*served_from_operation_cache*/ false,
            /*replaced_by_post_tool_use*/ false
        ));
        assert!(!should_store(
            /*served_from_operation_cache*/ true,
            /*replaced_by_post_tool_use*/ false
        ));
        assert!(!should_store(
            /*served_from_operation_cache*/ false,
            /*replaced_by_post_tool_use*/ true
        ));
        assert!(!should_store(
            /*served_from_operation_cache*/ true,
            /*replaced_by_post_tool_use*/ true
        ));
    }
}
