pub const AUTO_LOOP_MULTI_OPTION_NOTE: &str =
    "Think on your own and choose what is best in long-term perspective";

pub const AGENT_ROI_RUBRIC: &str = "new_agent_cost=3, reuse_cost=1, parallel_gain=0-3, context_gain=0-3, repeat_gain=0-4, loop_followup_gain=0-3, risk_penalty=0-3, net = parallel_gain + context_gain + repeat_gain + loop_followup_gain - cost - risk_penalty";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoLoopSubmissionContext {
    Periodic,
    AfterSelfReview,
}

impl AutoLoopSubmissionContext {
    pub fn trace_name(self) -> &'static str {
        match self {
            Self::Periodic => "periodic",
            Self::AfterSelfReview => "after_self_review",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentRoiEstimate {
    pub parallel_gain: i32,
    pub context_gain: i32,
    pub repeat_gain: i32,
    pub loop_followup_gain: i32,
    pub cost: i32,
    pub risk_penalty: i32,
}

impl AgentRoiEstimate {
    pub const fn net(self) -> i32 {
        self.parallel_gain + self.context_gain + self.repeat_gain + self.loop_followup_gain
            - self.cost
            - self.risk_penalty
    }

    pub const fn is_positive(self) -> bool {
        self.net() >= 2
    }
}

pub fn auto_loop_should_plan_first(message: &str, context: AutoLoopSubmissionContext) -> bool {
    matches!(context, AutoLoopSubmissionContext::AfterSelfReview)
        || is_continuation_message(message)
}

pub fn auto_loop_plan_first_message(original: &str, context: AutoLoopSubmissionContext) -> String {
    let trigger = match context {
        AutoLoopSubmissionContext::Periodic => "periodic loop continuation",
        AutoLoopSubmissionContext::AfterSelfReview => "post-self-review loop continuation",
    };
    let original = original.trim();
    format!(
        "Automatic {trigger}: {original}\n\nLoop mode is on, so follow-ups are likely. Enter Plan mode before acting. In the plan include an Agent ROI Estimate with loop_followup_gain, call list_agents before spawning related follow-up work, prefer followup_task/send_message/resume_agent over a replacement agent, compact useful token-heavy agents before reuse, and decide what work to give any idle relevant agent. Keep useful agents for the active loop task family unless they are stale, wrong, or slots are needed. After plan self-review produces the revised or final plan, allow auto-loop to accept the implementation prompt automatically unless a blocker or user-choice prompt remains."
    )
}

pub fn auto_loop_request_user_input_answers(
    is_secret: bool,
    is_other: bool,
    has_options: bool,
    other_option_label: &str,
) -> Option<Vec<String>> {
    if is_secret {
        return None;
    }

    let mut answers = Vec::new();
    if has_options && is_other {
        answers.push(other_option_label.to_string());
    }
    answers.push(format!("user_note: {AUTO_LOOP_MULTI_OPTION_NOTE}"));
    Some(answers)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpawnPolicyRejection {
    RootOwnedFinalization,
    ExactReadOnlyExplorerWithoutPositiveRoi,
    ExplorationWithoutScoutOrRoi,
}

impl SpawnPolicyRejection {
    pub fn message(self) -> &'static str {
        match self {
            Self::RootOwnedFinalization => {
                "spawn_agent blocked: this task looks like git finalization, deploy promotion, or wrapper promotion. Keep commit/push/tag/rebase/merge/deploy/promotion actions in the root agent after reviewing agent output; subagents may inspect git state but should not own irreversible finalization."
            }
            Self::ExactReadOnlyExplorerWithoutPositiveRoi => {
                "spawn_agent blocked: this looks like simple read-only exploration over exact files or symbols without a positive Agent ROI Estimate. Read exact files locally or reuse an existing relevant agent; retry only if WHY_AGENT / ROI shows net >= 2, a reuse check, expected repeated operations or context savings, and a token/time budget."
            }
            Self::ExplorationWithoutScoutOrRoi => {
                "spawn_agent blocked: this looks like an exploration/scouting agent without enough first_moves/context-scout evidence or positive Agent ROI justification. Run `first_moves_predict` locally first (or load it with `tool_search`), inspect the high-confidence candidates, then keep the work local if that is enough. If a separate explorer is still useful, retry with `SCOUT_EVIDENCE` naming the completed scout, `WHY_AGENT / ROI` showing independent parallel value, reuse check, net >= 2, token/time budget or stop condition, and `FIRST_READS` starting from scout output or a strictly exact file list that avoids raw broad `rg`/`find` sweeps."
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpawnPolicyInput<'a> {
    pub role_name: Option<&'a str>,
    pub task_name: &'a str,
    pub message: &'a str,
    pub first_moves_enabled: bool,
    pub whole_repo_exploration_prompt: bool,
}

pub fn evaluate_spawn_policy(input: SpawnPolicyInput<'_>) -> Result<(), SpawnPolicyRejection> {
    let text = format!(
        "{}\n{}\n{}",
        input.role_name.unwrap_or_default(),
        input.task_name,
        input.message
    );
    let lower = text.to_ascii_lowercase();
    let marked_explorer = input
        .role_name
        .is_some_and(|role| role.to_ascii_lowercase().contains("explor"));
    let exploration_role = marked_explorer
        || [
            "explor", "scout", "survey", "mapper", "map_", "mapping", "reader", "triage",
        ]
        .iter()
        .any(|term| lower.contains(term));

    if looks_like_root_owned_finalization(&lower) {
        return Err(SpawnPolicyRejection::RootOwnedFinalization);
    }

    let exact_first_reads = first_reads_are_exact_local_reads(input.message);
    let has_positive_roi = has_positive_agent_roi_contract(input.message);
    if exploration_role && exact_first_reads && !has_positive_roi {
        return Err(SpawnPolicyRejection::ExactReadOnlyExplorerWithoutPositiveRoi);
    }

    if !input.first_moves_enabled {
        return Ok(());
    }

    let has_explicit_scout_evidence = has_explicit_scout_evidence_contract(input.message);
    let has_scout_routing =
        has_explicit_scout_evidence || first_reads_starts_with_scout(input.message);
    let shell_first_reads = first_reads_contains_raw_reader(input.message);
    let scout_required = !exact_first_reads || !has_positive_roi;
    let exploration_contract_required = marked_explorer
        || input.whole_repo_exploration_prompt
        || (exploration_role && shell_first_reads);

    if (marked_explorer && scout_required && !has_explicit_scout_evidence)
        || (exploration_contract_required
            && ((!has_scout_routing && scout_required) || !has_positive_roi))
    {
        return Err(SpawnPolicyRejection::ExplorationWithoutScoutOrRoi);
    }

    Ok(())
}

fn is_continuation_message(message: &str) -> bool {
    let normalized = normalize_continuation_message(message);
    matches!(
        normalized.as_str(),
        "go on"
            | "please go on"
            | "go on please"
            | "continue"
            | "please continue"
            | "continue please"
            | "carry on"
            | "please carry on"
            | "resume"
            | "please resume"
            | "keep going"
            | "please keep going"
            | "keep going please"
            | "do it"
            | "please do it"
            | "fix it"
            | "please fix it"
            | "finish it"
            | "please finish it"
            | "go ahead"
            | "please go ahead"
    )
}

fn normalize_continuation_message(message: &str) -> String {
    let mut normalized = String::with_capacity(message.len());
    for ch in message.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(' ');
        }
    }
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_root_owned_finalization(lower: &str) -> bool {
    lower.lines().any(|line| {
        let line = line.trim();
        !is_finalization_safety_line(line)
            && contains_any(
                line,
                &[
                    "git push",
                    "push branch",
                    "push changes",
                    "push to origin",
                    "git commit",
                    "commit the changes",
                    "commit all",
                    "make a commit",
                    "create a commit",
                    "git tag",
                    "create tag",
                    "git rebase",
                    "rebase branch",
                    "git merge",
                    "merge branch",
                    "deploy system-wide",
                    "deploy the build",
                    "deployment promotion",
                    "promote wrapper",
                    "promote the wrapper",
                    "wrapper promotion",
                ],
            )
    })
}

fn is_finalization_safety_line(line: &str) -> bool {
    contains_any(
        line,
        &[
            "do_not_inspect:",
            "do not ",
            "don't ",
            "must not ",
            "should not ",
            "avoid ",
            "without ",
            "no git ",
            "no commit",
            "no push",
            "not commit",
            "not push",
            "not own",
            "root owns",
            "root should",
            "root agent",
            "main agent",
        ],
    )
}

fn has_explicit_scout_evidence_contract(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "scout_evidence:",
        "first_moves_evidence:",
        "routing_evidence:",
        "context_scout_evidence:",
    ]
    .iter()
    .filter_map(|label| contract_section(&lower, label))
    .map(trim_contract_prefix)
    .any(|section| mentions_scout_tool(section) && !starts_with_raw_reader(section))
}

fn first_reads_starts_with_scout(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    let Some(first_reads) = contract_section(&lower, "first_reads:") else {
        return false;
    };
    let trimmed = trim_contract_prefix(first_reads);
    starts_with_scout_step(trimmed)
}

fn first_reads_contains_raw_reader(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    let Some(first_reads) = contract_section(&lower, "first_reads:") else {
        return false;
    };

    first_reads
        .split(['\n', ';'])
        .map(trim_contract_prefix)
        .any(starts_with_raw_reader)
}

fn mentions_scout_tool(section: &str) -> bool {
    [
        "first_moves",
        "first_moves_predict",
        "mcp__wizard_codex__first_moves_predict",
        "tool_search for first_moves",
        "first moves",
        "first-moves",
        "<first_moves",
        "context scout",
        "repo_context_scout",
        "repo navigation index",
        "code_knowledge_base",
        "smart_context",
        "prepare_context",
    ]
    .iter()
    .any(|term| section.contains(term))
}

fn starts_with_scout_step(section: &str) -> bool {
    [
        "first_moves_predict",
        "mcp__wizard_codex__first_moves_predict",
        "tool_search for first_moves",
        "tool_search",
        "context scout",
        "repo_context_scout",
        "repo navigation index",
        "code_knowledge_base",
        "smart_context",
        "prepare_context",
    ]
    .iter()
    .any(|term| section.starts_with(term) && mentions_scout_tool(section))
}

fn has_positive_agent_roi_contract(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "why_agent:",
        "parallel_value:",
        "agent roi estimate:",
        "agent_roi:",
        "why_agent / roi:",
        "why_agent/roi:",
    ]
    .iter()
    .any(|label| {
        contract_section(&lower, label)
            .map(trim_contract_prefix)
            .is_some_and(|section| {
                !section.is_empty()
                    && !matches!(section, "none" | "n/a" | "not needed" | "not applicable")
                    && contains_any(
                        section,
                        &[
                            "parallel",
                            "independent",
                            "sidecar",
                            "reuse",
                            "context",
                            "repeat",
                        ],
                    )
                    && contains_any(
                        section,
                        &[
                            "token",
                            "budget",
                            "time",
                            "latency",
                            "wall clock",
                            "wall-clock",
                            "stop condition",
                        ],
                    )
                    && has_positive_roi_claim(section)
            })
    })
}

fn has_positive_roi_claim(section: &str) -> bool {
    contains_any(
        section,
        &[
            "net >= 2",
            "net=>2",
            "net positive",
            "positive roi",
            "roi positive",
        ],
    ) || (contains_any(section, &["net:", "net ="])
        && contains_any(section, &["+2", "+3", "+4", "+5", " 2", " 3", " 4", " 5"]))
}

fn contract_section<'a>(lower: &'a str, label: &str) -> Option<&'a str> {
    let start = lower.find(label)? + label.len();
    let rest = &lower[start..];
    let end = [
        "\ncontext_area:",
        "\ndo_not_inspect:",
        "\nfirst_reads:",
        "\ntool_hints:",
        "\ntoken_tip:",
        "\nverification:",
        "\nhandoff:",
        "\nscout_evidence:",
        "\nfirst_moves_evidence:",
        "\nrouting_evidence:",
        "\ncontext_scout_evidence:",
        "\nwhy_agent:",
        "\nwhy_agent / roi:",
        "\nwhy_agent/roi:",
        "\nparallel_value:",
        "\nagent roi estimate:",
        "\nagent_roi:",
    ]
    .iter()
    .filter_map(|next_label| rest.find(next_label))
    .min()
    .unwrap_or(rest.len());
    Some(&rest[..end])
}

fn trim_contract_prefix(section: &str) -> &str {
    section.trim_start_matches(|c: char| {
        c.is_whitespace() || c == '-' || c == '*' || c == ':' || c == '`'
    })
}

fn starts_with_raw_reader(section: &str) -> bool {
    [
        "rg ",
        "grep ",
        "find ",
        "glob ",
        "get-content ",
        "sed ",
        "ls ",
        "dir ",
        "cat ",
    ]
    .iter()
    .any(|term| section.starts_with(term))
}

fn first_reads_are_exact_local_reads(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    let Some(first_reads) = contract_section(&lower, "first_reads:") else {
        return false;
    };
    let first_reads = trim_contract_prefix(first_reads);
    contains_any(
        first_reads,
        &[
            ".rs", ".md", ".toml", ".json", ".yaml", ".yml", ".txt", ".ps1", ".ts", ".tsx", ".js",
            ".jsx", ".py", ".go", ".java", ".kt", ".cs", ".cpp", ".h", ".hpp", ".c", ".snap", "/",
            "\\",
        ],
    ) && !contains_any(
        first_reads,
        &[
            "whole repo",
            "entire repo",
            "all files",
            "all directories",
            "first_moves",
            "repo_context_scout",
            "rg ",
            "grep ",
            "find ",
            "glob ",
        ],
    )
}

fn contains_any(section: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| section.contains(term))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn auto_loop_continuation_detection_keeps_custom_messages_out() {
        assert!(auto_loop_should_plan_first(
            "Go on, please.",
            AutoLoopSubmissionContext::Periodic
        ));
        assert!(!auto_loop_should_plan_first(
            "run the configured smoke",
            AutoLoopSubmissionContext::Periodic
        ));
        assert!(auto_loop_should_plan_first(
            "resume after review",
            AutoLoopSubmissionContext::AfterSelfReview
        ));
    }

    #[test]
    fn auto_loop_plan_first_prompt_mentions_reviewed_plan_auto_accept() {
        let prompt = auto_loop_plan_first_message("go on", AutoLoopSubmissionContext::Periodic);

        assert!(prompt.contains("loop_followup_gain"));
        assert!(prompt.contains("list_agents"));
        assert!(prompt.contains("plan self-review produces the revised or final plan"));
        assert!(prompt.contains("accept the implementation prompt automatically"));
    }

    #[test]
    fn auto_loop_answers_other_with_long_term_note() {
        assert_eq!(
            auto_loop_request_user_input_answers(
                /*is_secret*/ false,
                /*is_other*/ true,
                /*has_options*/ true,
                "None of the above",
            ),
            Some(vec![
                "None of the above".to_string(),
                format!("user_note: {AUTO_LOOP_MULTI_OPTION_NOTE}"),
            ])
        );
    }

    #[test]
    fn auto_loop_answers_concrete_options_with_note_only() {
        assert_eq!(
            auto_loop_request_user_input_answers(
                /*is_secret*/ false,
                /*is_other*/ false,
                /*has_options*/ true,
                "None of the above",
            ),
            Some(vec![format!("user_note: {AUTO_LOOP_MULTI_OPTION_NOTE}")])
        );
    }

    #[test]
    fn auto_loop_does_not_answer_secret_questions() {
        assert_eq!(
            auto_loop_request_user_input_answers(
                /*is_secret*/ true,
                /*is_other*/ true,
                /*has_options*/ true,
                "None of the above",
            ),
            None
        );
    }

    #[test]
    fn roi_net_includes_loop_followup_gain() {
        let estimate = AgentRoiEstimate {
            parallel_gain: 0,
            context_gain: 1,
            repeat_gain: 1,
            loop_followup_gain: 3,
            cost: 1,
            risk_penalty: 1,
        };

        assert_eq!(estimate.net(), 3);
        assert!(estimate.is_positive());
    }

    #[test]
    fn spawn_policy_blocks_git_finalization() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("worker"),
                task_name: "git_pusher",
                message: "CONTEXT_AREA: repo root\nVERIFICATION: git push origin branch",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Err(SpawnPolicyRejection::RootOwnedFinalization)
        );
    }

    #[test]
    fn spawn_policy_allows_finalization_safety_wording() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("worker"),
                task_name: "diff_reviewer",
                message: "CONTEXT_AREA: codex-rs\nDO_NOT_INSPECT: do not git push or commit\nWHY_AGENT / ROI: independent sidecar review with net >= 2 and a 10k token budget\nFIRST_READS: codex-rs/core/src/config/mod.rs\nHANDOFF: root owns commit/push after review",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Ok(())
        );
    }

    #[test]
    fn spawn_policy_does_not_count_raw_rg_scout_evidence() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("explorer"),
                task_name: "docs_mapper",
                message: "CONTEXT_AREA: docs\nSCOUT_EVIDENCE: rg -n \"first_moves|repo_context_scout\" docs\nWHY_AGENT / ROI: independent parallel docs scan with net >= 2 and a 20k token budget\nFIRST_READS: rg -n \"agent|scout\" docs",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Err(SpawnPolicyRejection::ExplorationWithoutScoutOrRoi)
        );
    }

    #[test]
    fn spawn_policy_requires_roi_for_exact_read_explorer() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("explorer"),
                task_name: "config_reader",
                message: "CONTEXT_AREA: codex-rs/core/src/config/mod.rs\nFIRST_READS: codex-rs/core/src/config/mod.rs",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Err(SpawnPolicyRejection::ExactReadOnlyExplorerWithoutPositiveRoi)
        );
    }

    #[test]
    fn spawn_policy_allows_budgeted_exact_read_explorer() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("explorer"),
                task_name: "config_reader",
                message: "CONTEXT_AREA: codex-rs/core/src/config/mod.rs\nWHY_AGENT / ROI: independent sidecar exact-read comparison with repeat_gain=4, loop_followup_gain=3, net >= 2, and a 12k token budget stop condition\nFIRST_READS: codex-rs/core/src/config/mod.rs",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Ok(())
        );
    }
}
