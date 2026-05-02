//! Optional logic-programming adapters for comparing Codex reasoning decisions.
//!
//! This crate is intentionally isolated from `codex-core`: callers can compare
//! existing Rust decisions with real Prolog or ProbLog engines without making
//! either engine part of the default runtime path.

use std::fmt;
use std::fs;
use std::process::Command;
use std::sync::Arc;

use codex_execpolicy::Decision;
use codex_execpolicy::Evaluation;
use codex_execpolicy::MatchOptions;
use codex_execpolicy::PatternToken;
use codex_execpolicy::Policy;
use codex_execpolicy::PrefixPattern;
use codex_execpolicy::PrefixRule;
use codex_execpolicy::RuleMatch;
use codex_execpolicy::RuleRef;
use codex_utils_absolute_path::AbsolutePathBuf;
use multimap::MultiMap;
use serde::Deserialize;
use serde::Serialize;
use tempfile::tempdir;
use thiserror::Error;

/// Result alias for the optional logic reasoning adapters.
pub type Result<T> = std::result::Result<T, ReasoningError>;

/// Errors returned by optional logic reasoning engines.
#[derive(Debug, Error)]
pub enum ReasoningError {
    #[error("logic engine `{engine}` is unavailable: {reason}")]
    Unavailable {
        engine: &'static str,
        reason: String,
    },

    #[error("failed to write logic program for `{engine}`: {source}")]
    WriteProgram {
        engine: &'static str,
        source: std::io::Error,
    },

    #[error("failed to run logic engine `{engine}`: {source}")]
    Run {
        engine: &'static str,
        source: std::io::Error,
    },

    #[error("logic engine `{engine}` exited with status {status}: {stderr}")]
    Failed {
        engine: &'static str,
        status: String,
        stderr: String,
    },

    #[error("could not parse `{engine}` output: {output}")]
    Parse {
        engine: &'static str,
        output: String,
    },

    #[error("invalid reasoning case: {0}")]
    InvalidCase(String),
}

/// Describes whether an optional logic engine can run in this environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReasonerAvailability {
    Available,
    Unavailable(String),
}

/// Deterministic reasoners evaluate rule-like Codex decisions.
///
/// Implementations are expected to return the strictest decision implied by
/// the supplied facts and to preserve enough match metadata for comparison
/// with the Rust baseline.
pub trait DeterministicReasoner {
    fn availability(&self) -> ReasonerAvailability;

    fn evaluate_exec_policy(&self, case: &ExecPolicyCase) -> Result<ExecPolicyOutcome>;
}

/// Probabilistic reasoners evaluate uncertain Codex decisions.
///
/// Implementations are expected to return calibrated probabilities for the
/// same named queries as the Rust baseline so tests can compare both paths.
pub trait ProbabilisticReasoner {
    fn availability(&self) -> ReasonerAvailability;

    fn evaluate_tool_suggestion(
        &self,
        case: &ToolSuggestionCase,
    ) -> Result<ToolSuggestionProbabilities>;
}

/// Prefix rule fact used by the deterministic Prolog model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrefixRuleFact {
    pub id: String,
    pub pattern: Vec<String>,
    pub decision: Decision,
}

impl PrefixRuleFact {
    pub fn new(id: impl Into<String>, pattern: Vec<String>, decision: Decision) -> Self {
        Self {
            id: id.into(),
            pattern,
            decision,
        }
    }
}

/// Maps an absolute host executable path to its bare executable name.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PathAliasFact {
    pub path: String,
    pub name: String,
}

impl PathAliasFact {
    pub fn new(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
        }
    }
}

/// Optional allowlist for host executable resolution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HostExecutableFact {
    pub name: String,
    pub paths: Vec<String>,
}

/// Kind of installable tool considered by the ProbLog suggestion model.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedToolType {
    Connector,
    Plugin,
}

impl HostExecutableFact {
    pub fn new(name: impl Into<String>, paths: Vec<String>) -> Self {
        Self {
            name: name.into(),
            paths,
        }
    }
}

/// Deterministic exec-policy comparison case.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecPolicyCase {
    pub name: String,
    pub rules: Vec<PrefixRuleFact>,
    pub commands: Vec<Vec<String>>,
    pub fallback_decision: Decision,
    pub path_aliases: Vec<PathAliasFact>,
    pub host_executables: Vec<HostExecutableFact>,
    pub expected_decision: Decision,
}

impl ExecPolicyCase {
    pub fn new(
        name: impl Into<String>,
        rules: Vec<PrefixRuleFact>,
        commands: Vec<Vec<String>>,
        fallback_decision: Decision,
        expected_decision: Decision,
    ) -> Self {
        Self {
            name: name.into(),
            rules,
            commands,
            fallback_decision,
            path_aliases: Vec::new(),
            host_executables: Vec::new(),
            expected_decision,
        }
    }

    pub fn with_path_aliases(mut self, path_aliases: Vec<PathAliasFact>) -> Self {
        self.path_aliases = path_aliases;
        self
    }

    pub fn with_host_executables(mut self, host_executables: Vec<HostExecutableFact>) -> Self {
        self.host_executables = host_executables;
        self
    }
}

/// Outcome returned by deterministic reasoners.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecPolicyOutcome {
    pub decision: Decision,
    pub policy_match_count: usize,
}

impl From<Evaluation> for ExecPolicyOutcome {
    fn from(value: Evaluation) -> Self {
        let policy_match_count = value
            .matched_rules
            .iter()
            .filter(|rule_match| matches!(rule_match, RuleMatch::PrefixRuleMatch { .. }))
            .count();
        Self {
            decision: value.decision,
            policy_match_count,
        }
    }
}

/// Probabilistic tool-suggestion comparison case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSuggestionCase {
    pub name: String,
    pub tool_type: SuggestedToolType,
    pub task_relevance_probability: f64,
    pub tool_available_probability: f64,
    pub user_acceptance_probability: f64,
    pub install_success_probability: f64,
    pub search_failed: bool,
    pub client_supported: bool,
    pub disabled_by_user: bool,
    pub already_accessible: bool,
}

impl ToolSuggestionCase {
    pub fn completion_possible(&self) -> bool {
        self.search_failed
            && self.client_supported
            && !self.disabled_by_user
            && !self.already_accessible
    }
}

/// Named probabilities returned by the ProbLog model and Rust baseline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSuggestionProbabilities {
    pub suggest_tool: f64,
    pub completed_tool: f64,
    pub missed_without_suggestion: f64,
}

/// Rust implementation of the same facts used by the optional logic engines.
#[derive(Clone, Copy, Debug, Default)]
pub struct RustBaselineReasoner;

impl DeterministicReasoner for RustBaselineReasoner {
    fn availability(&self) -> ReasonerAvailability {
        ReasonerAvailability::Available
    }

    fn evaluate_exec_policy(&self, case: &ExecPolicyCase) -> Result<ExecPolicyOutcome> {
        if case.commands.is_empty() {
            return Err(ReasoningError::InvalidCase(format!(
                "{} has no commands",
                case.name
            )));
        }
        let policy = policy_from_case(case)?;
        let fallback_decision = case.fallback_decision;
        let options = MatchOptions {
            resolve_host_executables: true,
        };
        Ok(policy
            .check_multiple_with_options(case.commands.iter(), &|_| fallback_decision, &options)
            .into())
    }
}

impl ProbabilisticReasoner for RustBaselineReasoner {
    fn availability(&self) -> ReasonerAvailability {
        ReasonerAvailability::Available
    }

    fn evaluate_tool_suggestion(
        &self,
        case: &ToolSuggestionCase,
    ) -> Result<ToolSuggestionProbabilities> {
        let base = case.task_relevance_probability * case.tool_available_probability;
        let suggestion_gate = if case.completion_possible() { 1.0 } else { 0.0 };
        let suggest_tool = base * suggestion_gate;
        let completed_tool =
            suggest_tool * case.user_acceptance_probability * case.install_success_probability;
        let missed_without_suggestion = base * suggestion_gate;
        Ok(ToolSuggestionProbabilities {
            suggest_tool,
            completed_tool,
            missed_without_suggestion,
        })
    }
}

/// SWI-Prolog subprocess adapter for deterministic comparison.
#[derive(Clone, Debug)]
pub struct SwiplReasoner {
    executable: String,
}

impl SwiplReasoner {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Default for SwiplReasoner {
    fn default() -> Self {
        Self::new("swipl")
    }
}

impl DeterministicReasoner for SwiplReasoner {
    fn availability(&self) -> ReasonerAvailability {
        match Command::new(&self.executable).arg("--version").output() {
            Ok(output) if output.status.success() => ReasonerAvailability::Available,
            Ok(output) => ReasonerAvailability::Unavailable(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ),
            Err(err) => ReasonerAvailability::Unavailable(err.to_string()),
        }
    }

    fn evaluate_exec_policy(&self, case: &ExecPolicyCase) -> Result<ExecPolicyOutcome> {
        let dir = tempdir().map_err(|source| ReasoningError::WriteProgram {
            engine: "swipl",
            source,
        })?;
        let program_path = dir.path().join("codex_execpolicy_compare.pl");
        fs::write(&program_path, render_prolog_program(case)).map_err(|source| {
            ReasoningError::WriteProgram {
                engine: "swipl",
                source,
            }
        })?;

        let output = Command::new(&self.executable)
            .args(["-q", "-f", "none", "-s"])
            .arg(&program_path)
            .args(["-g", "main", "-t", "halt"])
            .output()
            .map_err(|source| ReasoningError::Run {
                engine: "swipl",
                source,
            })?;
        if !output.status.success() {
            return Err(ReasoningError::Failed {
                engine: "swipl",
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        parse_prolog_outcome(&String::from_utf8_lossy(&output.stdout))
    }
}

/// ProbLog subprocess adapter for probabilistic comparison.
#[derive(Clone, Debug)]
pub struct ProblogReasoner {
    executable: String,
}

impl ProblogReasoner {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Default for ProblogReasoner {
    fn default() -> Self {
        Self::new("problog")
    }
}

impl ProbabilisticReasoner for ProblogReasoner {
    fn availability(&self) -> ReasonerAvailability {
        match Command::new(&self.executable).arg("--help").output() {
            Ok(output) if output.status.success() => ReasonerAvailability::Available,
            Ok(output) => ReasonerAvailability::Unavailable(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ),
            Err(err) => ReasonerAvailability::Unavailable(err.to_string()),
        }
    }

    fn evaluate_tool_suggestion(
        &self,
        case: &ToolSuggestionCase,
    ) -> Result<ToolSuggestionProbabilities> {
        let dir = tempdir().map_err(|source| ReasoningError::WriteProgram {
            engine: "problog",
            source,
        })?;
        let program_path = dir.path().join("codex_tool_suggest_compare.pl");
        fs::write(&program_path, render_problog_program(case)).map_err(|source| {
            ReasoningError::WriteProgram {
                engine: "problog",
                source,
            }
        })?;

        let output = Command::new(&self.executable)
            .arg(&program_path)
            .output()
            .map_err(|source| ReasoningError::Run {
                engine: "problog",
                source,
            })?;
        if !output.status.success() {
            return Err(ReasoningError::Failed {
                engine: "problog",
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        parse_problog_probabilities(&String::from_utf8_lossy(&output.stdout))
    }
}

fn policy_from_case(case: &ExecPolicyCase) -> Result<Policy> {
    let mut rules_by_program = MultiMap::new();
    for rule in &case.rules {
        let (first, rest) = rule.pattern.split_first().ok_or_else(|| {
            ReasoningError::InvalidCase(format!("{} has an empty prefix rule", case.name))
        })?;
        let rule_ref: RuleRef = Arc::new(PrefixRule {
            pattern: PrefixPattern {
                first: Arc::from(first.as_str()),
                rest: rest
                    .iter()
                    .map(|token| PatternToken::Single(token.clone()))
                    .collect::<Vec<_>>()
                    .into(),
            },
            decision: rule.decision,
            justification: None,
        });
        rules_by_program.insert(first.clone(), rule_ref);
    }

    let mut host_executables = std::collections::HashMap::new();
    for fact in &case.host_executables {
        let paths = fact
            .paths
            .iter()
            .map(|path| {
                AbsolutePathBuf::try_from(path.clone()).map_err(|err| {
                    ReasoningError::InvalidCase(format!(
                        "invalid host executable path `{path}` in {}: {err}",
                        case.name
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?;
        host_executables.insert(fact.name.clone(), paths.into());
    }

    Ok(Policy::from_parts(
        rules_by_program,
        Vec::new(),
        host_executables,
    ))
}

fn render_prolog_program(case: &ExecPolicyCase) -> String {
    let mut program = String::from(
        r#"
decision_rank(allow, 1).
decision_rank(prompt, 2).
decision_rank(forbidden, 3).

:- dynamic prefix_rule/3.
:- dynamic path_alias/2.
:- dynamic host_executable/2.

starts_with(_, []).
starts_with([H|T], [H|PT]) :- starts_with(T, PT).

exact_matched(CommandId, RuleId, Decision) :-
    command(CommandId, Command),
    prefix_rule(RuleId, Prefix, Decision),
    starts_with(Command, Prefix).

has_exact_match(CommandId) :- once(exact_matched(CommandId, _, _)).

host_canonical([Path|Rest], [Name|Rest]) :-
    path_alias(Path, Name),
    \+ host_executable(Name, _),
    !.
host_canonical([Path|Rest], [Name|Rest]) :-
    path_alias(Path, Name),
    host_executable(Name, Paths),
    member(Path, Paths),
    !.

host_matched(CommandId, RuleId, Decision) :-
    command(CommandId, Command),
    host_canonical(Command, Canonical),
    prefix_rule(RuleId, Prefix, Decision),
    starts_with(Canonical, Prefix).

matched(CommandId, RuleId, Decision) :-
    command(CommandId, _),
    has_exact_match(CommandId),
    exact_matched(CommandId, RuleId, Decision).
matched(CommandId, RuleId, Decision) :-
    command(CommandId, _),
    \+ has_exact_match(CommandId),
    host_matched(CommandId, RuleId, Decision).

command_decision(CommandId, Decision) :-
    matched(CommandId, _, Decision).
command_decision(CommandId, Decision) :-
    command(CommandId, _),
    \+ matched(CommandId, _, _),
    fallback_decision(Decision).

best_decision(Decision) :-
    findall(Rank-Decision, (command_decision(_, Decision), decision_rank(Decision, Rank)), Pairs),
    Pairs \= [],
    sort(Pairs, Sorted),
    last(Sorted, _-Decision),
    !.
best_decision(Decision) :- fallback_decision(Decision).

main :-
    best_decision(Decision),
    findall(RuleId, matched(_, RuleId, _), RuleIds),
    length(RuleIds, Count),
    format('decision(~w).~n', [Decision]),
    format('match_count(~w).~n', [Count]).
"#,
    );

    for (index, command) in case.commands.iter().enumerate() {
        program.push_str(&format!("command({index}, {}).\n", prolog_list(command)));
    }
    for rule in &case.rules {
        program.push_str(&format!(
            "prefix_rule({}, {}, {}).\n",
            prolog_atom(&rule.id),
            prolog_list(&rule.pattern),
            decision_atom(rule.decision)
        ));
    }
    for alias in &case.path_aliases {
        program.push_str(&format!(
            "path_alias({}, {}).\n",
            prolog_atom(&alias.path),
            prolog_atom(&alias.name)
        ));
    }
    for executable in &case.host_executables {
        program.push_str(&format!(
            "host_executable({}, {}).\n",
            prolog_atom(&executable.name),
            prolog_list(&executable.paths)
        ));
    }
    program.push_str(&format!(
        "fallback_decision({}).\n",
        decision_atom(case.fallback_decision)
    ));
    program
}

fn parse_prolog_outcome(output: &str) -> Result<ExecPolicyOutcome> {
    let mut decision = None;
    let mut policy_match_count = None;
    for line in output.lines().map(str::trim) {
        if let Some(raw) = line
            .strip_prefix("decision(")
            .and_then(|value| value.strip_suffix(")."))
        {
            decision = Some(parse_decision_atom(raw, output)?);
        } else if let Some(raw) = line
            .strip_prefix("match_count(")
            .and_then(|value| value.strip_suffix(")."))
        {
            policy_match_count = Some(raw.parse::<usize>().map_err(|_| ReasoningError::Parse {
                engine: "swipl",
                output: output.to_string(),
            })?);
        }
    }

    match (decision, policy_match_count) {
        (Some(decision), Some(policy_match_count)) => Ok(ExecPolicyOutcome {
            decision,
            policy_match_count,
        }),
        _ => Err(ReasoningError::Parse {
            engine: "swipl",
            output: output.to_string(),
        }),
    }
}

fn render_problog_program(case: &ToolSuggestionCase) -> String {
    let mut program = String::new();
    write_probability_fact(
        &mut program,
        "task_relevant",
        case.task_relevance_probability,
    );
    write_probability_fact(
        &mut program,
        "tool_available",
        case.tool_available_probability,
    );
    write_probability_fact(
        &mut program,
        "user_accepts",
        case.user_acceptance_probability,
    );
    write_probability_fact(
        &mut program,
        "install_succeeds",
        case.install_success_probability,
    );
    write_probability_fact(
        &mut program,
        "search_failed",
        probability_for_bool(case.search_failed),
    );
    write_probability_fact(
        &mut program,
        "client_supported",
        probability_for_bool(case.client_supported),
    );
    write_probability_fact(
        &mut program,
        "not_disabled",
        probability_for_bool(!case.disabled_by_user),
    );
    write_probability_fact(
        &mut program,
        "not_already_accessible",
        probability_for_bool(!case.already_accessible),
    );
    program.push_str(
        r#"
suggest_tool :-
    task_relevant,
    tool_available,
    search_failed,
    client_supported,
    not_disabled,
    not_already_accessible.

completed_tool :-
    suggest_tool,
    user_accepts,
    install_succeeds.

missed_without_suggestion :- suggest_tool.

query(suggest_tool).
query(completed_tool).
query(missed_without_suggestion).
"#,
    );
    program
}

fn write_probability_fact(program: &mut String, name: &str, probability: f64) {
    program.push_str(&format!("{probability:.12}::{name}.\n"));
}

fn probability_for_bool(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn parse_problog_probabilities(output: &str) -> Result<ToolSuggestionProbabilities> {
    let mut suggest_tool = None;
    let mut completed_tool = None;
    let mut missed_without_suggestion = None;
    for line in output.lines().map(str::trim) {
        let Some((name, raw_probability)) = line.split_once(':') else {
            continue;
        };
        let probability =
            raw_probability
                .trim()
                .parse::<f64>()
                .map_err(|_| ReasoningError::Parse {
                    engine: "problog",
                    output: output.to_string(),
                })?;
        match name.trim() {
            "suggest_tool" => suggest_tool = Some(probability),
            "completed_tool" => completed_tool = Some(probability),
            "missed_without_suggestion" => missed_without_suggestion = Some(probability),
            _ => {}
        }
    }

    match (suggest_tool, completed_tool, missed_without_suggestion) {
        (Some(suggest_tool), Some(completed_tool), Some(missed_without_suggestion)) => {
            Ok(ToolSuggestionProbabilities {
                suggest_tool,
                completed_tool,
                missed_without_suggestion,
            })
        }
        _ => Err(ReasoningError::Parse {
            engine: "problog",
            output: output.to_string(),
        }),
    }
}

fn decision_atom(decision: Decision) -> &'static str {
    match decision {
        Decision::Allow => "allow",
        Decision::Prompt => "prompt",
        Decision::Forbidden => "forbidden",
    }
}

fn parse_decision_atom(raw: &str, output: &str) -> Result<Decision> {
    match raw {
        "allow" => Ok(Decision::Allow),
        "prompt" => Ok(Decision::Prompt),
        "forbidden" => Ok(Decision::Forbidden),
        _ => Err(ReasoningError::Parse {
            engine: "swipl",
            output: output.to_string(),
        }),
    }
}

fn prolog_list(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| prolog_atom(value))
        .collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
}

fn prolog_atom(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

impl fmt::Display for ReasonerAvailability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => f.write_str("available"),
            Self::Unavailable(reason) => write!(f, "unavailable: {reason}"),
        }
    }
}
