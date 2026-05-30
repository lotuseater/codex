use super::*;

pub(crate) fn build_prompt(
    input: Vec<ResponseItem>,
    router: &ToolRouter,
    turn_context: &TurnContext,
    base_instructions: BaseInstructions,
) -> Prompt {
    Prompt {
        input,
        tools: router.model_visible_specs(),
        parallel_tool_calls: turn_context.model_info.supports_parallel_tool_calls,
        base_instructions,
        personality: turn_context.personality,
        output_schema: turn_context.final_output_json_schema.clone(),
        output_schema_strict: !crate::guardian::is_guardian_reviewer_source(
            &turn_context.session_source,
        ),
    }
}

#[derive(Debug)]
pub(super) struct PromptReductionNotice {
    original_prompt_tokens: usize,
    reduced_prompt_tokens: usize,
    stats: PromptReductionStats,
}

pub(super) fn reduce_prompt_input_for_model(
    mut input: Vec<ResponseItem>,
    turn_context: &TurnContext,
) -> (Vec<ResponseItem>, Option<PromptReductionNotice>) {
    match turn_context.config.prompt_reduction_mode {
        PromptReductionModeToml::Off => (input, None),
        PromptReductionModeToml::Conservative => {
            let reduction_config = PromptReductionConfig::for_turn(&turn_context.sub_id);
            let original_input_tokens = estimate_prompt_input_tokens(&input);
            match codex_prompt_reducer::reduce_prompt_items(&mut input, &reduction_config) {
                Ok(stats) => {
                    let reduced_input_tokens = estimate_prompt_input_tokens(&input);
                    if stats.reductions > 0 {
                        trace!(
                            turn_id = %turn_context.sub_id,
                            reductions = stats.reductions,
                            artifacts = stats.artifacts,
                            original_tokens = stats.original_tokens,
                            reduced_tokens = stats.reduced_tokens,
                            saved_tokens = stats.saved_tokens,
                            artifact_dir = %reduction_config.artifact_dir.display(),
                            "reduced prompt input"
                        );
                    }
                    let notice = PromptReductionNotice {
                        original_prompt_tokens: original_input_tokens,
                        reduced_prompt_tokens: reduced_input_tokens,
                        stats,
                    };
                    (input, Some(notice))
                }
                Err(err) => {
                    warn!(
                        turn_id = %turn_context.sub_id,
                        error = %err,
                        "failed to reduce prompt input"
                    );
                    (input, None)
                }
            }
        }
    }
}

fn estimate_prompt_input_tokens(input: &[ResponseItem]) -> usize {
    let mut value = serde_json::to_value(input).unwrap_or(serde_json::Value::Null);
    redact_input_image_urls_for_text_estimate(&mut value);
    estimate_serialized_tokens(&value)
}

fn redact_input_image_urls_for_text_estimate(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                redact_input_image_urls_for_text_estimate(item);
            }
        }
        serde_json::Value::Object(map) => {
            if map.get("type").and_then(serde_json::Value::as_str) == Some("input_image")
                && let Some(serde_json::Value::String(image_url)) = map.get_mut("image_url")
            {
                *image_url = image_text_estimate_placeholder(image_url);
            }
            for value in map.values_mut() {
                redact_input_image_urls_for_text_estimate(value);
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

fn image_text_estimate_placeholder(image_url: &str) -> String {
    format!(
        "[image omitted from text-token estimate: {} chars]",
        image_url.chars().count()
    )
}

fn estimate_serialized_tokens<T: serde::Serialize>(value: &T) -> usize {
    serde_json::to_string(value)
        .map(|text| approx_tokens(&text))
        .unwrap_or_default()
}

fn approx_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

pub(super) fn add_static_prompt_tokens(
    notice: &mut PromptReductionNotice,
    router: &ToolRouter,
    turn_context: &TurnContext,
    base_instructions: &BaseInstructions,
) {
    let static_tokens = approx_tokens(&base_instructions.text)
        .saturating_add(estimate_serialized_tokens(&router.model_visible_specs()))
        .saturating_add(estimate_serialized_tokens(
            &turn_context.final_output_json_schema,
        ));
    notice.original_prompt_tokens = notice.original_prompt_tokens.saturating_add(static_tokens);
    notice.reduced_prompt_tokens = notice.reduced_prompt_tokens.saturating_add(static_tokens);
}

pub(super) async fn maybe_send_prompt_reduction_notice(
    sess: &Session,
    turn_context: &TurnContext,
    notice: Option<PromptReductionNotice>,
) {
    let Some(notice) = notice else {
        return;
    };
    if !should_show_prompt_reduction_notice(turn_context) || notice.original_prompt_tokens == 0 {
        return;
    }

    let message = prompt_reduction_notice_message(&notice);
    // This is a client event, not a prompt item, so it is user-visible without
    // becoming model-visible context on later turns.
    sess.send_event(turn_context, EventMsg::Warning(WarningEvent { message }))
        .await;
}

fn should_show_prompt_reduction_notice(turn_context: &TurnContext) -> bool {
    !turn_context.session_source.is_non_root_agent()
        && !crate::guardian::is_guardian_reviewer_source(&turn_context.session_source)
        && matches!(turn_context.thread_source, None | Some(ThreadSource::User))
}

fn prompt_reduction_notice_message(notice: &PromptReductionNotice) -> String {
    let saved_tokens = notice
        .original_prompt_tokens
        .saturating_sub(notice.reduced_prompt_tokens);
    let saved_percent = saved_tokens as f64 * 100.0 / notice.original_prompt_tokens as f64;
    let original = format_compact_tokens(notice.original_prompt_tokens);
    let reduced = format_compact_tokens(notice.reduced_prompt_tokens);
    if notice.stats.reductions == 0 {
        format!("Prompt reduction: prompt unchanged (0.0%; {original} estimated tokens).")
    } else {
        format!(
            "Prompt reduction: optimized prompt by {saved_percent:.1}% ({original} -> {reduced} estimated tokens; {} artifacts).",
            notice.stats.artifacts
        )
    }
}

fn format_compact_tokens(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_input_token_estimate_omits_input_image_data_urls() {
        let base64 = "A".repeat(800_000);
        let input = vec![ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![
                ContentItem::InputText {
                    text: "describe this image".to_string(),
                },
                ContentItem::InputImage {
                    image_url: format!("data:image/png;base64,{base64}"),
                    detail: None,
                },
            ],
            phase: None,
        }];

        let raw_tokens = estimate_serialized_tokens(&input);
        let text_tokens = estimate_prompt_input_tokens(&input);

        assert!(raw_tokens > 190_000);
        assert!(text_tokens < 200);
    }
}
