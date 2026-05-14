use crate::logic::LogicInput;
use crate::logic::assess_candidate;
use crate::predict::Candidate;
use crate::predict::Intent;
use crate::predict::repo_structure_score;
use crate::storage::normalize_path_text;
use crate::storage::short_hash;
use crate::types::FirstMove;
use crate::types::FirstMoveKind;
use crate::types::FirstMovesConfig;
use crate::types::FirstMovesStorage;
use crate::types::Result;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const VARIANT_PATH_LEXICAL: &str = "path_lexical";
const VARIANT_LOGIC_EVIDENCE: &str = "logic_evidence";
const VARIANT_CONTENT_SEEDED_COMPONENT_MERGE: &str = "content_seeded_component_merge";

pub(crate) struct ShadowPredictInput<'a> {
    pub(crate) codex_home: &'a Path,
    pub(crate) prompt: &'a str,
    pub(crate) session_id: Option<&'a str>,
    pub(crate) config: &'a FirstMovesConfig,
    pub(crate) storage: &'a FirstMovesStorage,
    pub(crate) candidates: &'a [Candidate],
    pub(crate) prompt_terms: &'a HashSet<String>,
    pub(crate) already_loaded: &'a HashSet<String>,
    pub(crate) memory_hints: &'a HashSet<String>,
    pub(crate) intent: Intent,
    pub(crate) native_moves: &'a [FirstMove],
}

#[derive(Debug, Serialize)]
struct ShadowRecord {
    r#type: &'static str,
    timestamp_unix: u64,
    prompt_hash: String,
    session_id: Option<String>,
    repo_key: String,
    native_paths: Vec<String>,
    native_estimated_tokens: usize,
    scan_candidate_count: usize,
    max_scan_files: usize,
    max_candidates: usize,
    scan_cap_reached: bool,
    variants: Vec<ShadowVariantRecord>,
    fallback_reasons: Vec<String>,
    verdict: &'static str,
}

#[derive(Debug, Serialize)]
struct ShadowVariantRecord {
    name: &'static str,
    paths: Vec<String>,
    reasons: BTreeMap<String, Vec<&'static str>>,
    overlap_with_native: Vec<String>,
    estimated_tokens: usize,
    candidate_count: usize,
    cap_reached: bool,
    fallback_reasons: Vec<String>,
    verdict: &'static str,
}

#[derive(Debug, Clone)]
struct ShadowCandidate {
    path: String,
    score: f64,
    reasons: Vec<&'static str>,
}

pub(crate) fn record_shadow_prediction(input: ShadowPredictInput<'_>) -> Result<()> {
    let native_paths = native_read_paths(input.native_moves);
    let variants = vec![
        shadow_variant(
            VARIANT_PATH_LEXICAL,
            rank_path_lexical(&input),
            &native_paths,
            input.config.max_candidates,
        ),
        shadow_variant(
            VARIANT_LOGIC_EVIDENCE,
            rank_logic_evidence(&input),
            &native_paths,
            input.config.max_candidates,
        ),
        shadow_variant(
            VARIANT_CONTENT_SEEDED_COMPONENT_MERGE,
            rank_content_seeded_component_merge(&input),
            &native_paths,
            input.config.max_candidates,
        ),
    ];

    let mut fallback_reasons = Vec::new();
    if input.candidates.len() >= input.config.max_scan_files {
        fallback_reasons.push("scan_candidate_cap_reached".to_string());
    }

    let record = ShadowRecord {
        r#type: "first_moves_shadow",
        timestamp_unix: unix_now(),
        prompt_hash: short_hash(input.prompt),
        session_id: input.session_id.map(ToString::to_string),
        repo_key: input.storage.repo_key.clone(),
        native_estimated_tokens: estimate_path_packet_tokens(&native_paths),
        native_paths,
        scan_candidate_count: input.candidates.len(),
        max_scan_files: input.config.max_scan_files,
        max_candidates: input.config.max_candidates,
        scan_cap_reached: input.candidates.len() >= input.config.max_scan_files,
        variants,
        verdict: if fallback_reasons.is_empty() {
            "recorded"
        } else {
            "recorded_with_fallback"
        },
        fallback_reasons,
    };
    append_jsonl(
        input
            .codex_home
            .join("first-moves-shadow")
            .join(&input.storage.repo_key)
            .join("shadow.jsonl")
            .as_path(),
        &record,
    )
}

fn shadow_variant(
    name: &'static str,
    mut ranked: Vec<ShadowCandidate>,
    native_paths: &[String],
    max_candidates: usize,
) -> ShadowVariantRecord {
    let candidate_count = ranked.len();
    let cap_reached = candidate_count > max_candidates;
    ranked.truncate(max_candidates);
    let paths = ranked
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();
    let reasons = ranked
        .iter()
        .map(|candidate| (candidate.path.clone(), candidate.reasons.clone()))
        .collect::<BTreeMap<_, _>>();
    let native = native_paths.iter().cloned().collect::<BTreeSet<_>>();
    let overlap_with_native = paths
        .iter()
        .filter(|path| native.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    let fallback_reasons = if cap_reached {
        vec!["variant_candidate_cap_reached".to_string()]
    } else {
        Vec::new()
    };
    ShadowVariantRecord {
        name,
        estimated_tokens: estimate_path_packet_tokens(&paths),
        paths,
        reasons,
        overlap_with_native,
        candidate_count,
        cap_reached,
        verdict: if fallback_reasons.is_empty() {
            "recorded"
        } else {
            "recorded_with_fallback"
        },
        fallback_reasons,
    }
}

fn rank_path_lexical(input: &ShadowPredictInput<'_>) -> Vec<ShadowCandidate> {
    let prompt_lower = input.prompt.to_ascii_lowercase().replace('\\', "/");
    input
        .candidates
        .iter()
        .filter(|candidate| !path_is_loaded(&candidate.rel_path, input.already_loaded))
        .filter_map(|candidate| {
            let mut reasons = Vec::new();
            let score = lexical_score(candidate, &prompt_lower, input.prompt_terms, &mut reasons);
            (score > 0.0).then(|| ShadowCandidate {
                path: candidate.rel_path.clone(),
                score,
                reasons,
            })
        })
        .collect::<Vec<_>>()
        .tap_sort()
}

fn rank_logic_evidence(input: &ShadowPredictInput<'_>) -> Vec<ShadowCandidate> {
    let prompt_lower = input.prompt.to_ascii_lowercase().replace('\\', "/");
    let logic_input = LogicInput {
        intent: input.intent,
        prompt_lower: &prompt_lower,
        prompt_terms: input.prompt_terms,
        memory_hints: input.memory_hints,
    };

    input
        .candidates
        .iter()
        .filter(|candidate| !path_is_loaded(&candidate.rel_path, input.already_loaded))
        .filter_map(|candidate| {
            let mut reasons = Vec::new();
            let lexical = lexical_score(candidate, &prompt_lower, input.prompt_terms, &mut reasons);
            let logic = assess_candidate(candidate, &logic_input);
            for reason in logic.reasons {
                add_reason(&mut reasons, reason);
            }
            let score =
                lexical + repo_structure_score(candidate) * 100.0 + logic.score_delta * 1_000.0;
            (score > 0.0).then(|| ShadowCandidate {
                path: candidate.rel_path.clone(),
                score,
                reasons,
            })
        })
        .collect::<Vec<_>>()
        .tap_sort()
}

fn rank_content_seeded_component_merge(input: &ShadowPredictInput<'_>) -> Vec<ShadowCandidate> {
    let prompt_lower = input.prompt.to_ascii_lowercase().replace('\\', "/");
    let mut scored = BTreeMap::<String, ShadowCandidate>::new();
    for candidate in input
        .candidates
        .iter()
        .filter(|candidate| !path_is_loaded(&candidate.rel_path, input.already_loaded))
    {
        let mut reasons = Vec::new();
        let lexical = lexical_score(candidate, &prompt_lower, input.prompt_terms, &mut reasons);
        if lexical > 0.0 {
            add_score(
                &mut scored,
                candidate,
                10_000.0 + lexical,
                &reasons,
                "path lexical merge",
            );
        }
    }

    let mut terms = input
        .prompt_terms
        .iter()
        .filter(|term| valuable_component_term(term))
        .cloned()
        .collect::<Vec<_>>();
    terms.sort_by(|left, right| {
        component_term_priority(right)
            .cmp(&component_term_priority(left))
            .then_with(|| left.cmp(right))
    });

    for term in terms {
        if let Some((candidate, score)) = input
            .candidates
            .iter()
            .filter(|candidate| !path_is_loaded(&candidate.rel_path, input.already_loaded))
            .filter_map(|candidate| {
                let component_score = component_match_score(&term, candidate);
                (component_score > 0.0).then_some((
                    candidate,
                    component_score
                        + repo_structure_score(candidate) * 100.0
                        + candidate
                            .path_terms
                            .intersection(input.prompt_terms)
                            .count() as f64
                            * 20.0,
                ))
            })
            .max_by(|left, right| {
                left.1
                    .partial_cmp(&right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.0.rel_path.cmp(&left.0.rel_path))
            })
        {
            add_score(
                &mut scored,
                candidate,
                5_000.0 + score,
                &["component sweep"],
                "component sweep",
            );
        }
    }

    scored.into_values().collect::<Vec<_>>().tap_sort()
}

fn lexical_score(
    candidate: &Candidate,
    prompt_lower: &str,
    prompt_terms: &HashSet<String>,
    reasons: &mut Vec<&'static str>,
) -> f64 {
    let rel_lower = candidate.rel_path.to_ascii_lowercase();
    let mut score = 0.0;
    if prompt_lower.contains(&rel_lower) {
        score += 1_000.0;
        reasons.push("explicit path mention");
    } else if prompt_lower.contains(&candidate.name) && candidate.name.len() >= 5 {
        score += 650.0;
        reasons.push("explicit filename mention");
    }
    let overlap = prompt_terms.intersection(&candidate.path_terms).count();
    if overlap > 0 {
        score += overlap as f64 * 100.0;
        reasons.push("prompt/path term overlap");
    }
    if important_repo_file(&rel_lower) {
        score += 60.0;
        reasons.push("important repo file");
    }
    score
}

fn add_score(
    scored: &mut BTreeMap<String, ShadowCandidate>,
    candidate: &Candidate,
    score: f64,
    reasons: &[&'static str],
    fallback_reason: &'static str,
) {
    let entry = scored
        .entry(candidate.rel_path.clone())
        .or_insert_with(|| ShadowCandidate {
            path: candidate.rel_path.clone(),
            score: 0.0,
            reasons: Vec::new(),
        });
    entry.score += score;
    if reasons.is_empty() {
        add_reason(&mut entry.reasons, fallback_reason);
    } else {
        for reason in reasons {
            add_reason(&mut entry.reasons, reason);
        }
    }
}

fn add_reason(reasons: &mut Vec<&'static str>, reason: &'static str) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn component_match_score(term: &str, candidate: &Candidate) -> f64 {
    if candidate.path_terms.contains(term) {
        return 500.0;
    }
    candidate
        .rel_path
        .split(['/', '\\', '-', '_', '.'])
        .filter(|component| !component.is_empty())
        .map(str::to_ascii_lowercase)
        .filter_map(|component| {
            if component == term {
                Some(400.0)
            } else if component.contains(term) {
                Some(120.0)
            } else if term.contains(&component) && component.len() >= 4 {
                Some(80.0)
            } else {
                None
            }
        })
        .fold(0.0, f64::max)
}

fn valuable_component_term(term: &str) -> bool {
    term.len() >= 4
        && !matches!(
            term,
            "please"
                | "about"
                | "with"
                | "from"
                | "that"
                | "this"
                | "current"
                | "implementation"
                | "implement"
                | "change"
                | "changes"
                | "file"
                | "files"
                | "repo"
                | "project"
        )
}

fn component_term_priority(term: &str) -> usize {
    if term.contains('_') || term.contains('-') {
        5
    } else if term.len() >= 10 {
        4
    } else if term.len() >= 7 {
        3
    } else {
        1
    }
}

fn path_is_loaded(path: &str, already_loaded: &HashSet<String>) -> bool {
    already_loaded.contains(&normalize_path_text(path))
}

fn native_read_paths(moves: &[FirstMove]) -> Vec<String> {
    moves
        .iter()
        .filter(|entry| matches!(entry.kind, FirstMoveKind::Read))
        .filter_map(|entry| entry.path.as_ref())
        .map(|path| normalize_path_text(path.to_string_lossy().as_ref()))
        .collect()
}

fn estimate_path_packet_tokens(paths: &[String]) -> usize {
    let bytes = paths.iter().map(|path| path.len() + 3).sum::<usize>();
    bytes.div_ceil(4).max(1)
}

fn important_repo_file(path: &str) -> bool {
    matches!(
        path.rsplit('/').next().unwrap_or(path),
        "agents.md" | "readme.md" | "cargo.toml" | "package.json" | "pyproject.toml"
    )
}

fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

trait SortShadowCandidates {
    fn tap_sort(self) -> Self;
}

impl SortShadowCandidates for Vec<ShadowCandidate> {
    fn tap_sort(mut self) -> Self {
        self.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.path.cmp(&right.path))
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FirstMovesStorage;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    fn candidate(path: &str) -> Candidate {
        Candidate {
            rel_path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            path_terms: path
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .map(str::to_ascii_lowercase)
                .filter(|term| term.len() >= 3)
                .collect(),
        }
    }

    fn storage() -> FirstMovesStorage {
        FirstMovesStorage {
            repo_key: "repo-123".to_string(),
            system_db: PathBuf::from("unused"),
            repo_db: None,
            repo_db_exists: false,
        }
    }

    #[test]
    fn path_lexical_prefers_explicit_path_without_loaded_agents() {
        let candidates = vec![candidate("AGENTS.md"), candidate("src/target.rs")];
        let prompt_terms = HashSet::from(["inspect".to_string(), "target".to_string()]);
        let already_loaded = HashSet::from(["agents.md".to_string()]);
        let memory_hints = HashSet::new();
        let storage = storage();
        let input = ShadowPredictInput {
            codex_home: Path::new("unused"),
            prompt: "inspect src/target.rs",
            session_id: None,
            config: &FirstMovesConfig::default(),
            storage: &storage,
            candidates: &candidates,
            prompt_terms: &prompt_terms,
            already_loaded: &already_loaded,
            memory_hints: &memory_hints,
            intent: Intent::General,
            native_moves: &[],
        };

        let ranked = rank_path_lexical(&input);

        assert_eq!(ranked[0].path, "src/target.rs");
        assert!(ranked.iter().all(|candidate| candidate.path != "AGENTS.md"));
    }

    #[test]
    fn content_seeded_component_merge_picks_one_path_per_prompt_component() {
        let candidates = vec![
            candidate("src/cache/store.rs"),
            candidate("src/session/first_moves.rs"),
            candidate("README.md"),
        ];
        let prompt_terms = HashSet::from([
            "cache".to_string(),
            "session".to_string(),
            "first_moves".to_string(),
        ]);
        let already_loaded = HashSet::new();
        let memory_hints = HashSet::new();
        let storage = storage();
        let input = ShadowPredictInput {
            codex_home: Path::new("unused"),
            prompt: "inspect cache and session first_moves",
            session_id: None,
            config: &FirstMovesConfig::default(),
            storage: &storage,
            candidates: &candidates,
            prompt_terms: &prompt_terms,
            already_loaded: &already_loaded,
            memory_hints: &memory_hints,
            intent: Intent::FirstMoves,
            native_moves: &[],
        };

        let ranked = rank_content_seeded_component_merge(&input)
            .into_iter()
            .map(|candidate| candidate.path)
            .collect::<Vec<_>>();

        assert!(ranked.contains(&"src/cache/store.rs".to_string()));
        assert!(ranked.contains(&"src/session/first_moves.rs".to_string()));
    }

    #[test]
    fn shadow_variant_reports_native_overlap_and_cap() {
        let ranked = vec![
            ShadowCandidate {
                path: "src/a.rs".to_string(),
                score: 10.0,
                reasons: vec!["test"],
            },
            ShadowCandidate {
                path: "src/b.rs".to_string(),
                score: 9.0,
                reasons: vec!["test"],
            },
        ];

        let record = shadow_variant("variant", ranked, &["src/a.rs".to_string()], 1);

        assert_eq!(record.paths, vec!["src/a.rs"]);
        assert_eq!(record.overlap_with_native, vec!["src/a.rs"]);
        assert!(record.cap_reached);
        assert_eq!(record.verdict, "recorded_with_fallback");
    }

    #[test]
    fn record_shadow_prediction_writes_jsonl_without_changing_native_moves() {
        let temp = tempfile::tempdir().expect("temp dir");
        let candidates = vec![candidate("src/lib.rs")];
        let prompt_terms = HashSet::from(["lib".to_string()]);
        let already_loaded = HashSet::new();
        let memory_hints = HashSet::new();
        let native_moves = vec![FirstMove {
            kind: FirstMoveKind::Read,
            confidence: 0.9,
            reason: "native".to_string(),
            source_layer: "intent".to_string(),
            path: Some(PathBuf::from("src/lib.rs")),
            query: None,
            excerpt: None,
        }];
        let storage = storage();

        record_shadow_prediction(ShadowPredictInput {
            codex_home: temp.path(),
            prompt: "inspect src/lib.rs",
            session_id: Some("session"),
            config: &FirstMovesConfig::default(),
            storage: &storage,
            candidates: &candidates,
            prompt_terms: &prompt_terms,
            already_loaded: &already_loaded,
            memory_hints: &memory_hints,
            intent: Intent::General,
            native_moves: &native_moves,
        })
        .expect("shadow record");

        let path = temp
            .path()
            .join("first-moves-shadow")
            .join("repo-123")
            .join("shadow.jsonl");
        let text = std::fs::read_to_string(path).expect("shadow jsonl");
        assert!(text.contains("\"type\":\"first_moves_shadow\""));
        assert!(text.contains("\"src/lib.rs\""));
        assert!(text.contains("\"logic_evidence\""));
    }
}
