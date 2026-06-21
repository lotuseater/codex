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
        metadata: None,
    }
}

fn shell_call(call_id: &str, command: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "shell_command".to_string(),
        namespace: None,
        arguments: serde_json::json!({ "command": command }).to_string(),
        call_id: call_id.to_string(),
        metadata: None,
    }
}

fn shell_output(call_id: &str, text: String) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        id: None,
        call_id: call_id.to_string(),
        output: FunctionCallOutputPayload::from_text(text),
        metadata: None,
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
        metadata: None,
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
        recency: RecencyPolicy::new(conservative_tiers(preserve_recent_items)),
        disabled_categories: BTreeSet::new(),
    }
}

enum MessageTextKind {
    Input,
    Output,
}

// ---- Recency-tier model tests -------------------------------------------

/// A config whose only reducible behaviour is driven by an explicit recency
/// policy, so each tier test controls tiering precisely. `min_reduce_chars` is
/// small so the global gate never masks the per-tier thresholds.
fn tiered_config(path: &Path, recency: RecencyPolicy) -> PromptReductionConfig {
    PromptReductionConfig {
        artifact_dir: path.to_path_buf(),
        min_reduce_chars: 100,
        path_list_threshold: 8,
        min_saved_tokens: 1,
        preserve_recent_items: 0,
        recency,
        disabled_categories: BTreeSet::new(),
    }
}

/// A large `cat`-style source-read output. Its call source contains `cat `, so
/// `exact_preserve_reason` classifies it as `source_read` -> `source_read_digest`
/// (which reduces regardless of recency, except under a `Preserve` tier).
fn source_read_pair(call_id: &str, body_chars: usize) -> [ResponseItem; 2] {
    // The call_id seeds the body so distinct pairs are not collapsed as exact
    // `duplicate_block` matches; each still classifies as a source read.
    let body = format!(
        "// recovered source file dump for tier tests ({call_id})\n{}",
        format!("let value_{call_id} = compute_widget(index, offset);\n")
            .repeat(body_chars / 44 + 1)
    );
    [
        shell_call(call_id, &format!("cat src/widget_{call_id}.rs")),
        shell_output(call_id, body),
    ]
}

#[test]
fn preserve_tier_keeps_newest_source_read_verbatim() {
    // Newest item is a reducible source read. Under a Preserve-first policy the
    // newest band is kept verbatim; under the Conservative binary policy the
    // same source read reduces (source reads reduce even when recent).
    let temp = TempDir::new().unwrap();
    let [call, output] = source_read_pair("read-1", 1_600);

    let mut preserved = vec![call.clone(), output.clone()];
    let preserve_policy = RecencyPolicy::new(recency_weighted_tiers(3, 6, 12, 0.5, 0.6));
    let stats =
        reduce_prompt_items(&mut preserved, &tiered_config(temp.path(), preserve_policy)).unwrap();
    assert_eq!(
        stats.reductions, 0,
        "Preserve tier must keep newest verbatim"
    );

    let mut reduced = vec![call, output];
    let conservative = RecencyPolicy::new(conservative_tiers(0));
    let stats =
        reduce_prompt_items(&mut reduced, &tiered_config(temp.path(), conservative)).unwrap();
    assert_eq!(
        stats.reductions, 1,
        "Conservative policy still reduces the same recent source read"
    );
}

#[test]
fn aggressive_old_tier_saves_more_than_standard_tier() {
    // Same large source read at the OLDEST slot under two policies. Both reduce
    // it, but the aggressive tail tier (excerpt_mult 0.6) emits a shorter digest
    // and therefore saves strictly more tokens than a standard all-categories
    // tier (excerpt_mult 1.0). Padding (too short to reduce) only moves the
    // source read to slot 0 and contributes equally to both runs, so the
    // saved-token delta is purely the excerpt multiplier.
    let temp = TempDir::new().unwrap();
    let [call, output] = source_read_pair("read-old", 1_600);
    let padding = (0..30)
        .map(|i| {
            message(
                "assistant",
                format!("status ping {i}"),
                MessageTextKind::Output,
            )
        })
        .collect::<Vec<_>>();

    let build_items = || {
        let mut v = vec![call.clone(), output.clone()];
        v.extend(padding.iter().cloned());
        v
    };

    // Aggressive tail: Preserve/RecentOnly/mid are size 1 so slot 0 lands in the
    // final All{REST, 0.5, 0.6} tier.
    let mut aggressive = build_items();
    let agg_policy = RecencyPolicy::new(recency_weighted_tiers(1, 1, 1, 0.5, 0.6));
    let agg_stats =
        reduce_prompt_items(&mut aggressive, &tiered_config(temp.path(), agg_policy)).unwrap();

    // Standard all-categories tier (excerpt_mult 1.0) at the same oldest slot.
    let mut normal = build_items();
    let normal_policy = RecencyPolicy::new(conservative_tiers(0));
    let normal_stats =
        reduce_prompt_items(&mut normal, &tiered_config(temp.path(), normal_policy)).unwrap();

    assert_eq!(normal_stats.reductions, 1, "standard tier reduces the read");
    assert_eq!(agg_stats.reductions, 1, "aggressive tier reduces the read");
    assert!(
        agg_stats.saved_tokens > normal_stats.saved_tokens,
        "aggressive old tier (excerpt_mult 0.6) must save more than standard \
         (agg={}, standard={})",
        agg_stats.saved_tokens,
        normal_stats.saved_tokens
    );
}

#[test]
fn disabled_category_is_skipped() {
    let temp = TempDir::new().unwrap();
    let [call, output] = source_read_pair("read-dis", 1_600);
    let mut items = vec![call, output];

    let mut config = tiered_config(temp.path(), RecencyPolicy::new(conservative_tiers(0)));
    config
        .disabled_categories
        .insert("source_read_digest".to_string());

    let stats = reduce_prompt_items(&mut items, &config).unwrap();
    assert_eq!(
        stats.reductions, 0,
        "disabled source_read_digest must never reduce"
    );
}

#[test]
fn recency_weighted_keeps_more_recent_detail_than_conservative() {
    // Two recent source reads plus a user turn. RecencyWeighted's Preserve band
    // keeps both source reads; Conservative(0) reduces both.
    let temp = TempDir::new().unwrap();
    let [c1, o1] = source_read_pair("rw-1", 1_600);
    let [c2, o2] = source_read_pair("rw-2", 1_600);
    let build = || {
        vec![
            c1.clone(),
            o1.clone(),
            c2.clone(),
            o2.clone(),
            message("user", "continue".to_string(), MessageTextKind::Input),
        ]
    };

    let mut weighted = build();
    let rw = RecencyPolicy::new(recency_weighted_tiers(3, 6, 12, 0.5, 0.6));
    let rw_stats = reduce_prompt_items(&mut weighted, &tiered_config(temp.path(), rw)).unwrap();

    let mut conservative = build();
    let cons = RecencyPolicy::new(conservative_tiers(0));
    let cons_stats =
        reduce_prompt_items(&mut conservative, &tiered_config(temp.path(), cons)).unwrap();

    assert_eq!(
        rw_stats.reductions, 0,
        "RecencyWeighted Preserve band retains recent source-read detail"
    );
    // Conservative is recency-blind and digests the same recent source reads that
    // RecencyWeighted preserves. `reductions` counts reduced ITEMS (call + output
    // per source-read pair), so the exact count is incidental; the invariant under
    // test is that Conservative reduces strictly more recent detail than
    // RecencyWeighted.
    assert!(
        cons_stats.reductions > rw_stats.reductions,
        "Conservative reduces the recent source reads that RecencyWeighted preserves \
         (conservative={}, recency_weighted={})",
        cons_stats.reductions,
        rw_stats.reductions
    );
}
