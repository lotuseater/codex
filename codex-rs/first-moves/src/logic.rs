use crate::predict::Candidate;
use crate::predict::Intent;
use std::collections::HashSet;

const MAX_LOGIC_DELTA: f64 = 0.18;

pub(crate) struct LogicInput<'a> {
    pub(crate) intent: Intent,
    pub(crate) prompt_lower: &'a str,
    pub(crate) prompt_terms: &'a HashSet<String>,
    pub(crate) memory_hints: &'a HashSet<String>,
}

#[derive(Debug, Default)]
pub(crate) struct LogicAssessment {
    pub(crate) score_delta: f64,
    pub(crate) reasons: Vec<&'static str>,
}

impl LogicAssessment {
    pub(crate) fn is_empty(&self) -> bool {
        self.score_delta.abs() < f64::EPSILON && self.reasons.is_empty()
    }
}

pub(crate) fn assess_candidate(candidate: &Candidate, input: &LogicInput<'_>) -> LogicAssessment {
    let rel_lower = candidate.rel_path.to_ascii_lowercase();
    let explicit_path = input.prompt_lower.contains(&rel_lower);
    let explicit_filename =
        input.prompt_lower.contains(&candidate.name) && candidate.name.len() >= 5;
    let memory_hint_match = input
        .memory_hints
        .iter()
        .any(|hint| hint.len() >= 3 && rel_lower.contains(hint));
    let overlap = candidate
        .path_terms
        .intersection(input.prompt_terms)
        .filter(|term| valuable_logic_term(term.as_str()))
        .count();

    let facts = PathFacts::from_path(&rel_lower);
    let mut assessment = LogicAssessment::default();
    let mut signals = Vec::new();
    let mut risks = Vec::new();

    if explicit_path {
        assessment.score_delta += 0.08;
        push_reason(
            &mut assessment.reasons,
            "logic gate: explicit structured path",
        );
        signals.push(0.95);
    } else if explicit_filename {
        assessment.score_delta += 0.04;
        push_reason(
            &mut assessment.reasons,
            "logic gate: explicit structured filename",
        );
        signals.push(0.74);
    }

    if memory_hint_match {
        assessment.score_delta += 0.05;
        push_reason(
            &mut assessment.reasons,
            "probabilistic evidence: memory hint support",
        );
        signals.push(0.70);
    }

    if intent_path_fit(input.intent, &facts, &rel_lower) {
        assessment.score_delta += 0.08;
        push_reason(
            &mut assessment.reasons,
            "probabilistic evidence: intent/path fit",
        );
        signals.push(0.62);
    } else if implementation_like(input.intent)
        && facts.docs
        && !explicit_path
        && !explicit_filename
        && !prompt_asks_for_docs(input.prompt_lower)
    {
        assessment.score_delta -= 0.12;
        push_reason(
            &mut assessment.reasons,
            "logic gate: implementation docs de-prioritized",
        );
        risks.push(0.45);
    }

    if facts.noisy_auxiliary && !explicit_path && !explicit_filename {
        assessment.score_delta -= 0.14;
        push_reason(
            &mut assessment.reasons,
            "logic gate: noisy auxiliary artifact",
        );
        risks.push(0.65);
    }

    if overlap > 0 {
        let term_signal = (0.20 + overlap as f64 * 0.08).min(0.55);
        signals.push(term_signal);
    }

    if !signals.is_empty() {
        let probability = bounded_probability(&signals, &risks);
        if probability >= 0.65 {
            assessment.score_delta += ((probability - 0.60) * 0.18).min(0.10);
            push_reason(
                &mut assessment.reasons,
                "probabilistic evidence: bounded support",
            );
        } else if probability <= 0.35 {
            assessment.score_delta -= ((0.40 - probability) * 0.16).min(0.08);
            push_reason(
                &mut assessment.reasons,
                "probabilistic evidence: bounded risk",
            );
        }
    } else if !risks.is_empty() {
        assessment.score_delta -= 0.04;
    }

    assessment.score_delta = assessment
        .score_delta
        .clamp(-MAX_LOGIC_DELTA, MAX_LOGIC_DELTA);
    assessment
}

fn bounded_probability(signals: &[f64], risks: &[f64]) -> f64 {
    let support = 1.0 - signals.iter().fold(1.0, |acc, value| acc * (1.0 - value));
    let risk = 1.0 - risks.iter().fold(1.0, |acc, value| acc * (1.0 - value));
    (support * (1.0 - risk * 0.65)).clamp(0.0, 1.0)
}

fn push_reason(reasons: &mut Vec<&'static str>, reason: &'static str) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn intent_path_fit(intent: Intent, facts: &PathFacts, rel_lower: &str) -> bool {
    match intent {
        Intent::FirstMoves => facts.source && rel_lower.contains("first-moves"),
        Intent::GuiAutomation => {
            rel_lower.contains("desktop_automation")
                || rel_lower.contains("desktop-automation")
                || rel_lower.contains("automation")
        }
        Intent::Cache => rel_lower.contains("cache"),
        Intent::BuildTest => facts.build || facts.test,
        Intent::Review => facts.test || rel_lower.contains("review"),
        Intent::Debug => {
            facts.test || rel_lower.contains("handler") || rel_lower.contains("registry")
        }
        Intent::Research => facts.docs,
        Intent::Implement => {
            facts.source || rel_lower.contains("tool") || rel_lower.contains("config")
        }
        Intent::General => false,
    }
}

fn implementation_like(intent: Intent) -> bool {
    matches!(
        intent,
        Intent::FirstMoves
            | Intent::GuiAutomation
            | Intent::Cache
            | Intent::Debug
            | Intent::Implement
    )
}

fn prompt_asks_for_docs(prompt_lower: &str) -> bool {
    prompt_lower.contains("doc")
        || prompt_lower.contains("research")
        || prompt_lower.contains("readme")
}

fn valuable_logic_term(term: &str) -> bool {
    !matches!(
        term,
        "add"
            | "bug"
            | "change"
            | "codex"
            | "debug"
            | "docs"
            | "fix"
            | "implement"
            | "please"
            | "review"
            | "test"
            | "tests"
            | "the"
            | "this"
    )
}

struct PathFacts {
    source: bool,
    test: bool,
    docs: bool,
    build: bool,
    noisy_auxiliary: bool,
}

impl PathFacts {
    fn from_path(rel_lower: &str) -> Self {
        let name = rel_lower.rsplit('/').next().unwrap_or(rel_lower);
        let source = rel_lower.starts_with("src/")
            || rel_lower.contains("/src/")
            || (is_code_file(name)
                && !rel_lower.starts_with("docs/")
                && !rel_lower.contains("/docs/")
                && !rel_lower.starts_with("tests/")
                && !rel_lower.contains("/tests/"));
        let test = rel_lower.starts_with("tests/")
            || rel_lower.contains("/tests/")
            || name.contains("test")
            || name.contains("spec");
        let docs = rel_lower.starts_with("docs/")
            || rel_lower.contains("/docs/")
            || name == "readme.md"
            || name.ends_with(".md");
        let build = name == "cargo.toml"
            || name == "build.bazel"
            || name == "package.json"
            || name == "pyproject.toml"
            || rel_lower.contains("scripts/build");
        let noisy_auxiliary = rel_lower.contains("/snapshots/")
            || rel_lower.contains("/fixtures/")
            || rel_lower.contains("/testdata/")
            || rel_lower.contains("/test-data/")
            || rel_lower.contains("/__snapshots__/")
            || name.ends_with(".snap")
            || name.contains(".golden.")
            || name.contains(".baseline.")
            || name.contains(".generated.");

        Self {
            source,
            test,
            docs,
            build,
            noisy_auxiliary,
        }
    }
}

fn is_code_file(name: &str) -> bool {
    name.ends_with(".rs")
        || name.ends_with(".ts")
        || name.ends_with(".tsx")
        || name.ends_with(".js")
        || name.ends_with(".jsx")
        || name.ends_with(".py")
        || name.ends_with(".cpp")
        || name.ends_with(".hpp")
        || name.ends_with(".h")
        || name.ends_with(".c")
        || name.ends_with(".cc")
        || name.ends_with(".go")
        || name.ends_with(".java")
        || name.ends_with(".cs")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(path: &str) -> Candidate {
        Candidate {
            rel_path: path.to_string(),
            name: path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase(),
            path_terms: path
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .map(str::to_ascii_lowercase)
                .filter(|term| term.len() >= 3)
                .collect(),
        }
    }

    #[test]
    fn first_moves_implementation_favors_source_over_docs() {
        let prompt_terms = HashSet::from([
            "implement".to_string(),
            "first".to_string(),
            "moves".to_string(),
            "logic".to_string(),
            "overlay".to_string(),
        ]);
        let memory_hints = HashSet::new();
        let input = LogicInput {
            intent: Intent::FirstMoves,
            prompt_lower: "implement first moves logic overlay",
            prompt_terms: &prompt_terms,
            memory_hints: &memory_hints,
        };

        let source = assess_candidate(&candidate("codex-rs/first-moves/src/predict.rs"), &input);
        let docs = assess_candidate(&candidate("docs/first-moves-logic-overlay.md"), &input);

        assert!(source.score_delta > 0.0);
        assert!(docs.score_delta < 0.0);
        assert!(
            source
                .reasons
                .contains(&"probabilistic evidence: intent/path fit")
        );
        assert!(
            docs.reasons
                .contains(&"logic gate: implementation docs de-prioritized")
        );
    }
}
