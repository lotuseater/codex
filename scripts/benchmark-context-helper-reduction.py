#!/usr/bin/env python3
"""Benchmark sidecar helper policies against local Codex session logs.

The benchmark is intentionally read-only with respect to Codex session data. It
models a helper agent that receives bounded artifacts, returns a compact summary,
and reduces the root prompt for later turns. It does not model a full-context
agent fork, because that defeats the purpose of context reduction.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import statistics
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


AGENT_TOOL_NAMES = {
    "spawn_agent",
    "followup_task",
    "send_message",
    "wait_agent",
    "compact_agent",
    "resume_agent",
    "restart_agent",
    "close_agent",
    "list_agents",
}


@dataclass(frozen=True)
class TokenEvent:
    index: int
    line: int
    timestamp: str
    input_tokens: int
    total_input_tokens: int
    context_window: int | None


@dataclass
class SessionSummary:
    path: Path
    session_id: str
    cwd: str
    started_at: str
    updated_at: str
    byte_len: int
    token_events: list[TokenEvent]
    agent_calls: Counter[str]

    @property
    def token_event_count(self) -> int:
        return len(self.token_events)

    @property
    def context_window(self) -> int | None:
        for event in self.token_events:
            if event.context_window:
                return event.context_window
        return None

    @property
    def max_input_tokens(self) -> int:
        if not self.token_events:
            return 0
        return max(event.input_tokens for event in self.token_events)

    @property
    def final_input_tokens(self) -> int:
        if not self.token_events:
            return 0
        return self.token_events[-1].input_tokens

    @property
    def median_input_tokens(self) -> int:
        if not self.token_events:
            return 0
        return int(statistics.median(event.input_tokens for event in self.token_events))

    @property
    def final_total_input_tokens(self) -> int:
        if not self.token_events:
            return 0
        return self.token_events[-1].total_input_tokens


@dataclass(frozen=True)
class Policy:
    threshold_percent: int
    cooldown_turns: int


@dataclass(frozen=True)
class HelperCostModel:
    name: str
    bundle_tokens: int
    parent_overhead_tokens: int
    summary_tokens: int
    state_growth_tokens: int
    compact_output_tokens: int
    compact_every: int | None


@dataclass
class PolicyResult:
    threshold_percent: int
    cooldown_turns: int
    helper_strategy: str
    summary_tokens: int
    helper_bundle_tokens: int
    parent_overhead_tokens: int
    triggers: int
    helper_compactions: int
    sessions_triggered: int
    gross_saved_tokens: int
    helper_cost_tokens: int
    net_saved_tokens: int
    max_helper_state_tokens: int
    net_saved_per_trigger: int


def parse_int_list(value: str) -> list[int]:
    values = []
    for raw in value.split(","):
        raw = raw.strip()
        if raw:
            values.append(int(raw))
    if not values:
        raise argparse.ArgumentTypeError("expected at least one integer")
    return values


def default_sessions_root() -> Path:
    codex_home = os.environ.get("CODEX_HOME")
    if codex_home:
        return Path(codex_home) / "sessions"
    return Path.home() / ".codex" / "sessions"


def json_default(value: Any) -> Any:
    if isinstance(value, Path):
        return str(value)
    if isinstance(value, Counter):
        return dict(value)
    if hasattr(value, "__dict__"):
        return value.__dict__
    raise TypeError(f"cannot serialize {type(value).__name__}")


def iter_session_files(root: Path, limit: int) -> list[Path]:
    files = [path for path in root.rglob("*.jsonl") if path.is_file()]
    files.sort(key=lambda path: path.stat().st_mtime, reverse=True)
    return files[:limit]


def parse_session(path: Path) -> SessionSummary:
    token_events: list[TokenEvent] = []
    agent_calls: Counter[str] = Counter()
    session_id = path.stem
    cwd = ""
    started_at = ""

    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line_number, line in enumerate(handle, 1):
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue

            payload = record.get("payload")
            if record.get("type") == "session_meta" and isinstance(payload, dict):
                session_id = str(payload.get("id") or session_id)
                cwd = str(payload.get("cwd") or cwd)
                started_at = str(payload.get("timestamp") or record.get("timestamp") or started_at)
                continue

            if not isinstance(payload, dict):
                continue

            if payload.get("type") == "function_call":
                name = payload.get("name")
                if name in AGENT_TOOL_NAMES:
                    agent_calls[str(name)] += 1
                continue

            if payload.get("type") != "token_count":
                continue

            info = payload.get("info")
            if not isinstance(info, dict):
                continue

            last_usage = info.get("last_token_usage")
            total_usage = info.get("total_token_usage")
            if not isinstance(last_usage, dict) or not isinstance(total_usage, dict):
                continue

            input_tokens = int(last_usage.get("input_tokens") or 0)
            total_input_tokens = int(total_usage.get("input_tokens") or 0)
            context_window = info.get("model_context_window")
            if context_window is not None:
                context_window = int(context_window)

            token_events.append(
                TokenEvent(
                    index=len(token_events),
                    line=line_number,
                    timestamp=str(record.get("timestamp") or ""),
                    input_tokens=input_tokens,
                    total_input_tokens=total_input_tokens,
                    context_window=context_window,
                )
            )

    stat = path.stat()
    updated_at = datetime.fromtimestamp(stat.st_mtime, timezone.utc).isoformat()
    return SessionSummary(
        path=path,
        session_id=session_id,
        cwd=cwd,
        started_at=started_at,
        updated_at=updated_at,
        byte_len=stat.st_size,
        token_events=token_events,
        agent_calls=agent_calls,
    )


def threshold_token_count(event: TokenEvent, threshold_percent: int) -> int | None:
    if event.context_window is None:
        return None
    return math.ceil(event.context_window * threshold_percent / 100)


def find_trigger_indices(
    events: list[TokenEvent],
    threshold_percent: int,
    cooldown_turns: int,
    new_input_cooldown_tokens: int,
    min_future_turns: int,
) -> list[int]:
    trigger_indices: list[int] = []
    last_trigger_index: int | None = None
    last_trigger_total_input = 0

    for index, event in enumerate(events):
        threshold_tokens = threshold_token_count(event, threshold_percent)
        if threshold_tokens is None or event.input_tokens < threshold_tokens:
            continue

        if len(events) - index - 1 < min_future_turns:
            continue

        if last_trigger_index is not None:
            turns_since = index - last_trigger_index
            new_input = max(0, event.total_input_tokens - last_trigger_total_input)
            bypass_enabled = new_input_cooldown_tokens > 0
            bypass_satisfied = bypass_enabled and new_input >= new_input_cooldown_tokens
            if turns_since < cooldown_turns and not bypass_satisfied:
                continue

        trigger_indices.append(index)
        last_trigger_index = index
        last_trigger_total_input = event.total_input_tokens

    return trigger_indices


def calculate_gross_saved(
    events: list[TokenEvent],
    trigger_indices: list[int],
    summary_tokens: int,
    cooldown_turns: int,
) -> int:
    if not trigger_indices:
        return 0

    trigger_set = set(trigger_indices)
    gross_saved = 0
    for position, trigger_index in enumerate(trigger_indices):
        next_trigger = trigger_indices[position + 1] if position + 1 < len(trigger_indices) else len(events)
        end_index = min(next_trigger, trigger_index + cooldown_turns + 1, len(events))
        for future_index in range(trigger_index + 1, end_index):
            if future_index in trigger_set:
                break
            gross_saved += max(0, events[future_index].input_tokens - summary_tokens)
    return gross_saved


def calculate_helper_cost(trigger_count: int, model: HelperCostModel) -> tuple[int, int, int]:
    if trigger_count == 0:
        return 0, 0, 0

    if model.compact_every is None and model.name == "one_shot":
        per_trigger = model.bundle_tokens + model.summary_tokens + model.parent_overhead_tokens
        return per_trigger * trigger_count, 0, 0

    helper_state = 0
    max_helper_state = 0
    compactions = 0
    turns_since_compact = 0
    total_cost = 0

    for trigger_number in range(trigger_count):
        total_cost += (
            model.parent_overhead_tokens
            + model.bundle_tokens
            + helper_state
            + model.summary_tokens
        )
        helper_state += model.state_growth_tokens
        max_helper_state = max(max_helper_state, helper_state)
        turns_since_compact += 1

        is_last_trigger = trigger_number == trigger_count - 1
        if (
            model.compact_every is not None
            and turns_since_compact >= model.compact_every
            and not is_last_trigger
        ):
            total_cost += helper_state + model.compact_output_tokens
            compactions += 1
            helper_state = model.compact_output_tokens
            turns_since_compact = 0

    return total_cost, compactions, max_helper_state


def evaluate_policy(
    sessions: list[SessionSummary],
    policy: Policy,
    helper_model: HelperCostModel,
    new_input_cooldown_tokens: int,
    min_future_turns: int,
) -> PolicyResult:
    trigger_count = 0
    sessions_triggered = 0
    gross_saved = 0

    for session in sessions:
        trigger_indices = find_trigger_indices(
            session.token_events,
            policy.threshold_percent,
            policy.cooldown_turns,
            new_input_cooldown_tokens,
            min_future_turns,
        )
        if trigger_indices:
            sessions_triggered += 1
        trigger_count += len(trigger_indices)
        gross_saved += calculate_gross_saved(
            session.token_events,
            trigger_indices,
            helper_model.summary_tokens,
            policy.cooldown_turns,
        )

    helper_cost, helper_compactions, max_helper_state = calculate_helper_cost(
        trigger_count,
        helper_model,
    )
    net_saved = gross_saved - helper_cost
    per_trigger = int(net_saved / trigger_count) if trigger_count else 0

    return PolicyResult(
        threshold_percent=policy.threshold_percent,
        cooldown_turns=policy.cooldown_turns,
        helper_strategy=helper_model.name,
        summary_tokens=helper_model.summary_tokens,
        helper_bundle_tokens=helper_model.bundle_tokens,
        parent_overhead_tokens=helper_model.parent_overhead_tokens,
        triggers=trigger_count,
        helper_compactions=helper_compactions,
        sessions_triggered=sessions_triggered,
        gross_saved_tokens=gross_saved,
        helper_cost_tokens=helper_cost,
        net_saved_tokens=net_saved,
        max_helper_state_tokens=max_helper_state,
        net_saved_per_trigger=per_trigger,
    )


def make_one_shot_model(bundle_tokens: int, parent_overhead: int, summary_tokens: int) -> HelperCostModel:
    return HelperCostModel(
        name="one_shot",
        bundle_tokens=bundle_tokens,
        parent_overhead_tokens=parent_overhead,
        summary_tokens=summary_tokens,
        state_growth_tokens=0,
        compact_output_tokens=0,
        compact_every=None,
    )


def make_persistent_model(
    bundle_tokens: int,
    parent_overhead: int,
    summary_tokens: int,
    compact_every: int | None,
) -> HelperCostModel:
    name = "persistent_no_compact" if compact_every is None else f"persistent_compact_every_{compact_every}"
    return HelperCostModel(
        name=name,
        bundle_tokens=bundle_tokens,
        parent_overhead_tokens=parent_overhead,
        summary_tokens=summary_tokens,
        state_growth_tokens=summary_tokens + parent_overhead,
        compact_output_tokens=summary_tokens,
        compact_every=compact_every,
    )


def pct_count(sessions: list[SessionSummary], percent: int) -> int:
    count = 0
    for session in sessions:
        window = session.context_window
        if not window:
            continue
        if session.max_input_tokens >= math.ceil(window * percent / 100):
            count += 1
    return count


def format_tokens(value: int) -> str:
    sign = "-" if value < 0 else ""
    value = abs(value)
    if value >= 1_000_000:
        return f"{sign}{value / 1_000_000:.1f}M"
    if value >= 1_000:
        return f"{sign}{value / 1_000:.1f}k"
    return f"{sign}{value}"


def format_int(value: int) -> str:
    return f"{value:,}"


def format_pct(value: float) -> str:
    return f"{value:.1f}%"


def markdown_table(headers: list[str], rows: list[list[str]]) -> str:
    lines = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
    ]
    lines.extend("| " + " | ".join(row) + " |" for row in rows)
    return "\n".join(lines)


def result_to_row(result: PolicyResult, session_count: int, token_event_count: int) -> list[str]:
    trigger_rate = result.triggers / token_event_count * 1000 if token_event_count else 0
    session_rate = result.sessions_triggered / session_count * 100 if session_count else 0
    return [
        f"{result.threshold_percent}%",
        str(result.cooldown_turns),
        result.helper_strategy,
        format_int(result.triggers),
        format_pct(session_rate),
        f"{trigger_rate:.1f}",
        format_tokens(result.gross_saved_tokens),
        format_tokens(result.helper_cost_tokens),
        format_tokens(result.net_saved_tokens),
        format_tokens(result.net_saved_per_trigger),
    ]


def summarize_sessions(sessions: list[SessionSummary]) -> dict[str, Any]:
    token_sessions = [session for session in sessions if session.token_events]
    windows = Counter(session.context_window for session in token_sessions if session.context_window)
    agent_call_sessions = sum(1 for session in sessions if session.agent_calls)
    total_agent_calls = Counter()
    for session in sessions:
        total_agent_calls.update(session.agent_calls)

    return {
        "session_count": len(sessions),
        "token_session_count": len(token_sessions),
        "token_event_count": sum(session.token_event_count for session in token_sessions),
        "context_windows": dict(windows),
        "sessions_over_80k": sum(1 for session in token_sessions if session.max_input_tokens >= 80_000),
        "sessions_over_55pct": pct_count(token_sessions, 55),
        "sessions_over_60pct": pct_count(token_sessions, 60),
        "sessions_over_65pct": pct_count(token_sessions, 65),
        "sessions_over_70pct": pct_count(token_sessions, 70),
        "sessions_over_75pct": pct_count(token_sessions, 75),
        "agent_call_sessions": agent_call_sessions,
        "agent_calls": dict(total_agent_calls),
    }


def top_sessions(sessions: list[SessionSummary], limit: int = 8) -> list[SessionSummary]:
    return sorted(sessions, key=SessionSummary.max_input_tokens.fget, reverse=True)[:limit]


def session_snapshot(session: SessionSummary) -> dict[str, Any]:
    return {
        "path": str(session.path),
        "session_id": session.session_id,
        "project": project_label(session),
        "cwd": session.cwd,
        "started_at": session.started_at,
        "updated_at": session.updated_at,
        "bytes": session.byte_len,
        "token_events": session.token_event_count,
        "context_window": session.context_window,
        "max_input_tokens": session.max_input_tokens,
        "median_input_tokens": session.median_input_tokens,
        "final_input_tokens": session.final_input_tokens,
        "final_total_input_tokens": session.final_total_input_tokens,
        "agent_calls": dict(session.agent_calls),
    }


def project_label(session: SessionSummary) -> str:
    if not session.cwd:
        return ""
    return Path(session.cwd).name


def short_session_label(session: SessionSummary) -> str:
    if session.path.name.startswith("rollout-"):
        return session.path.name.removesuffix(".jsonl").replace("rollout-", "")
    return session.path.stem


def render_markdown(
    *,
    sessions_root: Path,
    session_limit: int,
    generated_at: datetime,
    sessions: list[SessionSummary],
    core_results: list[PolicyResult],
    sensitivity_results: list[PolicyResult],
    helper_strategy_results: list[PolicyResult],
    cadence_results: list[tuple[int, PolicyResult]],
    summary: dict[str, Any],
    baseline_summary_tokens: int,
    baseline_helper_cost: int,
    baseline_parent_overhead: int,
    new_input_cooldown_tokens: int,
    min_future_turns: int,
) -> str:
    session_count = int(summary["session_count"])
    token_event_count = int(summary["token_event_count"])
    token_session_count = int(summary["token_session_count"])

    top_rows = []
    for session in top_sessions([item for item in sessions if item.token_events]):
        window = session.context_window or 0
        max_pct = session.max_input_tokens / window * 100 if window else 0
        agent_calls = sum(session.agent_calls.values())
        top_rows.append(
            [
                short_session_label(session),
                project_label(session),
                format_int(session.token_event_count),
                format_tokens(session.max_input_tokens),
                format_pct(max_pct),
                format_tokens(session.final_input_tokens),
                format_int(agent_calls),
            ]
        )

    core_rows = [
        result_to_row(result, session_count, token_event_count)
        for result in sorted(core_results, key=lambda item: (item.threshold_percent, item.cooldown_turns))
    ]

    sensitivity_rows = [
        [
            f"{result.threshold_percent}%",
            str(result.cooldown_turns),
            format_tokens(result.helper_bundle_tokens),
            format_tokens(result.summary_tokens),
            format_int(result.triggers),
            format_tokens(result.helper_cost_tokens),
            format_tokens(result.net_saved_tokens),
        ]
        for result in sorted(
            sensitivity_results,
            key=lambda item: (item.helper_bundle_tokens, item.summary_tokens),
        )
    ]

    helper_rows = [
        [
            result.helper_strategy,
            format_int(result.triggers),
            format_int(result.helper_compactions),
            format_tokens(result.max_helper_state_tokens),
            format_tokens(result.helper_cost_tokens),
            format_tokens(result.net_saved_tokens),
        ]
        for result in helper_strategy_results
    ]

    cadence_rows = [
        [
            "turn-only" if cooldown == 0 else format_tokens(cooldown),
            format_int(result.triggers),
            f"{result.triggers / token_event_count * 1000:.1f}" if token_event_count else "0.0",
            format_tokens(result.helper_cost_tokens),
            format_tokens(result.net_saved_tokens),
        ]
        for cooldown, result in cadence_results
    ]

    agent_calls = summary["agent_calls"]
    if agent_calls:
        agent_call_text = ", ".join(
            f"`{name}` {count}" for name, count in sorted(agent_calls.items(), key=lambda item: item[0])
        )
    else:
        agent_call_text = "none"

    windows = summary["context_windows"]
    window_text = ", ".join(f"{window}: {count}" for window, count in sorted(windows.items())) or "none"

    recommendation = (
        "Use a bounded sidecar helper at 65% context pressure with a 24-turn cooldown, "
        f"only when at least "
        f"{min_future_turns} more turns are expected. For automatic loop continuations where the "
        "agent is clearly going to keep working, allow an aggressive lane at 55%-60% with a "
        "12-turn cooldown. Keep the turn cooldown as the default gate; do not bypass it with "
        "accumulated total prompt input. Do not spawn the reducer with full root context."
    )

    helper_recommendation = (
        "Prefer one-shot helpers with `fork_turns: \"none\"` for the reducer. If a persistent helper "
        "is kept warm for several reductions, compact it after every 2 reductions or when its retained "
        "state approaches roughly 30k-40k tokens; the helper state is a real model input cost even "
        "when it is not retained by the root agent."
    )

    return f"""# Context Helper Reduction Benchmark - {generated_at.date().isoformat()}

Generated: {generated_at.isoformat(timespec="seconds")}

Session root: `{sessions_root}`

Session sample: {session_count} newest JSONL files by mtime, capped by `--limit {session_limit}`.

## Summary

- Parsed {token_session_count} sessions with `token_count` events and {format_int(token_event_count)} token-count samples.
- Context windows observed: {window_text}.
- Sessions crossing thresholds: 80k tokens = {summary["sessions_over_80k"]}; 55% = {summary["sessions_over_55pct"]}; 60% = {summary["sessions_over_60pct"]}; 65% = {summary["sessions_over_65pct"]}; 70% = {summary["sessions_over_70pct"]}; 75% = {summary["sessions_over_75pct"]}.
- Sessions with multi-agent tool calls: {summary["agent_call_sessions"]}; calls: {agent_call_text}.
- Baseline helper model: {format_tokens(baseline_helper_cost)} helper input bundle, {format_tokens(baseline_parent_overhead)} parent retained overhead, {format_tokens(baseline_summary_tokens)} helper response/root summary.

## Recommendation

{recommendation}

{helper_recommendation}

This is a sidecar-reducer recommendation, not a recommendation to fork the full root transcript. A full-context helper duplicates the expensive prompt and usually loses before it can save anything.

## Model

For each session, a policy triggers when `last_token_usage.input_tokens >= model_context_window * threshold_percent`. A trigger is suppressed unless at least {min_future_turns} later token-count samples exist in the same session. After a trigger, later root turns in the cooldown window are charged as if the prompt were reduced to the retained summary size:

`gross_saved += max(0, future_input_tokens - summary_tokens)`

`helper_cost = helper_input_bundle + prior_helper_state + helper_summary_output + parent_coordination_tokens`

`net_saved = gross_saved - helper_cost`

Cooldown is a hard turn count in the default model. The separate bypass stress table shows why accumulated root input tokens are the wrong bypass signal: repeated high-context prompts quickly satisfy a small token threshold even when no genuinely new context has appeared.

## Current Production Anchor

- Existing model metadata auto-compaction is anchored near 90% of resolved context.
- Slow context-budget mode clamps that to 75% of the model context window.
- Semantic checkpoint pressure is effectively earlier: 80% of the slow-mode limit, or about 60% of the model context window.

The benchmark therefore treats 60%-65% as the practical early-reduction band and 75% as a late safety net.

## Highest-Pressure Sessions

{markdown_table(["session", "project", "token samples", "max input", "max window", "final input", "agent calls"], top_rows)}

## Threshold And Cooldown Sweep

Baseline cost model: one-shot helper, {format_tokens(baseline_helper_cost)} helper input bundle, {format_tokens(baseline_parent_overhead)} parent overhead, {format_tokens(baseline_summary_tokens)} helper response/root summary.

{markdown_table(["threshold", "cooldown", "strategy", "triggers", "sessions", "triggers/1k samples", "gross saved", "helper cost", "net saved", "net/trigger"], core_rows)}

## Helper Cost Sensitivity

Sensitivity keeps the recommended 65% / 24-turn policy and varies helper bundle cost and retained summary size.

{markdown_table(["threshold", "cooldown", "helper bundle", "summary", "triggers", "helper cost", "net saved"], sensitivity_rows)}

## New-Input Bypass Stress

This keeps the recommended 65% / 24-turn policy and varies only the accumulated-total-input bypass. `turn-only` disables the bypass. Small bypass values are noisy because a single high-context model call can add more than the bypass threshold.

{markdown_table(["bypass", "triggers", "triggers/1k samples", "helper cost", "net saved"], cadence_rows)}

## Helper State And Compaction

This table keeps the recommended 65% / 24-turn policy, {format_tokens(baseline_helper_cost)} helper input bundle, and {format_tokens(baseline_summary_tokens)} helper response/root summary. Persistent helper state grows by `summary_tokens + parent_overhead_tokens` after each reduction. A helper compaction costs the current helper state plus the compact output, then resets helper state to the compact output.

{markdown_table(["helper strategy", "triggers", "helper compactions", "max helper state", "helper cost", "net saved"], helper_rows)}

## Public API And Behavior Changes

None. This benchmark and report do not change production compaction behavior.

## Acceptance Gates For Production Promotion

- Keep reducer helpers sidecar-only: `fork_turns: \"none\"` or a bounded explicit artifact bundle.
- Require positive net savings under a 60k helper-bundle sensitivity case.
- Keep retained root summaries at or below 10k tokens for default operation.
- Avoid helper spawning for short sessions below 80k observed input tokens unless loop mode indicates at least {min_future_turns} future turns.
- Do not use accumulated total prompt input as a cooldown bypass; require measured new source/context material if a token-based bypass is later added.
- Do not keep a persistent reducer helper un-compacted beyond 2 reductions or roughly 30k-40k retained helper state.
"""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sessions-root", type=Path, default=default_sessions_root())
    parser.add_argument("--limit", type=int, default=80)
    parser.add_argument("--thresholds", type=parse_int_list, default=parse_int_list("55,60,65,70,75"))
    parser.add_argument("--cooldowns", type=parse_int_list, default=parse_int_list("6,12,24,48"))
    parser.add_argument("--helper-costs", type=parse_int_list, default=parse_int_list("12000,30000,60000"))
    parser.add_argument("--summary-sizes", type=parse_int_list, default=parse_int_list("6000,8000,10000,20000"))
    parser.add_argument("--baseline-threshold", type=int, default=65)
    parser.add_argument("--baseline-cooldown", type=int, default=24)
    parser.add_argument("--baseline-helper-cost", type=int, default=12_000)
    parser.add_argument("--baseline-summary-size", type=int, default=8_000)
    parser.add_argument("--parent-overhead", type=int, default=2_500)
    parser.add_argument("--new-input-cooldown", type=int, default=0)
    parser.add_argument(
        "--cadence-new-input-cooldowns",
        type=parse_int_list,
        default=parse_int_list("0,32000,80000,160000"),
    )
    parser.add_argument("--min-future-turns", type=int, default=6)
    parser.add_argument("--out-json", type=Path)
    parser.add_argument("--out-markdown", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    sessions_root = args.sessions_root.expanduser().resolve()
    files = iter_session_files(sessions_root, args.limit)
    sessions = [parse_session(path) for path in files]
    token_sessions = [session for session in sessions if session.token_events]

    baseline_model = make_one_shot_model(
        args.baseline_helper_cost,
        args.parent_overhead,
        args.baseline_summary_size,
    )

    core_results = []
    for threshold in args.thresholds:
        for cooldown in args.cooldowns:
            core_results.append(
                evaluate_policy(
                    token_sessions,
                    Policy(threshold, cooldown),
                    baseline_model,
                    args.new_input_cooldown,
                    args.min_future_turns,
                )
            )

    sensitivity_results = []
    for helper_cost in args.helper_costs:
        for summary_size in args.summary_sizes:
            sensitivity_results.append(
                evaluate_policy(
                    token_sessions,
                    Policy(args.baseline_threshold, args.baseline_cooldown),
                    make_one_shot_model(helper_cost, args.parent_overhead, summary_size),
                    args.new_input_cooldown,
                    args.min_future_turns,
                )
            )

    helper_strategy_models = [
        make_one_shot_model(args.baseline_helper_cost, args.parent_overhead, args.baseline_summary_size),
        make_persistent_model(args.baseline_helper_cost, args.parent_overhead, args.baseline_summary_size, 1),
        make_persistent_model(args.baseline_helper_cost, args.parent_overhead, args.baseline_summary_size, 2),
        make_persistent_model(args.baseline_helper_cost, args.parent_overhead, args.baseline_summary_size, 3),
        make_persistent_model(args.baseline_helper_cost, args.parent_overhead, args.baseline_summary_size, None),
    ]
    helper_strategy_results = [
        evaluate_policy(
            token_sessions,
            Policy(args.baseline_threshold, args.baseline_cooldown),
            model,
            args.new_input_cooldown,
            args.min_future_turns,
        )
        for model in helper_strategy_models
    ]

    cadence_results = [
        (
            cooldown,
            evaluate_policy(
                token_sessions,
                Policy(args.baseline_threshold, args.baseline_cooldown),
                baseline_model,
                cooldown,
                args.min_future_turns,
            ),
        )
        for cooldown in args.cadence_new_input_cooldowns
    ]

    generated_at = datetime.now().astimezone()
    summary = summarize_sessions(sessions)
    recommended = evaluate_policy(
        token_sessions,
        Policy(args.baseline_threshold, args.baseline_cooldown),
        baseline_model,
        args.new_input_cooldown,
        args.min_future_turns,
    )
    output = {
        "generated_at": generated_at.isoformat(),
        "sessions_root": str(sessions_root),
        "session_limit": args.limit,
        "settings": {
            "thresholds": args.thresholds,
            "cooldowns": args.cooldowns,
            "helper_costs": args.helper_costs,
            "summary_sizes": args.summary_sizes,
            "baseline_threshold": args.baseline_threshold,
            "baseline_cooldown": args.baseline_cooldown,
            "baseline_helper_cost": args.baseline_helper_cost,
            "baseline_summary_size": args.baseline_summary_size,
            "parent_overhead": args.parent_overhead,
            "new_input_cooldown": args.new_input_cooldown,
            "min_future_turns": args.min_future_turns,
        },
        "summary": summary,
        "recommended": json_default(recommended),
        "core_results": core_results,
        "sensitivity_results": sensitivity_results,
        "helper_strategy_results": helper_strategy_results,
        "cadence_results": cadence_results,
        "top_sessions": [
            session_snapshot(session)
            for session in top_sessions([session for session in sessions if session.token_events])
        ],
    }

    if args.out_json:
        args.out_json.parent.mkdir(parents=True, exist_ok=True)
        args.out_json.write_text(json.dumps(output, indent=2, default=json_default) + "\n", encoding="utf-8")

    if args.out_markdown:
        markdown = render_markdown(
            sessions_root=sessions_root,
            session_limit=args.limit,
            generated_at=generated_at,
            sessions=sessions,
            core_results=core_results,
            sensitivity_results=sensitivity_results,
            helper_strategy_results=helper_strategy_results,
            cadence_results=cadence_results,
            summary=summary,
            baseline_summary_tokens=args.baseline_summary_size,
            baseline_helper_cost=args.baseline_helper_cost,
            baseline_parent_overhead=args.parent_overhead,
            new_input_cooldown_tokens=args.new_input_cooldown,
            min_future_turns=args.min_future_turns,
        )
        args.out_markdown.parent.mkdir(parents=True, exist_ok=True)
        args.out_markdown.write_text(markdown, encoding="utf-8")

    print(
        json.dumps(
            {
                "sessions": summary["session_count"],
                "token_sessions": summary["token_session_count"],
                "token_events": summary["token_event_count"],
                "recommended": json_default(recommended),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
