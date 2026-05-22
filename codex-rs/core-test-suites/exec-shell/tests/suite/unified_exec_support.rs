use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;

use anyhow::Context;
use anyhow::Result;
use codex_core_test_runtime::test_codex::TestCodex;
use codex_core_test_runtime::wait_for_event_match;
use codex_exec_server::CreateDirectoryOptions;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::SandboxPolicy;
use codex_protocol::user_input::UserInput;
use regex_lite::Regex;
use serde_json::Value;
use tokio::time::Duration;

pub(crate) const UNIFIED_EXEC_LAGGED_OUTPUT_TIMEOUT: Duration = Duration::from_secs(30);

fn extract_output_text(item: &Value) -> Option<&str> {
    item.get("output").and_then(|value| match value {
        Value::String(text) => Some(text.as_str()),
        Value::Object(obj) => obj.get("content").and_then(Value::as_str),
        _ => None,
    })
}

#[derive(Debug)]
pub(crate) struct ParsedUnifiedExecOutput {
    pub(crate) chunk_id: Option<String>,
    pub(crate) wall_time_seconds: f64,
    pub(crate) process_id: Option<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) original_token_count: Option<usize>,
    pub(crate) output: String,
}

#[allow(clippy::expect_used)]
pub(crate) fn parse_unified_exec_output(raw: &str) -> Result<ParsedUnifiedExecOutput> {
    static OUTPUT_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = OUTPUT_REGEX.get_or_init(|| {
        Regex::new(concat!(
            r#"(?s)^(?:Total output lines: \d+\n\n)?"#,
            r#"(?:Chunk ID: (?P<chunk_id>[^\n]+)\n)?"#,
            r#"Wall time: (?P<wall_time>-?\d+(?:\.\d+)?) seconds\n"#,
            r#"(?:Process exited with code (?P<exit_code>-?\d+)\n)?"#,
            r#"(?:Process running with session ID (?P<process_id>-?\d+)\n)?"#,
            r#"(?:Original token count: (?P<original_token_count>\d+)\n)?"#,
            r#"Output:\n?(?P<output>.*)$"#,
        ))
        .expect("valid unified exec output regex")
    });

    let cleaned = raw.trim_matches('\r');
    let captures = regex
        .captures(cleaned)
        .ok_or_else(|| anyhow::anyhow!("missing Output section in unified exec output {raw}"))?;

    let chunk_id = captures
        .name("chunk_id")
        .map(|value| value.as_str().to_string());

    let wall_time_seconds = captures
        .name("wall_time")
        .expect("wall_time group present")
        .as_str()
        .parse::<f64>()
        .context("failed to parse wall time seconds")?;

    let exit_code = captures
        .name("exit_code")
        .map(|value| {
            value
                .as_str()
                .parse::<i32>()
                .context("failed to parse exit code from unified exec output")
        })
        .transpose()?;

    let process_id = captures
        .name("process_id")
        .map(|value| value.as_str().to_string());

    let original_token_count = captures
        .name("original_token_count")
        .map(|value| {
            value
                .as_str()
                .parse::<usize>()
                .context("failed to parse original token count from unified exec output")
        })
        .transpose()?;

    let output = captures
        .name("output")
        .expect("output group present")
        .as_str()
        .to_string();

    Ok(ParsedUnifiedExecOutput {
        chunk_id,
        wall_time_seconds,
        process_id,
        exit_code,
        original_token_count,
        output,
    })
}

pub(crate) fn collect_tool_outputs(
    bodies: &[Value],
) -> Result<HashMap<String, ParsedUnifiedExecOutput>> {
    let mut outputs = HashMap::new();
    for body in bodies {
        if let Some(items) = body.get("input").and_then(Value::as_array) {
            for item in items {
                if item.get("type").and_then(Value::as_str) != Some("function_call_output") {
                    continue;
                }
                if let Some(call_id) = item.get("call_id").and_then(Value::as_str) {
                    let content = extract_output_text(item)
                        .ok_or_else(|| anyhow::anyhow!("missing tool output content"))?;
                    let trimmed = content.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let parsed = parse_unified_exec_output(content).with_context(|| {
                        format!("failed to parse unified exec output for {call_id}")
                    })?;
                    outputs.insert(call_id.to_string(), parsed);
                }
            }
        }
    }
    Ok(outputs)
}

pub(crate) async fn wait_for_raw_unified_exec_output(
    test: &TestCodex,
    call_id: &str,
) -> Result<ParsedUnifiedExecOutput> {
    let content = wait_for_event_match(&test.codex, |event| match event {
        EventMsg::RawResponseItem(raw) => match &raw.item {
            ResponseItem::FunctionCallOutput {
                call_id: output_call_id,
                output,
            } if output_call_id == call_id => output.text_content().map(str::to_string),
            _ => None,
        },
        _ => None,
    })
    .await;

    parse_unified_exec_output(&content)
        .with_context(|| format!("failed to parse raw unified exec output for {call_id}"))
}

pub(crate) async fn submit_unified_exec_turn(
    test: &TestCodex,
    prompt: &str,
    sandbox_policy: SandboxPolicy,
) -> Result<()> {
    let session_model = test.session_configured.model.clone();

    test.codex
        .submit(Op::UserTurn {
            environments: None,
            items: vec![UserInput::Text {
                text: prompt.into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            cwd: test.config.cwd.to_path_buf(),
            approval_policy: AskForApproval::Never,
            approvals_reviewer: None,
            sandbox_policy,
            permission_profile: None,
            model: session_model,
            effort: None,
            summary: None,
            service_tier: None,
            context_budget_mode: Some(codex_protocol::config_types::ContextBudgetMode::Standard),
            collaboration_mode: None,
            personality: None,
        })
        .await?;

    Ok(())
}

pub(crate) async fn create_workspace_directory(
    test: &TestCodex,
    rel_path: impl AsRef<std::path::Path>,
) -> Result<std::path::PathBuf> {
    let abs_path = test.config.cwd.join(rel_path.as_ref());
    test.fs()
        .create_directory(
            &abs_path,
            CreateDirectoryOptions { recursive: true },
            /*sandbox*/ None,
        )
        .await?;
    Ok(abs_path.into_path_buf())
}
