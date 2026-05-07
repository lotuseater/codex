use crate::approx_tokens;
use crate::types::ChangedAreas;
use crate::types::IndexState;
use crate::types::RepoContextScoutConfig;
use crate::types::RepoIndex;
use crate::types::ScoutCandidate;
use crate::types::SupportRoute;

pub(crate) fn format_packet(
    index: &RepoIndex,
    changed: &ChangedAreas,
    candidates: &[ScoutCandidate],
    routes: &[SupportRoute],
    index_state: IndexState,
    prompt: &str,
    config: &RepoContextScoutConfig,
) -> String {
    let mut lines = vec![
        "<repo_context_scout>".to_string(),
        "role: bounded repo-state scout; use as path hints, not proof".to_string(),
        format!(
            "index_state: {:?}; indexed_files: {}; changed_paths: {}",
            index_state,
            index.files.len(),
            changed.paths.len()
        ),
        format!("project_root: {}", index.project_root.display()),
        format!(
            "task_prompt_digest: {}",
            prompt.chars().take(180).collect::<String>()
        ),
    ];
    if let Some(head) = index.git_head.as_deref() {
        lines.push(format!(
            "git_head: {}",
            head.chars().take(12).collect::<String>()
        ));
    }
    if index.file_limit_reached {
        lines.push("warning: file inventory hit max_files; scout may be incomplete".to_string());
    }
    if !changed.paths.is_empty() {
        lines.push("changed_paths:".to_string());
        for changed_path in changed.paths.iter().take(16) {
            lines.push(format!("- {} {}", changed_path.status, changed_path.path));
        }
        if changed.paths.len() > 16 {
            lines.push(format!(
                "- omitted_changed_paths: {}",
                changed.paths.len() - 16
            ));
        }
    }
    if !candidates.is_empty() {
        lines.push("recommended_reads:".to_string());
        for candidate in candidates {
            lines.push(format!(
                "- {} ({:.1}) - {}",
                candidate.path,
                candidate.score,
                candidate.reasons.join(", ")
            ));
            if !candidate.anchors.is_empty() {
                let anchors = candidate
                    .anchors
                    .iter()
                    .take(3)
                    .map(|anchor| format!("L{} {}", anchor.line, anchor.text))
                    .collect::<Vec<_>>()
                    .join("; ");
                lines.push(format!("  anchors: {anchors}"));
            }
            if approx_tokens(&lines.join("\n")) >= config.max_output_tokens {
                lines.push(
                    "fallback_required: true; packet capped by max_output_tokens".to_string(),
                );
                lines.push("</repo_context_scout>".to_string());
                return cap_packet(lines.join("\n"), config.max_output_tokens);
            }
        }
    }
    if !routes.is_empty() {
        lines.push("support_routes:".to_string());
        for route in routes {
            lines.push(format!("- {} - {}", route.name, route.reason));
        }
    }
    lines.push("</repo_context_scout>".to_string());
    cap_packet(lines.join("\n"), config.max_output_tokens)
}

fn cap_packet(packet: String, max_tokens: usize) -> String {
    if max_tokens == 0 || approx_tokens(&packet) <= max_tokens {
        return packet;
    }
    let suffix =
        "\nfallback_required: true; packet capped by max_output_tokens\n</repo_context_scout>";
    let max_chars = max_tokens.saturating_mul(4);
    if suffix.len() >= max_chars {
        return suffix.trim_start().to_string();
    }
    let prefix_chars = max_chars - suffix.len();
    let prefix = packet.chars().take(prefix_chars).collect::<String>();
    format!("{}{}", prefix.trim_end(), suffix)
}
