use codex_git_utils::merge_base_with_head;
use codex_protocol::protocol::ReviewRequest;
use codex_protocol::protocol::ReviewTarget;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_template::Template;
use std::sync::LazyLock;

/// Review thread system prompt.
pub const REVIEW_PROMPT: &str = include_str!("../templates/review/rubric.md");

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedReviewRequest {
    pub target: ReviewTarget,
    pub prompt: String,
    pub user_facing_hint: String,
}

const UNCOMMITTED_PROMPT: &str = "Review the current code changes (staged, unstaged, and untracked files) and provide prioritized findings.";

const BASE_BRANCH_PROMPT_BACKUP: &str = "Review the code changes against the base branch '{{branch}}'. Start by finding the merge diff between the current branch and {{branch}}'s upstream e.g. (`git merge-base HEAD \"$(git rev-parse --abbrev-ref \"{{branch}}@{upstream}\")\"`), then run `git diff` against that SHA to see what changes we would merge into the {{branch}} branch. Provide prioritized, actionable findings.";
const BASE_BRANCH_PROMPT: &str = "Review the code changes against the base branch '{{base_branch}}'. The merge base commit for this comparison is {{merge_base_sha}}. Run `git diff {{merge_base_sha}}` to inspect the changes relative to {{base_branch}}. Provide prioritized, actionable findings.";
static BASE_BRANCH_PROMPT_BACKUP_TEMPLATE: LazyLock<Template> = LazyLock::new(|| {
    Template::parse(BASE_BRANCH_PROMPT_BACKUP)
        .unwrap_or_else(|err| panic!("base branch backup review prompt must parse: {err}"))
});
static BASE_BRANCH_PROMPT_TEMPLATE: LazyLock<Template> = LazyLock::new(|| {
    Template::parse(BASE_BRANCH_PROMPT)
        .unwrap_or_else(|err| panic!("base branch review prompt must parse: {err}"))
});

const COMMIT_PROMPT_WITH_TITLE: &str = "Review the code changes introduced by commit {{sha}} (\"{{title}}\"). Provide prioritized, actionable findings.";
const COMMIT_PROMPT: &str = "Review the code changes introduced by commit {{sha}}. Provide prioritized, actionable findings.";
static COMMIT_PROMPT_WITH_TITLE_TEMPLATE: LazyLock<Template> = LazyLock::new(|| {
    Template::parse(COMMIT_PROMPT_WITH_TITLE)
        .unwrap_or_else(|err| panic!("commit review prompt with title must parse: {err}"))
});
static COMMIT_PROMPT_TEMPLATE: LazyLock<Template> = LazyLock::new(|| {
    Template::parse(COMMIT_PROMPT)
        .unwrap_or_else(|err| panic!("commit review prompt must parse: {err}"))
});

pub fn resolve_review_request(
    request: ReviewRequest,
    cwd: &AbsolutePathBuf,
) -> anyhow::Result<ResolvedReviewRequest> {
    let target = request.target;
    let prompt = review_prompt(&target, cwd)?;
    let user_facing_hint = request
        .user_facing_hint
        .unwrap_or_else(|| user_facing_hint(&target));

    Ok(ResolvedReviewRequest {
        target,
        prompt,
        user_facing_hint,
    })
}

pub fn review_prompt(target: &ReviewTarget, cwd: &AbsolutePathBuf) -> anyhow::Result<String> {
    match target {
        ReviewTarget::UncommittedChanges => Ok(UNCOMMITTED_PROMPT.to_string()),
        ReviewTarget::BaseBranch { branch } => {
            if let Some(commit) = merge_base_with_head(cwd, branch)? {
                Ok(render_review_prompt(
                    &BASE_BRANCH_PROMPT_TEMPLATE,
                    [
                        ("base_branch", branch.as_str()),
                        ("merge_base_sha", commit.as_str()),
                    ],
                ))
            } else {
                Ok(render_review_prompt(
                    &BASE_BRANCH_PROMPT_BACKUP_TEMPLATE,
                    [("branch", branch.as_str())],
                ))
            }
        }
        ReviewTarget::Commit { sha, title } => {
            if let Some(title) = title {
                Ok(render_review_prompt(
                    &COMMIT_PROMPT_WITH_TITLE_TEMPLATE,
                    [("sha", sha.as_str()), ("title", title.as_str())],
                ))
            } else {
                Ok(render_review_prompt(
                    &COMMIT_PROMPT_TEMPLATE,
                    [("sha", sha.as_str())],
                ))
            }
        }
        ReviewTarget::Custom { instructions } => {
            let prompt = instructions.trim();
            if prompt.is_empty() {
                anyhow::bail!("Review prompt cannot be empty");
            }
            Ok(prompt.to_string())
        }
    }
}

fn render_review_prompt<'a, const N: usize>(
    template: &Template,
    variables: [(&'a str, &'a str); N],
) -> String {
    template
        .render(variables)
        .unwrap_or_else(|err| panic!("review prompt template must render: {err}"))
}

pub fn user_facing_hint(target: &ReviewTarget) -> String {
    match target {
        ReviewTarget::UncommittedChanges => "current changes".to_string(),
        ReviewTarget::BaseBranch { branch } => format!("changes against '{branch}'"),
        ReviewTarget::Commit { sha, title } => {
            let short_sha: String = sha.chars().take(7).collect();
            if let Some(title) = title {
                format!("commit {short_sha}: {title}")
            } else {
                format!("commit {short_sha}")
            }
        }
        ReviewTarget::Custom { instructions } => custom_review_hint(instructions),
    }
}

fn custom_review_hint(instructions: &str) -> String {
    const MAX_HINT_CHARS: usize = 80;
    let hint = instructions
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("custom review");
    let collapsed = hint.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let truncated = chars.by_ref().take(MAX_HINT_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else if truncated.is_empty() {
        "custom review".to_string()
    } else {
        truncated
    }
}

impl From<ResolvedReviewRequest> for ReviewRequest {
    fn from(resolved: ResolvedReviewRequest) -> Self {
        ReviewRequest {
            target: resolved.target,
            user_facing_hint: Some(resolved.user_facing_hint),
        }
    }
}

#[cfg(test)]
#[path = "review_request_tests.rs"]
mod review_request_tests;

// fork-local: self-review hint bounding test. Upstream extracted review tests into
// review_request_tests.rs (not owned by this fork slice), so this fork-specific case is
// kept inline here to preserve coverage of the self-review user-facing hint feature.
#[cfg(test)]
mod fork_review_request_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn custom_review_hint_is_bounded_to_first_line() {
        assert_eq!(
            user_facing_hint(&ReviewTarget::Custom {
                instructions:
                    "Automatic self-review of the just-completed work slice.\n\nLong details"
                        .to_string(),
            }),
            "Automatic self-review of the just-completed work slice."
        );
        assert_eq!(
            user_facing_hint(&ReviewTarget::Custom {
                instructions: "x".repeat(100),
            }),
            format!("{}...", "x".repeat(80))
        );
    }
}
