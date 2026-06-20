#!/usr/bin/env python3
"""Generate qualitative Markdown reports for context-helper prompt benchmarks."""

from __future__ import annotations

import argparse
import importlib.util
import json
import math
import statistics
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
BENCHMARK_SCRIPT = REPO_ROOT / "scripts" / "benchmark-context-helper-prompt-variants.py"
DEFAULT_RUN_DIR = (
    REPO_ROOT / "logs" / "context-helper-prompt-benchmarks" / "full-20-30-s6-v4"
)
VARIANTS = ("no_nudge", "standard_compaction_template", "prune", "delta", "evidence")


VARIANT_NAMES = {
    "no_nudge": "No-nudge control",
    "standard_compaction_template": "Standard compaction template",
    "prune": "Simple prune",
    "delta": "Delta merge",
    "evidence": "Evidence-preserving",
}


@dataclass(frozen=True)
class QualityRow:
    sample_id: str
    variant: str
    threshold_percent: int
    bucket: str
    prompt_tokens: int
    output_tokens: int
    saved_percent: float
    path_retain: float | None
    command_retain: float | None
    number_retain: float | None
    constraint_retain: float | None
    evidence_score: float
    readiness_score: float
    concision_score: float
    low_noise_score: float
    judge_score: float | None
    quality_score: float
    output: str
    prompt_path: Path | None
    output_path: Path | None
    raw_jsonl_path: Path | None
    stderr_path: Path | None


def load_benchmark_module() -> Any:
    spec = importlib.util.spec_from_file_location(
        "context_prompt_benchmark", BENCHMARK_SCRIPT
    )
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load benchmark script: {BENCHMARK_SCRIPT}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")


def abs_path(path: Path | None) -> str:
    if path is None:
        return ""
    return str(path.resolve())


def rel_path(path: Path, base: Path) -> str:
    try:
        return path.resolve().relative_to(base.resolve()).as_posix()
    except ValueError:
        return path.resolve().as_posix()


def table_cell(value: Any) -> str:
    text = "" if value is None else str(value)
    return text.replace("|", "\\|").replace("\n", "<br>")


def pct(value: float | None, digits: int = 1) -> str:
    if value is None:
        return "n/a"
    return f"{value:.{digits}f}%"


def num(value: float | None, digits: int = 1) -> str:
    if value is None:
        return "n/a"
    return f"{value:.{digits}f}"


def variant_name(variant: str) -> str:
    return VARIANT_NAMES.get(variant, variant.replace("_", " ").title())


def quality_cell(
    rows_by_sample_variant: dict[tuple[str, str], QualityRow],
    sample_id: str,
    variant: str,
) -> str:
    row = rows_by_sample_variant.get((sample_id, variant))
    if row is None:
        return "n/a"
    return num(row.quality_score)


def sample_judge_winner(
    sample_id: str, judge_index: dict[tuple[str, str], dict[str, Any]]
) -> str:
    for variant in VARIANTS:
        winner = judge_index.get((sample_id, variant), {}).get("best_variant")
        if winner:
            return winner
    return "n/a"


def markdown_table(headers: list[str], rows: list[list[Any]]) -> str:
    lines = [
        "| " + " | ".join(table_cell(header) for header in headers) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
    ]
    for row in rows:
        lines.append("| " + " | ".join(table_cell(value) for value in row) + " |")
    return "\n".join(lines)


def fence(text: str, language: str = "text") -> str:
    longest = 0
    current = 0
    for char in text:
        if char == "`":
            current += 1
            longest = max(longest, current)
        else:
            current = 0
    marker = "`" * max(4, longest + 1)
    return f"{marker}{language}\n{text.rstrip()}\n{marker}"


def clamp(value: float, minimum: float = 0.0, maximum: float = 100.0) -> float:
    return min(max(value, minimum), maximum)


def weighted_ratio_score(items: list[tuple[float | None, float]]) -> float:
    total_weight = 0.0
    total_score = 0.0
    for ratio, weight in items:
        if ratio is None:
            continue
        total_weight += weight
        total_score += clamp(ratio * 100.0) * weight
    if total_weight == 0:
        return 50.0
    return total_score / total_weight


def retained_items(items: list[str], output: str) -> tuple[list[str], list[str]]:
    output_lower = output.lower()
    kept: list[str] = []
    lost: list[str] = []
    for item in items:
        if item.lower() in output_lower:
            kept.append(item)
        else:
            lost.append(item)
    return kept, lost


def representative(items: list[str], limit: int = 8) -> list[str]:
    ranked = sorted(items, key=lambda item: (-len(item), item.lower()))
    return ranked[:limit]


def bullet_items(items: list[str], empty: str = "none in sampled evidence") -> str:
    if not items:
        return f"- {empty}"
    return "\n".join(f"- `{item}`" for item in items)


def has_any(text_lower: str, terms: tuple[str, ...]) -> bool:
    return any(term in text_lower for term in terms)


def readiness_score(output: str, evidence: dict[str, list[str]]) -> float:
    output_lower = output.lower()
    checks = [
        bool(evidence["paths"])
        and has_any(
            output_lower, tuple(item.lower() for item in evidence["paths"][:20])
        ),
        bool(evidence["commands"])
        and has_any(
            output_lower, tuple(item.lower() for item in evidence["commands"][:20])
        ),
        has_any(
            output_lower,
            ("next", "remaining", "todo", "rerun", "verify", "validation", "action"),
        ),
        has_any(
            output_lower,
            (
                "passed",
                "failed",
                "blocked",
                "fixed",
                "changed",
                "implemented",
                "committed",
                "status",
            ),
        ),
        has_any(
            output_lower,
            ("user asked", "goal", "must", "do not", "constraint", "assumption"),
        ),
        output.count("\n- ") + output.count("\n* ") + output.count("\n#") >= 2,
    ]
    applicable = [check for check in checks if check is not None]
    if not applicable:
        return 50.0
    return 100.0 * sum(1 for check in applicable if check) / len(applicable)


def resolve_run_path(run_dir: Path, raw: str | None) -> Path | None:
    if not raw:
        return None
    path = Path(raw)
    if path.is_absolute():
        return path
    repo_relative = REPO_ROOT / path
    if repo_relative.exists():
        return repo_relative
    return run_dir / path


def judge_by_sample_variant(
    judge_rows: list[dict[str, Any]],
) -> dict[tuple[str, str], dict[str, Any]]:
    indexed: dict[tuple[str, str], dict[str, Any]] = {}
    for row in judge_rows:
        judge = row.get("judge") or {}
        scores = judge.get("variant_scores") or {}
        reasons = judge.get("variant_reasons") or {}
        best = judge.get("best_variant")
        critical_losses = judge.get("critical_losses") or []
        for variant, score in scores.items():
            indexed[(row["sample_id"], variant)] = {
                "score": score,
                "reason": reasons.get(variant, ""),
                "best": variant == best,
                "best_variant": best,
                "critical_losses": critical_losses,
            }
    return indexed


def sample_from_row(module: Any, row: dict[str, Any]) -> Any:
    fields = module.dataclasses.fields(module.Sample)
    return module.Sample(**{field.name: row[field.name] for field in fields})


def quality_from_row(
    run_dir: Path,
    row: dict[str, Any],
    evidence: dict[str, list[str]],
    judge_index: dict[tuple[str, str], dict[str, Any]],
) -> QualityRow:
    variant = row["variant"]
    saved_percent = (1.0 - float(row["compression_ratio"])) * 100.0
    evidence_score = weighted_ratio_score(
        [
            (row.get("retained_path_ratio"), 0.30),
            (row.get("retained_command_ratio"), 0.25),
            (row.get("retained_number_ratio"), 0.15),
            (row.get("retained_constraint_ratio"), 0.30),
        ]
    )
    concision_score = clamp((saved_percent - 50.0) / 35.0 * 100.0)
    low_noise_score = clamp(100.0 - float(row.get("noise_marker_count") or 0) * 15.0)
    readiness = readiness_score(row["output"], evidence)
    judge = judge_index.get((row["sample_id"], variant), {})
    judge_score = judge.get("score")
    judge_percent = 50.0 if judge_score is None else clamp(float(judge_score) * 10.0)
    quality = (
        evidence_score * 0.35
        + readiness * 0.25
        + concision_score * 0.20
        + low_noise_score * 0.10
        + judge_percent * 0.10
    )
    run = row.get("run") or {}
    return QualityRow(
        sample_id=row["sample_id"],
        variant=variant,
        threshold_percent=int(row["threshold_percent"]),
        bucket=row["bucket"],
        prompt_tokens=int(row["prompt_tokens_estimate"]),
        output_tokens=int(row["output_tokens_estimate"]),
        saved_percent=saved_percent,
        path_retain=row.get("retained_path_ratio"),
        command_retain=row.get("retained_command_ratio"),
        number_retain=row.get("retained_number_ratio"),
        constraint_retain=row.get("retained_constraint_ratio"),
        evidence_score=evidence_score,
        readiness_score=readiness,
        concision_score=concision_score,
        low_noise_score=low_noise_score,
        judge_score=None if judge_score is None else float(judge_score),
        quality_score=quality,
        output=row["output"],
        prompt_path=resolve_run_path(run_dir, run.get("prompt_path")),
        output_path=resolve_run_path(run_dir, run.get("last_message_path")),
        raw_jsonl_path=resolve_run_path(run_dir, run.get("raw_jsonl_path")),
        stderr_path=resolve_run_path(run_dir, run.get("stderr_path")),
    )


def mean_present(values: Iterable[float | None]) -> float | None:
    present = [value for value in values if value is not None]
    if not present:
        return None
    return statistics.mean(present)


def aggregate_variant(rows: list[QualityRow]) -> dict[str, Any]:
    total_prompt = sum(row.prompt_tokens for row in rows)
    total_output = sum(row.output_tokens for row in rows)
    weighted_saved = (
        (1.0 - total_output / total_prompt) * 100.0 if total_prompt else 0.0
    )
    return {
        "outputs": len(rows),
        "weighted_saved_percent": weighted_saved,
        "avg_saved_percent": statistics.mean(row.saved_percent for row in rows),
        "avg_output_tokens": statistics.mean(row.output_tokens for row in rows),
        "avg_quality": statistics.mean(row.quality_score for row in rows),
        "weighted_quality": sum(row.quality_score * row.prompt_tokens for row in rows)
        / total_prompt,
        "avg_evidence": statistics.mean(row.evidence_score for row in rows),
        "avg_readiness": statistics.mean(row.readiness_score for row in rows),
        "avg_concision": statistics.mean(row.concision_score for row in rows),
        "avg_judge": mean_present(row.judge_score for row in rows),
        "path_retain": mean_present(row.path_retain for row in rows),
        "command_retain": mean_present(row.command_retain for row in rows),
        "number_retain": mean_present(row.number_retain for row in rows),
        "constraint_retain": mean_present(row.constraint_retain for row in rows),
    }


def threshold_aggregate(
    rows: list[QualityRow],
) -> dict[tuple[int, str], dict[str, Any]]:
    grouped: dict[tuple[int, str], list[QualityRow]] = defaultdict(list)
    for row in rows:
        grouped[(row.threshold_percent, row.variant)].append(row)
    return {key: aggregate_variant(value) for key, value in sorted(grouped.items())}


def variant_observation(variant: str) -> tuple[str, str, str]:
    if variant == "no_nudge":
        return (
            "Control path for the benchmark wrapper without an explicit reducer instruction.",
            "It can compress aggressively while losing task, path, command, and verification evidence.",
            "Useful as a floor for judging whether any reduction instruction improves continuation quality.",
        )
    if variant == "standard_compaction_template":
        return (
            "Direct baseline using the production compaction template from codex-rs/core/templates/compact/prompt.md.",
            "It is intentionally general and may preserve high-level handoff structure more than benchmark-specific evidence.",
            "Best comparison point for deciding whether custom nudges improve over the shipped checkpoint prompt.",
        )
    if variant == "prune":
        return (
            "Closest to the user's natural-language reduction prompt. It usually keeps the current goal, paths, and short next-action lists when those are prominent.",
            "It often drops exact constraints and commands when they look like incidental logs, so later implementation may lose verification details.",
            "Good as a lightweight default when savings matter and the recent context is already task-focused; weaker when exact reproducibility matters.",
        )
    if variant == "delta":
        return (
            "Strongest at deduplicating prior summary plus new delta and producing a small merged handoff.",
            "It is most likely to omit raw evidence, command details, and older constraints if the model decides they were already represented.",
            "Best for frequent reductions where a prior checkpoint is trusted; riskier as a standalone context checkpoint for another main agent.",
        )
    if variant == "evidence":
        return (
            "Best at preserving concrete paths, statuses, commands, numbers, and uncertainty markers.",
            "It spends more tokens and sometimes keeps evidence that may no longer matter after the immediate task is clear.",
            "Best continuation quality when the next agent must implement or verify from the checkpoint without rereading raw logs.",
        )
    return (
        "No curated observation is available for this run-specific variant.",
        "Review the per-sample evidence table before using it as a default.",
        "Treat it as an exploratory prompt until it has enough benchmark rows and judge coverage.",
    )


def collect_evidence_examples(
    sample_inputs: dict[str, str],
    outputs_by_sample_variant: dict[tuple[str, str], str],
    module: Any,
) -> dict[str, dict[str, dict[str, list[str]]]]:
    examples: dict[str, dict[str, dict[str, list[str]]]] = {}
    for sample_id, input_text in sample_inputs.items():
        evidence = module.extract_evidence_items(input_text)
        examples[sample_id] = {}
        for variant in VARIANTS:
            output = outputs_by_sample_variant[(sample_id, variant)]
            examples[sample_id][variant] = {}
            for category, items in evidence.items():
                kept, lost = retained_items(items, output)
                examples[sample_id][variant][f"{category}_kept"] = representative(kept)
                examples[sample_id][variant][f"{category}_lost"] = representative(lost)
    return examples


def prompt_variant_text(prompt_variants_path: Path) -> str:
    if prompt_variants_path.exists():
        return prompt_variants_path.read_text(encoding="utf-8").rstrip()
    return "(prompt_variants.md missing)"


def build_main_report(
    run_dir: Path,
    report_dir: Path,
    rows: list[QualityRow],
    sample_rows: list[dict[str, Any]],
    reductions: list[dict[str, Any]],
    judge_index: dict[tuple[str, str], dict[str, Any]],
) -> str:
    by_variant = {
        variant: aggregate_variant([row for row in rows if row.variant == variant])
        for variant in VARIANTS
    }
    by_threshold = threshold_aggregate(rows)
    lines: list[str] = []
    lines.append("# Context Helper Prompt Quality Analysis")
    lines.append("")
    lines.append(f"Source run: `{run_dir.resolve()}`.")
    lines.append("")
    lines.append(
        "Scope: 36 real reducer outputs across 12 sampled Codex context windows, three prompt variants, and 20%/30% modeled trigger thresholds."
    )
    lines.append("")
    lines.append("## Quality Scoring Method")
    lines.append("")
    lines.append(
        "The score is a deterministic 0-100 continuation-readiness heuristic. It is not a substitute for human review, but it makes the tradeoffs auditable across all outputs."
    )
    lines.append("")
    lines.append(
        markdown_table(
            ["Component", "Weight", "What it rewards"],
            [
                [
                    "Evidence retention",
                    "35%",
                    "Retained paths, commands, numbers, and explicit constraints from the reducer input.",
                ],
                [
                    "Continuation readiness",
                    "25%",
                    "Action/status/verification markers, concrete references, and structured handoff shape.",
                ],
                [
                    "Concision",
                    "20%",
                    "Token savings, with full credit near 85% savings and no credit at 50%.",
                ],
                ["Low noise", "10%", "Few speculative or conversational markers."],
                [
                    "Judge score",
                    "10%",
                    "The LLM pairwise judge score from the existing benchmark.",
                ],
            ],
        )
    )
    lines.append("")
    lines.append("## Overall Results")
    lines.append("")
    lines.append(
        markdown_table(
            [
                "Variant",
                "Quality",
                "Weighted quality",
                "Weighted tokens saved",
                "Avg output tokens",
                "Evidence",
                "Readiness",
                "Judge",
                "Path retain",
                "Command retain",
                "Constraint retain",
            ],
            [
                [
                    variant_name(variant),
                    num(by_variant[variant]["avg_quality"]),
                    num(by_variant[variant]["weighted_quality"]),
                    pct(by_variant[variant]["weighted_saved_percent"]),
                    f"{by_variant[variant]['avg_output_tokens']:.0f}",
                    num(by_variant[variant]["avg_evidence"]),
                    num(by_variant[variant]["avg_readiness"]),
                    num(by_variant[variant]["avg_judge"]),
                    num(by_variant[variant]["path_retain"]),
                    num(by_variant[variant]["command_retain"]),
                    num(by_variant[variant]["constraint_retain"]),
                ]
                for variant in VARIANTS
            ],
        )
    )
    lines.append("")
    lines.append("## Threshold Split")
    lines.append("")
    threshold_rows = []
    for threshold in sorted({row.threshold_percent for row in rows}):
        for variant in VARIANTS:
            agg = by_threshold[(threshold, variant)]
            threshold_rows.append(
                [
                    f"{threshold}%",
                    variant_name(variant),
                    num(agg["avg_quality"]),
                    pct(agg["weighted_saved_percent"]),
                    f"{agg['avg_output_tokens']:.0f}",
                    num(agg["avg_evidence"]),
                    num(agg["avg_readiness"]),
                ]
            )
    lines.append(
        markdown_table(
            [
                "Threshold",
                "Variant",
                "Quality",
                "Weighted tokens saved",
                "Avg output tokens",
                "Evidence",
                "Readiness",
            ],
            threshold_rows,
        )
    )
    lines.append("")
    lines.append("## Variant Assessment")
    lines.append("")
    for variant in VARIANTS:
        preserved, omitted, fit = variant_observation(variant)
        agg = by_variant[variant]
        wins = sum(
            1
            for sample in sample_rows
            if judge_index.get((sample["sample_id"], variant), {}).get("best")
        )
        lines.append(f"### {variant_name(variant)}")
        lines.append("")
        lines.append(f"- Average quality: {agg['avg_quality']:.1f}/100.")
        lines.append(f"- Weighted tokens saved: {agg['weighted_saved_percent']:.1f}%.")
        lines.append(f"- Judge wins: {wins}/{len(sample_rows)}.")
        lines.append(f"- Preserved: {preserved}")
        lines.append(f"- Omitted: {omitted}")
        lines.append(f"- Main-agent fit: {fit}")
        lines.append("")
    lines.append("## Per-Sample Quality")
    lines.append("")
    sample_table = []
    rows_by_sample_variant = {(row.sample_id, row.variant): row for row in rows}
    quality_headers = [f"{variant_name(variant)} quality" for variant in VARIANTS]
    for sample in sample_rows:
        sample_id = sample["sample_id"]
        sample_table.append(
            [
                sample_id,
                f"{sample['threshold_percent']}%",
                sample["bucket"],
                *[
                    quality_cell(rows_by_sample_variant, sample_id, variant)
                    for variant in VARIANTS
                ],
                sample_judge_winner(sample_id, judge_index),
                f"test-cases/{sample_id}.md",
            ]
        )
    lines.append(
        markdown_table(
            [
                "Sample",
                "Threshold",
                "Bucket",
                *quality_headers,
                "Judge winner",
                "Readable case",
            ],
            sample_table,
        )
    )
    lines.append("")
    lines.append("## Files Written")
    lines.append("")
    file_rows = [
        ["Main quality analysis", abs_path(report_dir / "quality-analysis.md")],
        *[
            [
                f"{variant_name(variant)} report",
                abs_path(report_dir / f"{variant}-quality-and-examples.md"),
            ]
            for variant in VARIANTS
        ],
        ["Test case index", abs_path(report_dir / "test-case-index.md")],
        ["Readable test cases directory", abs_path(report_dir / "test-cases")],
    ]
    lines.append(markdown_table(["Artifact", "Path"], file_rows))
    lines.append("")
    lines.append("## Prompt Variants")
    lines.append("")
    lines.append(fence(prompt_variant_text(run_dir / "prompt_variants.md"), "markdown"))
    return "\n".join(lines).rstrip() + "\n"


def build_variant_report(
    run_dir: Path,
    report_dir: Path,
    variant: str,
    rows: list[QualityRow],
    samples_by_id: dict[str, dict[str, Any]],
    sample_inputs: dict[str, str],
    examples: dict[str, dict[str, dict[str, list[str]]]],
    judge_index: dict[tuple[str, str], dict[str, Any]],
) -> str:
    variant_rows = [row for row in rows if row.variant == variant]
    agg = aggregate_variant(variant_rows)
    preserved, omitted, fit = variant_observation(variant)
    lines: list[str] = []
    lines.append(f"# {variant_name(variant)} Quality And Examples")
    lines.append("")
    lines.append(f"Source run: `{run_dir.resolve()}`.")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(
        markdown_table(
            ["Metric", "Value"],
            [
                ["Average quality", num(agg["avg_quality"])],
                ["Weighted quality", num(agg["weighted_quality"])],
                ["Weighted tokens saved", pct(agg["weighted_saved_percent"])],
                ["Average output tokens", f"{agg['avg_output_tokens']:.0f}"],
                ["Average evidence score", num(agg["avg_evidence"])],
                ["Average readiness score", num(agg["avg_readiness"])],
                ["Average judge score", num(agg["avg_judge"])],
                ["Path retain ratio", num(agg["path_retain"])],
                ["Command retain ratio", num(agg["command_retain"])],
                ["Constraint retain ratio", num(agg["constraint_retain"])],
            ],
        )
    )
    lines.append("")
    lines.append("## Qualitative Assessment")
    lines.append("")
    lines.append(f"- Preserved: {preserved}")
    lines.append(f"- Omitted: {omitted}")
    lines.append(f"- Pros: {fit}")
    if variant == "prune":
        lines.append(
            "- Cons: it depends heavily on what the model subjectively considers useful, so exact reproducibility details may disappear."
        )
    elif variant == "delta":
        lines.append(
            "- Cons: it assumes the prior reduced context is reliable and can under-preserve evidence from the new delta."
        )
    else:
        lines.append(
            "- Cons: it is larger and can keep more raw evidence than a high-frequency reduction loop may need."
        )
    lines.append("")
    lines.append("## Per-Sample Results")
    lines.append("")
    table_rows = []
    for row in variant_rows:
        judge = judge_index.get((row.sample_id, variant), {})
        table_rows.append(
            [
                row.sample_id,
                f"{row.threshold_percent}%",
                row.bucket,
                num(row.quality_score),
                pct(row.saved_percent),
                row.output_tokens,
                num(row.evidence_score),
                num(row.readiness_score),
                num(row.judge_score),
                "yes" if judge.get("best") else "no",
                f"test-cases/{row.sample_id}.md",
            ]
        )
    lines.append(
        markdown_table(
            [
                "Sample",
                "Threshold",
                "Bucket",
                "Quality",
                "Tokens saved",
                "Output tokens",
                "Evidence",
                "Readiness",
                "Judge",
                "Judge win",
                "Readable case",
            ],
            table_rows,
        )
    )
    lines.append("")
    lines.append("## Preserved And Omitted Evidence Examples")
    lines.append("")
    for row in variant_rows:
        sample = samples_by_id[row.sample_id]
        ex = examples[row.sample_id][variant]
        judge = judge_index.get((row.sample_id, variant), {})
        lines.append(f"### {row.sample_id}")
        lines.append("")
        lines.append(
            f"Threshold {sample['threshold_percent']}%, bucket `{sample['bucket']}`, quality {row.quality_score:.1f}, tokens saved {row.saved_percent:.1f}%."
        )
        reason = judge.get("reason")
        if reason:
            lines.append("")
            lines.append(f"Judge note: {reason}")
        lines.append("")
        lines.append("Preserved paths:")
        lines.append(bullet_items(ex["paths_kept"]))
        lines.append("")
        lines.append("Omitted paths:")
        lines.append(bullet_items(ex["paths_lost"]))
        lines.append("")
        lines.append("Preserved commands:")
        lines.append(bullet_items(ex["commands_kept"]))
        lines.append("")
        lines.append("Omitted commands:")
        lines.append(bullet_items(ex["commands_lost"]))
        lines.append("")
        lines.append("Preserved constraints:")
        lines.append(bullet_items(ex["constraints_kept"]))
        lines.append("")
        lines.append("Omitted constraints:")
        lines.append(bullet_items(ex["constraints_lost"]))
        lines.append("")
        lines.append(f"Full input/output: `test-cases/{row.sample_id}.md`.")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def build_test_case_index(
    run_dir: Path,
    report_dir: Path,
    sample_rows: list[dict[str, Any]],
    rows_by_sample_variant: dict[tuple[str, str], QualityRow],
    judge_index: dict[tuple[str, str], dict[str, Any]],
) -> str:
    lines = ["# Context Helper Prompt Benchmark Test Cases", ""]
    lines.append(f"Source run: `{run_dir.resolve()}`.")
    lines.append("")
    table_rows = []
    quality_headers = [f"{variant_name(variant)} quality" for variant in VARIANTS]
    for sample in sample_rows:
        sample_id = sample["sample_id"]
        table_rows.append(
            [
                sample_id,
                f"{sample['threshold_percent']}%",
                sample["bucket"],
                sample["session_label"],
                sample["trigger_line"],
                sample["context_tokens_estimate"],
                *[
                    quality_cell(rows_by_sample_variant, sample_id, variant)
                    for variant in VARIANTS
                ],
                sample_judge_winner(sample_id, judge_index),
                abs_path(report_dir / "test-cases" / f"{sample_id}.md"),
            ]
        )
    lines.append(
        markdown_table(
            [
                "Sample",
                "Threshold",
                "Bucket",
                "Session label",
                "Trigger line",
                "Context tokens",
                *quality_headers,
                "Judge winner",
                "Readable file",
            ],
            table_rows,
        )
    )
    lines.append("")
    lines.append(
        "Each readable file contains the full canonical reducer input, the full source transcript window, the exact saved reducer prompt for each variant, the full reduced output, metrics, judge notes, and raw artifact paths."
    )
    return "\n".join(lines).rstrip() + "\n"


def build_test_case(
    run_dir: Path,
    sample: dict[str, Any],
    canonical_input: str,
    rows_by_variant: dict[str, QualityRow],
    examples_by_variant: dict[str, dict[str, list[str]]],
    judge_index: dict[tuple[str, str], dict[str, Any]],
) -> str:
    sample_id = sample["sample_id"]
    lines: list[str] = []
    lines.append(f"# Test Case {sample_id}")
    lines.append("")
    lines.append("## Metadata")
    lines.append("")
    lines.append(
        markdown_table(
            ["Field", "Value"],
            [
                ["Sample ID", sample_id],
                ["Threshold", f"{sample['threshold_percent']}%"],
                ["Bucket", sample["bucket"]],
                ["Session label", sample["session_label"]],
                ["Session path", sample["session_path"]],
                ["CWD", sample["cwd"]],
                ["Trigger line", sample["trigger_line"]],
                ["Trigger index", sample["trigger_index"]],
                ["Input tokens at trigger", sample["input_tokens"]],
                ["Total input tokens at trigger", sample["total_input_tokens"]],
                ["Context window", sample["context_window"]],
                ["Context tokens estimate", sample["context_tokens_estimate"]],
            ],
        )
    )
    lines.append("")
    lines.append("## Full Canonical Reducer Input")
    lines.append("")
    lines.append(
        "This is the reducer input reconstructed from `prior_reduced_context` plus `new_context_delta`. For `delta`, the saved prompt passes those two parts in separate tags; for `prune` and `evidence`, the saved prompt wraps this canonical input in one context tag."
    )
    lines.append("")
    lines.append(fence(canonical_input, "text"))
    lines.append("")
    lines.append("## Full Source Transcript Window")
    lines.append("")
    lines.append(
        "This transcript window was used to build the sample. It was not sent directly to every reducer prompt, but it is included here so the test case remains auditable."
    )
    lines.append("")
    lines.append(fence(sample["transcript"], "text"))
    for variant in VARIANTS:
        row = rows_by_variant[variant]
        judge = judge_index.get((sample_id, variant), {})
        ex = examples_by_variant[variant]
        prompt_text = ""
        if row.prompt_path and row.prompt_path.exists():
            prompt_text = row.prompt_path.read_text(encoding="utf-8")
        else:
            prompt_text = "(prompt artifact missing)"
        lines.append("")
        lines.append(f"## Variant: {variant_name(variant)}")
        lines.append("")
        lines.append(
            markdown_table(
                ["Metric", "Value"],
                [
                    ["Quality score", num(row.quality_score)],
                    ["Tokens saved", pct(row.saved_percent)],
                    ["Prompt tokens estimate", row.prompt_tokens],
                    ["Output tokens estimate", row.output_tokens],
                    ["Evidence score", num(row.evidence_score)],
                    ["Readiness score", num(row.readiness_score)],
                    ["Concision score", num(row.concision_score)],
                    ["Low-noise score", num(row.low_noise_score)],
                    ["Judge score", num(row.judge_score)],
                    ["Judge winner", "yes" if judge.get("best") else "no"],
                ],
            )
        )
        if judge.get("reason"):
            lines.append("")
            lines.append(f"Judge note: {judge['reason']}")
        if judge.get("critical_losses"):
            lines.append("")
            lines.append("Judge critical losses:")
            lines.extend(f"- {loss}" for loss in judge["critical_losses"])
        lines.append("")
        lines.append("Representative preserved evidence:")
        for category in ("paths", "commands", "constraints", "numbers"):
            lines.append(
                f"- {category}: "
                + (", ".join(f"`{item}`" for item in ex[f"{category}_kept"]) or "none")
            )
        lines.append("")
        lines.append("Representative omitted evidence:")
        for category in ("paths", "commands", "constraints", "numbers"):
            lines.append(
                f"- {category}: "
                + (", ".join(f"`{item}`" for item in ex[f"{category}_lost"]) or "none")
            )
        lines.append("")
        lines.append("### Full Saved Reducer Prompt")
        lines.append("")
        lines.append(fence(prompt_text, "markdown"))
        lines.append("")
        lines.append("### Full Reduced Output")
        lines.append("")
        lines.append(fence(row.output, "markdown"))
        lines.append("")
        lines.append("### Raw Artifact Paths")
        lines.append("")
        lines.append(
            markdown_table(
                ["Artifact", "Path"],
                [
                    ["Prompt", abs_path(row.prompt_path)],
                    ["Last message output", abs_path(row.output_path)],
                    ["Raw events JSONL", abs_path(row.raw_jsonl_path)],
                    ["stderr", abs_path(row.stderr_path)],
                ],
            )
        )
    return "\n".join(lines).rstrip() + "\n"


def build_reports(run_dir: Path) -> list[Path]:
    global VARIANTS

    module = load_benchmark_module()
    sample_rows = read_jsonl(run_dir / "samples.jsonl")
    reduction_rows = read_jsonl(run_dir / "reductions.jsonl")
    judge_rows = read_jsonl(run_dir / "judge_scores.jsonl")
    present_variants = {
        row.get("variant")
        for row in reduction_rows
        if isinstance(row.get("variant"), str)
    }
    ordered_variants = [
        variant for variant in module.ALL_VARIANTS if variant in present_variants
    ]
    ordered_variants.extend(sorted(present_variants.difference(ordered_variants)))
    VARIANTS = tuple(ordered_variants)
    report_dir = run_dir / "reports"
    test_case_dir = report_dir / "test-cases"
    judge_index = judge_by_sample_variant(judge_rows)

    samples_by_id = {row["sample_id"]: row for row in sample_rows}
    sample_inputs: dict[str, str] = {}
    sample_objects: dict[str, Any] = {}
    for row in sample_rows:
        sample = sample_from_row(module, row)
        sample_objects[row["sample_id"]] = sample
        sample_inputs[row["sample_id"]] = module.canonical_reducer_input(sample)

    evidence_by_sample = {
        sample_id: module.extract_evidence_items(input_text)
        for sample_id, input_text in sample_inputs.items()
    }

    quality_rows = [
        quality_from_row(
            run_dir, row, evidence_by_sample[row["sample_id"]], judge_index
        )
        for row in reduction_rows
    ]
    rows_by_sample_variant = {(row.sample_id, row.variant): row for row in quality_rows}
    outputs_by_sample_variant = {
        (row.sample_id, row.variant): row.output for row in quality_rows
    }
    examples = collect_evidence_examples(
        sample_inputs, outputs_by_sample_variant, module
    )

    written: list[Path] = []
    main_report = report_dir / "quality-analysis.md"
    write_text(
        main_report,
        build_main_report(
            run_dir, report_dir, quality_rows, sample_rows, reduction_rows, judge_index
        ),
    )
    written.append(main_report)

    for variant in VARIANTS:
        path = report_dir / f"{variant}-quality-and-examples.md"
        write_text(
            path,
            build_variant_report(
                run_dir,
                report_dir,
                variant,
                quality_rows,
                samples_by_id,
                sample_inputs,
                examples,
                judge_index,
            ),
        )
        written.append(path)

    index_path = report_dir / "test-case-index.md"
    write_text(
        index_path,
        build_test_case_index(
            run_dir, report_dir, sample_rows, rows_by_sample_variant, judge_index
        ),
    )
    written.append(index_path)

    for sample in sample_rows:
        sample_id = sample["sample_id"]
        rows_by_variant = {
            variant: rows_by_sample_variant[(sample_id, variant)]
            for variant in VARIANTS
        }
        path = test_case_dir / f"{sample_id}.md"
        write_text(
            path,
            build_test_case(
                run_dir,
                sample,
                sample_inputs[sample_id],
                rows_by_variant,
                examples[sample_id],
                judge_index,
            ),
        )
        written.append(path)

    summary_path = report_dir / "quality-report-generation-summary.json"
    written.append(summary_path)
    summary = {
        "run_dir": str(run_dir.resolve()),
        "reports": [str(path.resolve()) for path in written],
        "sample_count": len(sample_rows),
        "reduction_count": len(reduction_rows),
        "variants": list(VARIANTS),
    }
    summary_path.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    return written


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--run-dir",
        type=Path,
        default=DEFAULT_RUN_DIR,
        help="Benchmark run directory containing samples.jsonl, reductions.jsonl, and judge_scores.jsonl.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    run_dir = args.run_dir.resolve()
    written = build_reports(run_dir)
    print(f"Wrote {len(written)} report artifacts:")
    for path in written:
        print(path)


if __name__ == "__main__":
    main()
