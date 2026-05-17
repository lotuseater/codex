mod plan_prompt;

pub use plan_prompt::DEFAULT_MULTI_AGENT_V2_ROOT_USAGE_HINT_TEXT;
pub use plan_prompt::DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT;
pub use plan_prompt::MAIN_AGENT_PLAN_DELEGATION_PROMPT;

pub const AUTO_LOOP_MULTI_OPTION_NOTE: &str =
    "Think on your own and choose what is best in long-term perspective";

pub const AGENT_ROI_RUBRIC: &str = "new_agent_cost=3, reuse_cost=1, parallel_gain=0-3, context_gain=0-3, repeat_gain=0-4, loop_followup_gain=0-3, risk_penalty=0-3, net = parallel_gain + context_gain + repeat_gain + loop_followup_gain - cost - risk_penalty";

pub const MULTI_AGENT_V2_NESTED_SPAWN_REJECTION: &str = "Only the root agent can spawn MultiAgentV2 helpers; send a concise handoff to the root instead.";

pub fn multi_agent_v2_root_can_spawn_child(parent_is_root: bool, child_depth: i32) -> bool {
    parent_is_root && child_depth == 1
}

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
        "Automatic {trigger}: {original}\n\nLoop mode is on, so follow-ups are likely. Enter Plan mode before acting. {MAIN_AGENT_PLAN_DELEGATION_PROMPT} Include an Agent ROI Estimate with loop_followup_gain, call list_agents before spawning related follow-up work, prefer followup_task/send_message/resume_agent over a replacement agent, compact useful token-heavy agents before reuse, and decide what work to give any idle relevant agent. Keep useful agents for the active loop task family unless they are stale, wrong, or slots are needed. After plan self-review produces the revised or final plan, allow auto-loop to accept the implementation prompt automatically unless a blocker or user-choice prompt remains."
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
                "spawn_agent blocked: this looks like simple bounded read-only exploration without a positive Agent ROI Estimate. Read exact files locally or reuse an existing relevant agent; retry with agent_type=\"helper\" or an existing agent only if WHY_AGENT / ROI shows net >= 2, a reuse check, expected repeated operations or context savings, and a token/time budget."
            }
            Self::ExplorationWithoutScoutOrRoi => {
                "spawn_agent blocked: this looks like exploration/scouting without enough first_moves/context-scout evidence or positive Agent ROI justification. Run `first_moves_predict` locally first (or load it with `tool_search`), inspect the high-confidence candidates, then keep the work local if that is enough. If a separate helper/explorer is still useful, retry with `SCOUT_EVIDENCE` naming the completed scout, `WHY_AGENT / ROI` showing independent parallel value, reuse check, net >= 2, token/time budget or stop condition, and `FIRST_READS` starting from scout output or a strictly bounded exact file/diff/test list that avoids raw broad `rg`/`find` sweeps."
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
    let marked_helper = input
        .role_name
        .is_some_and(|role| role.eq_ignore_ascii_case("helper"));
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
    let bounded_first_reads = exact_first_reads || first_reads_are_bounded_read_only(input.message);
    let has_positive_roi = has_positive_agent_roi_contract(input.message);
    if (exploration_role || marked_helper) && bounded_first_reads && !has_positive_roi {
        return Err(SpawnPolicyRejection::ExactReadOnlyExplorerWithoutPositiveRoi);
    }

    if !input.first_moves_enabled {
        return Ok(());
    }

    let has_explicit_scout_evidence = has_explicit_scout_evidence_contract(input.message);
    let has_scout_routing =
        has_explicit_scout_evidence || first_reads_starts_with_scout(input.message);
    let shell_first_reads = first_reads_contains_raw_reader(input.message);
    let broad_context_area = has_broad_context_area(input.message);
    let scout_required = !bounded_first_reads || !has_positive_roi;
    let exploration_contract_required = marked_explorer
        || input.whole_repo_exploration_prompt
        || broad_context_area
        || (exploration_role && shell_first_reads);
    let helper_contract_required = marked_helper && (broad_context_area || shell_first_reads);

    if (marked_explorer && scout_required && !has_explicit_scout_evidence)
        || (exploration_contract_required
            && ((!has_scout_routing && scout_required) || !has_positive_roi))
        || (helper_contract_required
            && ((!has_scout_routing && scout_required) || !has_positive_roi))
    {
        return Err(SpawnPolicyRejection::ExplorationWithoutScoutOrRoi);
    }

    Ok(())
}

pub fn is_continuation_message(message: &str) -> bool {
    let normalized = normalize_continuation_message(message);
    if is_normalized_continuation_message(&normalized) {
        return true;
    }

    let Some(automatic_prompt) = normalized.strip_prefix("automatic ") else {
        return false;
    };
    let Some((trigger_and_original, _)) = automatic_prompt.split_once(" loop mode is on") else {
        return false;
    };
    for trigger in [
        "periodic loop continuation",
        "post self review loop continuation",
    ] {
        if let Some(original) = trigger_and_original.strip_prefix(trigger) {
            return is_normalized_continuation_message(original.trim());
        }
    }

    false
}

fn is_normalized_continuation_message(normalized: &str) -> bool {
    matches!(
        normalized,
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

#[cfg(test)]
mod continuation_message_tests {
    use super::*;

    #[test]
    fn recognizes_short_continuation_messages() {
        for message in [
            "go on",
            "please continue",
            "resume",
            "finish it",
            "go ahead",
        ] {
            assert!(is_continuation_message(message));
        }
    }

    #[test]
    fn recognizes_auto_loop_plan_first_wrapper() {
        let prompt = auto_loop_plan_first_message("go on", AutoLoopSubmissionContext::Periodic);

        assert!(is_continuation_message(&prompt));
    }
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
        "agent_graph_scout",
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
        "agent_graph_scout",
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
    let compact = section
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    contains_any(
        section,
        &[
            "net >= 2",
            "net=>2",
            "net positive",
            "positive roi",
            "roi positive",
        ],
    ) || contains_any(
        compact.as_str(),
        &[
            "net>=2", "net=>2", "net=2", "net=+2", "net:+2", "net=3", "net=+3", "net:+3", "net=4",
            "net=+4", "net:+4", "net=5", "net=+5", "net:+5", "net=6", "net=+6", "net:+6", "net=7",
            "net=+7", "net:+7", "net=8", "net=+8", "net:+8", "net=9", "net=+9", "net:+9",
        ],
    ) || (contains_any(section, &["net:", "net ="])
        && contains_any(
            section,
            &[
                "+2", "+3", "+4", "+5", "+6", "+7", "+8", "+9", " 2", " 3", " 4", " 5", " 6", " 7",
                " 8", " 9",
            ],
        ))
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

fn first_reads_are_bounded_read_only(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    let Some(first_reads) = contract_section(&lower, "first_reads:") else {
        return false;
    };
    let first_reads = trim_contract_prefix(first_reads);
    let bounded_command = contains_bounded_read_only_command(first_reads)
        || first_reads
            .split(['\n', ';'])
            .map(trim_contract_prefix)
            .any(contains_bounded_read_only_command);
    bounded_command
        && !contains_any(
            first_reads,
            &[
                "whole repo",
                "entire repo",
                "all files",
                "all directories",
                "rg ",
                "grep ",
                "find ",
                "glob ",
            ],
        )
}

fn contains_bounded_read_only_command(section: &str) -> bool {
    contains_any(
        section,
        &[
            "git diff -- ",
            "git diff --stat -- ",
            "git status --short",
            "git status --porcelain",
            "cargo test -p ",
            "cargo test --release -p ",
        ],
    )
}

fn has_broad_context_area(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    contract_section(&lower, "context_area:")
        .map(trim_contract_prefix)
        .is_some_and(|section| {
            contains_any(
                section,
                &[
                    "whole repo",
                    "entire repo",
                    "all files",
                    "all directories",
                    "whole tree",
                    "entire tree",
                    "codebase",
                    "workspace",
                ],
            )
        })
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
        assert!(prompt.contains("what to delegate to subagents"));
        assert!(prompt.contains("up to three persistent high-capability helper agents"));
        assert!(prompt.contains("short summary or short result only when the main agent needs"));
        assert!(prompt.contains("plan self-review produces the revised or final plan"));
        assert!(prompt.contains("accept the implementation prompt automatically"));
    }

    #[test]
    fn default_multi_agent_v2_hints_include_delegation_policy() {
        assert!(
            DEFAULT_MULTI_AGENT_V2_ROOT_USAGE_HINT_TEXT.contains(MAIN_AGENT_PLAN_DELEGATION_PROMPT)
        );
        assert!(
            DEFAULT_MULTI_AGENT_V2_ROOT_USAGE_HINT_TEXT
                .contains("Only the main/root agent spawns helpers")
        );
        assert!(
            DEFAULT_MULTI_AGENT_V2_ROOT_USAGE_HINT_TEXT
                .contains("Compact helpers after bulky reads")
        );
        assert!(
            DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT
                .contains("A short summary or short result is optional")
        );
        assert!(
            DEFAULT_MULTI_AGENT_V2_SUBAGENT_USAGE_HINT_TEXT.contains("Do not spawn more agents")
        );
    }

    #[test]
    fn multi_agent_v2_root_only_spawn_policy_allows_only_root_children() {
        assert!(!multi_agent_v2_root_can_spawn_child(
            /*parent_is_root*/ true, 0
        ));
        assert!(multi_agent_v2_root_can_spawn_child(
            /*parent_is_root*/ true, 1
        ));
        assert!(!multi_agent_v2_root_can_spawn_child(
            /*parent_is_root*/ true, 2
        ));
        assert!(!multi_agent_v2_root_can_spawn_child(
            /*parent_is_root*/ false, 1
        ));
        assert!(!multi_agent_v2_root_can_spawn_child(
            /*parent_is_root*/ false,
            i32::MAX
        ));
        assert!(MULTI_AGENT_V2_NESTED_SPAWN_REJECTION.contains("Only the root agent"));
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

    #[test]
    fn spawn_policy_allows_budgeted_helper_for_bounded_diff_review() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("helper"),
                task_name: "helper_review",
                message: "CONTEXT_AREA: current diff only\nWHY_AGENT / ROI: reuse check found no relevant existing helper; independent helper diff review with net = 2 and an 8k token budget while root keeps implementation local\nFIRST_READS: git diff -- codex-rs/agent-policy/src/lib.rs\nTOOL_HINTS: path-scoped git diff only\nTOKEN_TIP: do not broaden beyond this diff\nVERIFICATION: report findings only",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Ok(())
        );
    }

    #[test]
    fn spawn_policy_allows_budgeted_helper_for_short_git_status() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("helper"),
                task_name: "helper_status",
                message: "CONTEXT_AREA: repo root status only\nWHY_AGENT / ROI: reuse check found no relevant existing helper; independent helper status check with net: +3 and a 2k token budget while root keeps implementation local\nFIRST_READS: git status --short\nTOOL_HINTS: read-only status only\nTOKEN_TIP: stop after status\nVERIFICATION: report dirty files only",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Ok(())
        );
    }

    #[test]
    fn spawn_policy_blocks_helper_broad_raw_scan_without_scout() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("helper"),
                task_name: "helper_repo_scan",
                message: "CONTEXT_AREA: whole repo\nWHY_AGENT / ROI: reuse check found no relevant existing helper; independent helper scan with net >= 2 and a 15k token budget while root keeps implementation local\nFIRST_READS: rg -n \"TODO|FIXME\" .\nTOOL_HINTS: use rg\nTOKEN_TIP: summarize only\nVERIFICATION: report findings",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Err(SpawnPolicyRejection::ExplorationWithoutScoutOrRoi)
        );
    }

    #[test]
    fn spawn_policy_allows_budgeted_helper_after_first_moves_scout() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("helper"),
                task_name: "helper_repo_check",
                message: "CONTEXT_AREA: whole repo\nSCOUT_EVIDENCE: first_moves_predict returned codex-rs/agent-policy/src/lib.rs with high confidence\nWHY_AGENT / ROI: reuse check found no relevant existing helper; independent helper verification with loop_followup_gain=3, net=+3, and a 12k token budget while root keeps implementation local\nFIRST_READS: first_moves_predict output only\nTOOL_HINTS: use optimized context tools before shell search\nTOKEN_TIP: stop if the scout is enough\nVERIFICATION: report policy files only",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Ok(())
        );
    }

    #[test]
    fn spawn_policy_allows_budgeted_helper_after_agent_graph_scout() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("helper"),
                task_name: "helper_agent_reuse",
                message: "CONTEXT_AREA: agent reuse only\nSCOUT_EVIDENCE: agent_graph_scout reported one idle reusable helper with matching context\nWHY_AGENT / ROI: reuse check found a relevant helper; independent follow-up with reuse_cost=1, loop_followup_gain=3, net=+4, and a 6k token budget\nFIRST_READS: agent_graph_scout output only\nTOOL_HINTS: do not inspect repository files\nTOKEN_TIP: stop after reuse recommendation\nVERIFICATION: report reuse action only",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Ok(())
        );
    }

    #[test]
    fn spawn_policy_requires_roi_for_bounded_helper() {
        assert_eq!(
            evaluate_spawn_policy(SpawnPolicyInput {
                role_name: Some("helper"),
                task_name: "helper_review",
                message: "CONTEXT_AREA: current diff only\nFIRST_READS: git diff -- codex-rs/agent-policy/src/lib.rs\nTOOL_HINTS: path-scoped git diff only\nTOKEN_TIP: do not broaden beyond this diff\nVERIFICATION: report findings only",
                first_moves_enabled: true,
                whole_repo_exploration_prompt: false,
            }),
            Err(SpawnPolicyRejection::ExactReadOnlyExplorerWithoutPositiveRoi)
        );
    }
}
