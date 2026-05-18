use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use serde_json::Value;
use sha1::Digest;
use sha1::Sha1;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const DEFAULT_MIN_REDUCE_CHARS: usize = 2_000;
const DEFAULT_PATH_LIST_THRESHOLD: usize = 12;
const DEFAULT_MIN_SAVED_TOKENS: usize = 128;
const DEFAULT_PRESERVE_RECENT_ITEMS: usize = 4;
const EXCERPT_CHARS: usize = 220;
const SHORT_TOOL_BUNDLE_MIN_ITEMS: usize = 4;
const SHORT_TOOL_BUNDLE_MIN_TOKENS: usize = 384;
const SHORT_TOOL_ITEM_MIN_TOKENS: usize = 12;
const SHORT_TOOL_ITEM_MAX_TOKENS: usize = 160;
const SHORT_ASSISTANT_STATUS_BUNDLE_MIN_ITEMS: usize = 4;
const SHORT_ASSISTANT_STATUS_BUNDLE_MIN_TOKENS: usize = 160;
const SHORT_ASSISTANT_STATUS_ITEM_MIN_TOKENS: usize = 8;
const SHORT_ASSISTANT_STATUS_ITEM_MAX_TOKENS: usize = 110;

/// Configuration for prompt-only reduction.
///
/// The reducer mutates only the prompt clone sent to the model. Callers must
/// keep persisted rollout history unchanged so exact artifacts can be
/// recovered or re-read on demand.
#[derive(Debug, Clone)]
pub struct PromptReductionConfig {
    pub artifact_dir: PathBuf,
    pub min_reduce_chars: usize,
    pub path_list_threshold: usize,
    pub min_saved_tokens: usize,
    pub preserve_recent_items: usize,
}

impl PromptReductionConfig {
    pub fn for_turn(turn_id: &str) -> Self {
        let safe_turn_id = safe_label(turn_id);
        Self {
            artifact_dir: std::env::temp_dir().join("codex-prompt-reducer").join(
                if safe_turn_id.is_empty() {
                    std::process::id().to_string()
                } else {
                    safe_turn_id
                },
            ),
            min_reduce_chars: DEFAULT_MIN_REDUCE_CHARS,
            path_list_threshold: DEFAULT_PATH_LIST_THRESHOLD,
            min_saved_tokens: DEFAULT_MIN_SAVED_TOKENS,
            preserve_recent_items: DEFAULT_PRESERVE_RECENT_ITEMS,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptReductionStats {
    pub original_tokens: usize,
    pub reduced_tokens: usize,
    pub saved_tokens: usize,
    pub artifacts: usize,
    pub reductions: usize,
}

#[derive(Debug, Clone)]
struct CandidateReduction {
    reason: &'static str,
    digest: String,
    disposition: CandidateDisposition,
}

#[derive(Debug, Clone, Copy)]
struct CandidateThreshold {
    min_chars: usize,
    min_saved_tokens: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateDisposition {
    ArtifactReplacement,
    OmitFromPrompt,
}

#[derive(Debug)]
struct ShortToolOutputBundle {
    indices: BTreeSet<usize>,
    first_index: usize,
    artifact_path: PathBuf,
    replacement: String,
}

#[derive(Debug)]
struct ShortAssistantStatusBundle {
    indices: BTreeSet<usize>,
    first_index: usize,
    artifact_path: PathBuf,
    replacement: String,
}

pub fn reduce_prompt_items(
    items: &mut [ResponseItem],
    config: &PromptReductionConfig,
) -> std::io::Result<PromptReductionStats> {
    fs::create_dir_all(&config.artifact_dir)?;
    let total_text_slots = count_text_slots(items);
    let recent_text_start = total_text_slots.saturating_sub(config.preserve_recent_items);
    let mut text_slot_index = 0usize;
    let mut seen_hashes = HashMap::<String, String>::new();
    let mut call_sources = collect_call_sources(items);
    let mut stats = PromptReductionStats::default();
    let short_tool_bundle =
        short_tool_output_bundle(items, config, recent_text_start, &call_sources)?;
    let short_assistant_status_bundle =
        short_assistant_status_bundle(items, config, recent_text_start)?;
    if let Some(bundle) = &short_tool_bundle {
        let artifact_text =
            short_tool_output_bundle_artifact(items, &bundle.indices, &call_sources);
        write_artifact(&bundle.artifact_path, &artifact_text)?;
        seen_hashes.insert(
            sha1_hex(&artifact_text),
            "short_tool_output_bundle".to_string(),
        );
        stats.artifacts += 1;
    }
    if let Some(bundle) = &short_assistant_status_bundle {
        let artifact_text = short_assistant_status_bundle_artifact(items, &bundle.indices);
        write_artifact(&bundle.artifact_path, &artifact_text)?;
        seen_hashes.insert(
            sha1_hex(&artifact_text),
            "short_assistant_status_bundle".to_string(),
        );
        stats.artifacts += 1;
    }

    for item in items {
        match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => {
                call_sources.insert(call_id.clone(), call_source(name, arguments));
            }
            ResponseItem::CustomToolCall {
                call_id,
                name,
                input,
                ..
            } => {
                call_sources.insert(
                    call_id.clone(),
                    format!("custom_tool_output:{name}:{input}"),
                );
            }
            ResponseItem::Message { role, content, .. } => {
                for content_item in content {
                    let text = match content_item {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => text,
                        ContentItem::InputImage { .. } => continue,
                    };
                    let recent_prompt_item = text_slot_index >= recent_text_start;
                    if reduce_short_assistant_status_bundle_slot(
                        text,
                        text_slot_index,
                        short_assistant_status_bundle.as_ref(),
                        &mut stats,
                    ) {
                        text_slot_index += 1;
                        continue;
                    }
                    text_slot_index += 1;
                    reduce_text_slot(
                        text,
                        &format!("message:{role}"),
                        text_slot_index,
                        recent_prompt_item,
                        config,
                        &mut seen_hashes,
                        &mut stats,
                    )?;
                }
            }
            ResponseItem::FunctionCallOutput {
                call_id, output, ..
            }
            | ResponseItem::CustomToolCallOutput {
                call_id, output, ..
            } => {
                let source = call_sources
                    .get(call_id)
                    .cloned()
                    .unwrap_or_else(|| "tool_output".to_string());
                reduce_function_output_text_slots(
                    output,
                    &source,
                    &mut text_slot_index,
                    recent_text_start,
                    short_tool_bundle.as_ref(),
                    config,
                    &mut seen_hashes,
                    &mut stats,
                )?;
            }
            ResponseItem::ToolSearchOutput { tools, .. } => {
                if !tools.is_empty() {
                    let slot_index = text_slot_index;
                    text_slot_index += 1;
                    reduce_tool_search_output(tools, slot_index, config, &mut stats)?;
                }
            }
            _ => {}
        }
    }

    stats.saved_tokens = stats.original_tokens.saturating_sub(stats.reduced_tokens);
    Ok(stats)
}

fn count_text_slots(items: &[ResponseItem]) -> usize {
    items
        .iter()
        .map(|item| match item {
            ResponseItem::Message { content, .. } => content
                .iter()
                .filter(|content_item| {
                    matches!(
                        content_item,
                        ContentItem::InputText { .. } | ContentItem::OutputText { .. }
                    )
                })
                .count(),
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                function_output_text_slot_count(output)
            }
            ResponseItem::ToolSearchOutput { tools, .. } => usize::from(!tools.is_empty()),
            _ => 0,
        })
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn reduce_function_output_text_slots(
    output: &mut FunctionCallOutputPayload,
    source: &str,
    text_slot_index: &mut usize,
    recent_text_start: usize,
    short_tool_bundle: Option<&ShortToolOutputBundle>,
    config: &PromptReductionConfig,
    seen_hashes: &mut HashMap<String, String>,
    stats: &mut PromptReductionStats,
) -> std::io::Result<()> {
    if let Some(text) = output.text_content_mut() {
        reduce_function_output_text_slot(
            text,
            source,
            text_slot_index,
            recent_text_start,
            short_tool_bundle,
            config,
            seen_hashes,
            stats,
        )?;
        return Ok(());
    }

    let Some(content_items) = output.content_items_mut() else {
        return Ok(());
    };
    for content_item in content_items {
        let FunctionCallOutputContentItem::InputText { text } = content_item else {
            continue;
        };
        reduce_function_output_text_slot(
            text,
            source,
            text_slot_index,
            recent_text_start,
            short_tool_bundle,
            config,
            seen_hashes,
            stats,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn reduce_function_output_text_slot(
    text: &mut String,
    source: &str,
    text_slot_index: &mut usize,
    recent_text_start: usize,
    short_tool_bundle: Option<&ShortToolOutputBundle>,
    config: &PromptReductionConfig,
    seen_hashes: &mut HashMap<String, String>,
    stats: &mut PromptReductionStats,
) -> std::io::Result<()> {
    let recent_prompt_item = *text_slot_index >= recent_text_start;
    *text_slot_index += 1;
    if reduce_short_tool_output_bundle_slot(text, *text_slot_index, short_tool_bundle, stats) {
        return Ok(());
    }
    reduce_text_slot(
        text,
        source,
        *text_slot_index,
        recent_prompt_item,
        config,
        seen_hashes,
        stats,
    )
}

fn collect_call_sources(items: &[ResponseItem]) -> HashMap<String, String> {
    let mut call_sources = HashMap::new();
    for item in items {
        match item {
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => {
                call_sources.insert(call_id.clone(), call_source(name, arguments));
            }
            ResponseItem::CustomToolCall {
                name,
                input,
                call_id,
                ..
            } => {
                call_sources.insert(
                    call_id.clone(),
                    format!("custom_tool_output:{name}:{input}"),
                );
            }
            _ => {}
        }
    }
    call_sources
}

fn short_tool_output_bundle(
    items: &[ResponseItem],
    config: &PromptReductionConfig,
    recent_text_start: usize,
    call_sources: &HashMap<String, String>,
) -> std::io::Result<Option<ShortToolOutputBundle>> {
    let mut indices = BTreeSet::new();
    let mut total_tokens = 0usize;
    let mut text_slot_index = 0usize;
    for item in items {
        match item {
            ResponseItem::Message { content, .. } => {
                text_slot_index += content
                    .iter()
                    .filter(|content_item| {
                        matches!(
                            content_item,
                            ContentItem::InputText { .. } | ContentItem::OutputText { .. }
                        )
                    })
                    .count();
            }
            ResponseItem::FunctionCallOutput { call_id, output }
            | ResponseItem::CustomToolCallOutput {
                call_id, output, ..
            } => {
                visit_function_output_texts(output, |text| {
                    let slot_zero = text_slot_index;
                    text_slot_index += 1;
                    if slot_zero >= recent_text_start {
                        return;
                    }
                    let source = call_sources
                        .get(call_id)
                        .map(String::as_str)
                        .unwrap_or("tool_output");
                    if is_subagent_notification_message(text)
                        || !is_short_recoverable_tool_output(source, text)
                    {
                        return;
                    }
                    let tokens = approx_tokens(text);
                    if !(SHORT_TOOL_ITEM_MIN_TOKENS..=SHORT_TOOL_ITEM_MAX_TOKENS).contains(&tokens)
                    {
                        return;
                    }
                    indices.insert(slot_zero + 1);
                    total_tokens += tokens;
                });
            }
            ResponseItem::ToolSearchOutput { tools, .. } => {
                text_slot_index += usize::from(!tools.is_empty());
            }
            _ => {}
        }
    }

    if indices.len() < SHORT_TOOL_BUNDLE_MIN_ITEMS || total_tokens < SHORT_TOOL_BUNDLE_MIN_TOKENS {
        return Ok(None);
    }
    let artifact_text = short_tool_output_bundle_artifact(items, &indices, call_sources);
    let sha1 = sha1_hex(&artifact_text);
    let artifact_path = artifact_path_for(
        &config.artifact_dir,
        *indices.first().unwrap_or(&0),
        "short_tool_output_bundle",
        &sha1,
    );
    let replacement =
        render_short_tool_output_bundle_replacement(items, &indices, &artifact_path, total_tokens);
    Ok(Some(ShortToolOutputBundle {
        first_index: *indices.first().unwrap_or(&0),
        indices,
        artifact_path,
        replacement,
    }))
}

fn short_assistant_status_bundle(
    items: &[ResponseItem],
    config: &PromptReductionConfig,
    recent_text_start: usize,
) -> std::io::Result<Option<ShortAssistantStatusBundle>> {
    let mut indices = BTreeSet::new();
    let mut total_tokens = 0usize;
    for (slot_index, source, text) in assistant_message_text_slots(items) {
        if slot_index >= recent_text_start {
            continue;
        }
        if !is_short_assistant_status_update(&source, text) {
            continue;
        }
        let tokens = approx_tokens(text);
        if !(SHORT_ASSISTANT_STATUS_ITEM_MIN_TOKENS..=SHORT_ASSISTANT_STATUS_ITEM_MAX_TOKENS)
            .contains(&tokens)
        {
            continue;
        }
        indices.insert(slot_index);
        total_tokens += tokens;
    }
    if indices.len() < SHORT_ASSISTANT_STATUS_BUNDLE_MIN_ITEMS
        || total_tokens < SHORT_ASSISTANT_STATUS_BUNDLE_MIN_TOKENS
    {
        return Ok(None);
    }
    let bundle_text = assistant_message_text_slots(items)
        .into_iter()
        .filter(|(slot_index, _, _)| indices.contains(slot_index))
        .map(|(_, _, text)| text.to_string())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    let sha1 = sha1_hex(&bundle_text);
    let artifact_path = artifact_path_for(
        &config.artifact_dir,
        *indices.first().unwrap_or(&0),
        "short_assistant_status_bundle",
        &sha1,
    );
    let replacement = render_short_assistant_status_bundle_replacement(
        items,
        &indices,
        &artifact_path,
        total_tokens,
    );

    Ok(Some(ShortAssistantStatusBundle {
        first_index: *indices.first().unwrap_or(&0),
        indices,
        artifact_path,
        replacement,
    }))
}

fn reduce_short_tool_output_bundle_slot(
    text: &mut String,
    text_slot_index: usize,
    bundle: Option<&ShortToolOutputBundle>,
    stats: &mut PromptReductionStats,
) -> bool {
    let Some(bundle) = bundle else {
        return false;
    };
    if !bundle.indices.contains(&text_slot_index) {
        return false;
    }

    let original_tokens = approx_tokens(text);
    stats.original_tokens = stats.original_tokens.saturating_add(original_tokens);
    if text_slot_index == bundle.first_index {
        *text = bundle.replacement.clone();
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(approx_tokens(text));
    } else {
        text.clear();
    }
    stats.reductions += 1;
    true
}

fn reduce_short_assistant_status_bundle_slot(
    text: &mut String,
    text_slot_index: usize,
    bundle: Option<&ShortAssistantStatusBundle>,
    stats: &mut PromptReductionStats,
) -> bool {
    let Some(bundle) = bundle else {
        return false;
    };
    if !bundle.indices.contains(&text_slot_index) {
        return false;
    }

    let original_tokens = approx_tokens(text);
    stats.original_tokens = stats.original_tokens.saturating_add(original_tokens);
    if text_slot_index == bundle.first_index {
        *text = bundle.replacement.clone();
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(approx_tokens(text));
    } else {
        text.clear();
    }
    stats.reductions += 1;
    true
}

#[allow(clippy::too_many_arguments)]
fn reduce_text_slot(
    text: &mut String,
    source: &str,
    text_slot_index: usize,
    recent_prompt_item: bool,
    config: &PromptReductionConfig,
    seen_hashes: &mut HashMap<String, String>,
    stats: &mut PromptReductionStats,
) -> std::io::Result<()> {
    let original = text.clone();
    let original_tokens = approx_tokens(&original);
    stats.original_tokens = stats.original_tokens.saturating_add(original_tokens);
    if should_preserve_text_slot_from_reduction(&original) {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }

    let sha1 = sha1_hex(&original);
    let exact_preserve_reason = exact_preserve_reason(source, &original);
    let candidate = classify_candidate(
        source,
        &original,
        config,
        seen_hashes,
        exact_preserve_reason,
        recent_prompt_item,
    );
    seen_hashes.insert(sha1.clone(), format!("text-slot-{text_slot_index}"));

    let Some(candidate) = candidate else {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    };

    let artifact_path = artifact_path_for(
        &config.artifact_dir,
        text_slot_index,
        candidate.reason,
        &sha1,
    );
    if candidate.disposition == CandidateDisposition::OmitFromPrompt {
        if candidate.reason != "single_use_subagent_status_notice" {
            write_artifact(&artifact_path, &original)?;
            stats.artifacts += 1;
        }
        text.clear();
        stats.reductions += 1;
        return Ok(());
    }

    let threshold = candidate_threshold(candidate.reason, config);
    if original.chars().count() < threshold.min_chars {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }

    let replacement = render_replacement(
        candidate.reason,
        &candidate.digest,
        &artifact_path,
        original.chars().count(),
        original_tokens,
        &sha1,
    );
    let reduced_tokens = approx_tokens(&replacement);
    if original_tokens.saturating_sub(reduced_tokens) < threshold.min_saved_tokens {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }

    write_artifact(&artifact_path, &original)?;
    *text = replacement;
    stats.reduced_tokens = stats.reduced_tokens.saturating_add(reduced_tokens);
    stats.artifacts += 1;
    stats.reductions += 1;
    Ok(())
}

fn should_preserve_text_slot_from_reduction(text: &str) -> bool {
    is_auto_loop_continuation_prompt(text) || is_prompt_reduction_replacement(text)
}

fn is_auto_loop_continuation_prompt(text: &str) -> bool {
    let trimmed = text.trim_start();
    (trimmed.starts_with("Automatic periodic loop continuation:")
        || trimmed.starts_with("Automatic post-self-review loop continuation:"))
        && trimmed.contains("Loop mode is on")
        && trimmed.contains("Enter Plan mode before acting")
}

fn is_prompt_reduction_replacement(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("[prompt reduction:")
        && trimmed.contains("artifact:")
        && trimmed.contains("recovery:")
}

fn reduce_tool_search_output(
    tools: &mut Vec<Value>,
    text_slot_index: usize,
    config: &PromptReductionConfig,
    stats: &mut PromptReductionStats,
) -> std::io::Result<()> {
    let Ok(original) = serde_json::to_string_pretty(tools) else {
        return Ok(());
    };
    let original_tokens = approx_tokens(&original);
    stats.original_tokens = stats.original_tokens.saturating_add(original_tokens);
    if original.chars().count() < config.min_reduce_chars {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }
    let sha1 = sha1_hex(&original);
    let digest = json_digest(&original).unwrap_or_else(|| {
        format!(
            "tool_search_digest\nitems_total: {}\nexcerpt:\n{}",
            tools.len(),
            excerpt(&original)
        )
    });
    let artifact_path =
        artifact_path_for(&config.artifact_dir, text_slot_index, "tool_search", &sha1);
    let replacement = render_replacement(
        "tool_search_digest",
        &digest,
        &artifact_path,
        original.chars().count(),
        original_tokens,
        &sha1,
    );
    let reduced_tokens = approx_tokens(&replacement);
    if original_tokens.saturating_sub(reduced_tokens) < config.min_saved_tokens {
        stats.reduced_tokens = stats.reduced_tokens.saturating_add(original_tokens);
        return Ok(());
    }

    write_artifact(&artifact_path, &original)?;
    *tools = vec![serde_json::json!({
        "type": "prompt_reduction",
        "reason": "tool_search_digest",
        "artifact": artifact_path.display().to_string(),
        "summary": replacement,
    })];
    stats.reduced_tokens = stats.reduced_tokens.saturating_add(reduced_tokens);
    stats.artifacts += 1;
    stats.reductions += 1;
    Ok(())
}

fn classify_candidate(
    source: &str,
    text: &str,
    config: &PromptReductionConfig,
    seen_hashes: &HashMap<String, String>,
    exact_preserve_reason: Option<&'static str>,
    recent_prompt_item: bool,
) -> Option<CandidateReduction> {
    let sha1 = sha1_hex(text);
    if let Some(first_item) = seen_hashes.get(&sha1) {
        return Some(CandidateReduction {
            reason: "duplicate_block",
            digest: format!("Exact duplicate of earlier prompt item `{first_item}`."),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }

    if !recent_prompt_item
        && exact_preserve_reason.is_none()
        && let Some(digest) = workflow_batch_success_digest(source, text)
    {
        return Some(CandidateReduction {
            reason: "workflow_batch_success_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }

    if exact_preserve_reason == Some("source_read") {
        if let Some(digest) = build_status_digest(text) {
            return Some(CandidateReduction {
                reason: "build_status_digest",
                digest,
                disposition: CandidateDisposition::ArtifactReplacement,
            });
        }
        if let Some(digest) = search_result_digest(source, text, config.path_list_threshold) {
            return Some(CandidateReduction {
                reason: "search_result_digest",
                digest,
                disposition: CandidateDisposition::ArtifactReplacement,
            });
        }
        return Some(CandidateReduction {
            reason: "source_read_digest",
            digest: source_read_digest(source, text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }

    if recent_prompt_item {
        if exact_preserve_reason.is_none()
            && let Some(candidate) = recent_tool_output_candidate(source, text, config)
        {
            return Some(candidate);
        }
        return None;
    }

    if exact_preserve_reason.is_none()
        && let Some(candidate) = single_use_prompt_candidate(source, text)
    {
        return Some(candidate);
    }

    if exact_preserve_reason == Some("diff_hunk") {
        return Some(CandidateReduction {
            reason: "diff_hunk_digest",
            digest: diff_hunk_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }

    if exact_preserve_reason == Some("compiler_diagnostic") {
        return Some(CandidateReduction {
            reason: "compiler_diagnostic_digest",
            digest: compiler_diagnostic_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }

    if exact_preserve_reason.is_some() {
        return None;
    }

    if is_self_review_anchor(text) {
        return Some(CandidateReduction {
            reason: "self_review_inventory",
            digest: self_review_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if is_plan_review_prompt(text) {
        return Some(CandidateReduction {
            reason: "plan_review_prompt",
            digest: plan_review_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if is_completed_plan_checkpoint(text) {
        return Some(CandidateReduction {
            reason: "completed_plan_checkpoint",
            digest: completed_plan_checkpoint_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if is_single_use_helper_prompt(text) {
        return Some(CandidateReduction {
            reason: "single_use_helper_prompt",
            digest: helper_prompt_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if is_proposed_plan_message(text) {
        return Some(CandidateReduction {
            reason: "proposed_plan_digest",
            digest: proposed_plan_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if is_review_result_message(text) {
        return Some(CandidateReduction {
            reason: "review_result_digest",
            digest: review_result_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(reduction) = subagent_notification_candidate(text) {
        return Some(reduction);
    }
    if is_assistant_findings_message(text) {
        return Some(CandidateReduction {
            reason: "assistant_findings_digest",
            digest: assistant_findings_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if is_context_pack_message(text) {
        return Some(CandidateReduction {
            reason: "context_pack_digest",
            digest: context_pack_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(digest) = build_status_digest(text) {
        return Some(CandidateReduction {
            reason: "build_status_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(digest) = search_result_digest(source, text, config.path_list_threshold) {
        return Some(CandidateReduction {
            reason: "search_result_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    let path_set = inventory_paths(text);
    if path_set.len() >= config.path_list_threshold {
        return Some(CandidateReduction {
            reason: "path_inventory",
            digest: render_compact_path_list("path_inventory_digest", &path_set, 24),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(digest) = assistant_status_json_digest(text) {
        return Some(CandidateReduction {
            reason: "assistant_status_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(digest) = json_digest(text) {
        return Some(CandidateReduction {
            reason: "json_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if command_log_candidate(source, text) {
        return Some(CandidateReduction {
            reason: "command_log_digest",
            digest: command_log_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if !recent_prompt_item
        && exact_preserve_reason.is_none()
        && let Some(digest) = recoverable_prior_context_digest(source, text)
    {
        return Some(CandidateReduction {
            reason: "recoverable_prior_context",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    None
}

fn candidate_threshold(reason: &str, config: &PromptReductionConfig) -> CandidateThreshold {
    match reason {
        "duplicate_block" => CandidateThreshold {
            min_chars: 256,
            min_saved_tokens: 24,
        },
        "source_read_digest"
        | "diff_hunk_digest"
        | "compiler_diagnostic_digest"
        | "build_status_digest"
        | "search_result_digest"
        | "path_inventory"
        | "assistant_status_digest"
        | "json_digest"
        | "command_log_digest"
        | "recent_build_status_digest"
        | "recent_search_result_digest"
        | "recent_path_inventory"
        | "recent_assistant_status_digest"
        | "recent_json_digest"
        | "recent_command_log_digest" => CandidateThreshold {
            min_chars: 600,
            min_saved_tokens: 32,
        },
        "self_review_inventory"
        | "plan_review_prompt"
        | "completed_plan_checkpoint"
        | "proposed_plan_digest"
        | "review_result_digest"
        | "subagent_notification_digest"
        | "assistant_findings_digest"
        | "context_pack_digest"
        | "single_use_self_review_prompt"
        | "single_use_plan_review_prompt"
        | "single_use_completed_plan_checkpoint"
        | "single_use_proposed_plan"
        | "single_use_prompt_reduction_notice" => CandidateThreshold {
            min_chars: 900,
            min_saved_tokens: 48,
        },
        "recoverable_prior_context" => CandidateThreshold {
            min_chars: 1_200,
            min_saved_tokens: 64,
        },
        "workflow_batch_success_digest" => CandidateThreshold {
            min_chars: 320,
            min_saved_tokens: 8,
        },
        _ => CandidateThreshold {
            min_chars: config.min_reduce_chars,
            min_saved_tokens: config.min_saved_tokens,
        },
    }
}

fn is_short_assistant_status_update(source: &str, text: &str) -> bool {
    let lower_source = source.to_ascii_lowercase();
    if !lower_source.starts_with("message:assistant") {
        return false;
    }
    if exact_preserve_reason(source, text).is_some() {
        return false;
    }
    let trimmed = text.trim();
    if trimmed.chars().count() > 520 || trimmed.lines().count() > 4 {
        return false;
    }
    let lower = trimmed
        .to_ascii_lowercase()
        .replace(['\u{2019}', '\u{2018}'], "'");
    if has_durable_status_marker(&lower) {
        return false;
    }
    let first_person_progress = [
        "i'm ",
        "i am ",
        "i'll ",
        "i will ",
        "i've ",
        "i have ",
        "i started ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
        && [
            "checking",
            "reading",
            "inspecting",
            "looking",
            "running",
            "rerunning",
            "watching",
            "waiting",
            "continuing",
            "updating",
            "testing",
            "verifying",
            "measuring",
            "simulating",
            "porting",
            "working",
            "gathering",
            "building",
            "starting",
            "keeping",
            "using",
            "folding",
        ]
        .iter()
        .any(|word| lower.contains(word));
    let process_progress = lower.starts_with("the ")
        && (lower.contains(" is still running")
            || lower.contains(" is running")
            || lower.contains(" is still working"));
    first_person_progress || process_progress
}

fn has_durable_status_marker(lower: &str) -> bool {
    [
        "<proposed_plan",
        "finding",
        "blocker",
        "failed",
        "failure",
        "error",
        "panic",
        "passed",
        "verified",
        "verification",
        "handoff",
        "final answer",
        "artifact:",
        "report:",
        "committed",
        "commit ",
        "diff --git",
        "*** begin patch",
        "assert_eq!",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn workflow_batch_success_digest(source: &str, text: &str) -> Option<String> {
    if !is_successful_workflow_batch_output(source, text) {
        return None;
    }
    let summary = workflow_batch_summary_lines(text);
    Some(format!(
        "workflow_batch_success_digest\n{}\nlines_total: {}\nexcerpt:\n{}",
        if summary.is_empty() {
            "summary: successful workflow_batch output; exact step details are recoverable from the artifact"
                .to_string()
        } else {
            summary.join("\n")
        },
        text.lines().count(),
        excerpt(text)
    ))
}

fn is_successful_workflow_batch_output(source: &str, text: &str) -> bool {
    let lower_source = source.to_ascii_lowercase();
    if !lower_source.contains("workflow_batch") {
        return false;
    }
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(text) {
        let status_success = map
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(is_successful_workflow_batch_status);
        let steps_failed = map.get("steps_failed").and_then(Value::as_u64).unwrap_or(0);
        if status_success && steps_failed == 0 {
            return true;
        }
    }
    let lower = text.to_ascii_lowercase();
    let compact = lower.split_whitespace().collect::<String>();
    (compact.contains("\"status\":\"ok\"")
        || compact.contains("\"status\":\"success\"")
        || lower.contains("status: ok")
        || lower.contains("status: success")
        || lower.contains("status = ok")
        || lower.contains("status = success"))
        && !compact.contains("\"status\":\"failed\"")
        && !lower.contains("status: failed")
        && !workflow_batch_steps_failed_nonzero(text)
}

fn is_successful_workflow_batch_status(status: &str) -> bool {
    status.eq_ignore_ascii_case("ok") || status.eq_ignore_ascii_case("success")
}

fn workflow_batch_steps_failed_nonzero(text: &str) -> bool {
    text.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("steps_failed") && line.chars().any(|ch| ch.is_ascii_digit() && ch != '0')
    })
}

fn workflow_batch_summary_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();
            if [
                "status",
                "report_path",
                "log_path",
                "steps_total",
                "steps_failed",
                "steps_skipped",
                "vars",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
            {
                Some(truncate(trimmed, 160))
            } else {
                None
            }
        })
        .take(16)
        .collect()
}

fn recent_tool_output_candidate(
    source: &str,
    text: &str,
    config: &PromptReductionConfig,
) -> Option<CandidateReduction> {
    let lower_source = source.to_ascii_lowercase();
    if lower_source.starts_with("message:assistant")
        && let Some(digest) = assistant_status_json_digest(text)
    {
        return Some(CandidateReduction {
            reason: "recent_assistant_status_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if !(lower_source.starts_with("shell_output:")
        || lower_source.starts_with("tool_output:")
        || lower_source.contains("build_status"))
    {
        return None;
    }

    if let Some(digest) = build_status_digest(text) {
        return Some(CandidateReduction {
            reason: "recent_build_status_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(digest) = search_result_digest(source, text, config.path_list_threshold) {
        return Some(CandidateReduction {
            reason: "recent_search_result_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    let path_set = inventory_paths(text);
    if path_set.len() >= config.path_list_threshold {
        return Some(CandidateReduction {
            reason: "recent_path_inventory",
            digest: render_compact_path_list("path_inventory_digest", &path_set, 24),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(digest) = assistant_status_json_digest(text) {
        return Some(CandidateReduction {
            reason: "recent_assistant_status_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if let Some(digest) = json_digest(text) {
        return Some(CandidateReduction {
            reason: "recent_json_digest",
            digest,
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if command_log_candidate(source, text) {
        return Some(CandidateReduction {
            reason: "recent_command_log_digest",
            digest: command_log_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }

    None
}

fn is_short_recoverable_tool_output(source: &str, text: &str) -> bool {
    let lower_source = source.to_ascii_lowercase();
    if !(lower_source.starts_with("shell_output:")
        || lower_source.starts_with("tool_output:")
        || lower_source.starts_with("custom_tool_output:"))
    {
        return false;
    }
    if exact_preserve_reason(source, text).is_some()
        || should_preserve_text_slot_from_reduction(text)
        || (high_next_turn_utility_tool_output(text)
            && !is_successful_workflow_batch_output(source, text))
    {
        return false;
    }
    true
}

fn high_next_turn_utility_tool_output(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("exit code: 1")
        || lower.contains("exit code: 101")
        || lower.contains("error:")
        || lower.contains("panicked at")
        || lower.contains("failed")
        || lower.contains("test result: failed")
        || lower.contains("git status")
        || lower.contains("changes not staged")
        || lower.contains("untracked files")
        || lower.contains("conflict")
        || lower.contains("merge ")
}

fn short_tool_output_bundle_artifact(
    items: &[ResponseItem],
    indices: &BTreeSet<usize>,
    call_sources: &HashMap<String, String>,
) -> String {
    let mut lines = vec![
        "short_tool_output_bundle_artifact".to_string(),
        format!("items: {}", indices.len()),
        "utility_estimate: low next-turn utility; older successful short tool outputs are recoverable if a later prompt needs exact evidence".to_string(),
        String::new(),
    ];
    for (slot_index, call_id, text) in text_function_outputs(items) {
        if !indices.contains(&slot_index) {
            continue;
        }
        let source = call_sources
            .get(call_id)
            .map(String::as_str)
            .unwrap_or("tool_output");
        lines.extend([
            format!("## text slot {slot_index}"),
            format!("source: {source}"),
            format!("tokens_estimate: {}", approx_tokens(text)),
            format!("sha1: {}", sha1_hex(text)),
            "content:".to_string(),
            text.to_string(),
            String::new(),
        ]);
    }
    lines.join("\n")
}

fn render_short_tool_output_bundle_replacement(
    items: &[ResponseItem],
    indices: &BTreeSet<usize>,
    artifact_path: &Path,
    original_tokens: usize,
) -> String {
    let sources = text_function_outputs(items)
        .into_iter()
        .filter(|(slot_index, _, _)| indices.contains(slot_index))
        .map(|(_, call_id, _)| truncate(call_id, 80))
        .take(8)
        .collect::<Vec<_>>();
    format!(
        "[prompt reduction: short_tool_output_bundle]\noriginal_items: {}\noriginal_tokens_estimate: {original_tokens}\nartifact: `{}`\nrecovery: read artifact before using exact short tool outputs.\n\nshort_tool_output_bundle\nutility_estimate: low next-turn utility\nselection: older successful short tool/command outputs; recent, failing, exact-evidence, and user/developer/system items preserved\ncall_ids: {}",
        indices.len(),
        artifact_path.display(),
        if sources.is_empty() {
            "(unknown)".to_string()
        } else {
            sources.join(" | ")
        }
    )
}

fn short_assistant_status_bundle_artifact(
    items: &[ResponseItem],
    indices: &BTreeSet<usize>,
) -> String {
    let mut lines = vec![
        "short_assistant_status_bundle_artifact".to_string(),
        format!("items: {}", indices.len()),
        "selection: stale assistant progress/status messages with low next-turn utility"
            .to_string(),
        String::new(),
    ];
    for (slot_index, source, text) in assistant_message_text_slots(items) {
        if indices.contains(&slot_index) {
            lines.extend([
                format!("--- text slot {slot_index}"),
                format!("source: {source}"),
                format!("tokens_estimate: {}", approx_tokens(text)),
                format!("sha1: {}", sha1_hex(text)),
                "content:".to_string(),
                text.to_string(),
                String::new(),
            ]);
        }
    }
    lines.join("\n")
}

fn render_short_assistant_status_bundle_replacement(
    items: &[ResponseItem],
    indices: &BTreeSet<usize>,
    artifact_path: &Path,
    original_tokens: usize,
) -> String {
    let samples = assistant_message_text_slots(items)
        .into_iter()
        .filter(|(slot_index, _, _)| indices.contains(slot_index))
        .map(|(_, _, text)| truncate(text.trim(), 96))
        .take(4)
        .collect::<Vec<_>>();
    format!(
        "[prompt reduction: short_assistant_status_bundle]\noriginal_items: {}\noriginal_tokens_estimate: {original_tokens}\nartifact: `{}`\nrecovery: read artifact before using exact progress/status updates.\n\nshort_assistant_status_bundle\nutility_estimate: low next-turn utility\nselection: older assistant progress/status updates; recent, findings, handoffs, plans, failures, and user/developer/system items preserved\nsamples: {}",
        indices.len(),
        artifact_path.display(),
        if samples.is_empty() {
            "(none)".to_string()
        } else {
            samples.join(" | ")
        }
    )
}

fn assistant_message_text_slots(items: &[ResponseItem]) -> Vec<(usize, String, &str)> {
    let mut outputs = Vec::new();
    let mut text_slot_index = 0usize;
    for item in items {
        match item {
            ResponseItem::Message { role, content, .. } => {
                for content_item in content {
                    match content_item {
                        ContentItem::InputText { text } | ContentItem::OutputText { text } => {
                            let source = format!("message:{role}");
                            outputs.push((text_slot_index, source, text.as_str()));
                            text_slot_index += 1;
                        }
                        ContentItem::InputImage { .. } => {}
                    }
                }
            }
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                text_slot_index += function_output_text_slot_count(output);
            }
            ResponseItem::ToolSearchOutput { tools, .. } => {
                text_slot_index += usize::from(!tools.is_empty());
            }
            _ => {}
        }
    }
    outputs
}

fn text_function_outputs(items: &[ResponseItem]) -> Vec<(usize, &str, &str)> {
    let mut outputs = Vec::new();
    let mut text_slot_index = 0usize;
    for item in items {
        match item {
            ResponseItem::Message { content, .. } => {
                text_slot_index += content
                    .iter()
                    .filter(|content_item| {
                        matches!(
                            content_item,
                            ContentItem::InputText { .. } | ContentItem::OutputText { .. }
                        )
                    })
                    .count();
            }
            ResponseItem::FunctionCallOutput { call_id, output }
            | ResponseItem::CustomToolCallOutput {
                call_id, output, ..
            } => {
                visit_function_output_texts(output, |text| {
                    text_slot_index += 1;
                    outputs.push((text_slot_index, call_id.as_str(), text));
                });
            }
            ResponseItem::ToolSearchOutput { tools, .. } => {
                text_slot_index += usize::from(!tools.is_empty());
            }
            _ => {}
        }
    }
    outputs
}

fn function_output_text_slot_count(output: &FunctionCallOutputPayload) -> usize {
    if output.text_content().is_some() {
        return 1;
    }
    output
        .content_items()
        .map(|items| {
            items
                .iter()
                .filter(|item| matches!(item, FunctionCallOutputContentItem::InputText { .. }))
                .count()
        })
        .unwrap_or(0)
}

fn visit_function_output_texts<'a>(
    output: &'a FunctionCallOutputPayload,
    mut visit: impl FnMut(&'a str),
) {
    if let Some(text) = output.text_content() {
        visit(text);
        return;
    }
    if let Some(content_items) = output.content_items() {
        for content_item in content_items {
            if let FunctionCallOutputContentItem::InputText { text } = content_item {
                visit(text);
            }
        }
    }
}

fn call_source(name: &str, arguments: &str) -> String {
    if name == "shell_command"
        && let Ok(value) = serde_json::from_str::<Value>(arguments)
        && let Some(command) = value.get("command").and_then(Value::as_str)
    {
        return format!("shell_output:{command}");
    }
    format!("tool_output:{name}")
}

fn render_replacement(
    reason: &str,
    digest: &str,
    artifact_path: &Path,
    original_chars: usize,
    original_tokens: usize,
    sha1: &str,
) -> String {
    format!(
        "[prompt reduction: {reason}]\noriginal_chars: {original_chars}\noriginal_tokens_estimate: {original_tokens}\nsha1: {sha1}\nartifact: `{}`\nrecovery: read artifact before using exact lines.\n\n{digest}",
        artifact_path.display()
    )
}

fn artifact_path_for(artifact_dir: &Path, index: usize, reason: &str, sha1: &str) -> PathBuf {
    artifact_dir.join(format!(
        "prompt-item-{index:04}-{reason}-{}.txt",
        &sha1[..12]
    ))
}

fn write_artifact(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn safe_label(label: &str) -> String {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn sha1_hex(text: &str) -> String {
    let digest = Sha1::digest(text.as_bytes());
    let mut hex = String::with_capacity(40);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn approx_tokens(text: &str) -> usize {
    text.chars().count().max(1).div_ceil(4)
}

fn normalize_slashes(path: impl AsRef<str>) -> String {
    path.as_ref().replace('\\', "/")
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= max_chars {
            output.push_str("...");
            break;
        }
        output.push(ch);
    }
    output
}

fn exact_preserve_reason(source: &str, text: &str) -> Option<&'static str> {
    let lower_source = source.to_ascii_lowercase();
    let lower = text.to_ascii_lowercase();
    if is_durable_instruction_message(source, text) {
        return Some("durable_instruction");
    }
    if lower_source.contains("apply_patch")
        || ((lower.contains("*** begin patch") || lower.contains("*** end patch"))
            && !lower_source.starts_with("shell_output:")
            && !lower_source.contains("get-content")
            && !lower_source.contains("select-object -skip"))
    {
        return Some("patch_output");
    }
    if text.contains("diff --git ") || text.lines().any(|line| line.starts_with("@@ ")) {
        return Some("diff_hunk");
    }
    if lower.contains("error[e")
        || lower.contains("traceback (most recent call last)")
        || lower.contains("panic at ")
    {
        return Some("compiler_diagnostic");
    }
    if lower.contains("process running with session id")
        || lower.contains("background process started")
    {
        return Some("active_process");
    }
    if lower_source.contains("get-content")
        || lower_source.contains("select-object -skip")
        || lower_source.starts_with("shell_output:cat ")
        || lower_source.contains(" cat ")
        || lower_source.contains(" sed ")
        || lower_source.contains(" nl ")
    {
        return Some("source_read");
    }
    None
}

fn is_durable_instruction_message(source: &str, text: &str) -> bool {
    let lower_source = source.to_ascii_lowercase();
    if !(lower_source.starts_with("message:developer")
        || lower_source.starts_with("message:system"))
    {
        return false;
    }
    looks_like_durable_instruction_block(text)
}

fn looks_like_durable_instruction_block(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("<skills_instructions>")
        || lower.contains("# agents.md instructions")
        || lower.contains("<collaboration_mode>")
        || lower.contains("</collaboration_mode>")
        || lower.contains("<instructions>")
        || lower.contains("</instructions>")
        || (lower.contains("## skills") && lower.contains("skill"))
        || (lower.contains("continuation rule") && lower.contains("working with the user"))
        || (lower.contains("you are codex") && lower.contains("editing constraints"))
}

fn is_self_review_anchor(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("automatic self-review")
        && (lower.contains("dirty tracked files")
            || lower.contains("dirty-at-anchor")
            || lower.contains("exact diff commands")
            || lower.contains("compact work notes")
            || lower.contains("git status --short")
            || lower.contains("just-completed work slice"))
}

fn single_use_prompt_candidate(source: &str, text: &str) -> Option<CandidateReduction> {
    if is_self_review_anchor(text) {
        return Some(CandidateReduction {
            reason: "single_use_self_review_prompt",
            digest: self_review_digest(text),
            disposition: CandidateDisposition::OmitFromPrompt,
        });
    }
    if is_plan_review_prompt(text) {
        return Some(CandidateReduction {
            reason: "single_use_plan_review_prompt",
            digest: plan_review_digest(text),
            disposition: CandidateDisposition::OmitFromPrompt,
        });
    }
    if is_completed_plan_checkpoint(text) {
        return Some(CandidateReduction {
            reason: "single_use_completed_plan_checkpoint",
            digest: completed_plan_checkpoint_digest(text),
            disposition: CandidateDisposition::OmitFromPrompt,
        });
    }
    if is_single_use_helper_prompt(text) {
        return Some(CandidateReduction {
            reason: "single_use_helper_prompt",
            digest: helper_prompt_digest(text),
            disposition: CandidateDisposition::ArtifactReplacement,
        });
    }
    if is_proposed_plan_message(text) {
        return Some(CandidateReduction {
            reason: "single_use_proposed_plan",
            digest: proposed_plan_digest(text),
            disposition: CandidateDisposition::OmitFromPrompt,
        });
    }
    if let Some(candidate) = subagent_notification_candidate(text) {
        return Some(candidate);
    }
    if is_prompt_reduction_status_notice(source, text) {
        return Some(CandidateReduction {
            reason: "single_use_prompt_reduction_notice",
            digest: prompt_reduction_status_digest(text),
            disposition: CandidateDisposition::OmitFromPrompt,
        });
    }
    None
}

fn is_single_use_helper_prompt(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if !(lower.contains("helper") || lower.contains("worker") || lower.contains("agent")) {
        return false;
    }
    [
        "context_area:",
        "do_not_inspect:",
        "scout_evidence:",
        "why_agent / roi:",
        "first_reads:",
        "tool_hints:",
        "token_tip:",
        "verification:",
        "handoff:",
    ]
    .iter()
    .filter(|marker| lower.contains(*marker))
    .count()
        >= 6
}

fn helper_prompt_digest(text: &str) -> String {
    let markers = [
        "CONTEXT_AREA:",
        "DO_NOT_INSPECT:",
        "SCOUT_EVIDENCE:",
        "WHY_AGENT / ROI:",
        "FIRST_READS:",
        "TOOL_HINTS:",
        "TOKEN_TIP:",
        "VERIFICATION:",
        "HANDOFF:",
    ]
    .iter()
    .filter(|marker| text.contains(*marker))
    .copied()
    .collect::<Vec<_>>();
    format!(
        "single_use_helper_prompt_digest\nmarkers: {}\nlines_total: {}\nexcerpt:\n{}",
        if markers.is_empty() {
            "(none)".to_string()
        } else {
            markers.join(", ")
        },
        text.lines().count(),
        excerpt(text)
    )
}

fn is_prompt_reduction_status_notice(source: &str, text: &str) -> bool {
    let lower_source = source.to_ascii_lowercase();
    let lower = text
        .trim_start()
        .trim_start_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .trim_start()
        .to_ascii_lowercase();
    (lower_source.contains("message:assistant") || lower_source.contains("client_event"))
        && (lower.starts_with("prompt reduction")
            || lower.starts_with("prompt reduced")
            || lower.starts_with("prompt reducer"))
        && lower.contains('%')
        && (lower.contains("saved")
            || lower.contains("reduced")
            || lower.contains("optimized")
            || lower.contains("unchanged"))
}

fn prompt_reduction_status_digest(text: &str) -> String {
    format!(
        "prompt_reduction_status_digest\nlines_total: {}\nexcerpt:\n{}",
        text.lines().count(),
        excerpt(text)
    )
}

fn self_review_digest(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let inventory_paths = inventory_paths(text);
    let diff_commands = lower.matches("git diff").count();
    let no_index_commands = lower.matches("git diff --no-index").count();
    let sections = [
        "dirty tracked files",
        "staged files",
        "untracked files",
        "changed files since anchor",
        "exact diff commands",
    ]
    .iter()
    .filter(|section| lower.contains(**section))
    .copied()
    .collect::<Vec<_>>();
    format!(
        "self_review_inventory_digest\nsections: {}\npaths_total: {}\ndiff_commands: {}\nno_index_diff_commands: {}\nexcerpt:\n{}",
        if sections.is_empty() {
            "(none)".to_string()
        } else {
            sections.join(", ")
        },
        inventory_paths.len(),
        diff_commands,
        no_index_commands,
        excerpt(text)
    )
}

fn is_plan_review_prompt(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    lower.starts_with("self-review the plan below before implementation.")
        && lower.contains("current plan:")
}

fn plan_review_digest(text: &str) -> String {
    let title = first_heading_after_marker(text, "Current plan:").unwrap_or("(unknown)");
    let sections = markdown_headings(text, 12);
    format!(
        "plan_review_prompt_digest\ncurrent_plan_title: {}\nsections: {}\nlines_total: {}\nexcerpt:\n{}",
        title,
        if sections.is_empty() {
            "(none)".to_string()
        } else {
            sections.join(" | ")
        },
        text.lines().count(),
        excerpt(text)
    )
}

fn is_completed_plan_checkpoint(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("the current plan appears complete")
        && lower.contains("completed plan:")
        && lower.contains("review the completed work")
}

fn completed_plan_checkpoint_digest(text: &str) -> String {
    let completed_items = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("- completed:") || trimmed.starts_with("completed:")
        })
        .take(16)
        .map(|line| truncate(line.trim(), 180))
        .collect::<Vec<_>>();
    format!(
        "completed_plan_checkpoint_digest\ncompleted_items: {}\nlines_total: {}\nexcerpt:\n{}",
        if completed_items.is_empty() {
            "(none)".to_string()
        } else {
            completed_items.join(" | ")
        },
        text.lines().count(),
        excerpt(text)
    )
}

fn is_proposed_plan_message(text: &str) -> bool {
    text.contains("<proposed_plan>") && text.contains("</proposed_plan>")
}

fn proposed_plan_digest(text: &str) -> String {
    let title = first_markdown_heading(text).unwrap_or("(unknown)");
    let sections = markdown_headings(text, 14);
    format!(
        "proposed_plan_digest\ntitle: {}\nsections: {}\nlines_total: {}\nexcerpt:\n{}",
        title,
        if sections.is_empty() {
            "(none)".to_string()
        } else {
            sections.join(" | ")
        },
        text.lines().count(),
        excerpt(text)
    )
}

fn is_review_result_message(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("<user_action>")
        && lower.contains("<action>review</action>")
        && lower.contains("<results>")
}

fn review_result_digest(text: &str) -> String {
    let comments = text
        .lines()
        .filter(|line| line.trim_start().starts_with("- [P"))
        .take(12)
        .map(|line| truncate(line.trim(), 220))
        .collect::<Vec<_>>();
    format!(
        "review_result_digest\ncomments_total: {}\ncomments: {}\nlines_total: {}\nexcerpt:\n{}",
        text.lines()
            .filter(|line| line.trim_start().starts_with("- [P"))
            .count(),
        if comments.is_empty() {
            "(none)".to_string()
        } else {
            comments.join(" | ")
        },
        text.lines().count(),
        excerpt(text)
    )
}

fn is_subagent_notification_message(text: &str) -> bool {
    text.contains("<subagent_notification>")
        || (text.contains("\"agent_path\"") && text.contains("\"status\""))
}

#[derive(Debug, Clone)]
struct SubagentNotificationDigestParts {
    agent: String,
    status: String,
    detail: Option<String>,
}

fn subagent_notification_candidate(text: &str) -> Option<CandidateReduction> {
    if !is_subagent_notification_message(text) {
        return None;
    }

    let parsed = parse_subagent_notification(text)?;
    let has_handoff_detail = parsed
        .detail
        .as_deref()
        .is_some_and(|detail| !detail.trim().is_empty());
    if !has_handoff_detail && !text.contains("<subagent_notification>") {
        return None;
    }
    Some(CandidateReduction {
        reason: if has_handoff_detail {
            "subagent_notification_digest"
        } else {
            "single_use_subagent_status_notice"
        },
        digest: subagent_notification_digest(text),
        disposition: if has_handoff_detail {
            CandidateDisposition::ArtifactReplacement
        } else {
            CandidateDisposition::OmitFromPrompt
        },
    })
}

fn subagent_notification_digest(text: &str) -> String {
    let parts = parse_subagent_notification(text).unwrap_or_else(|| {
        let lower = text.to_ascii_lowercase();
        let status = if lower.contains("completed") {
            "completed".to_string()
        } else if lower.contains("errored") || lower.contains("failed") {
            "errored".to_string()
        } else {
            "(unknown)".to_string()
        };
        SubagentNotificationDigestParts {
            agent: "(unknown)".to_string(),
            status,
            detail: None,
        }
    });
    let detail_excerpt = parts
        .detail
        .as_deref()
        .filter(|detail| !detail.trim().is_empty())
        .map(excerpt)
        .unwrap_or_else(|| "(none)".to_string());
    format!(
        "subagent_notification_digest\nagent: {}\nstatus: {}\ndetail_excerpt:\n{}\nlines_total: {}\nexcerpt:\n{}",
        truncate(&parts.agent, 220),
        parts.status,
        detail_excerpt,
        text.lines().count(),
        excerpt(text)
    )
}

fn parse_subagent_notification(text: &str) -> Option<SubagentNotificationDigestParts> {
    if let Ok(value) = serde_json::from_str::<Value>(text.trim())
        && let Some(parts) = parse_subagent_notification_value(&value)
    {
        return Some(parts);
    }
    let body = subagent_notification_json_body(text)?;
    let value = serde_json::from_str::<Value>(body).ok()?;
    parse_subagent_notification_value(&value)
}

fn parse_subagent_notification_value(value: &Value) -> Option<SubagentNotificationDigestParts> {
    let outer_agent = value
        .get("agent_path")
        .or_else(|| value.get("author"))
        .or_else(|| value.pointer("/content/agent_path"))
        .and_then(Value::as_str)
        .unwrap_or("(unknown)")
        .to_string();
    if let Some(content) = value.get("content").and_then(Value::as_str)
        && is_subagent_notification_message(content)
        && let Some(mut parts) = parse_subagent_notification(content)
    {
        if parts.agent == "(unknown)" {
            parts.agent = outer_agent;
        }
        return Some(parts);
    }

    let status_value = value
        .get("status")
        .or_else(|| value.pointer("/content/status"));
    let (status, detail) = parse_subagent_status(status_value);
    Some(SubagentNotificationDigestParts {
        agent: outer_agent,
        status,
        detail,
    })
}

fn subagent_notification_json_body(text: &str) -> Option<&str> {
    let start_marker = "<subagent_notification>";
    let end_marker = "</subagent_notification>";
    let start = text.find(start_marker)? + start_marker.len();
    let rest = &text[start..];
    let end = rest.find(end_marker)?;
    Some(rest[..end].trim())
}

fn parse_subagent_status(status: Option<&Value>) -> (String, Option<String>) {
    match status {
        Some(Value::String(status)) => (status.clone(), None),
        Some(Value::Object(map)) => {
            if let Some(value) = map.get("completed") {
                return ("completed".to_string(), value.as_str().map(str::to_string));
            }
            if let Some(value) = map.get("errored").or_else(|| map.get("failed")) {
                return ("errored".to_string(), value.as_str().map(str::to_string));
            }
            let label = map
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "(unknown)".to_string());
            (label, None)
        }
        Some(value) => (
            serde_json::to_string(value).unwrap_or_else(|_| "(unknown)".to_string()),
            None,
        ),
        None => ("(unknown)".to_string(), None),
    }
}

fn is_assistant_findings_message(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("**Findings**")
        || (trimmed.starts_with("Findings") && text.contains("**Source Handoff**"))
        || (text.contains("**Findings**") && text.contains("**Source Handoff**"))
}

fn assistant_findings_digest(text: &str) -> String {
    let findings = text
        .lines()
        .filter(|line| line.trim_start().starts_with("- "))
        .take(12)
        .map(|line| truncate(line.trim(), 220))
        .collect::<Vec<_>>();
    let headings = markdown_headings(text, 10);
    format!(
        "assistant_findings_digest\nheadings: {}\nbullets_sample: {}\nlines_total: {}\nexcerpt:\n{}",
        if headings.is_empty() {
            "(none)".to_string()
        } else {
            headings.join(" | ")
        },
        if findings.is_empty() {
            "(none)".to_string()
        } else {
            findings.join(" | ")
        },
        text.lines().count(),
        excerpt(text)
    )
}

fn is_context_pack_message(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("<context_pack") && trimmed.contains("</context_pack>")
}

fn context_pack_digest(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or("<context_pack>");
    let path_set = inventory_paths(text);
    let candidates = text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- "))
        .take(16)
        .map(|line| truncate(line, 220))
        .collect::<Vec<_>>();
    format!(
        "context_pack_digest\nheader: {}\npaths_total: {}\ncandidates_sample: {}\nlines_total: {}\nexcerpt:\n{}",
        truncate(first_line.trim(), 220),
        path_set.len(),
        if candidates.is_empty() {
            "(none)".to_string()
        } else {
            candidates.join(" | ")
        },
        text.lines().count(),
        excerpt(text)
    )
}

fn first_heading_after_marker<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let mut after_marker = false;
    for line in text.lines() {
        if after_marker {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return Some(trimmed.trim_start_matches('#').trim());
            }
        } else if line.trim().eq_ignore_ascii_case(marker) {
            after_marker = true;
        }
    }
    None
}

fn first_markdown_heading(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim())
        .filter(|line| !line.is_empty())
}

fn markdown_headings(text: &str, limit: usize) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| line.starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .filter(|line| !line.is_empty())
        .take(limit)
        .collect()
}

fn inventory_paths(text: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for line in text.lines() {
        let trimmed = clean_path_candidate(line);
        if looks_like_inventory_path(&trimmed) {
            paths.insert(normalize_slashes(trimmed));
        }
        for token in line.split([',', ';']) {
            for part in token.split(" -> ") {
                for candidate in part.split(" | ") {
                    let cleaned = clean_path_candidate(candidate);
                    if looks_like_inventory_path(&cleaned) {
                        paths.insert(normalize_slashes(cleaned));
                    }
                }
            }
        }
    }
    paths
}

fn clean_path_candidate(text: &str) -> String {
    text.trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim_start_matches("?? ")
        .trim_start_matches("M ")
        .trim_start_matches("D ")
        .trim_start_matches("A ")
        .trim_start_matches("R ")
        .trim()
        .to_string()
}

fn looks_like_inventory_path(text: &str) -> bool {
    let normalized = normalize_slashes(text);
    if normalized.len() < 5 || normalized.contains(' ') {
        return false;
    }
    if normalized.starts_with("C:/") {
        return true;
    }
    let lower = normalized.to_ascii_lowercase();
    lower.contains('/')
        && [
            ".rs", ".toml", ".json", ".md", ".snap", ".ps1", ".txt", ".lock", ".yaml", ".yml",
        ]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn render_compact_path_list(operation: &str, paths: &BTreeSet<String>, limit: usize) -> String {
    let selected_paths = paths.iter().take(limit).cloned().collect::<Vec<_>>();
    let omitted = paths.len().saturating_sub(selected_paths.len());
    let mut extension_counts = BTreeMap::<String, usize>::new();
    for path in paths {
        let extension = Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| !extension.is_empty())
            .unwrap_or("(none)")
            .to_string();
        *extension_counts.entry(extension).or_default() += 1;
    }
    let mut lines = vec![
        operation.to_string(),
        format!("paths_total: {}", paths.len()),
        format!("extensions: {}", render_counts(&extension_counts)),
        format!("paths: {} shown, {omitted} omitted", selected_paths.len()),
    ];
    if omitted > 0 {
        lines.push("fallback_required: true".to_string());
        lines.push("fallback_reason: max_paths".to_string());
    }
    lines.extend(selected_paths);
    lines.join("\n")
}

fn render_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn assistant_status_json_digest(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('{') {
        return None;
    }

    let Value::Object(map) = serde_json::from_str::<Value>(trimmed).ok()? else {
        return None;
    };
    if map.get("author").and_then(Value::as_str) != Some("assistant")
        || !map.contains_key("recipient")
        || !map.contains_key("content")
    {
        return None;
    }

    let recipient = map
        .get("recipient")
        .and_then(Value::as_str)
        .unwrap_or("(none)");
    let other_recipients = map
        .get("other_recipients")
        .map(json_sample)
        .unwrap_or_else(|| "(none)".to_string());
    let trigger_turn = map
        .get("trigger_turn")
        .map(json_sample)
        .unwrap_or_else(|| "(unknown)".to_string());
    let content = status_content_excerpt(map.get("content")?);

    Some(format!(
        "assistant_status_digest\nrecipient: {}\nother_recipients: {}\ntrigger_turn: {}\ncontent:\n{}",
        truncate(recipient, 160),
        truncate(&other_recipients, 160),
        truncate(&trigger_turn, 160),
        content
    ))
}

fn status_content_excerpt(value: &Value) -> String {
    match value {
        Value::String(text) => excerpt(text),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.as_object()
                    .and_then(|map| map.get("text").or_else(|| map.get("content")))
                    .and_then(Value::as_str)
                    .map(excerpt)
                    .or_else(|| Some(json_sample(item)))
            })
            .take(4)
            .collect::<Vec<_>>()
            .join("\n---\n"),
        other => json_sample(other),
    }
}

fn json_digest(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }
    let value = serde_json::from_str::<Value>(trimmed).ok()?;
    let digest = match value {
        Value::Array(items) => {
            let samples = items
                .iter()
                .take(5)
                .map(json_sample)
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "json_digest\narray_len: {}\nsamples: {samples}",
                items.len()
            )
        }
        Value::Object(map) => {
            let keys = map.keys().take(30).cloned().collect::<Vec<_>>();
            format!(
                "json_digest\nobject_keys_total: {}\nkeys: {}",
                map.len(),
                keys.join(", ")
            )
        }
        other => format!("json_digest\nscalar: {}", json_sample(&other)),
    };
    Some(digest)
}

fn build_status_digest(text: &str) -> Option<String> {
    build_status_json_digest(text)
}

fn build_status_json_digest(text: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(text.trim()).ok()?;
    let map = value.as_object()?;
    if !map.contains_key("active_build_processes")
        || !map.contains_key("release_profile_state")
        || map.get("mode").and_then(Value::as_str) != Some("Status")
    {
        return None;
    }
    let active = map
        .get("active_build_processes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let process_names = active
        .iter()
        .filter_map(|process| process.get("process_name").and_then(Value::as_str))
        .take(12)
        .collect::<Vec<_>>();
    let release_binary_time = map
        .get("release_binary")
        .and_then(|value| value.get("last_write_time"))
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");
    let wrapper = map
        .get("wrapper_real_exe")
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");
    let free_c_bytes = map
        .get("free_c_drive_bytes")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let profile_matches = map
        .get("release_profile_state")
        .and_then(|value| value.get("matches"))
        .and_then(Value::as_bool)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "(unknown)".to_string());
    Some(format!(
        "build_status_digest\nstatus: {}\nactive_processes: {} ({})\nrelease_binary_last_write: {}\nwrapper_real_exe: {}\nrelease_profile_matches: {}\nfree_c_drive_bytes: {}\ncommand_lines: omitted; read artifact for exact process commands",
        map.get("status")
            .and_then(Value::as_str)
            .unwrap_or("(unknown)"),
        active.len(),
        if process_names.is_empty() {
            "none".to_string()
        } else {
            process_names.join(", ")
        },
        release_binary_time,
        truncate(wrapper, 220),
        profile_matches,
        free_c_bytes
    ))
}

fn json_sample(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let keys = map.keys().take(8).cloned().collect::<Vec<_>>();
            format!("object({})", keys.join(","))
        }
        Value::Array(items) => format!("array(len={})", items.len()),
        Value::String(text) => format!("string({})", truncate(text, 60)),
        Value::Number(number) => number.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_string(),
    }
}

fn looks_like_command_log(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.lines().count() >= 40
        || lower.contains("exit code:")
        || lower.contains("wall time:")
        || lower.contains("stdout")
        || lower.contains("stderr")
}

fn command_log_candidate(source: &str, text: &str) -> bool {
    let lower_source = source.to_ascii_lowercase();
    (lower_source.starts_with("shell_output:")
        || lower_source.starts_with("tool_output:")
        || lower_source.starts_with("message:tool")
        || lower_source.contains("command"))
        && looks_like_command_log(text)
        && !looks_like_durable_instruction_block(text)
}

fn command_log_digest(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let mut status_lines = Vec::new();
    for line in &lines {
        let lower = line.to_ascii_lowercase();
        if lower.contains("exit code:")
            || lower.contains("wall time:")
            || lower.starts_with("error:")
            || lower.starts_with("warning:")
        {
            status_lines.push((*line).to_string());
        }
        if status_lines.len() >= 12 {
            break;
        }
    }
    format!(
        "command_log_digest\nlines_total: {}\nstatus_lines: {}\nexcerpt:\n{}",
        lines.len(),
        if status_lines.is_empty() {
            "(none)".to_string()
        } else {
            status_lines.join(" | ")
        },
        excerpt(text)
    )
}

fn recoverable_prior_context_digest(source: &str, text: &str) -> Option<String> {
    let lower_source = source.to_ascii_lowercase();
    let recoverable_source = lower_source.starts_with("message:assistant")
        || lower_source.starts_with("shell_output:")
        || lower_source.starts_with("tool_output:")
        || lower_source.starts_with("message:tool");
    if !recoverable_source
        || lower_source.starts_with("message:user")
        || lower_source.starts_with("message:developer")
        || lower_source.starts_with("message:system")
        || looks_like_durable_instruction_block(text)
    {
        return None;
    }

    Some(format!(
        "recoverable_prior_context_digest\nsource: {}\nlines_total: {}\nchars_total: {}\nsafety: recoverable artifact for prior non-user context unlikely to be needed next\nexcerpt:\n{}",
        truncate(source, 160),
        text.lines().count(),
        text.chars().count(),
        excerpt(text)
    ))
}

fn search_result_digest(source: &str, text: &str, threshold: usize) -> Option<String> {
    let source_lower = source.to_ascii_lowercase();
    let source_suggests_search = source_lower.contains("rg ")
        || source_lower.contains("select-string")
        || source_lower.contains("grep ");
    let mut paths = BTreeSet::new();
    let mut samples = Vec::new();
    let mut matches_total = 0usize;
    for line in text.lines() {
        let Some((path, line_number, body)) = parse_search_result_line(line) else {
            continue;
        };
        paths.insert(path.clone());
        matches_total += 1;
        if samples.len() < 12 {
            samples.push(format!("{path}:{line_number}:{}", truncate(&body, 120)));
        }
    }
    if matches_total < threshold || (!source_suggests_search && paths.len() < threshold) {
        return None;
    }
    let mut extension_counts = BTreeMap::<String, usize>::new();
    for path in &paths {
        let extension = Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .filter(|extension| !extension.is_empty())
            .unwrap_or("(none)")
            .to_string();
        *extension_counts.entry(extension).or_default() += 1;
    }
    Some(format!(
        "search_result_digest\nmatches_total: {}\npaths_total: {}\nextensions: {}\nsamples: {}\nexcerpt:\n{}",
        matches_total,
        paths.len(),
        render_counts(&extension_counts),
        if samples.is_empty() {
            "(none)".to_string()
        } else {
            samples.join(" | ")
        },
        excerpt(text)
    ))
}

fn parse_search_result_line(line: &str) -> Option<(String, usize, String)> {
    for (colon_index, _) in line.match_indices(':') {
        let rest = &line[colon_index + 1..];
        let Some(next_colon_offset) = rest.find(':') else {
            continue;
        };
        let number_text = &rest[..next_colon_offset];
        let Ok(line_number) = number_text.parse::<usize>() else {
            continue;
        };
        let path = line[..colon_index].trim();
        if !looks_like_inventory_path(path) {
            continue;
        }
        let body = rest[next_colon_offset + 1..].trim().to_string();
        return Some((normalize_slashes(path), line_number, body));
    }
    None
}

fn source_read_digest(source: &str, text: &str) -> String {
    let paths = inventory_paths(source);
    let path_summary = if paths.is_empty() {
        "(unknown)".to_string()
    } else {
        paths.into_iter().take(12).collect::<Vec<_>>().join(", ")
    };
    format!(
        "source_read_digest\nsource: {}\nlines_total: {}\nchars_total: {}\npaths: {}\nexcerpt:\n{}",
        truncate(source, 220),
        text.lines().count(),
        text.chars().count(),
        path_summary,
        excerpt(text)
    )
}

fn diff_hunk_digest(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let files = lines
        .iter()
        .filter_map(|line| line.strip_prefix("diff --git "))
        .take(20)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let hunk_count = lines.iter().filter(|line| line.starts_with("@@ ")).count();
    let additions = lines
        .iter()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .count();
    let removals = lines
        .iter()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .count();
    format!(
        "diff_hunk_digest\nlines_total: {}\nfiles: {}\nhunks: {}\nadditions: {}\nremovals: {}\nexcerpt:\n{}",
        lines.len(),
        if files.is_empty() {
            "(unknown)".to_string()
        } else {
            files.join(" | ")
        },
        hunk_count,
        additions,
        removals,
        excerpt(text)
    )
}

fn compiler_diagnostic_digest(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let mut diagnostic_lines = Vec::new();
    for line in &lines {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error[")
            || lower.starts_with("error:")
            || lower.starts_with("warning:")
            || lower.contains("failed")
            || lower.contains("panicked")
        {
            diagnostic_lines.push((*line).to_string());
        }
        if diagnostic_lines.len() >= 20 {
            break;
        }
    }
    let error_count = lines
        .iter()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("error[") || lower.starts_with("error:")
        })
        .count();
    let warning_count = lines
        .iter()
        .filter(|line| line.to_ascii_lowercase().starts_with("warning:"))
        .count();
    format!(
        "compiler_diagnostic_digest\nlines_total: {}\nerrors: {}\nwarnings: {}\nprimary_lines: {}\nexcerpt:\n{}",
        lines.len(),
        error_count,
        warning_count,
        if diagnostic_lines.is_empty() {
            "(none)".to_string()
        } else {
            diagnostic_lines.join(" | ")
        },
        excerpt(text)
    )
}

fn excerpt(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= EXCERPT_CHARS * 2 {
        return text.to_string();
    }
    let head = text.chars().take(EXCERPT_CHARS).collect::<String>();
    let tail = text
        .chars()
        .rev()
        .take(EXCERPT_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{head}\n...\n{tail}")
}

#[cfg(test)]
mod batch_reduction_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::models::FunctionCallOutputBody;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn reduces_source_reads_even_when_recent_with_artifact_recovery() {
        let old_source = (0..180)
            .map(|index| format!("pub fn function_{index}() -> usize {{ {index} }}"))
            .collect::<Vec<_>>()
            .join("\n");
        let recent_source = (0..180)
            .map(|index| format!("pub fn recent_function_{index}() -> usize {{ {index} }}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut items = vec![
            shell_call("call-old", "Get-Content -LiteralPath src/lib.rs"),
            shell_output("call-old", old_source),
            shell_call("call-recent", "Get-Content -LiteralPath src/main.rs"),
            shell_output("call-recent", recent_source),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 2);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert!(
            output
                .text_content()
                .unwrap()
                .contains("source_read_digest")
        );
        let ResponseItem::FunctionCallOutput { output, .. } = &items[3] else {
            panic!("expected output");
        };
        assert!(
            output
                .text_content()
                .unwrap()
                .contains("source_read_digest")
        );
    }

    #[test]
    fn reduces_duplicate_recent_blocks() {
        let paths = (0..80)
            .map(|index| format!("codex-rs/core/src/file_{index}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut items = vec![
            shell_call("call-a", "rg --files"),
            shell_output("call-a", paths.clone()),
            shell_call("call-b", "rg --files"),
            shell_output("call-b", paths),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 2);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert!(output.text_content().unwrap().contains("path_inventory"));
        let ResponseItem::FunctionCallOutput { output, .. } = &items[3] else {
            panic!("expected output");
        };
        assert!(output.text_content().unwrap().contains("duplicate_block"));
    }

    #[test]
    fn disabled_by_threshold_keeps_small_outputs_inline() {
        let mut items = vec![
            shell_call("call", "rg --files"),
            shell_output("call", "src/lib.rs".to_string()),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert_eq!(output.text_content().unwrap(), "src/lib.rs");
    }

    #[test]
    fn omits_stale_self_review_prompt_text() {
        let text = [
            "Automatic self-review of the just-completed work slice.",
            "dirty tracked files at anchor: codex-rs/core/src/session/turn.rs, codex-rs/config/src/types.rs",
            "exact diff commands:",
            "`git diff -- codex-rs/core/src/session/turn.rs`",
            "compact work notes:",
        ]
        .join("\n");
        let mut items = vec![message("user", text, MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        assert_eq!(stats.artifacts, 1);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        assert_eq!(
            content,
            &vec![ContentItem::InputText {
                text: String::new()
            }]
        );
    }

    #[test]
    fn preserves_recent_plan_review_prompt() {
        let text = "Self-review the plan below before implementation.\n\nCurrent plan:\n# Deploy Prompt Reducer\n- run tests".to_string();
        let mut items = vec![message("user", text.clone(), MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        assert_eq!(content, &vec![ContentItem::InputText { text }]);
    }

    #[test]
    fn preserves_repeated_auto_loop_continuations() {
        let text = [
            "Automatic periodic loop continuation: go on",
            "",
            "Loop mode is on, so follow-ups are likely. Enter Plan mode before acting. Every main-agent task plan prompt must inject a delegation decision: state what to delegate to subagents when delegation is useful, or state that the work stays local and why.",
            "Include an Agent ROI Estimate with loop_followup_gain, call list_agents before spawning related follow-up work, prefer followup_task/send_message/resume_agent over a replacement agent, compact useful token-heavy agents before reuse, and decide what work to give any idle relevant agent.",
        ]
        .join("\n");
        let mut items = vec![
            message("user", text.clone(), MessageTextKind::Input),
            message("user", text.clone(), MessageTextKind::Input),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        for item in items {
            let ResponseItem::Message { content, .. } = item else {
                panic!("expected message");
            };
            assert_eq!(content, vec![ContentItem::InputText { text: text.clone() }]);
        }
    }

    #[test]
    fn preserves_existing_prompt_reduction_replacement_blocks() {
        let text = [
            "[prompt reduction: duplicate_block]",
            "original_chars: 1267",
            "original_tokens_estimate: 317",
            "sha1: 096392ae6bed98a68e774c836500913cb3099de7",
            "artifact: `C:\\Users\\Oleh\\AppData\\Local\\Temp\\codex-prompt-reducer\\019e3a14-4ce9-7253-8eb9-c8c7858e7526\\prompt-item-0101-duplicate_block-096392ae6bed.txt`",
            "recovery: read artifact before using exact lines.",
            "",
            "Exact duplicate of earlier prompt item `text-slot-98`.",
        ]
        .join("\n");
        let mut items = (0..6)
            .map(|index| structured_text_output(&format!("call-{index}"), text.clone()))
            .collect::<Vec<_>>();
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        for item in items {
            let ResponseItem::FunctionCallOutput { output, .. } = item else {
                panic!("expected function output");
            };
            let FunctionCallOutputBody::ContentItems(content) = output.body else {
                panic!("expected structured function output");
            };
            assert_eq!(
                content,
                vec![
                    FunctionCallOutputContentItem::InputText { text: text.clone() },
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "data:image/png;base64,AA==".to_string(),
                        detail: None,
                    },
                ]
            );
        }
    }

    #[test]
    fn omits_stale_prompt_reduction_notice() {
        assert!(is_prompt_reduction_status_notice(
            "message:assistant",
            "Prompt reducer: sent prompt reduced by 76% (3 artifacts)."
        ));
        assert!(is_prompt_reduction_status_notice(
            "client_event",
            "🌈 Prompt reduction: prompt unchanged (0.0%; 8.5k estimated tokens)."
        ));
        let mut items = vec![message(
            "assistant",
            "🌈 Prompt reduction: optimized prompt by 76% (55.5k -> 13.3k estimated tokens; 3 artifacts)."
                .to_string(),
            MessageTextKind::Output,
        )];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        assert_eq!(
            content,
            &vec![ContentItem::OutputText {
                text: String::new()
            }]
        );
    }

    #[test]
    fn reduces_recent_search_results() {
        let output = (0..60)
            .map(|index| {
                format!(
                    "codex-rs/core/src/session/file_{index}.rs:{}:fn prompt_reduction_{index}() {{}}",
                    index + 1
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut items = vec![
            shell_call("call", "rg -n prompt_reduction codex-rs"),
            shell_output("call", output),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert!(
            output
                .text_content()
                .unwrap()
                .contains("recent_search_result_digest")
        );
    }

    #[test]
    fn reduces_tool_search_output_and_recent_source_read() {
        let old_source = "let stale = 1;\n".repeat(100);
        let recent_source = "let recent = 2;\n".repeat(100);
        let mut items = vec![
            shell_call("old", "Get-Content -LiteralPath src/old.rs"),
            shell_output("old", old_source),
            tool_search_output(vec![serde_json::json!({
                "name": "expensive_tool",
                "description": "x".repeat(500),
            })]),
            shell_call("recent", "Get-Content -LiteralPath src/recent.rs"),
            shell_output("recent", recent_source),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 3);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[4] else {
            panic!("expected recent source output");
        };
        assert!(
            output
                .text_content()
                .unwrap()
                .contains("source_read_digest")
        );
    }

    #[test]
    fn reduces_structured_function_output_text_with_artifact_recovery() {
        let source = (0..180)
            .map(|index| format!("pub fn structured_function_{index}() -> usize {{ {index} }}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut items = vec![
            shell_call("structured", "Get-Content -LiteralPath src/structured.rs"),
            structured_text_output("structured", source),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected structured output");
        };
        let content_items = output.content_items().unwrap();
        let FunctionCallOutputContentItem::InputText { text } = &content_items[0] else {
            panic!("expected structured text output");
        };
        assert!(text.contains("source_read_digest"));
        assert!(matches!(
            &content_items[1],
            FunctionCallOutputContentItem::InputImage { .. }
        ));
    }

    #[test]
    fn reduces_build_status_json_without_command_lines() {
        let long_command = "cargo test -p codex-core --release ".repeat(80);
        let output = serde_json::json!({
            "status": "ok",
            "mode": "Status",
            "active_build_processes": [
                {
                    "process_name": "cargo.exe",
                    "command_line": long_command,
                }
            ],
            "release_binary": {
                "last_write_time": "2026-05-16T03:00:00+03:00",
            },
            "wrapper_real_exe": "C:/Users/Oleh/.codex/local-builds/codex-custom/codex.exe",
            "release_profile_state": {
                "matches": true,
            },
            "free_c_drive_bytes": 123456789u64,
        })
        .to_string();
        let mut items = vec![
            shell_call("call", "scripts\\build-local-codex.ps1 -Mode Status"),
            shell_output("call", output),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        let text = output.text_content().unwrap();
        assert!(text.contains("recent_build_status_digest"));
        assert!(!text.contains(&long_command));
    }

    #[test]
    fn preserves_durable_developer_instruction_blocks() {
        let instructions = format!(
            "<skills_instructions>\n## Skills\n{}\n</skills_instructions>",
            "- Use the local build and deployment rules exactly.\n".repeat(120)
        );
        let mut items = vec![message(
            "developer",
            instructions.clone(),
            MessageTextKind::Input,
        )];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        assert_eq!(
            content,
            &vec![ContentItem::InputText { text: instructions }]
        );
    }

    #[test]
    fn preserves_collaboration_mode_developer_blocks() {
        let instructions = format!(
            "<collaboration_mode>\n# Plan Mode (Conversational)\n{}\n</collaboration_mode>",
            "Do not mutate files in Plan mode.\n".repeat(120)
        );
        let mut items = vec![message(
            "developer",
            instructions.clone(),
            MessageTextKind::Input,
        )];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        assert_eq!(
            content,
            &vec![ContentItem::InputText { text: instructions }]
        );
    }

    #[test]
    fn patch_markers_inside_source_reads_reduce_as_source_read() {
        let source = format!(
            "fn before() {{}}\n*** Begin Patch\n{}\n*** End Patch\nfn after() {{}}",
            "let x = 1;\n".repeat(250)
        );
        let mut items = vec![
            shell_call("call", "Get-Content -LiteralPath docs/patch-notes.md"),
            shell_output("call", source),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        let text = output.text_content().unwrap();
        assert!(text.contains("source_read_digest"));
        assert!(!text.contains("patch_output"));
    }

    #[test]
    fn long_instruction_like_tool_outputs_are_not_command_logs() {
        let original = format!(
            "<skills_instructions>\n## Skills\n{}\n</skills_instructions>",
            "- Follow this durable instruction line.\n".repeat(120)
        );
        let mut items = vec![
            shell_call("call", "custom instruction loader"),
            shell_output("call", original.clone()),
        ];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert_eq!(output.text_content().unwrap(), original);
    }

    #[test]
    fn reduces_stale_context_pack() {
        let paths = (0..80)
            .map(|index| format!("- codex-rs/core/src/file_{index}.rs | score=0.{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut items = vec![message(
            "assistant",
            format!("<context_pack variant=\"graphify_scout_pack\">\n{paths}\n</context_pack>"),
            MessageTextKind::Output,
        )];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::OutputText { text } = &content[0] else {
            panic!("expected output text");
        };
        assert!(text.contains("context_pack_digest"));
    }

    #[test]
    fn omits_old_subagent_status_notification_without_handoff_detail() {
        let text = r#"<subagent_notification>
{"agent_path":"/root/helper","status":"running"}
</subagent_notification>"#
            .to_string();
        let mut items = vec![message("user", text, MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        assert_eq!(stats.artifacts, 0);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::InputText { text } = &content[0] else {
            panic!("expected input text");
        };
        assert!(text.is_empty());
    }

    #[test]
    fn reduces_completed_subagent_notification_to_digest() {
        let handoff = "helper found prompt reducer insertion points\n".repeat(80);
        let text = format!(
            "<subagent_notification>\n{}\n</subagent_notification>",
            serde_json::json!({
                "agent_path": "/root/helper",
                "status": { "completed": handoff },
            })
        );
        let mut items = vec![message("user", text, MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        assert_eq!(stats.artifacts, 1);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::InputText { text } = &content[0] else {
            panic!("expected input text");
        };
        assert!(text.contains("subagent_notification_digest"));
        assert!(text.contains("agent: /root/helper"));
        assert!(text.contains("status: completed"));
        assert!(text.contains("detail_excerpt:"));
        assert!(!text.contains(&handoff));
    }

    #[test]
    fn reduces_wrapped_subagent_notification_content_to_digest() {
        let handoff = "wrapped helper handoff\n".repeat(80);
        let notification = format!(
            "<subagent_notification>\n{}\n</subagent_notification>",
            serde_json::json!({
                "agent_path": "/root/helper",
                "status": { "completed": handoff },
            })
        );
        let text = serde_json::json!({
            "author": "/root/helper",
            "content": notification,
        })
        .to_string();
        let mut items = vec![message("user", text, MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        assert_eq!(stats.artifacts, 1);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::InputText { text } = &content[0] else {
            panic!("expected input text");
        };
        assert!(text.contains("subagent_notification_digest"));
        assert!(text.contains("agent: /root/helper"));
        assert!(text.contains("status: completed"));
    }

    #[test]
    fn preserves_recent_subagent_notification() {
        let text = r#"<subagent_notification>
{"agent_path":"/root/helper","status":"running"}
</subagent_notification>"#
            .to_string();
        let mut items = vec![message("user", text.clone(), MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::InputText { text: actual } = &content[0] else {
            panic!("expected input text");
        };
        assert_eq!(actual, &text);
    }

    #[test]
    fn does_not_reduce_search_result_that_mentions_subagent_fields() {
        let text = "search hit: code mentions \"agent_path\" and \"status\", but this is not a notification\n"
            .repeat(20);
        let mut items = vec![message("user", text.clone(), MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::InputText { text: actual } = &content[0] else {
            panic!("expected input text");
        };
        assert_eq!(actual, &text);
    }

    #[test]
    fn does_not_reduce_malformed_subagent_notification_marker() {
        let text = "<subagent_notification>\nnot json\n</subagent_notification>\n".repeat(20);
        let mut items = vec![message("user", text.clone(), MessageTextKind::Input)];
        let temp = TempDir::new().unwrap();
        let config = test_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::InputText { text: actual } = &content[0] else {
            panic!("expected input text");
        };
        assert_eq!(actual, &text);
    }

    #[test]
    fn reduces_small_structured_command_logs_below_global_threshold() {
        let output = format!(
            "Exit code: 0\nWall time: 0.4 seconds\nOutput:\n{}",
            (0..36)
                .map(|index| format!("line {index}: prompt reduction audit output"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let mut items = vec![
            shell_call("call", "rg prompt reduction"),
            shell_output("call", output),
        ];
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        assert!(stats.saved_tokens > 32);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected output");
        };
        assert!(
            output
                .text_content()
                .unwrap()
                .contains("command_log_digest")
        );
    }

    #[test]
    fn bundles_many_short_low_utility_tool_outputs() {
        let mut items = Vec::new();
        for index in 0..12 {
            let call_id = format!("call-{index}");
            items.push(shell_call(&call_id, &format!("echo short-{index}")));
            items.push(shell_output(
                &call_id,
                format!(
                    "Exit code: 0\nWall time: 0.{index} seconds\nOutput:\n{}",
                    "successful short command output with no next-step signal\n".repeat(4)
                ),
            ));
        }
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 12);
        assert_eq!(stats.artifacts, 1);
        assert!(stats.saved_tokens > 300);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[1] else {
            panic!("expected first output");
        };
        assert!(
            output
                .text_content()
                .unwrap()
                .contains("short_tool_output_bundle")
        );
        for item in items.iter().skip(3).step_by(2) {
            let ResponseItem::FunctionCallOutput { output, .. } = item else {
                panic!("expected output");
            };
            assert_eq!(output.text_content().unwrap(), "");
        }
    }

    #[test]
    fn short_tool_bundle_artifact_ignores_non_text_outputs_without_slot_shift() {
        let mut items = vec![
            shell_call("image", "capture screenshot"),
            image_output("image"),
        ];
        for index in 0..12 {
            let call_id = format!("call-{index}");
            items.push(shell_call(&call_id, &format!("echo short-{index}")));
            items.push(shell_output(
                &call_id,
                format!(
                    "Exit code: 0\nWall time: 0.{index} seconds\nOutput:\n{}",
                    "successful short command output with no next-step signal\n".repeat(4)
                ),
            ));
        }
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 12);
        assert_eq!(stats.artifacts, 1);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[3] else {
            panic!("expected first text output");
        };
        let replacement = output.text_content().unwrap();
        assert!(replacement.contains("call-0"));
        let artifact_path = fs::read_dir(temp.path())
            .unwrap()
            .next()
            .expect("expected bundle artifact")
            .unwrap()
            .path();
        let artifact = fs::read_to_string(artifact_path).unwrap();
        assert!(artifact.contains("echo short-0"));
        assert!(artifact.contains("echo short-11"));
    }

    #[test]
    fn short_tool_bundle_artifact_tracks_mixed_text_slots() {
        let mut items = vec![
            message("user", "keep this slot".to_string(), MessageTextKind::Input),
            tool_search_output(vec![serde_json::json!({ "name": "tiny_tool" })]),
            shell_call("image", "capture screenshot"),
            image_output("image"),
        ];
        for index in 0..12 {
            let call_id = format!("call-{index}");
            items.push(shell_call(&call_id, &format!("echo mixed-{index}")));
            items.push(shell_output(
                &call_id,
                format!(
                    "Exit code: 0\nWall time: 0.{index} seconds\nOutput:\n{}",
                    "successful mixed short command output with no next-step signal\n".repeat(4)
                ),
            ));
        }
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 12);
        let ResponseItem::FunctionCallOutput { output, .. } = &items[5] else {
            panic!("expected first text output");
        };
        let replacement = output.text_content().unwrap();
        assert!(replacement.contains("call-0"));
        assert!(!replacement.contains("image"));
        let artifact = fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| fs::read_to_string(entry.path()).unwrap())
            .find(|text| text.contains("short_tool_output_bundle_artifact"))
            .expect("expected bundle artifact");
        assert!(artifact.contains("echo mixed-0"));
        assert!(artifact.contains("echo mixed-11"));
        assert!(!artifact.contains("capture screenshot"));
    }

    #[test]
    fn does_not_bundle_short_failed_tool_outputs() {
        let mut items = Vec::new();
        let mut originals = Vec::new();
        for index in 0..12 {
            let call_id = format!("call-{index}");
            let text = format!(
                "Exit code: 1\nWall time: 0.{index} seconds\nOutput:\nerror: important failure {index}"
            );
            items.push(shell_call(&call_id, &format!("test-command-{index}")));
            items.push(shell_output(&call_id, text.clone()));
            originals.push(text);
        }
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 0);
        for (original, item) in originals.iter().zip(items.iter().skip(1).step_by(2)) {
            let ResponseItem::FunctionCallOutput { output, .. } = item else {
                panic!("expected output");
            };
            assert_eq!(output.text_content().unwrap(), original);
        }
    }

    #[test]
    fn reduces_assistant_status_json_with_content_excerpt() {
        let content = format!(
            "I am checking the reducer and found the assistant status payload path. {}",
            "This progress detail should remain visible after compaction. ".repeat(20)
        );
        let status = serde_json::json!({
            "author": "assistant",
            "recipient": "functions.update_plan",
            "other_recipients": [],
            "content": content,
            "trigger_turn": 42,
        })
        .to_string();
        let mut items = vec![message("assistant", status, MessageTextKind::Output)];
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::OutputText { text } = &content[0] else {
            panic!("expected output text");
        };
        assert!(text.contains("assistant_status_digest"));
        assert!(text.contains("recipient: functions.update_plan"));
        assert!(text.contains("This progress detail should remain visible"));
        assert!(!text.contains("object_keys_total"));
    }

    #[test]
    fn reduces_recent_assistant_status_json_before_generic_json() {
        let content = format!(
            "Recent progress update with implementation state. {}",
            "Keep this status readable in the reduced prompt. ".repeat(80)
        );
        let status = serde_json::json!({
            "author": "assistant",
            "recipient": "functions.shell_command",
            "other_recipients": null,
            "content": content,
            "trigger_turn": 99,
        })
        .to_string();
        let mut items = vec![message("assistant", status, MessageTextKind::Output)];
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 1);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::OutputText { text } = &content[0] else {
            panic!("expected output text");
        };
        assert!(text.contains("recent_assistant_status_digest"));
        assert!(text.contains("assistant_status_digest"));
        assert!(text.contains("recipient: functions.shell_command"));
        assert!(!text.contains("object_keys_total"));
    }

    #[test]
    fn reduces_recoverable_prior_assistant_context() {
        let text = "Earlier assistant progress update with local observations. ".repeat(40);
        let mut items = vec![message("assistant", text, MessageTextKind::Output)];
        let temp = TempDir::new().unwrap();
        let config = default_threshold_config(temp.path(), 0);

        let stats = reduce_prompt_items(&mut items, &config).unwrap();

        assert_eq!(stats.reductions, 1);
        assert!(stats.saved_tokens > 64);
        let ResponseItem::Message { content, .. } = &items[0] else {
            panic!("expected message");
        };
        let ContentItem::OutputText { text } = &content[0] else {
            panic!("expected output text");
        };
        assert!(text.contains("recoverable_prior_context_digest"));
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

    fn default_threshold_config(
        path: &Path,
        preserve_recent_items: usize,
    ) -> PromptReductionConfig {
        PromptReductionConfig {
            artifact_dir: path.to_path_buf(),
            min_reduce_chars: 2_000,
            path_list_threshold: 12,
            min_saved_tokens: 128,
            preserve_recent_items,
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

    fn image_output(call_id: &str) -> ResponseItem {
        ResponseItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output: FunctionCallOutputPayload::from_content_items(vec![
                FunctionCallOutputContentItem::InputImage {
                    image_url: "data:image/png;base64,AA==".to_string(),
                    detail: None,
                },
            ]),
        }
    }

    fn structured_text_output(call_id: &str, text: String) -> ResponseItem {
        ResponseItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output: FunctionCallOutputPayload::from_content_items(vec![
                FunctionCallOutputContentItem::InputText { text },
                FunctionCallOutputContentItem::InputImage {
                    image_url: "data:image/png;base64,AA==".to_string(),
                    detail: None,
                },
            ]),
        }
    }

    fn tool_search_output(tools: Vec<Value>) -> ResponseItem {
        ResponseItem::ToolSearchOutput {
            call_id: Some("tool-search".to_string()),
            status: "completed".to_string(),
            execution: "test".to_string(),
            tools,
        }
    }

    enum MessageTextKind {
        Input,
        Output,
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
}
