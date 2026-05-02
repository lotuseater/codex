use codex_execpolicy::Decision;
use codex_reasoning_logic::DeterministicReasoner;
use codex_reasoning_logic::ExecPolicyCase;
use codex_reasoning_logic::HostExecutableFact;
use codex_reasoning_logic::PathAliasFact;
use codex_reasoning_logic::PrefixRuleFact;
use codex_reasoning_logic::ProbabilisticReasoner;
use codex_reasoning_logic::ProblogReasoner;
use codex_reasoning_logic::ReasonerAvailability;
use codex_reasoning_logic::RustBaselineReasoner;
use codex_reasoning_logic::SuggestedToolType;
use codex_reasoning_logic::SwiplReasoner;
use codex_reasoning_logic::ToolSuggestionCase;
use pretty_assertions::assert_eq;

fn tokens(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn rule(id: &str, pattern: &[&str], decision: Decision) -> PrefixRuleFact {
    PrefixRuleFact::new(id, tokens(pattern), decision)
}

fn strictest(left: Decision, right: Decision) -> Decision {
    left.max(right)
}

fn host_path(name: &str) -> String {
    if cfg!(windows) {
        format!(r"C:\Tools\{name}.exe")
    } else {
        format!("/usr/local/bin/{name}")
    }
}

fn deterministic_cases() -> Vec<ExecPolicyCase> {
    let decisions = [Decision::Allow, Decision::Prompt, Decision::Forbidden];
    let mut cases = Vec::new();

    for rule_decision in decisions {
        for fallback in decisions {
            for suffix in ["plain", "short", "long"] {
                let command = match suffix {
                    "plain" => tokens(&["tool", "inspect"]),
                    "short" => tokens(&["tool", "inspect", "--short"]),
                    "long" => tokens(&["tool", "inspect", "--format", "json"]),
                    _ => unreachable!(),
                };
                cases.push(ExecPolicyCase::new(
                    format!("single_prefix_{rule_decision:?}_{fallback:?}_{suffix}"),
                    vec![rule("tool_inspect", &["tool", "inspect"], rule_decision)],
                    vec![command],
                    fallback,
                    rule_decision,
                ));
            }
        }
    }

    for broad in decisions {
        for specific in decisions {
            cases.push(ExecPolicyCase::new(
                format!("strictest_overlapping_{broad:?}_{specific:?}"),
                vec![
                    rule("git_broad", &["git"], broad),
                    rule("git_commit", &["git", "commit"], specific),
                ],
                vec![tokens(&["git", "commit", "-m", "msg"])],
                Decision::Allow,
                strictest(broad, specific),
            ));
        }
    }

    for fallback in decisions {
        for command in [
            tokens(&["custom", "lint"]),
            tokens(&["python", "-m", "pytest"]),
            tokens(&["cargo", "metadata", "--format-version", "1"]),
        ] {
            cases.push(ExecPolicyCase::new(
                format!("fallback_{fallback:?}_{:?}", command.join("_")),
                Vec::new(),
                vec![command],
                fallback,
                fallback,
            ));
        }
    }

    for first in decisions {
        for second in [Decision::Allow, Decision::Prompt] {
            cases.push(ExecPolicyCase::new(
                format!("multi_command_{first:?}_{second:?}_with_policy"),
                vec![
                    rule("cargo", &["cargo"], first),
                    rule("git_status", &["git", "status"], second),
                ],
                vec![
                    tokens(&["cargo", "test", "-p", "codex-core"]),
                    tokens(&["git", "status"]),
                ],
                Decision::Allow,
                strictest(first, second),
            ));
        }
    }
    for fallback in decisions {
        for explicit in [Decision::Allow, Decision::Prompt] {
            cases.push(ExecPolicyCase::new(
                format!("multi_command_policy_and_fallback_{explicit:?}_{fallback:?}"),
                vec![rule("git_status", &["git", "status"], explicit)],
                vec![tokens(&["git", "status"]), tokens(&["unknown", "tool"])],
                fallback,
                strictest(explicit, fallback),
            ));
        }
    }

    let git_path = host_path("git");
    let rg_path = host_path("rg");
    for decision in decisions {
        cases.push(
            ExecPolicyCase::new(
                format!("host_resolution_without_allowlist_{decision:?}"),
                vec![rule("git_any", &["git"], decision)],
                vec![vec![git_path.clone(), "status".to_string()]],
                Decision::Allow,
                decision,
            )
            .with_path_aliases(vec![PathAliasFact::new(git_path.clone(), "git")]),
        );
        cases.push(
            ExecPolicyCase::new(
                format!("host_resolution_with_allowlist_{decision:?}"),
                vec![rule("git_any", &["git"], decision)],
                vec![vec![git_path.clone(), "status".to_string()]],
                Decision::Allow,
                decision,
            )
            .with_path_aliases(vec![PathAliasFact::new(git_path.clone(), "git")])
            .with_host_executables(vec![HostExecutableFact::new("git", vec![git_path.clone()])]),
        );
        cases.push(
            ExecPolicyCase::new(
                format!("host_resolution_allowlist_miss_{decision:?}"),
                vec![rule("git_any", &["git"], decision)],
                vec![vec![git_path.clone(), "status".to_string()]],
                Decision::Prompt,
                Decision::Prompt,
            )
            .with_path_aliases(vec![PathAliasFact::new(git_path.clone(), "git")])
            .with_host_executables(vec![HostExecutableFact::new("git", vec![rg_path.clone()])]),
        );
        cases.push(
            ExecPolicyCase::new(
                format!("host_resolution_empty_allowlist_{decision:?}"),
                vec![rule("git_any", &["git"], decision)],
                vec![vec![git_path.clone(), "status".to_string()]],
                Decision::Forbidden,
                Decision::Forbidden,
            )
            .with_path_aliases(vec![PathAliasFact::new(git_path.clone(), "git")])
            .with_host_executables(vec![HostExecutableFact::new("git", Vec::new())]),
        );
        cases.push(
            ExecPolicyCase::new(
                format!("host_resolution_exact_match_wins_{decision:?}"),
                vec![
                    PrefixRuleFact::new("exact_path", vec![git_path.clone()], Decision::Allow),
                    rule("git_any", &["git"], decision),
                ],
                vec![vec![git_path.clone(), "status".to_string()]],
                Decision::Forbidden,
                Decision::Allow,
            )
            .with_path_aliases(vec![PathAliasFact::new(git_path.clone(), "git")])
            .with_host_executables(vec![HostExecutableFact::new("git", vec![git_path.clone()])]),
        );
        cases.push(
            ExecPolicyCase::new(
                format!("host_resolution_specific_prefix_{decision:?}"),
                vec![rule("git_status", &["git", "status"], decision)],
                vec![vec![
                    git_path.clone(),
                    "status".to_string(),
                    "-sb".to_string(),
                ]],
                Decision::Allow,
                decision,
            )
            .with_path_aliases(vec![PathAliasFact::new(git_path.clone(), "git")]),
        );
    }

    cases.push(
        ExecPolicyCase::new(
            "exact_rule_for_absolute_path_blocks_host_alias",
            vec![
                PrefixRuleFact::new("exact_path", vec![git_path.clone()], Decision::Forbidden),
                rule("git_status", &["git", "status"], Decision::Allow),
            ],
            vec![vec![git_path.clone(), "status".to_string()]],
            Decision::Allow,
            Decision::Forbidden,
        )
        .with_path_aliases(vec![PathAliasFact::new(git_path.clone(), "git")]),
    );
    cases.push(
        ExecPolicyCase::new(
            "host_alias_applies_when_exact_rule_does_not_match",
            vec![
                PrefixRuleFact::new(
                    "exact_other",
                    vec![git_path.clone(), "commit".to_string()],
                    Decision::Forbidden,
                ),
                rule("git_status", &["git", "status"], Decision::Allow),
            ],
            vec![vec![git_path.clone(), "status".to_string()]],
            Decision::Prompt,
            Decision::Allow,
        )
        .with_path_aliases(vec![PathAliasFact::new(git_path, "git")]),
    );
    cases.push(ExecPolicyCase::new(
        "forbidden_second_command_dominates_allowed_first_command",
        vec![
            rule("git_status", &["git", "status"], Decision::Allow),
            rule("rm", &["rm"], Decision::Forbidden),
        ],
        vec![tokens(&["git", "status"]), tokens(&["rm", "-rf", "target"])],
        Decision::Allow,
        Decision::Forbidden,
    ));

    for fallback in decisions {
        cases.push(ExecPolicyCase::new(
            format!("absolute_path_without_alias_uses_fallback_{fallback:?}"),
            vec![rule("python_rule", &["python"], Decision::Allow)],
            vec![vec![host_path("node"), "--version".to_string()]],
            fallback,
            fallback,
        ));
        cases.push(ExecPolicyCase::new(
            format!("near_miss_prefix_uses_fallback_{fallback:?}"),
            vec![rule("git_status", &["git", "status"], Decision::Allow)],
            vec![tokens(&["git", "diff", "--stat"])],
            fallback,
            fallback,
        ));
    }

    cases
}

fn probabilistic_cases() -> Vec<ToolSuggestionCase> {
    let tool_profiles = [
        (
            "gmail_connector",
            SuggestedToolType::Connector,
            0.92,
            0.88,
            0.76,
            0.81,
        ),
        (
            "github_connector",
            SuggestedToolType::Connector,
            0.84,
            0.91,
            0.69,
            0.86,
        ),
        (
            "drive_connector",
            SuggestedToolType::Connector,
            0.73,
            0.79,
            0.63,
            0.74,
        ),
        (
            "review_plugin",
            SuggestedToolType::Plugin,
            0.81,
            0.66,
            0.58,
            0.68,
        ),
    ];
    let states = [
        ("normal", true, true, false, false),
        ("unsupported_client", true, false, false, false),
        ("already_accessible", true, true, false, true),
    ];
    let probability_scales = [
        ("high", 1.0),
        ("medium", 0.75),
        ("low", 0.40),
        ("none", 0.0),
    ];

    let mut cases = Vec::new();
    for (tool_name, tool_type, relevance, available, acceptance, success) in tool_profiles {
        for (state_name, search_failed, client_supported, disabled_by_user, already_accessible) in
            states
        {
            for (scale_name, scale) in probability_scales {
                cases.push(ToolSuggestionCase {
                    name: format!("{tool_name}_{state_name}_{scale_name}"),
                    tool_type,
                    task_relevance_probability: relevance * scale,
                    tool_available_probability: available,
                    user_acceptance_probability: acceptance,
                    install_success_probability: success,
                    search_failed,
                    client_supported,
                    disabled_by_user,
                    already_accessible,
                });
            }
        }
    }

    cases
}

#[test]
fn rust_baseline_matches_expected_deterministic_cases() {
    let reasoner = RustBaselineReasoner;
    let cases = deterministic_cases();
    assert!(cases.len() >= 84, "expected at least 84 cases");

    for case in &cases {
        let outcome = reasoner
            .evaluate_exec_policy(case)
            .unwrap_or_else(|err| panic!("{} failed: {err}", case.name));
        assert_eq!(
            case.expected_decision, outcome.decision,
            "case {}",
            case.name
        );
    }
}

#[test]
fn swipl_matches_rust_baseline_for_deterministic_cases() {
    let swipl = SwiplReasoner::default();
    if let ReasonerAvailability::Unavailable(reason) = swipl.availability() {
        if std::env::var_os("CODEX_REQUIRE_LOGIC_ENGINES").is_some() {
            panic!("swipl unavailable: {reason}");
        }
        eprintln!("skipping SWI-Prolog comparison: {reason}");
        return;
    }

    let baseline = RustBaselineReasoner;
    let cases = deterministic_cases();
    assert!(cases.len() >= 84, "expected at least 84 cases");
    for case in &cases {
        let expected = baseline
            .evaluate_exec_policy(case)
            .unwrap_or_else(|err| panic!("baseline {} failed: {err}", case.name));
        let actual = swipl
            .evaluate_exec_policy(case)
            .unwrap_or_else(|err| panic!("swipl {} failed: {err}", case.name));
        assert_eq!(expected, actual, "case {}", case.name);
    }
}

#[test]
fn rust_baseline_matches_expected_probabilistic_cases() {
    let reasoner = RustBaselineReasoner;
    let cases = probabilistic_cases();
    assert!(cases.len() >= 48, "expected at least 48 cases");

    for case in &cases {
        let probabilities = reasoner
            .evaluate_tool_suggestion(case)
            .unwrap_or_else(|err| panic!("{} failed: {err}", case.name));
        if case.completion_possible() {
            assert!(
                probabilities.suggest_tool >= 0.0,
                "case {} should have a valid suggestion probability",
                case.name
            );
        } else {
            assert_eq!(0.0, probabilities.suggest_tool, "case {}", case.name);
            assert_eq!(0.0, probabilities.completed_tool, "case {}", case.name);
            assert_eq!(
                0.0, probabilities.missed_without_suggestion,
                "case {}",
                case.name
            );
        }
    }
}

#[test]
fn problog_matches_rust_baseline_for_probabilistic_cases() {
    let problog = ProblogReasoner::default();
    if let ReasonerAvailability::Unavailable(reason) = problog.availability() {
        if std::env::var_os("CODEX_REQUIRE_LOGIC_ENGINES").is_some() {
            panic!("problog unavailable: {reason}");
        }
        eprintln!("skipping ProbLog comparison: {reason}");
        return;
    }

    let baseline = RustBaselineReasoner;
    let cases = probabilistic_cases();
    assert!(cases.len() >= 48, "expected at least 48 cases");
    for case in &cases {
        let expected = baseline
            .evaluate_tool_suggestion(case)
            .unwrap_or_else(|err| panic!("baseline {} failed: {err}", case.name));
        let actual = problog
            .evaluate_tool_suggestion(case)
            .unwrap_or_else(|err| panic!("problog {} failed: {err}", case.name));
        assert_probability_close(expected.suggest_tool, actual.suggest_tool, &case.name);
        assert_probability_close(expected.completed_tool, actual.completed_tool, &case.name);
        assert_probability_close(
            expected.missed_without_suggestion,
            actual.missed_without_suggestion,
            &case.name,
        );
    }
}

fn assert_probability_close(expected: f64, actual: f64, case_name: &str) {
    let delta = (expected - actual).abs();
    assert!(
        delta <= 0.000_001,
        "case {case_name}: expected {expected}, got {actual}, delta {delta}"
    );
}
