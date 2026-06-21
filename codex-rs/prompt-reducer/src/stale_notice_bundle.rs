use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use crate::PromptReductionStats;
use crate::approx_tokens;
use crate::artifact_path_for;
use crate::sha1_hex;
use crate::write_artifact;

const STALE_REDUCTION_NOTICE_BUNDLE_REASON: &str = "stale_reduction_notice_bundle";
const STALE_REDUCTION_NOTICE_BUNDLE_MIN_ITEMS: usize = 2;
const SOURCE_ACCESS_HISTORY_MAX_ENTRIES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
struct StaleReductionNotice {
    text_slot_index: usize,
    reason: String,
    text: String,
    tokens: usize,
}

pub(crate) fn bundle_stale_reduction_notices(
    items: &mut [ResponseItem],
    artifact_dir: &Path,
    recent_text_start: usize,
    stats: &mut PromptReductionStats,
) -> std::io::Result<()> {
    let notices = stale_reduction_notices(items, recent_text_start);
    if notices.len() < STALE_REDUCTION_NOTICE_BUNDLE_MIN_ITEMS {
        return Ok(());
    }

    let artifact_text = render_artifact(&notices);
    let sha1 = sha1_hex(&artifact_text);
    let first_index = notices
        .first()
        .map(|notice| notice.text_slot_index)
        .unwrap_or(0);
    let artifact_path = artifact_path_for(
        artifact_dir,
        first_index,
        STALE_REDUCTION_NOTICE_BUNDLE_REASON,
        &sha1,
    );
    let replacement = render_replacement(&notices, &artifact_path);
    write_artifact(&artifact_path, &artifact_text)?;

    let notice_indices = notices
        .iter()
        .map(|notice| notice.text_slot_index)
        .collect::<BTreeSet<_>>();
    apply_bundle(items, &notice_indices, first_index, &replacement);

    let original_tokens = notices.iter().map(|notice| notice.tokens).sum::<usize>();
    stats.reduced_tokens = stats
        .reduced_tokens
        .saturating_sub(original_tokens)
        .saturating_add(approx_tokens(&replacement));
    stats.artifacts += 1;
    stats.reductions += notices.len();
    Ok(())
}

fn stale_reduction_notices(
    items: &[ResponseItem],
    recent_text_start: usize,
) -> Vec<StaleReductionNotice> {
    let mut notices = Vec::new();
    let mut text_slot_zero = 0usize;
    for item in items {
        match item {
            ResponseItem::Message { role, content, .. } => {
                for content_item in content {
                    let text = match content_item {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => text,
                        ContentItem::InputImage { .. } => continue,
                    };
                    text_slot_zero += 1;
                    if role == "assistant" {
                        push_stale_notice(
                            &mut notices,
                            text_slot_zero,
                            text_slot_zero - 1,
                            recent_text_start,
                            text,
                        );
                    }
                }
            }
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                collect_function_output_notices(
                    output,
                    recent_text_start,
                    &mut text_slot_zero,
                    &mut notices,
                );
            }
            ResponseItem::ToolSearchOutput { tools, .. } => {
                text_slot_zero += usize::from(!tools.is_empty());
            }
            _ => {}
        }
    }
    notices
}

fn collect_function_output_notices(
    output: &FunctionCallOutputPayload,
    recent_text_start: usize,
    text_slot_zero: &mut usize,
    notices: &mut Vec<StaleReductionNotice>,
) {
    if let Some(text) = output.text_content() {
        *text_slot_zero += 1;
        push_stale_notice(
            notices,
            *text_slot_zero,
            *text_slot_zero - 1,
            recent_text_start,
            text,
        );
        return;
    }

    let Some(content_items) = output.content_items() else {
        return;
    };
    for content_item in content_items {
        let FunctionCallOutputContentItem::InputText { text } = content_item else {
            continue;
        };
        *text_slot_zero += 1;
        push_stale_notice(
            notices,
            *text_slot_zero,
            *text_slot_zero - 1,
            recent_text_start,
            text,
        );
    }
}

fn push_stale_notice(
    notices: &mut Vec<StaleReductionNotice>,
    text_slot_index: usize,
    text_slot_zero: usize,
    recent_text_start: usize,
    text: &str,
) {
    if text_slot_zero >= recent_text_start {
        return;
    }
    let Some(reason) = eligible_notice_reason(text) else {
        return;
    };
    notices.push(StaleReductionNotice {
        text_slot_index,
        reason: reason.to_string(),
        text: text.to_string(),
        tokens: approx_tokens(text),
    });
}

fn eligible_notice_reason(text: &str) -> Option<&str> {
    if !has_artifact_recovery_semantics(text) || has_preserved_evidence(text) {
        return None;
    }

    let reason = prompt_reduction_reason(text)?;
    if low_utility_recoverable_reason(reason) {
        Some(reason)
    } else {
        None
    }
}

fn prompt_reduction_reason(text: &str) -> Option<&str> {
    let line = text.trim_start().lines().next()?.trim();
    let reason = line.strip_prefix("[prompt reduction: ")?;
    reason.strip_suffix(']').map(str::trim)
}

fn has_artifact_recovery_semantics(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("[prompt reduction:")
        && trimmed.contains("artifact:")
        && trimmed.contains("recovery:")
}

fn low_utility_recoverable_reason(reason: &str) -> bool {
    matches!(
        reason,
        "source_read_digest"
            | "command_log_digest"
            | "path_inventory"
            | "path_inventory_digest"
            | "search_result_digest"
            | "short_tool_output_bundle"
            | "recoverable_prior_context_digest"
            | "recent_source_read_digest"
            | "recent_command_log_digest"
            | "recent_path_inventory"
            | "recent_path_inventory_digest"
            | "recent_search_result_digest"
    )
}

fn has_preserved_evidence(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if nonzero_exit_code(&lower) {
        return true;
    }
    const PRESERVE_MARKERS: &[&str] = &[
        "approval",
        "build failed",
        "compiler diagnostic",
        "context_pack",
        "diagnostic",
        "error:",
        "failed tests",
        "failure",
        "finding",
        "lint failed",
        "nonzero",
        "panic",
        "safety",
        "test failure",
        "tests failed",
    ];
    PRESERVE_MARKERS.iter().any(|marker| lower.contains(marker))
}

fn nonzero_exit_code(lower_text: &str) -> bool {
    lower_text.lines().any(|line| {
        let Some((_, after)) = line.split_once("exit code:") else {
            return false;
        };
        let code = after
            .trim_start()
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
            .collect::<String>();
        !matches!(code.as_str(), "" | "0")
    })
}

fn render_replacement(notices: &[StaleReductionNotice], artifact_path: &Path) -> String {
    let original_tokens = notices.iter().map(|notice| notice.tokens).sum::<usize>();
    let reason_counts = format_reason_counts(notices);
    let mut output = format!(
        "[prompt reduction: {STALE_REDUCTION_NOTICE_BUNDLE_REASON}]\n\
         original_items: {}\n\
         original_tokens_estimate: {original_tokens}\n\
         reason_counts: {reason_counts}\n\
         artifact: `{}`\n\
         recovery: read artifact before using exact prior reduction notices.\n\
         selection: old low-utility prompt-reduction notices with artifact/recovery semantics only.\n\
         keep_inline: current instructions, recent verification, actionable failures/diagnostics, findings, context packs, approvals/safety, and API/behavior decisions.",
        notices.len(),
        artifact_path.display()
    );
    if let Some(source_access_history) = format_source_access_history(notices) {
        output.push_str("\n\n");
        output.push_str(&source_access_history);
    }
    output
}

fn render_artifact(notices: &[StaleReductionNotice]) -> String {
    let mut output = String::new();
    writeln!(&mut output, "stale_reduction_notice_bundle_artifact").unwrap();
    writeln!(&mut output, "items: {}", notices.len()).unwrap();
    writeln!(
        &mut output,
        "selection_policy: bundled old low-utility prompt-reduction notices that can be recovered from their own artifacts."
    )
    .unwrap();
    writeln!(
        &mut output,
        "keep_inline_policy: keep current task instructions, recent verification, actionable failures, exact diagnostics, current plans/handoffs, final findings, context packs, approvals/safety, and API/behavior decisions inline."
    )
    .unwrap();
    for (index, notice) in notices.iter().enumerate() {
        writeln!(
            &mut output,
            "\n--- notice {} text_slot={} reason={} ---",
            index + 1,
            notice.text_slot_index,
            notice.reason
        )
        .unwrap();
        output.push_str(&notice.text);
        if !notice.text.ends_with('\n') {
            output.push('\n');
        }
        writeln!(&mut output, "--- end notice {} ---", index + 1).unwrap();
    }
    output
}

fn format_source_access_history(notices: &[StaleReductionNotice]) -> Option<String> {
    let mut entries = BTreeSet::<String>::new();
    for notice in notices {
        let Some(ledger) = access_ledger_line(&notice.text) else {
            continue;
        };
        let artifact = artifact_reference(&notice.text).unwrap_or("(unknown)");
        entries.insert(format!(
            "text-slot-{} reason={}; {}; artifact={}",
            notice.text_slot_index, notice.reason, ledger, artifact
        ));
    }
    if entries.is_empty() {
        return None;
    }

    let total = entries.len();
    let shown = entries
        .into_iter()
        .take(SOURCE_ACCESS_HISTORY_MAX_ENTRIES)
        .collect::<Vec<_>>();
    let omitted = total.saturating_sub(shown.len());
    let mut output = format!(
        "source_access_history\nentries_total: {total}\nrepeated_access_guard: compare future reads/searches against these entries and recover artifacts before repeating exact or overlapping source access.\nentries:"
    );
    for entry in shown {
        output.push_str("\n- ");
        output.push_str(&entry);
    }
    if omitted > 0 {
        output.push_str(&format!("\n- ... +{omitted} more"));
    }
    Some(output)
}

fn access_ledger_line(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with("access_ledger:"))
}

fn artifact_reference(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("artifact:").map(str::trim))
}

fn format_reason_counts(notices: &[StaleReductionNotice]) -> String {
    let mut counts = BTreeMap::<&str, usize>::new();
    for notice in notices {
        *counts.entry(&notice.reason).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(reason, count)| format!("{reason}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn apply_bundle(
    items: &mut [ResponseItem],
    notice_indices: &BTreeSet<usize>,
    first_index: usize,
    replacement: &str,
) {
    let mut text_slot_index = 0usize;
    for item in items {
        match item {
            ResponseItem::Message { content, .. } => {
                for content_item in content {
                    let text = match content_item {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => text,
                        ContentItem::InputImage { .. } => continue,
                    };
                    text_slot_index += 1;
                    replace_notice_text(
                        text,
                        text_slot_index,
                        notice_indices,
                        first_index,
                        replacement,
                    );
                }
            }
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                apply_function_output_bundle(
                    output,
                    notice_indices,
                    first_index,
                    replacement,
                    &mut text_slot_index,
                );
            }
            ResponseItem::ToolSearchOutput { tools, .. } => {
                text_slot_index += usize::from(!tools.is_empty());
            }
            _ => {}
        }
    }
}

fn apply_function_output_bundle(
    output: &mut FunctionCallOutputPayload,
    notice_indices: &BTreeSet<usize>,
    first_index: usize,
    replacement: &str,
    text_slot_index: &mut usize,
) {
    if let Some(text) = output.text_content_mut() {
        *text_slot_index += 1;
        replace_notice_text(
            text,
            *text_slot_index,
            notice_indices,
            first_index,
            replacement,
        );
        return;
    }

    let Some(content_items) = output.content_items_mut() else {
        return;
    };
    for content_item in content_items {
        let FunctionCallOutputContentItem::InputText { text } = content_item else {
            continue;
        };
        *text_slot_index += 1;
        replace_notice_text(
            text,
            *text_slot_index,
            notice_indices,
            first_index,
            replacement,
        );
    }
}

fn replace_notice_text(
    text: &mut String,
    text_slot_index: usize,
    notice_indices: &BTreeSet<usize>,
    first_index: usize,
    replacement: &str,
) {
    if !notice_indices.contains(&text_slot_index) {
        return;
    }
    if text_slot_index == first_index {
        *text = replacement.to_string();
    } else {
        text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputPayload;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn bundles_old_low_utility_notices_with_artifact_recovery() {
        let old_source = notice("source_read_digest", "old-source");
        let old_search = notice("search_result_digest", "old-search");
        let recent_command = notice("recent_command_log_digest", "recent-command");
        let mut items = vec![
            output("old-source", old_source.clone()),
            output("old-search", old_search.clone()),
            output("recent-command", recent_command.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let mut stats = stats_for_current_items(&items);

        bundle_stale_reduction_notices(&mut items, temp.path(), 2, &mut stats).unwrap();

        let texts = output_texts(&items);
        assert!(texts[0].contains("[prompt reduction: stale_reduction_notice_bundle]"));
        assert!(texts[0].contains("reason_counts: search_result_digest=1, source_read_digest=1"));
        assert!(texts[0].contains("artifact: `"));
        assert!(
            texts[0]
                .contains("recovery: read artifact before using exact prior reduction notices.")
        );
        assert_eq!("", texts[1]);
        assert_eq!(recent_command, texts[2]);

        let artifact = only_artifact(temp.path());
        let artifact_text = std::fs::read_to_string(artifact).unwrap();
        assert!(artifact_text.contains(&old_source));
        assert!(artifact_text.contains(&old_search));
        assert!(!artifact_text.contains(&recent_command));
        assert!(artifact_text.contains("keep_inline_policy:"));
    }

    #[test]
    fn leaves_single_eligible_notice_inline() {
        let old_source = notice("source_read_digest", "old-source");
        let mut items = vec![output("old-source", old_source.clone())];
        let temp = TempDir::new().unwrap();
        let mut stats = stats_for_current_items(&items);

        bundle_stale_reduction_notices(&mut items, temp.path(), 1, &mut stats).unwrap();

        assert_eq!(vec![old_source], output_texts(&items));
        assert!(std::fs::read_dir(temp.path()).unwrap().next().is_none());
    }

    #[test]
    fn leaves_source_access_ledger_notices_inline() {
        let old_source = format!(
            "{}\naccess_ledger: kind=read; source=shell_output:Get-Content src/lib.rs; paths=src/lib.rs; requested_lines=1-80; result_lines=80; result_chars=3200",
            notice("source_read_digest", "old-source")
        );
        let old_search = notice("search_result_digest", "old-search");
        let mut items = vec![
            output("old-source", old_source.clone()),
            output("old-search", old_search.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let mut stats = stats_for_current_items(&items);

        bundle_stale_reduction_notices(&mut items, temp.path(), 1, &mut stats).unwrap();

        assert_eq!(vec![old_source, old_search], output_texts(&items));
        assert!(std::fs::read_dir(temp.path()).unwrap().next().is_none());
    }

    #[test]
    fn leaves_recent_eligible_notices_inline() {
        let recent_source = notice("source_read_digest", "recent-source");
        let recent_command = notice("command_log_digest", "recent-command");
        let mut items = vec![
            output("recent-source", recent_source.clone()),
            output("recent-command", recent_command.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let mut stats = stats_for_current_items(&items);

        bundle_stale_reduction_notices(&mut items, temp.path(), 0, &mut stats).unwrap();

        assert_eq!(vec![recent_source, recent_command], output_texts(&items));
        assert!(std::fs::read_dir(temp.path()).unwrap().next().is_none());
    }

    #[test]
    fn preserves_failure_findings_and_context_pack_notices() {
        let old_source = notice("source_read_digest", "old-source");
        let failed_command = format!(
            "{}\nstatus_lines: Exit code: 1\nerror: tests failed",
            notice("command_log_digest", "failed-command")
        );
        let findings = notice("assistant_findings_digest", "findings");
        let context_pack = notice("context_pack_digest", "context-pack");
        let mut items = vec![
            output("old-source", old_source.clone()),
            output("failed-command", failed_command.clone()),
            output("findings", findings.clone()),
            output("context-pack", context_pack.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let mut stats = stats_for_current_items(&items);

        bundle_stale_reduction_notices(&mut items, temp.path(), 4, &mut stats).unwrap();

        assert_eq!(
            vec![old_source, failed_command, findings, context_pack],
            output_texts(&items)
        );
        assert!(std::fs::read_dir(temp.path()).unwrap().next().is_none());
    }

    #[test]
    fn preserves_user_developer_and_system_message_notices() {
        let developer_notice = notice("source_read_digest", "developer-instruction");
        let old_source = notice("source_read_digest", "old-source");
        let old_search = notice("search_result_digest", "old-search");
        let mut items = vec![
            message("developer", developer_notice.clone()),
            output("old-source", old_source.clone()),
            output("old-search", old_search.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let mut stats = stats_for_current_items(&items);

        bundle_stale_reduction_notices(&mut items, temp.path(), 3, &mut stats).unwrap();

        let texts = all_texts(&items);
        assert_eq!(developer_notice, texts[0]);
        assert!(texts[1].contains("[prompt reduction: stale_reduction_notice_bundle]"));
        assert_eq!("", texts[2]);

        let artifact = only_artifact(temp.path());
        let artifact_text = std::fs::read_to_string(artifact).unwrap();
        assert!(!artifact_text.contains("developer-instruction"));
        assert!(artifact_text.contains(&old_source));
        assert!(artifact_text.contains(&old_search));
    }

    #[test]
    fn stale_bundle_keeps_source_access_history_inline() {
        let old_source = format!(
            "{}\naccess_ledger: kind=read; source=shell_output:Get-Content src/lib.rs; paths=src/lib.rs; requested_lines=1-80; result_lines=80; result_chars=3200",
            notice("source_read_digest", "old-source")
        );
        let old_search = format!(
            "{}\naccess_ledger: kind=search; source=shell_output:rg -n \"needle\" src tests; paths=src/lib.rs, tests/test.rs; requested_lines=(unknown); result_lines=4; result_chars=420",
            notice("search_result_digest", "old-search")
        );
        let mut items = vec![
            output("old-source", old_source.clone()),
            output("old-search", old_search.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let mut stats = stats_for_current_items(&items);

        bundle_stale_reduction_notices(&mut items, temp.path(), 3, &mut stats).unwrap();

        let texts = output_texts(&items);
        assert!(texts[0].contains("[prompt reduction: stale_reduction_notice_bundle]"));
        assert!(texts[0].contains("source_access_history"));
        assert!(texts[0].contains("entries_total: 2"));
        assert!(texts[0].contains("requested_lines=1-80"));
        assert!(texts[0].contains("kind=search"));
        assert!(texts[0].contains("artifact=`C:\\Temp\\old-source.txt`"));
        assert_eq!("", texts[1]);

        let artifact = only_artifact(temp.path());
        let artifact_text = std::fs::read_to_string(artifact).unwrap();
        assert!(artifact_text.contains(&old_source));
        assert!(artifact_text.contains(&old_search));
    }

    fn notice(reason: &str, marker: &str) -> String {
        format!(
            "[prompt reduction: {reason}]\n\
             original_chars: 2048\n\
             original_tokens_estimate: 512\n\
             artifact: `C:\\Temp\\{marker}.txt`\n\
             recovery: read artifact before using exact lines.\n\n\
             {reason}\n\
             marker: {marker}"
        )
    }

    fn output(call_id: &str, text: String) -> ResponseItem {
        ResponseItem::FunctionCallOutput {
            id: None,
            call_id: call_id.to_string(),
            output: FunctionCallOutputPayload::from_text(text),
            metadata: None,
        }
    }

    fn message(role: &str, text: String) -> ResponseItem {
        ResponseItem::Message {
            role: role.to_string(),
            content: vec![ContentItem::InputText { text }],
            phase: None,
            id: None,
            metadata: None,
        }
    }

    fn all_texts(items: &[ResponseItem]) -> Vec<String> {
        items
            .iter()
            .filter_map(|item| match item {
                ResponseItem::Message { content, .. } => content.first().and_then(|content_item| {
                    let ContentItem::InputText { text } = content_item else {
                        return None;
                    };
                    Some(text.clone())
                }),
                ResponseItem::FunctionCallOutput { output, .. } => {
                    output.text_content().map(ToString::to_string)
                }
                _ => None,
            })
            .collect()
    }

    fn output_texts(items: &[ResponseItem]) -> Vec<String> {
        items
            .iter()
            .map(|item| {
                let ResponseItem::FunctionCallOutput { output, .. } = item else {
                    panic!("expected function output");
                };
                output.text_content().unwrap().to_string()
            })
            .collect()
    }

    fn stats_for_current_items(items: &[ResponseItem]) -> PromptReductionStats {
        let original_tokens = all_texts(items)
            .iter()
            .map(|text| approx_tokens(text))
            .sum();
        PromptReductionStats {
            original_tokens,
            reduced_tokens: original_tokens,
            saved_tokens: 0,
            artifacts: 0,
            reductions: 0,
        }
    }

    fn only_artifact(path: &Path) -> std::path::PathBuf {
        let artifacts = std::fs::read_dir(path)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert_eq!(1, artifacts.len());
        artifacts.into_iter().next().unwrap()
    }
}
