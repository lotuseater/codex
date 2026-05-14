use codex_operation_cache::tool_is_cacheable;
use codex_protocol::ThreadId;
use codex_state::DirectionalThreadSpawnEdgeStatus;
use codex_tools::AGENT_GRAPH_SCOUT_TOOL_NAME;
use codex_tools::CODE_RELATION_SCOUT_TOOL_NAME;
use codex_tools::EVIDENCE_FUSION_SUMMARY_TOOL_NAME;
use codex_tools::MISSION_TRACE_EXPORT_TOOL_NAME;
use codex_tools::OPERATION_CACHE_STATS_TOOL_NAME;
use codex_tools::PROBLEM_MEMORY_LOOKUP_TOOL_NAME;
use codex_tools::ToolName;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

const DEFAULT_PROBLEM_MEMORY_MAX_MATCHES: usize = 3;

pub struct CognosOpsHandler {
    tool_name: ToolName,
}

impl CognosOpsHandler {
    pub fn new(tool_name: ToolName) -> Self {
        Self { tool_name }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProblemMemoryLookupArgs {
    project_root: Option<String>,
    prompt: Option<String>,
    max_matches: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodeRelationScoutArgs {
    project_root: Option<String>,
    prompt: Option<String>,
    max_paths: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentGraphScoutArgs {
    status: Option<AgentGraphStatusArg>,
    max_agents: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentGraphStatusArg {
    Open,
    Closed,
    All,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationCacheStatsArgs {
    project_root: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct EvidenceFusionSummaryArgs {
    #[serde(default)]
    tests: Vec<String>,
    #[serde(default)]
    diff_summary: String,
    #[serde(default)]
    review_findings: Vec<String>,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    unresolved_caveats: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct MissionTraceExportArgs {
    #[serde(default)]
    task_prompt: String,
    #[serde(default)]
    tests: Vec<String>,
    #[serde(default)]
    diff_summary: String,
    #[serde(default)]
    review_findings: Vec<String>,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    unresolved_caveats: Vec<String>,
    #[serde(default)]
    agent_notes: Vec<String>,
    #[serde(default)]
    tool_notes: Vec<String>,
    #[serde(default)]
    cache_notes: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct EvidenceDecision {
    decision: &'static str,
    repair_policy: &'static str,
    failing_tests: bool,
}

impl ToolHandler for CognosOpsHandler {
    type Output = FunctionToolOutput;

    fn tool_name(&self) -> ToolName {
        self.tool_name.clone()
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments.clone(),
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "cognos ops handler received unsupported payload".to_string(),
                ));
            }
        };

        match invocation.tool_name.name.as_str() {
            PROBLEM_MEMORY_LOOKUP_TOOL_NAME => {
                handle_problem_memory_lookup(invocation, arguments.as_str()).await
            }
            CODE_RELATION_SCOUT_TOOL_NAME => {
                handle_code_relation_scout(invocation, arguments.as_str()).await
            }
            AGENT_GRAPH_SCOUT_TOOL_NAME => {
                handle_agent_graph_scout(invocation, arguments.as_str()).await
            }
            OPERATION_CACHE_STATS_TOOL_NAME => {
                handle_operation_cache_stats(invocation, arguments.as_str()).await
            }
            EVIDENCE_FUSION_SUMMARY_TOOL_NAME => {
                handle_evidence_fusion_summary(arguments.as_str()).await
            }
            MISSION_TRACE_EXPORT_TOOL_NAME => handle_mission_trace_export(arguments.as_str()).await,
            other => Err(FunctionCallError::RespondToModel(format!(
                "unknown cognos ops tool: {other}"
            ))),
        }
    }
}

async fn handle_problem_memory_lookup(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: ProblemMemoryLookupArgs = parse_arguments(arguments)?;
    let project_root = invocation.turn.resolve_path(args.project_root);
    let prompt = args.prompt.unwrap_or_default();
    let max_matches = args
        .max_matches
        .unwrap_or(DEFAULT_PROBLEM_MEMORY_MAX_MATCHES)
        .clamp(1, 20);
    let memory_root = invocation.turn.config.codex_home.join("memories");
    let terms = terms(&prompt);
    let project_root_text = normalize_path_text(project_root.as_path());
    let mut candidates = Vec::new();

    for index_name in ["project_index.jsonl", "problem_index.jsonl"] {
        let path = memory_root.join(index_name);
        let Ok(contents) = tokio::fs::read_to_string(path.as_path()).await else {
            continue;
        };
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let haystack = value.to_string().to_ascii_lowercase();
            let cwd = value
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .replace('\\', "/")
                .to_ascii_lowercase();
            if !memory_entry_matches_project_scope(cwd.as_str(), project_root_text.as_str()) {
                continue;
            }
            let Some(score) =
                score_problem_memory_candidate(&terms, haystack.as_str(), !cwd.is_empty())
            else {
                continue;
            };
            candidates.push((score, index_name, value));
        }
    }

    candidates.sort_by(|left, right| {
        right.0.cmp(&left.0).then_with(|| {
            let left_updated = left
                .2
                .get("source_updated_at")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let right_updated = right
                .2
                .get("source_updated_at")
                .and_then(Value::as_str)
                .unwrap_or_default();
            right_updated.cmp(left_updated)
        })
    });
    candidates.truncate(max_matches);

    json_output(json!({
        "project_root": project_root.display().to_string(),
        "prompt_terms": terms,
        "matches": candidates
            .into_iter()
            .map(|(score, source, value)| json!({
                "score": score,
                "source": source,
                "hint": value,
                "caveat": "routing evidence only; verify against current repo state",
            }))
            .collect::<Vec<_>>(),
    }))
}

async fn handle_code_relation_scout(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: CodeRelationScoutArgs = parse_arguments(arguments)?;
    let project_root = invocation.turn.resolve_path(args.project_root);
    let prompt = args.prompt.unwrap_or_default();
    let max_paths = args.max_paths.unwrap_or(16).clamp(1, 64);
    let project_root_path = project_root.to_path_buf();
    let bundle = tokio::task::spawn_blocking(move || {
        code_relation_bundle(project_root_path.as_path(), &prompt, max_paths)
    })
    .await
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    json_output(bundle)
}

async fn handle_agent_graph_scout(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: AgentGraphScoutArgs = parse_arguments(arguments)?;
    let max_agents = args.max_agents.unwrap_or(20).clamp(1, 100);
    let status_filter = match args.status.unwrap_or(AgentGraphStatusArg::Open) {
        AgentGraphStatusArg::Open => Some(DirectionalThreadSpawnEdgeStatus::Open),
        AgentGraphStatusArg::Closed => Some(DirectionalThreadSpawnEdgeStatus::Closed),
        AgentGraphStatusArg::All => None,
    };
    let Some(state_db) = invocation.session.state_db() else {
        return json_output(json!({
            "available": false,
            "reason": "state_db_unavailable",
            "recommendation": "fresh_spawn_if_roi_positive",
        }));
    };
    let root_thread_id: ThreadId = invocation.session.conversation_id;
    let mut children = match status_filter {
        Some(status) => {
            state_db
                .list_thread_spawn_children_with_status(root_thread_id, status)
                .await
        }
        None => state_db.list_thread_spawn_children(root_thread_id).await,
    }
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    children.truncate(max_agents);
    let descendants = match status_filter {
        Some(status) => {
            state_db
                .list_thread_spawn_descendants_with_status(root_thread_id, status)
                .await
        }
        None => state_db.list_thread_spawn_descendants(root_thread_id).await,
    }
    .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;

    let recommendation = if children.is_empty() {
        "fresh_spawn_if_roi_positive"
    } else {
        "reuse_or_followup_before_fresh_spawn"
    };

    json_output(json!({
        "available": true,
        "root_thread_id": root_thread_id.to_string(),
        "direct_children": children.into_iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        "descendant_count": descendants.len(),
        "recommendation": recommendation,
        "valid_scout_evidence": true,
    }))
}

async fn handle_operation_cache_stats(
    invocation: ToolInvocation,
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: OperationCacheStatsArgs = parse_arguments(arguments)?;
    let project_root = invocation.turn.resolve_path(args.project_root);
    let env_enabled = std::env::var("WIZARD_CODEX_OPERATION_CACHE").ok();
    let bridge = std::env::var("WIZARD_CODEX_CACHE_BRIDGE_PY").ok();
    let python = std::env::var("WIZARD_CODEX_CACHE_PYTHON").ok();
    let timeout_ms = std::env::var("WIZARD_CODEX_CACHE_TIMEOUT_MS").ok();
    let sample_tools = [
        "shell_command",
        "first_moves_predict",
        "repo_context_scout",
        "file_outline",
        "search_text",
        "dab_screenshot",
        "dab_visual_scan",
    ];

    json_output(json!({
        "project_root": project_root.display().to_string(),
        "enabled_env": env_enabled,
        "bridge": bridge,
        "python": python,
        "timeout_ms": timeout_ms,
        "cacheable_samples": sample_tools
            .iter()
            .map(|name| json!({
                "tool": name,
                "cacheable": tool_is_cacheable(name),
                "reason": if tool_is_cacheable(name) { "cacheable_by_default" } else { "live_desktop_automation_not_cacheable" },
            }))
            .collect::<Vec<_>>(),
        "known_miss_reasons": [
            "operation cache disabled by env",
            "bridge path not configured",
            "bridge timeout",
            "tool marked mutating",
            "live DAB tool",
            "input or cwd scope changed",
            "cache invalidated by edit hook"
        ],
    }))
}

async fn handle_evidence_fusion_summary(
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: EvidenceFusionSummaryArgs = parse_arguments(arguments)?;
    let blockers = non_empty(args.blockers);
    let findings = non_empty(args.review_findings);
    let caveats = non_empty(args.unresolved_caveats);
    let tests = non_empty(args.tests);
    let decision = classify_evidence(&tests, &findings, &blockers, &caveats);

    json_output(json!({
        "decision": decision.decision,
        "repair_policy": decision.repair_policy,
        "tests": tests,
        "diff_summary": args.diff_summary,
        "review_findings": findings,
        "blockers": blockers,
        "unresolved_caveats": caveats,
    }))
}

async fn handle_mission_trace_export(
    arguments: &str,
) -> Result<FunctionToolOutput, FunctionCallError> {
    let args: MissionTraceExportArgs = parse_arguments(arguments)?;
    let tests = non_empty(args.tests);
    let findings = non_empty(args.review_findings);
    let blockers = non_empty(args.blockers);
    let caveats = non_empty(args.unresolved_caveats);
    let agent_notes = non_empty(args.agent_notes);
    let tool_notes = non_empty(args.tool_notes);
    let cache_notes = non_empty(args.cache_notes);
    let decision = classify_evidence(&tests, &findings, &blockers, &caveats);

    json_output(json!({
        "trace_kind": "mission_trace_export",
        "task_prompt": args.task_prompt.trim(),
        "decision": decision.decision,
        "repair_policy": decision.repair_policy,
        "signals": {
            "failing_tests": decision.failing_tests,
            "has_review_findings": !findings.is_empty(),
            "has_blockers": !blockers.is_empty(),
            "has_unresolved_caveats": !caveats.is_empty(),
        },
        "evidence": {
            "tests": tests,
            "diff_summary": args.diff_summary.trim(),
            "review_findings": findings,
            "blockers": blockers,
            "unresolved_caveats": caveats,
        },
        "notes": {
            "agent": agent_notes,
            "tool": tool_notes,
            "cache": cache_notes,
        },
    }))
}

fn classify_evidence(
    tests: &[String],
    findings: &[String],
    blockers: &[String],
    caveats: &[String],
) -> EvidenceDecision {
    let failing_tests = tests.iter().any(|test| test_output_indicates_failure(test));

    let decision = if !blockers.is_empty() {
        "Stop"
    } else if failing_tests || !findings.is_empty() || !caveats.is_empty() {
        "Modify"
    } else {
        "Accept"
    };
    let repair_policy = if decision == "Modify" {
        "one bounded repair pass may run when caveats are concrete, repo-controlled, and directly verifiable"
    } else {
        "no repair pass needed"
    };

    EvidenceDecision {
        decision,
        repair_policy,
        failing_tests,
    }
}

fn test_output_indicates_failure(test: &str) -> bool {
    let text = test.to_ascii_lowercase();
    if text.contains("not green")
        || text.contains("timeout")
        || text.contains("timed out")
        || text.contains("could not compile")
    {
        return true;
    }

    if (text.contains("exit code") || text.contains("exit status"))
        && !(text.contains("exit code 0")
            || text.contains("exit code: 0")
            || text.contains("exit status 0")
            || text.contains("exit status: 0"))
    {
        return true;
    }

    if text.contains("error:") || text.contains("error[") || text.contains("errors emitted") {
        return true;
    }

    contains_unnegated_failure_word(&text)
}

fn contains_unnegated_failure_word(text: &str) -> bool {
    let tokens = text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    for (index, token) in tokens.iter().enumerate() {
        if !matches!(
            *token,
            "fail" | "failed" | "failure" | "failures" | "failing"
        ) {
            continue;
        }

        let previous = index.checked_sub(1).and_then(|i| tokens.get(i)).copied();
        let next = tokens.get(index + 1).copied();
        if matches!(previous, Some("no" | "zero" | "0")) || matches!(next, Some("0")) {
            continue;
        }

        return true;
    }

    false
}

fn code_relation_bundle(project_root: &Path, prompt: &str, max_paths: usize) -> Value {
    let terms = terms(prompt);
    let mut files = Vec::new();
    collect_files(project_root, project_root, &mut files);
    let mut ranked = files
        .into_iter()
        .map(|path| {
            let path_text = path.to_string_lossy().replace('\\', "/");
            let role = classify_path(&path_text);
            let score = score_terms(&terms, &path_text.to_ascii_lowercase()) + role_boost(role);
            (score, path_text, role)
        })
        .filter(|(score, _, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    ranked.truncate(max_paths);
    let selected_paths = ranked
        .iter()
        .map(|(_, path, _)| path.clone())
        .collect::<BTreeSet<_>>();
    let edges = relation_edges(&selected_paths);

    json!({
        "project_root": project_root.display().to_string(),
        "prompt_terms": terms,
        "candidates": ranked
            .into_iter()
            .map(|(score, path, role)| json!({
                "path": path,
                "role": role,
                "score": score,
                "relation_reason": relation_reason(role),
            }))
            .collect::<Vec<_>>(),
        "edges": edges,
    })
}

fn collect_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    const MAX_FILES: usize = 5_000;
    if files.len() >= MAX_FILES {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if skip_name(name.as_ref()) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, path.as_path(), files);
        } else if path.is_file()
            && let Ok(relative) = path.strip_prefix(root)
        {
            files.push(relative.to_path_buf());
        }
        if files.len() >= MAX_FILES {
            break;
        }
    }
}

fn skip_name(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | "dist" | "build" | ".next" | ".cache"
    )
}

fn classify_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.contains("/test") || lower.contains("_test") || lower.ends_with("tests.rs") {
        "test"
    } else if lower.ends_with(".md") || lower.contains("/docs/") {
        "docs"
    } else if lower.ends_with(".toml")
        || lower.ends_with(".json")
        || lower.ends_with(".yml")
        || lower.ends_with(".yaml")
    {
        "config"
    } else if lower.contains("/generated/") || lower.contains("/target/") {
        "generated"
    } else if lower.ends_with("mod.rs") || lower.ends_with("lib.rs") || lower.ends_with("main.rs") {
        "entrypoint"
    } else if lower.ends_with(".h") || lower.ends_with(".hpp") || lower.contains("/protocol/") {
        "interface"
    } else {
        "implementation"
    }
}

fn role_boost(role: &str) -> i32 {
    match role {
        "entrypoint" | "interface" => 2,
        "test" | "implementation" => 1,
        _ => 0,
    }
}

fn relation_reason(role: &str) -> &'static str {
    match role {
        "entrypoint" => "likely entrypoint or module aggregator",
        "interface" => "likely API or type boundary",
        "implementation" => "likely behavior implementation",
        "test" => "likely verification surface",
        "config" => "likely build/runtime configuration",
        "docs" => "likely design or usage context",
        "generated" => "likely generated/artifact path; inspect only with care",
        _ => "candidate path relation",
    }
}

fn relation_edges(paths: &BTreeSet<String>) -> Vec<Value> {
    let mut edges = Vec::new();
    for path in paths {
        if classify_path(path) == "test" {
            let stem = path
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .replace("_test", "")
                .replace("tests", "")
                .replace(".rs", "")
                .replace(".cpp", "");
            if let Some(target) = paths
                .iter()
                .find(|candidate| *candidate != path && candidate.contains(&stem))
            {
                edges.push(json!({
                    "from": path,
                    "to": target,
                    "relation": "tests",
                    "reason": "test filename overlaps implementation candidate",
                }));
            }
        }
    }
    edges
}

fn non_empty(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn terms(prompt: &str) -> Vec<String> {
    prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .map(str::trim)
        .filter(|term| term.len() >= 3)
        .map(str::to_ascii_lowercase)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn score_terms(terms: &[String], haystack: &str) -> i32 {
    terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count() as i32
}

fn score_problem_memory_candidate(terms: &[String], haystack: &str, scoped: bool) -> Option<i32> {
    let term_hits = score_terms(terms, haystack);
    if term_hits == 0 {
        return None;
    }
    Some(term_hits + if scoped { 4 } else { 0 })
}

fn memory_entry_matches_project_scope(cwd: &str, project_root: &str) -> bool {
    if cwd.is_empty() {
        return true;
    }

    path_text_has_boundary_prefix(cwd, project_root)
        || path_text_has_boundary_prefix(project_root, cwd)
}

fn path_text_has_boundary_prefix(path: &str, prefix: &str) -> bool {
    let path = path.trim_end_matches('/');
    let prefix = prefix.trim_end_matches('/');
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn normalize_path_text(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn json_output(value: Value) -> Result<FunctionToolOutput, FunctionCallError> {
    let text = serde_json::to_string_pretty(&value)
        .map_err(|err| FunctionCallError::RespondToModel(err.to_string()))?;
    let mut output = FunctionToolOutput::from_text(text, Some(true));
    output.post_tool_use_response = Some(value);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn mission_trace_export_returns_modify_for_fixable_caveat() {
        let arguments = json!({
            "task_prompt": "finish memory ops",
            "tests": ["cargo test passed"],
            "diff_summary": "added trace export",
            "review_findings": [],
            "blockers": [],
            "unresolved_caveats": ["needs targeted release test"],
            "agent_notes": ["helper reviewed tool wiring"],
            "tool_notes": ["read-only operation"],
            "cache_notes": ["no cache mutation"]
        })
        .to_string();

        let output = handle_mission_trace_export(&arguments)
            .await
            .expect("mission trace output");
        let response = output
            .post_tool_use_response
            .expect("post tool response json");

        assert_eq!(response["trace_kind"], "mission_trace_export");
        assert_eq!(response["decision"], "Modify");
        assert_eq!(response["signals"]["has_unresolved_caveats"], true);
        assert_eq!(response["notes"]["tool"][0], "read-only operation");
    }

    #[test]
    fn evidence_classification_accepts_green_test_summaries() {
        for tests in [
            vec!["cargo test passed with no errors".to_string()],
            vec!["no failures".to_string()],
            vec!["error count: 0".to_string()],
            vec!["test result: ok. 10 passed; 0 failed".to_string()],
            vec!["all tests passed".to_string()],
        ] {
            let decision = classify_evidence(&tests, &[], &[], &[]);

            assert_eq!(
                decision,
                EvidenceDecision {
                    decision: "Accept",
                    repair_policy: "no repair pass needed",
                    failing_tests: false,
                }
            );
        }
    }

    #[test]
    fn evidence_classification_modifies_for_failing_test_summaries() {
        for tests in [
            vec!["cargo test failed".to_string()],
            vec!["error: could not compile codex-core".to_string()],
            vec!["test command timed out".to_string()],
            vec!["verification is not green".to_string()],
            vec!["process exited with exit code 101".to_string()],
        ] {
            let decision = classify_evidence(&tests, &[], &[], &[]);

            assert_eq!(
                decision,
                EvidenceDecision {
                    decision: "Modify",
                    repair_policy: "one bounded repair pass may run when caveats are concrete, repo-controlled, and directly verifiable",
                    failing_tests: true,
                }
            );
        }
    }

    #[test]
    fn problem_memory_lookup_scope_boost_requires_term_match() {
        let terms = terms("donut physics");

        assert_eq!(
            score_problem_memory_candidate(
                &terms,
                "same repo memory about deployment wrappers",
                true,
            ),
            None
        );
        assert_eq!(
            score_problem_memory_candidate(
                &terms,
                "same repo memory about donut rendering physics",
                true,
            ),
            Some(6)
        );
    }

    #[test]
    fn problem_memory_lookup_scope_rejects_other_repos() {
        assert!(memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex/codex-rs",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(memory_entry_matches_project_scope(
            "",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(!memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/other_repo",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
        assert!(!memory_entry_matches_project_scope(
            "c:/users/oleh/documents/github/open_ai/codex-old",
            "c:/users/oleh/documents/github/open_ai/codex"
        ));
    }
}
