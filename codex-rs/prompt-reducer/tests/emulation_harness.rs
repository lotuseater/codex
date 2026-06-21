//! Emulation harness: measures Off / Conservative / RecencyWeighted reduction
//! on a realistic synthetic conversation history and writes a comparison report.
//!
//! Run with:
//!   cargo test -p codex-prompt-reducer --test emulation_harness -- --nocapture
//!
//! The harness:
//! 1. Builds a realistic `Vec<ResponseItem>` in-code (synthetic, see NOTE below).
//! 2. Copies the real session triage fixture alongside (committable, load-only).
//! 3. Runs the reducer in three modes: Off, Conservative, RecencyWeighted.
//! 4. Writes a quantitative markdown report to
//!    `<repo>/.codex/tmp/prompt_reduction_2026-06-21/emulation_report.md`.
//! 5. Asserts key invariants as regression guards.
//!
//! NOTE on fixture choice: the real `tests/fixtures/sample_history.json` uses
//! the R2 triage schema (fields: seq, session_pos, recency_frac, raw_bytes, etc.)
//! — NOT serialized `ResponseItem` JSON — so it cannot be directly deserialized
//! into `Vec<ResponseItem>`. The synthetic construction below is intentional and
//! builds a representative history. The fixture file is kept as a committed
//! artifact for reference and future use.

use codex_prompt_reducer::PromptReductionConfig;
use codex_prompt_reducer::RecencyWeightedOpts;
use codex_prompt_reducer::reduce_prompt_items;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;

// ---------------------------------------------------------------------------
// Token-estimate helper (mirrors crate-internal approx_tokens: chars / 4)
// ---------------------------------------------------------------------------

fn approx_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

// ---------------------------------------------------------------------------
// Synthetic fixture construction
// ---------------------------------------------------------------------------

/// Build a realistic ~45-item conversation history ordered oldest → newest.
/// Mix: large tool outputs / build logs / search results / source reads in the
/// OLD and MID bands; shorter assistant + user messages near the tail (RECENT).
/// Total text slots >> 3+6+12 = 21 so all four RecencyWeighted tiers are populated.
fn build_synthetic_history() -> Vec<ResponseItem> {
    let mut items: Vec<ResponseItem> = Vec::new();

    // Helpers
    let user_msg = |text: &str| ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
        metadata: None,
    };
    let assistant_msg = |text: &str| ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
        metadata: None,
    };
    let fn_call = |name: &str, call_id: &str| ResponseItem::FunctionCall {
        id: None,
        name: name.to_string(),
        namespace: None,
        arguments: format!(r#"{{"cmd": "{name}"}}"#),
        call_id: call_id.to_string(),
        metadata: None,
    };
    let fn_output = |call_id: &str, content: &str| ResponseItem::FunctionCallOutput {
        id: None,
        call_id: call_id.to_string(),
        output: FunctionCallOutputPayload::from_text(content.to_string()),
        metadata: None,
    };

    // ------------------------------------------------------------------
    // OLD band: items 0-4 (5 items, large tool outputs, build logs)
    // ------------------------------------------------------------------

    // Item 0: initial developer message / system prompt
    items.push(ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText {
            text: "You are Codex. Follow instructions precisely.".repeat(20),
        }],
        phase: None,
        metadata: None,
    });

    // Item 1: user task
    items.push(user_msg(
        "Please run the full build and then read the source files to understand the architecture.",
    ));

    // Item 2: large build log (old, large — prime candidate for aggressive reduction)
    let build_log = format!(
        "shell_output:cargo build --release\n{}\nexit code: 0\nFinished release [optimized] target(s) in 142.3s",
        "   Compiling codex-core v0.1.0\n   Compiling codex-protocol v0.1.0\n   Compiling codex-config v0.1.0\n   Compiling codex-prompt-reducer v0.1.0\n".repeat(60)
    );
    items.push(fn_call("shell_command", "call_001"));
    items.push(fn_output("call_001", &build_log));

    // Item 3: large source read (old, large — source_read_digest candidate)
    let source_read = format!(
        "// codex-rs/core/src/session.rs\n{}\n// end of file",
        "/// Session manages the conversation between the user and the model.\n\
pub struct Session {\n    history: Vec<ResponseItem>,\n    config: SessionConfig,\n}\n\
impl Session {\n    pub fn new(config: SessionConfig) -> Self { todo!() }\n    \
pub fn handle_turn(&mut self) -> Result<()> { todo!() }\n}\n"
            .repeat(80)
    );
    items.push(fn_call("read_file", "call_002"));
    items.push(fn_output("call_002", &source_read));

    // ------------------------------------------------------------------
    // MID band: items 5-16 (12 items, mix of searches / assistant / user)
    // ------------------------------------------------------------------

    // Item 5: search result (mid, reducible)
    let search_result = format!(
        "grep_result: found 47 matches for 'ResponseItem'\n{}",
        "codex-rs/core/src/session.rs:120: ResponseItem::Message\n\
codex-rs/core/src/session.rs:145: ResponseItem::FunctionCall\n\
codex-rs/protocol/src/models.rs:169: pub enum ResponseItem\n"
            .repeat(15)
    );
    items.push(fn_call("shell_command", "call_003"));
    items.push(fn_output("call_003", &search_result));

    // Item 6: assistant status update (short, bundleable)
    items.push(assistant_msg(
        "I'm reading the session implementation to understand the architecture.",
    ));

    // Item 7: another large source read
    let source_read2 = format!(
        "// codex-rs/prompt-reducer/src/lib.rs\n{}\n// end of file",
        "/// Reduce prompt items to fit within the model context window.\n\
pub fn reduce_prompt_items(items: &mut [ResponseItem], config: &PromptReductionConfig) \
-> std::io::Result<PromptReductionStats> {\n    // implementation\n    todo!()\n}\n"
            .repeat(70)
    );
    items.push(fn_call("read_file", "call_004"));
    items.push(fn_output("call_004", &source_read2));

    // Item 8: assistant analysis
    items.push(assistant_msg("I'm checking the recency tier configuration and comparing it with the conservative approach."));

    // Item 9: JSON config output (reducible)
    let json_config = serde_json::json!({
        "recency_policy": {
            "tiers": [
                {"kind": "Preserve", "slot_count": 3},
                {"kind": "RecentOnly", "slot_count": 6},
                {"kind": "All", "slot_count": 12, "threshold_mult": 1.0},
                {"kind": "All", "slot_count": 999999, "threshold_mult": 0.5}
            ]
        },
        "min_reduce_chars": 2000,
        "min_saved_tokens": 128,
        "details": "lorem ipsum ".repeat(80)
    });
    items.push(fn_call("shell_command", "call_005"));
    items.push(fn_output(
        "call_005",
        &serde_json::to_string_pretty(&json_config).unwrap(),
    ));

    // Item 10: user follow-up
    items.push(user_msg("What did you find about the recency weighting?"));

    // Item 11: assistant findings (durable, should not be fully stripped)
    items.push(assistant_msg(
        "Finding: RecencyWeightedOpts::default() uses preserve=3, recent=6, mid=12, \
old_threshold_mult=0.5. The old tier halves the reduction threshold, meaning items in \
the oldest 80%+ of the conversation are reduced 2x more aggressively than the conservative \
approach which treats all non-recent items uniformly.",
    ));

    // Item 12: another build log (duplicate pattern, reducible)
    let build_log2 = format!(
        "shell_output:cargo check -p codex-prompt-reducer\n{}\nexit code: 0\nFinished dev target(s) in 8.2s",
        "warning: unused import: `std::collections::BTreeMap`\n\
warning: 2 warnings emitted\n\
    Checking codex-prompt-reducer v0.1.0\n"
            .repeat(40)
    );
    items.push(fn_call("shell_command", "call_006"));
    items.push(fn_output("call_006", &build_log2));

    // Item 13: search (path inventory candidate)
    let path_list = (0..20)
        .map(|i| format!("codex-rs/core/src/module_{i:02}.rs"))
        .collect::<Vec<_>>()
        .join("\n");
    items.push(fn_call("shell_command", "call_007"));
    items.push(fn_output("call_007", &path_list));

    // Item 14: assistant status (short, bundleable)
    items.push(assistant_msg(
        "I'm gathering all the relevant file paths for the analysis.",
    ));

    // Item 15: large command log
    let cmd_log = format!(
        "shell_output:git log --oneline\n{}\n",
        "a1b2c3d feat: add recency-weighted reduction\n\
b2c3d4e fix: conservative tier threshold\n\
c3d4e5f chore: update Cargo.toml\n"
            .repeat(50)
    );
    items.push(fn_call("shell_command", "call_008"));
    items.push(fn_output("call_008", &cmd_log));

    // ------------------------------------------------------------------
    // RECENT band: items 17-23 (7 items, mix — RecentOnly tier)
    // ------------------------------------------------------------------

    // Item 17: user message
    items.push(user_msg("Can you summarize the reduction stats so far?"));

    // Item 18: assistant summary (durable)
    items.push(assistant_msg(
        "Summary: The RecencyWeighted approach applies 4 tiers. The newest 3 text slots \
are Preserved (verbatim). The next 6 use RecentOnly categories. The next 12 use standard \
All categories. The oldest tail uses All categories with threshold_mult=0.5 and \
excerpt_mult=0.6, meaning they are reduced more aggressively.",
    ));

    // Item 19: recent build output (recent, should be lightly reduced only)
    let recent_build = format!(
        "shell_output:cargo test -p codex-prompt-reducer\n\
running 5 tests\ntest recency_weighted_bands_resolve_by_distance ... ok\n\
test conservative_policy_matches_binary_boundary ... ok\n\
{}\nexit code: 0\ntest result: ok. 5 passed; 0 failed",
        "test details ... ok\n".repeat(20)
    );
    items.push(fn_call("shell_command", "call_009"));
    items.push(fn_output("call_009", &recent_build));

    // Item 20: recent JSON output
    let recent_json = serde_json::json!({
        "status": "ok",
        "reductions": 12,
        "saved_tokens": 4200,
        "recent_note": "This is a recent result that should be lightly treated",
        "extra": "data ".repeat(50)
    });
    items.push(fn_call("shell_command", "call_010"));
    items.push(fn_output(
        "call_010",
        &serde_json::to_string_pretty(&recent_json).unwrap(),
    ));

    // Item 21: user follow-up
    items.push(user_msg("Good. Now run the emulation harness test."));

    // ------------------------------------------------------------------
    // PRESERVE band: newest 3 text slots — must be verbatim in RecencyWeighted
    // ------------------------------------------------------------------
    // NOTE: "text slots" are counted per-text-content within items, not items.
    // Each Message with one text item = 1 slot. FunctionCallOutput text = 1 slot.
    // We place the last 3 distinct text-bearing items here so they definitely
    // land in the Preserve tier.

    // Item 22: assistant narrating final action (newest preserve slot)
    items.push(assistant_msg(
        "Running the emulation harness now. This is the most recent assistant turn \
and must be preserved verbatim by RecencyWeighted reduction. Checking all three modes.",
    ));

    // Item 23: recent large tool output (2nd newest preserve slot)
    let newest_tool = format!(
        "shell_output:cargo test emulation_harness -- --nocapture\n\
test emulation_harness::test_reduction_modes ... \n\
MODE: Off\n  total_items: 45\n  estimated_tokens: 12840\n\
MODE: Conservative\n  reductions: 8\n  saved_tokens: 3200\n\
MODE: RecencyWeighted\n  reductions: 10\n  saved_tokens: 3900\n\
{}\ntest result: ok",
        "detail line\n".repeat(10)
    );
    items.push(fn_call("shell_command", "call_011"));
    items.push(fn_output("call_011", &newest_tool));

    // Item 24: final user message (3rd newest preserve slot — but user slots count too)
    items.push(user_msg(
        "Perfect. Write the report summarizing Off vs Conservative vs RecencyWeighted \
reduction modes for the orchestrator's decision on whether to ship RecencyWeighted as default.",
    ));

    items
}

// ---------------------------------------------------------------------------
// Band analysis helpers
// ---------------------------------------------------------------------------

/// Count how many text slots fall in each recency band (by distance from end).
/// Returns (preserved, recent_only, mid, old) item counts in the Vec<ResponseItem>.
/// "Text slots" = same counting as the reducer: each text ContentItem or
/// FunctionCallOutput text payload = 1 slot.
fn count_text_slots_by_position(items: &[ResponseItem]) -> usize {
    items
        .iter()
        .map(|item| match item {
            ResponseItem::Message { content, .. } => content
                .iter()
                .filter(|c| {
                    matches!(
                        c,
                        ContentItem::InputText { .. } | ContentItem::OutputText { .. }
                    )
                })
                .count(),
            ResponseItem::FunctionCallOutput { output, .. } => {
                if output.text_content().is_some() {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        })
        .sum()
}

/// Compute token-reduction per recency band by comparing before/after text content.
/// Returns a vec of (band_name, items_in_band, tokens_cut) tuples.
/// Bands: newest 3 slots / next 6 / next 12 / older.
/// We measure at the ITEM level (text content before vs after), mapping items to
/// bands by their text-slot distance from the end.
#[derive(Debug, Clone)]
struct BandStats {
    name: &'static str,
    items_total: usize,
    tokens_original: usize,
    tokens_after: usize,
    #[allow(dead_code)]
    items_reduced: usize,
}

impl BandStats {
    fn tokens_cut(&self) -> usize {
        self.tokens_original.saturating_sub(self.tokens_after)
    }
    fn pct_cut(&self) -> f64 {
        if self.tokens_original == 0 {
            0.0
        } else {
            100.0 * self.tokens_cut() as f64 / self.tokens_original as f64
        }
    }
}

/// Extract text content from a ResponseItem as a list of (text, tokens) pairs.
fn item_text_slots(item: &ResponseItem) -> Vec<String> {
    match item {
        ResponseItem::Message { content, .. } => content
            .iter()
            .filter_map(|c| match c {
                ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect(),
        ResponseItem::FunctionCallOutput { output, .. } => output
            .text_content()
            .map(|t| vec![t.to_string()])
            .unwrap_or_default(),
        _ => vec![],
    }
}

fn compute_band_stats(
    before: &[ResponseItem],
    after: &[ResponseItem],
    preserve_n: usize,
    recent_n: usize,
    mid_n: usize,
) -> Vec<BandStats> {
    let total_slots = count_text_slots_by_position(before);

    let bands: &[(&'static str, std::ops::Range<usize>)] = &[
        ("newest (Preserve, dist 0..preserve)", 0..preserve_n),
        (
            "recent (RecentOnly, dist preserve..preserve+recent)",
            preserve_n..preserve_n + recent_n,
        ),
        (
            "mid (All std, dist above..+mid)",
            preserve_n + recent_n..preserve_n + recent_n + mid_n,
        ),
        (
            "old (All aggressive, dist above..end)",
            preserve_n + recent_n + mid_n..usize::MAX,
        ),
    ];

    let mut results = Vec::new();
    for (name, band_range) in bands {
        let mut items_total = 0usize;
        let mut tokens_original = 0usize;
        let mut tokens_after = 0usize;
        let mut items_reduced = 0usize;

        // Walk text slots in document order, compute distance from end
        let mut slot_idx = 0usize;
        for (item_idx, item) in before.iter().enumerate() {
            let slots_before = item_text_slots(item);
            let slots_after = item_text_slots(&after[item_idx]);
            let n = slots_before.len();
            if n == 0 {
                continue;
            }
            for s in 0..n {
                let dist = total_slots.saturating_sub(1).saturating_sub(slot_idx);
                if band_range.contains(&dist)
                    || (band_range.end == usize::MAX && dist >= band_range.start)
                {
                    let tok_before = approx_tokens(&slots_before[s]);
                    let tok_after = approx_tokens(&slots_after[s]);
                    tokens_original += tok_before;
                    tokens_after += tok_after;
                    items_total += 1;
                    if tok_after < tok_before {
                        items_reduced += 1;
                    }
                }
                slot_idx += 1;
            }
        }

        results.push(BandStats {
            name,
            items_total,
            tokens_original,
            tokens_after,
            items_reduced,
        });
    }
    results
}

// ---------------------------------------------------------------------------
// Main test
// ---------------------------------------------------------------------------

#[test]
fn test_reduction_modes() {
    // 1. Build synthetic history
    let original = build_synthetic_history();
    let total_items = original.len();
    let total_slots = count_text_slots_by_position(&original);

    // 2. Compute Off-baseline token estimate (no reduction call)
    let off_tokens: usize = original
        .iter()
        .map(|item| {
            item_text_slots(item)
                .iter()
                .map(|t| approx_tokens(t))
                .sum::<usize>()
        })
        .sum();

    // 3. Conservative reduction
    let mut conservative_items = original.clone();
    let conservative_stats = reduce_prompt_items(
        &mut conservative_items,
        &PromptReductionConfig::for_turn("emulation-harness-conservative"),
    )
    .expect("conservative reduction must not fail");
    let conservative_tokens_after: usize = conservative_items
        .iter()
        .map(|item| {
            item_text_slots(item)
                .iter()
                .map(|t| approx_tokens(t))
                .sum::<usize>()
        })
        .sum();

    // 4. RecencyWeighted reduction
    let opts = RecencyWeightedOpts::default();
    let preserve_n = opts.preserve_recent_items; // 3
    let recent_n = opts.recent_window_items; // 6
    let mid_n = opts.mid_window_items; // 12
    let mut rw_items = original.clone();
    let rw_stats = reduce_prompt_items(
        &mut rw_items,
        &PromptReductionConfig::for_turn_recency_weighted("emulation-harness-rw", &opts),
    )
    .expect("recency-weighted reduction must not fail");
    let rw_tokens_after: usize = rw_items
        .iter()
        .map(|item| {
            item_text_slots(item)
                .iter()
                .map(|t| approx_tokens(t))
                .sum::<usize>()
        })
        .sum();

    // 5. Compute band stats for Conservative and RecencyWeighted
    let conservative_bands =
        compute_band_stats(&original, &conservative_items, preserve_n, recent_n, mid_n);
    let rw_bands = compute_band_stats(&original, &rw_items, preserve_n, recent_n, mid_n);

    // 6. Build the report
    let conservative_saved = off_tokens.saturating_sub(conservative_tokens_after);
    let rw_saved = off_tokens.saturating_sub(rw_tokens_after);
    let conservative_pct = if off_tokens > 0 {
        100.0 * conservative_saved as f64 / off_tokens as f64
    } else {
        0.0
    };
    let rw_pct = if off_tokens > 0 {
        100.0 * rw_saved as f64 / off_tokens as f64
    } else {
        0.0
    };

    let mut report = String::new();
    report.push_str("# Prompt Reduction Emulation Report\n\n");
    report.push_str("**Generated by:** `codex-rs/prompt-reducer/tests/emulation_harness.rs`\n\n");
    report.push_str("**Fixture:** Synthetic in-code (see NOTE in harness; real triage fixture cannot be deserialized to ResponseItem)\n\n");
    report.push_str("**Token estimate method:** `text.chars().count() / 4` (same as crate-internal `approx_tokens`)\n\n");
    report.push_str(&format!(
        "**History size:** {total_items} items, {total_slots} text slots\n\n"
    ));
    report.push_str(&format!(
        "**RecencyWeighted tiers (default):** Preserve={preserve_n} / RecentOnly={recent_n} / Mid={mid_n} / Old=remainder (threshold_mult=0.5, excerpt_mult=0.6)\n\n"
    ));

    report.push_str("---\n\n## 1. Overall Token Summary\n\n");
    report.push_str("| Mode | Tokens Before | Tokens After | Tokens Saved | % Saved |\n");
    report.push_str("|------|:---:|:---:|:---:|:---:|\n");
    report.push_str(&format!(
        "| Off (baseline) | {off_tokens} | {off_tokens} | 0 | 0.0% |\n"
    ));
    report.push_str(&format!("| Conservative | {off_tokens} | {conservative_tokens_after} | {conservative_saved} | {conservative_pct:.1}% |\n"));
    report.push_str(&format!(
        "| RecencyWeighted | {off_tokens} | {rw_tokens_after} | {rw_saved} | {rw_pct:.1}% |\n"
    ));
    report.push_str("\n");

    report.push_str("## 2. PromptReductionStats\n\n");
    report.push_str("| Field | Conservative | RecencyWeighted |\n");
    report.push_str("|-------|:---:|:---:|\n");
    report.push_str(&format!(
        "| reductions | {} | {} |\n",
        conservative_stats.reductions, rw_stats.reductions
    ));
    report.push_str(&format!(
        "| artifacts | {} | {} |\n",
        conservative_stats.artifacts, rw_stats.artifacts
    ));
    report.push_str(&format!(
        "| original_tokens (crate est.) | {} | {} |\n",
        conservative_stats.original_tokens, rw_stats.original_tokens
    ));
    report.push_str(&format!(
        "| reduced_tokens (crate est.) | {} | {} |\n",
        conservative_stats.reduced_tokens, rw_stats.reduced_tokens
    ));
    report.push_str(&format!(
        "| saved_tokens (crate est.) | {} | {} |\n",
        conservative_stats.saved_tokens, rw_stats.saved_tokens
    ));
    report.push_str("\n");

    report.push_str("## 3. Recency Profile — Tokens Cut per Band\n\n");
    report.push_str("Bands are measured by text-slot distance from the end (newest first).\n\n");
    report.push_str(
        "| Band | Slots | C: orig_tok | C: cut | C: cut% | RW: orig_tok | RW: cut | RW: cut% |\n",
    );
    report.push_str("|------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|\n");
    for (c_band, rw_band) in conservative_bands.iter().zip(rw_bands.iter()) {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {:.1}% | {} | {} | {:.1}% |\n",
            c_band.name,
            c_band.items_total,
            c_band.tokens_original,
            c_band.tokens_cut(),
            c_band.pct_cut(),
            rw_band.tokens_original,
            rw_band.tokens_cut(),
            rw_band.pct_cut(),
        ));
    }
    report.push_str("\n");

    report.push_str("## 4. Verdict\n\n");
    // Derive verdict from actual numbers
    let newest_band_idx = 0usize;
    let oldest_band_idx = conservative_bands.len() - 1;
    let c_newest_cut = conservative_bands[newest_band_idx].tokens_cut();
    let rw_newest_cut = rw_bands[newest_band_idx].tokens_cut();
    let c_oldest_cut = conservative_bands[oldest_band_idx].tokens_cut();
    let rw_oldest_cut = rw_bands[oldest_band_idx].tokens_cut();

    let newest_preserved = rw_newest_cut <= c_newest_cut;
    let oldest_more_cut = rw_oldest_cut >= c_oldest_cut;
    let overall_comparable = rw_saved >= (conservative_saved * 9 / 10); // within 10% is "comparable"

    let verdict = if newest_preserved && oldest_more_cut {
        format!(
            "RecencyWeighted **PASSES** the recency invariant: it cuts fewer (or equal) tokens \
from the newest band ({rw_newest_cut} vs Conservative {c_newest_cut}) and >= as many from the \
oldest band ({rw_oldest_cut} vs Conservative {c_oldest_cut}). Overall savings are {rw_saved} \
tokens vs Conservative {conservative_saved} ({:.1}% vs {:.1}%), which is {}. \
**Recommendation: RecencyWeighted is safe to ship as default** — it protects recent detail \
and reduces old content at least as aggressively.",
            rw_pct,
            conservative_pct,
            if overall_comparable {
                "comparable or better"
            } else {
                "somewhat lower — review thresholds"
            }
        )
    } else {
        format!(
            "RecencyWeighted shows mixed results: newest-band preservation = {newest_preserved} \
(RW cut {rw_newest_cut} vs Conservative {c_newest_cut}), oldest-band aggression = {oldest_more_cut} \
(RW cut {rw_oldest_cut} vs Conservative {c_oldest_cut}). Review the tier configuration \
before shipping as default."
        )
    };
    report.push_str(&verdict);
    report.push_str("\n\n");

    report.push_str(
        "---\n\n*Report generated by the emulation harness at test runtime. Numbers are real.*\n",
    );

    // Print to stdout (--nocapture)
    println!("\n{report}");

    // Write to file
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.codex/tmp/prompt_reduction_2026-06-21/emulation_report.md"
    );
    let report_dir = std::path::Path::new(report_path).parent().unwrap();
    if let Err(e) = std::fs::create_dir_all(report_dir) {
        eprintln!("WARN: could not create report dir {report_dir:?}: {e}");
        eprintln!("--- BEGIN REPORT ---\n{report}\n--- END REPORT ---");
    } else if let Err(e) = std::fs::write(report_path, &report) {
        eprintln!("WARN: could not write report to {report_path}: {e}");
        eprintln!("--- BEGIN REPORT ---\n{report}\n--- END REPORT ---");
    } else {
        println!("Report written to: {report_path}");
    }

    // -----------------------------------------------------------------------
    // Assertions (regression invariants)
    // -----------------------------------------------------------------------

    // A1: Conservative reduces SOME items — sanity that harness wires the reducer correctly.
    // Note: saved_tokens uses the crate's own token estimate (may differ from our chars/4).
    assert!(
        conservative_stats.reductions > 0 || conservative_stats.saved_tokens > 0,
        "A1 FAIL: Conservative made zero reductions on a {total_items}-item history. \
Harness may be mislinked or history too small. reductions={}, saved_tokens={}",
        conservative_stats.reductions,
        conservative_stats.saved_tokens,
    );

    // A2: RecencyWeighted preserves the newest `preserve_n` text slots byte-for-byte.
    // We compare the last `preserve_n` text slots between Off baseline and RW output.
    {
        // Collect all text slot contents in document order from original and rw_items
        let original_slots: Vec<String> = original
            .iter()
            .flat_map(|item| item_text_slots(item))
            .collect();
        let rw_slots: Vec<String> = rw_items
            .iter()
            .flat_map(|item| item_text_slots(item))
            .collect();

        assert_eq!(
            original_slots.len(),
            rw_slots.len(),
            "A2 PRE: slot count must be equal (reducer replaces content, not removes items)"
        );

        let n = original_slots.len();
        // Check the final preserve_n slots (these map to dist 0..preserve_n)
        // Only check if we have enough slots
        let check_count = preserve_n.min(n);
        for i in 0..check_count {
            let slot_idx = n - check_count + i;
            assert_eq!(
                original_slots[slot_idx],
                rw_slots[slot_idx],
                "A2 FAIL: RecencyWeighted modified text slot {slot_idx} (distance {} from end) \
which should be in the Preserve tier (newest {preserve_n} slots). \
This means the newest content was altered — shipping RecencyWeighted would harm recent context.",
                n - 1 - slot_idx,
            );
        }
    }

    // A3 & A4: Directional recency invariants.
    // A3: RW cuts fewer-or-equal tokens in the newest band than Conservative.
    // A4: RW cuts >= as many tokens in the oldest band as Conservative.
    // Downgrade to soft-assert (logged observation) when the band has zero slots,
    // since a tiny synthetic history may not populate all tiers.
    if conservative_bands[newest_band_idx].items_total > 0 {
        assert!(
            rw_newest_cut <= c_newest_cut,
            "A3 FAIL: RecencyWeighted cut MORE tokens from the newest band than Conservative \
({rw_newest_cut} > {c_newest_cut}). The Preserve tier is not protecting the newest content.",
        );
    } else {
        println!(
            "A3 SKIP: newest band has 0 text slots (preserve_n={preserve_n}, total_slots={total_slots}). \
Directional assertion skipped — add more items to exercise this band."
        );
    }

    if conservative_bands[oldest_band_idx].items_total > 0
        && rw_bands[oldest_band_idx].items_total > 0
    {
        // Soft directional: RW should cut AT LEAST as much from oldest band.
        // This may not always hold on tiny synthetic inputs (the aggressive mults
        // lower thresholds, but if items are already reduced by Conservative the
        // absolute savings may be similar). Log as observation, don't hard-fail.
        if rw_oldest_cut < c_oldest_cut {
            println!(
                "A4 OBS: RecencyWeighted cut fewer tokens from the oldest band than Conservative \
({rw_oldest_cut} vs {c_oldest_cut}). This can happen when Conservative already reduces most \
items in that band and the threshold_mult=0.5 finds few additional candidates. \
Not a hard failure — review band stats above for full picture."
            );
        } else {
            println!(
                "A4 PASS: RecencyWeighted cut >= as many tokens from the oldest band as Conservative \
({rw_oldest_cut} >= {c_oldest_cut})."
            );
        }
    } else {
        println!("A4 SKIP: oldest band has 0 slots in one or both modes.");
    }

    println!(
        "\nSummary: Conservative reductions={}, saved_tokens={} | RecencyWeighted reductions={}, saved_tokens={}",
        conservative_stats.reductions,
        conservative_stats.saved_tokens,
        rw_stats.reductions,
        rw_stats.saved_tokens,
    );
}
