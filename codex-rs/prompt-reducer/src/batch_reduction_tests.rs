use super::*;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use pretty_assertions::assert_eq;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn bundles_many_stale_assistant_status_updates() {
    let mut items = (0..8)
        .map(|index| {
            message(
                "assistant",
                format!(
                    "I'm checking prompt reducer batch {index} and reading the current \
                     status output before the next targeted pass. This is repeated \
                     coordination chatter about the same local loop, with no durable \
                     result and no new user-facing decision."
                ),
                MessageTextKind::Output,
            )
        })
        .chain(std::iter::once(message(
            "user",
            "keep going".to_string(),
            MessageTextKind::Input,
        )))
        .collect::<Vec<_>>();
    let temp = TempDir::new().unwrap();
    let config = test_config(temp.path(), 2);

    let stats = reduce_prompt_items(&mut items, &config).unwrap();

    assert_eq!(stats.artifacts, 1);
    assert_eq!(stats.reductions, 7);
    let ResponseItem::Message { content, .. } = &items[0] else {
        panic!("expected first assistant message");
    };
    let text = content_text(&content[0]);
    assert!(text.contains("[prompt reduction: short_assistant_status_bundle]"));
    assert!(text.contains("original_items: 7"));
    for item in &items[1..7] {
        let ResponseItem::Message { content, .. } = item else {
            panic!("expected bundled assistant message");
        };
        assert_eq!(content_text(&content[0]), "");
    }
}

#[test]
fn keeps_recent_or_durable_assistant_status_updates() {
    let mut items = vec![
        message(
            "assistant",
            "Verification failed in the reducer canary; keeping this status is required."
                .to_string(),
            MessageTextKind::Output,
        ),
        message(
            "assistant",
            "I'm checking the current release prompt-reducer test lane.".to_string(),
            MessageTextKind::Output,
        ),
        message("user", "keep going".to_string(), MessageTextKind::Input),
    ];
    let temp = TempDir::new().unwrap();
    let config = test_config(temp.path(), 2);

    let stats = reduce_prompt_items(&mut items, &config).unwrap();

    assert_eq!(stats.reductions, 0);
    let ResponseItem::Message { content, .. } = &items[0] else {
        panic!("expected assistant message");
    };
    assert!(content_text(&content[0]).contains("failed"));
}

#[test]
fn reduces_stale_workflow_batch_success_summary() {
    let steps = (0..24)
        .map(|index| {
            serde_json::json!({
                "id": format!("step-{index}"),
                "status": "ok",
                "summary": "completed the deterministic workflow-batch step and recorded repeated low-utility bookkeeping details for the same successful path"
            })
        })
        .collect::<Vec<_>>();
    let summary = serde_json::json!({
        "status": "ok",
        "report_path": "reports/workflow_batch_codex_reacher_matrix.json",
        "log_path": "reports/workflow_batch_codex_reacher_matrix.log",
        "steps_total": 24,
        "steps_failed": 0,
        "steps_skipped": 0,
        "vars": {
            "scenario": "codex-reacher",
            "matrix": "new reducer preserves failures and compacts successful batch output",
            "notes": "successful workflow-batch bookkeeping ".repeat(120)
        },
        "steps": steps
    })
    .to_string();
    let mut items = vec![workflow_call("batch-1"), shell_output("batch-1", summary)];
    let temp = TempDir::new().unwrap();
    let config = test_config(temp.path(), 0);

    let stats = reduce_prompt_items(&mut items, &config).unwrap();

    assert_eq!(stats.reductions, 1);
    let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
        panic!("expected workflow_batch output");
    };
    let text = output.text_content().unwrap();
    assert!(text.contains("[prompt reduction: workflow_batch_success_digest]"));
    assert!(text.contains("report_path"));
}

#[test]
fn reduces_recent_source_read_with_artifact_recovery() {
    let source_text = (0..80)
        .map(|index| {
            format!("{index}: pub fn generated_case_{index}() {{ println!(\"case {index}\"); }}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let output = format!("Exit code: 0\nOutput:\n{source_text}");
    let mut items = vec![
        shell_call(
            "source-1",
            "Get-Content -Raw codex-rs/prompt-reducer/src/lib.rs",
        ),
        shell_output("source-1", output),
    ];
    let temp = TempDir::new().unwrap();
    let config = test_config(temp.path(), 4);

    let stats = reduce_prompt_items(&mut items, &config).unwrap();

    assert_eq!(stats.reductions, 1);
    assert_eq!(stats.artifacts, 1);
    let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
        panic!("expected shell output");
    };
    let text = output.text_content().unwrap();
    assert!(text.contains("[prompt reduction: source_read_digest]"));
    assert!(text.contains("recovery: read artifact before using exact lines."));
}

#[test]
fn preserves_workflow_batch_failed_summary() {
    let summary = serde_json::json!({
        "status": "failed",
        "report_path": "reports/workflow_batch_codex_reacher_matrix.json",
        "steps_total": 7,
        "steps_failed": 1,
        "error": "assertion failed"
    })
    .to_string();
    let mut items = vec![workflow_call("batch-1"), shell_output("batch-1", summary)];
    let temp = TempDir::new().unwrap();
    let config = test_config(temp.path(), 0);

    let stats = reduce_prompt_items(&mut items, &config).unwrap();

    assert_eq!(stats.reductions, 0);
    let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
        panic!("expected workflow_batch output");
    };
    assert!(output.text_content().unwrap().contains("failed"));
}

#[test]
fn reduces_stale_single_use_helper_prompt_to_recoverable_digest() {
    let helper_prompt = [
        "CONTEXT_AREA: prompt reducer review with a long scoped description of stale helper prompt handling and workflow-batch summary behavior",
        "DO_NOT_INSPECT: unrelated tui files, generated artifacts, unrelated app-server code, and any broad workspace sweep outside the reducer and tool exposure paths",
        "SCOUT_EVIDENCE: git diff showed reducer-only heuristics, workflow-batch success summaries, helper prompt contracts, and short assistant status chatter as the important areas",
        "WHY_AGENT: bounded review lane for stale prompt removal, model-visible recovery hints, and avoiding expensive root context expansion during continuation turns",
        "FIRST_READS: codex-rs/prompt-reducer/src/lib.rs plus the focused batch reduction tests and the workflow-batch handler output shape",
        "TOOL_HINTS: use rg and small file slices, avoid broad generated outputs, and keep commands read-only while another release lane owns the target directory",
        "TOKEN_TIP: stay narrow and return only concrete findings with file paths, line numbers, and a short explanation of the risk",
        "VERIFICATION: report the smallest proof from unit tests, release logs, or direct reducer fixtures without adding unrelated assertions",
        "HANDOFF: return findings only, name any file read, and include enough context for root to patch without rereading the whole workspace",
    ]
    .join("\n");
    let mut items = vec![message("user", helper_prompt, MessageTextKind::Input)];
    let temp = TempDir::new().unwrap();
    let config = test_config(temp.path(), 0);

    let stats = reduce_prompt_items(&mut items, &config).unwrap();

    assert_eq!(stats.reductions, 1);
    let ResponseItem::Message { content, .. } = &items[0] else {
        panic!("expected helper prompt");
    };
    let text = content_text(&content[0]);
    assert!(text.contains("[prompt reduction: single_use_helper_prompt]"));
    assert!(text.contains("recovery: read artifact before using exact lines."));
}

fn workflow_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "workflow_batch".to_string(),
        namespace: None,
        arguments: serde_json::json!({ "spec": {} }).to_string(),
        call_id: call_id.to_string(),
    }
}

fn shell_call(call_id: &str, command: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "shell_command".to_string(),
        namespace: None,
        arguments: serde_json::json!({ "command": command }).to_string(),
        call_id: call_id.to_string(),
    }
}

fn shell_output(call_id: &str, text: String) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        call_id: call_id.to_string(),
        output: FunctionCallOutputPayload::from_text(text),
    }
}

fn message(role: &str, text: String, kind: MessageTextKind) -> ResponseItem {
    let content = match kind {
        MessageTextKind::Input => vec![ContentItem::InputText { text }],
        MessageTextKind::Output => vec![ContentItem::OutputText { text }],
    };
    ResponseItem::Message {
        id: None,
        role: role.to_string(),
        content,
        phase: None,
    }
}

fn content_text(content: &ContentItem) -> &str {
    match content {
        ContentItem::InputText { text } | ContentItem::OutputText { text } => text,
        _ => panic!("expected text content item"),
    }
}

fn test_config(path: &Path, preserve_recent_items: usize) -> PromptReductionConfig {
    PromptReductionConfig {
        artifact_dir: path.to_path_buf(),
        min_reduce_chars: 100,
        path_list_threshold: 8,
        min_saved_tokens: 1,
        preserve_recent_items,
    }
}

enum MessageTextKind {
    Input,
    Output,
}
