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

    pub fn intent(&self) -> &'static str {
        match self {
            Self::RgFileSetDigest => "file_discovery",
            Self::SearchText { .. } => "snippet_discovery",
            Self::RgCountDigest => "distribution_counting",
            Self::RgJsonDigest | Self::SelectStringDigest => "search_output_digest",
            Self::RgFilesCompact | Self::DirectoryListingCompact => "repo_inventory",
            Self::FileOutline { .. } | Self::FileExcerptDigest => "read_orientation",
            Self::GitDiffStatCompact => "git_summary",
            Self::GitChangedFiles | Self::GitNameStatusCompact | Self::GitNumstatCompact => {
                "git_changed_path_summary"
            }
            Self::DiffHunkSummary | Self::GitFilteredDiffDigest => "diff_orientation",
            Self::GitHistoryDigest => "history_orientation",
            Self::RunCheckDigest => "check_diagnostics",
            Self::ProcessTableCompact => "runtime_orientation",
        }
    }

    pub fn replacement_policy(&self) -> &'static str {
        match self {
            Self::GitDiffStatCompact => "direct_only_when_original_is_summary",
            Self::RgFileSetDigest => "shadow_or_direct_file_set_only",
            Self::SearchText { .. } | Self::FileOutline { .. } => "explicit_tool_or_shadow_only",
            Self::DiffHunkSummary | Self::GitFilteredDiffDigest | Self::RunCheckDigest => {
                "needs_artifact"
            }
            Self::GitChangedFiles
            | Self::RgFilesCompact
            | Self::FileExcerptDigest
            | Self::SelectStringDigest
            | Self::RgCountDigest
            | Self::RgJsonDigest
            | Self::GitNameStatusCompact
            | Self::GitNumstatCompact
            | Self::GitHistoryDigest
            | Self::DirectoryListingCompact
            | Self::ProcessTableCompact => "shadow_only",
        }
    }

    pub fn direct_replacement_safe(&self) -> bool {
        matches!(self, Self::GitDiffStatCompact)
    }

    pub fn useful_when(&self) -> &'static str {
        match self {
            Self::RgFileSetDigest => "deciding which files mention a pattern before targeted reads",
            Self::SearchText { .. } => "finding representative matches that guide next file reads",
            Self::RgCountDigest => "estimating where a pattern is concentrated",
            Self::RgJsonDigest | Self::SelectStringDigest => {
                "summarizing large search outputs for routing"
            }
            Self::RgFilesCompact | Self::DirectoryListingCompact => {
                "orienting to repo layout or selecting likely directories"
            }
            Self::FileOutline { .. } | Self::FileExcerptDigest => {
                "choosing where in a file to read next"
            }
            Self::GitDiffStatCompact => "checking change size and touched files",
            Self::GitChangedFiles | Self::GitNameStatusCompact | Self::GitNumstatCompact => {
                "finding which files changed and rough change shape"
            }
            Self::DiffHunkSummary | Self::GitFilteredDiffDigest => {
                "triaging where a large diff changed"
            }
            Self::GitHistoryDigest => "finding likely recent commits or touched files",
            Self::RunCheckDigest => "surfacing likely build or test failures quickly",
            Self::ProcessTableCompact => "checking whether relevant processes exist",
        }
    }

    pub fn not_for(&self) -> &'static str {
        match self {
            Self::RgFileSetDigest => {
                "line evidence, surrounding code context, or exhaustive grep output"
            }
            Self::SearchText { .. } => "auditing every grep hit or preserving raw rg formatting",
            Self::RgCountDigest => "understanding the matched code or proving exact occurrences",
            Self::RgJsonDigest | Self::SelectStringDigest => {
                "exact command output, JSON consumers, or copyable diagnostics"
            }
            Self::RgFilesCompact | Self::DirectoryListingCompact => {
                "exhaustive file inventory or scripts that consume the raw list"
            }
            Self::FileOutline { .. } | Self::FileExcerptDigest => {
                "editing without reading exact source"
            }
            Self::GitDiffStatCompact => "reviewing changed code hunks",
            Self::GitChangedFiles | Self::GitNameStatusCompact | Self::GitNumstatCompact => {
                "reviewing exact diff content"
            }
            Self::DiffHunkSummary | Self::GitFilteredDiffDigest => {
                "code review or patch reconstruction"
            }
            Self::GitHistoryDigest => "audit trails that need exact git log output",
            Self::RunCheckDigest => {
                "full build logs, flaky timing analysis, or copied compiler output"
            }
            Self::ProcessTableCompact => "exact PID/table automation without raw output",
        }
    }

    pub fn unsafe_to_replace_reason(&self) -> &'static str {
        match self {
            Self::GitDiffStatCompact => "safe only for stat-shaped git diff commands",
            Self::GitChangedFiles => "path-only diff output does not satisfy hunk review intent",
            Self::RgFilesCompact | Self::DirectoryListingCompact => {
                "path lists are capped and sampled when large"
            }
            Self::DiffHunkSummary | Self::GitFilteredDiffDigest => {
                "hunk content is lossy without artifact-backed continuation"
            }
            Self::RunCheckDigest => "successful context and non-diagnostic output can be omitted",
            Self::FileExcerptDigest | Self::FileOutline { .. } => "body content can be omitted",
            Self::SelectStringDigest | Self::RgJsonDigest => {
                "structured or raw match details can be lost or summarized"
            }
            Self::RgCountDigest => "match text and line numbers are intentionally omitted",
            Self::RgFileSetDigest => {
                "full match lines are omitted unless the original command requested only files"
            }
            Self::GitNameStatusCompact | Self::GitNumstatCompact => {
                "status or numstat output may not satisfy hunk review intent"
            }
            Self::GitHistoryDigest => {
                "commit bodies, ordering details, and full stats can be omitted"
            }
            Self::ProcessTableCompact => "process rows and columns are summarized",
            Self::SearchText { .. } => "grouping and caps can omit files or matches",
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

    #[test]
    fn exposes_intent_gate_metadata_for_shadow_records() {
        assert_eq!(
            ReplacementCandidate::RgFileSetDigest.intent(),
            "file_discovery"
        );
        assert_eq!(
            ReplacementCandidate::RgFileSetDigest.replacement_policy(),
            "shadow_or_direct_file_set_only"
        );
        assert_eq!(
            ReplacementCandidate::GitDiffStatCompact.replacement_policy(),
            "direct_only_when_original_is_summary"
        );
        assert!(ReplacementCandidate::GitDiffStatCompact.direct_replacement_safe());
        assert!(!ReplacementCandidate::RgFileSetDigest.direct_replacement_safe());
        assert!(
            ReplacementCandidate::RgFileSetDigest
                .useful_when()
                .contains("which files")
        );
        assert!(
            ReplacementCandidate::RgCountDigest
                .not_for()
                .contains("matched code")
        );
        assert!(
            ReplacementCandidate::SearchText {
                pattern: "needle".to_string(),
                globs: Vec::new(),
                paths: Vec::new()
            }
            .unsafe_to_replace_reason()
            .contains("caps")
        );
    }
}
