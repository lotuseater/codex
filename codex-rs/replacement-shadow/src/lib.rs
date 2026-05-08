use std::path::PathBuf;

mod baseline_digest;
mod classify;

const MIN_REPLACE_SAVED_PERCENT: f64 = 30.0;
const MIN_REPLACE_SAVED_TOKENS: isize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplacementCandidate {
    GitDiffStatCompact,
    GitChangedFiles,
    RgFilesCompact,
    DiffHunkSummary,
    RunCheckDigest,
    FileExcerptDigest,
    SelectStringDigest,
    RgCountDigest,
    RgFileSetDigest,
    RgJsonDigest,
    GitNameStatusCompact,
    GitNumstatCompact,
    GitFilteredDiffDigest,
    GitHistoryDigest,
    DirectoryListingCompact,
    ProcessTableCompact,
    SearchText {
        pattern: String,
        globs: Vec<String>,
        paths: Vec<String>,
    },
    FileOutline {
        path: PathBuf,
    },
}

impl ReplacementCandidate {
    pub fn name(&self) -> &'static str {
        match self {
            Self::GitDiffStatCompact => "git_diffstat_compact",
            Self::GitChangedFiles => "git_changed_files",
            Self::RgFilesCompact => "rg_files_compact",
            Self::DiffHunkSummary => "diff_hunk_summary",
            Self::RunCheckDigest => "run_check_digest",
            Self::FileExcerptDigest => "file_excerpt_digest",
            Self::SelectStringDigest => "select_string_digest",
            Self::RgCountDigest => "rg_count_digest",
            Self::RgFileSetDigest => "rg_file_set_digest",
            Self::RgJsonDigest => "rg_json_digest",
            Self::GitNameStatusCompact => "git_name_status_compact",
            Self::GitNumstatCompact => "git_numstat_compact",
            Self::GitFilteredDiffDigest => "git_filtered_diff_digest",
            Self::GitHistoryDigest => "git_history_digest",
            Self::DirectoryListingCompact => "directory_listing_compact",
            Self::ProcessTableCompact => "process_table_compact",
            Self::SearchText { .. } => "search_text",
            Self::FileOutline { .. } => "file_outline",
        }
    }

    pub fn strategy(&self) -> &'static str {
        match self {
            Self::SearchText { .. } | Self::FileOutline { .. } => "context_op_rerun",
            Self::GitDiffStatCompact
            | Self::GitChangedFiles
            | Self::RgFilesCompact
            | Self::DiffHunkSummary
            | Self::RunCheckDigest
            | Self::FileExcerptDigest
            | Self::SelectStringDigest
            | Self::RgCountDigest
            | Self::RgFileSetDigest
            | Self::RgJsonDigest
            | Self::GitNameStatusCompact
            | Self::GitNumstatCompact
            | Self::GitFilteredDiffDigest
            | Self::GitHistoryDigest
            | Self::DirectoryListingCompact
            | Self::ProcessTableCompact => "baseline_digest",
        }
    }

    pub fn render_baseline_digest(&self, baseline_model_visible_output: &str) -> Option<String> {
        match self {
            Self::GitDiffStatCompact => Some(baseline_digest::render_git_diffstat_compact(
                baseline_model_visible_output,
            )),
            Self::GitChangedFiles => Some(baseline_digest::render_changed_files_compact(
                "git_changed_files",
                baseline_model_visible_output,
            )),
            Self::RgFilesCompact => Some(baseline_digest::render_changed_files_compact(
                "rg_files_compact",
                baseline_model_visible_output,
            )),
            Self::DiffHunkSummary => Some(baseline_digest::render_diff_hunk_summary(
                baseline_model_visible_output,
            )),
            Self::RunCheckDigest => Some(baseline_digest::render_run_check_digest(
                baseline_model_visible_output,
            )),
            Self::FileExcerptDigest => Some(baseline_digest::render_file_excerpt_digest(
                baseline_model_visible_output,
            )),
            Self::SelectStringDigest => Some(baseline_digest::render_select_string_digest(
                baseline_model_visible_output,
            )),
            Self::RgCountDigest => Some(baseline_digest::render_rg_count_digest(
                baseline_model_visible_output,
            )),
            Self::RgFileSetDigest => Some(baseline_digest::render_changed_files_compact(
                "rg_file_set_digest",
                baseline_model_visible_output,
            )),
            Self::RgJsonDigest => Some(baseline_digest::render_rg_json_digest(
                baseline_model_visible_output,
            )),
            Self::GitNameStatusCompact => Some(baseline_digest::render_git_name_status_compact(
                baseline_model_visible_output,
            )),
            Self::GitNumstatCompact => Some(baseline_digest::render_git_numstat_compact(
                baseline_model_visible_output,
            )),
            Self::GitFilteredDiffDigest => Some(baseline_digest::render_git_filtered_diff_digest(
                baseline_model_visible_output,
            )),
            Self::GitHistoryDigest => Some(baseline_digest::render_git_history_digest(
                baseline_model_visible_output,
            )),
            Self::DirectoryListingCompact => Some(
                baseline_digest::render_directory_listing_compact(baseline_model_visible_output),
            ),
            Self::ProcessTableCompact => Some(baseline_digest::render_process_table_compact(
                baseline_model_visible_output,
            )),
            Self::SearchText { .. } | Self::FileOutline { .. } => None,
        }
    }
}

pub fn classify_shell_replacement(command: &str) -> Option<ReplacementCandidate> {
    classify::classify_shell_replacement(command)
}

pub fn classify_promoted_replacement(command: &str) -> Option<ReplacementCandidate> {
    classify::classify_promoted_replacement(command)
}

pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4).max(1)
}

pub fn render_replacement_output(
    command: &str,
    operation: &str,
    replacement_output: &str,
) -> String {
    format!(
        "context_ops_replace: {operation}\nraw_command: {command}\nraw_output: omitted; rerun the raw command if exact output is needed.\n{replacement_output}"
    )
}

pub fn should_replace_model_output(
    baseline_model_visible_output: &str,
    replacement_model_visible_output: &str,
) -> bool {
    let baseline_tokens = estimate_tokens(baseline_model_visible_output);
    let replacement_tokens = estimate_tokens(replacement_model_visible_output);
    let saved_tokens = baseline_tokens as isize - replacement_tokens as isize;
    if saved_tokens < MIN_REPLACE_SAVED_TOKENS {
        return false;
    }
    let saved_percent = saved_tokens as f64 / baseline_tokens as f64 * 100.0;
    saved_percent >= MIN_REPLACE_SAVED_PERCENT
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn estimates_tokens_without_zero() {
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens("12345"), 2);
    }

    #[test]
    fn replacement_requires_meaningful_token_savings() {
        assert!(should_replace_model_output(
            "x".repeat(1_000).as_str(),
            "context_ops_replace\nsmall"
        ));
        assert!(!should_replace_model_output("tiny", "larger replacement"));
        assert!(!should_replace_model_output(
            "x".repeat(1_000).as_str(),
            "y".repeat(760).as_str()
        ));
    }

    #[test]
    fn reports_shadow_strategy_for_rerun_and_baseline_candidates() {
        assert_eq!(
            ReplacementCandidate::SearchText {
                pattern: "needle".to_string(),
                globs: Vec::new(),
                paths: Vec::new()
            }
            .strategy(),
            "context_op_rerun"
        );
        assert_eq!(
            ReplacementCandidate::GitDiffStatCompact.strategy(),
            "baseline_digest"
        );
        assert_eq!(
            ReplacementCandidate::GitFilteredDiffDigest.strategy(),
            "baseline_digest"
        );
    }
}
