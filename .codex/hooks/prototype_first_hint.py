#!/usr/bin/env python3
"""Sparse prototype-first advisory hook for Codex.

The hook is intentionally non-blocking. It emits model-visible context only
when prompt/tool/repetition evidence says a small script, canary, or lab
prototype would probably shorten the implementation loop.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
import time
from pathlib import Path
from typing import Any


DEFAULT_THRESHOLD = 5
REPEAT_THRESHOLD = 3
FAILURE_THRESHOLD = 2
MAX_STATE_FILES = 80

DEFAULT_CACHE_DIR = Path(os.environ.get("USERPROFILE", str(Path.home()))) / ".codex" / "cache" / "prototype-first-hook"

STRONG_GROUPS: list[tuple[str, int, tuple[str, ...], str]] = [
    (
        "tool-runtime",
        5,
        (
            "dab",
            "desktop automation",
            "hook",
            "hooks",
            "skill",
            "skills",
            "mcp",
            "first_moves",
            "first-moves",
            "repo-context-scout",
            "context scout",
            "cache",
            "shadow",
            "reducer",
            "multiagent",
            "multi-agent",
            "agent policy",
            "supervision",
        ),
        "make a tiny lab canary or fixture before changing the main tool path",
    ),
    (
        "gui-live",
        5,
        (
            "gui",
            "window",
            "screenshot",
            "ocr",
            "visible terminal",
            "visible codex",
            "desktop",
            "live app",
            "smoke test",
            "canary",
            "march 8",
            "8 march",
        ),
        "prove the visible/live path with a small canary first",
    ),
    (
        "expensive-verification",
        5,
        (
            "build-local-codex",
            "fastrelease",
            "lowmemrelease",
            "release build",
            "build and deploy",
            "compile",
            "cargo test",
            "cmake",
            "ninja",
            "wrapper",
            "deploy system-wide",
        ),
        "avoid repeated expensive builds by adding a focused fixture or smoke lane first",
    ),
    (
        "systematic-change",
        5,
        (
            "systematic",
            "conversion",
            "convert all",
            "many files",
            "refactor",
            "migration",
            "mechanical",
            "replace everywhere",
        ),
        "script the repeated transformation or verification before editing broadly",
    ),
    (
        "repeated-work",
        5,
        (
            "again and again",
            "same check",
            "same command",
            "same task",
            "manual comparison",
            "manually comparing",
            "compare outputs",
            "current output vs alternative output",
            "fixture runner",
            "rerun tests",
            "rerunning tests",
            "repeated task",
            "repeated tasks",
        ),
        "promote the repeated loop into a script, fixture runner, or durable tool",
    ),
    (
        "token-saving",
        5,
        (
            "token",
            "tokens",
            "exploration agent",
            "explorer agent",
            "first_moves",
            "first-moves",
            "cache hit",
            "output truncation",
            "context reducer",
            "context-reducer",
            "save tokens",
        ),
        "use first_moves/cache/lab comparison before broad exploration or agent spawning",
    ),
]

WATCH_PATHS = (
    "codex-rs/desktop-automation",
    "codex-rs\\desktop-automation",
    "codex-rs/first-moves",
    "codex-rs\\first-moves",
    "codex-rs/repo-context-scout",
    "codex-rs\\repo-context-scout",
    "codex-rs/tools",
    "codex-rs\\tools",
    "codex-rs/core/src/tools",
    "codex-rs\\core\\src\\tools",
    ".codex/skills",
    ".codex\\skills",
    ".codex/hooks",
    ".codex\\hooks",
    "scripts/",
    "scripts\\",
)

EXACT_ONE_OFF_PATTERNS = (
    "run exactly this read-only shell command once",
    "do not run extra exploration",
    "git diff --stat",
    "git status",
    "git log",
    "git show",
)

PROTOTYPE_ALREADY_PRESENT_TERMS = (
    "already prototyped",
    "existing prototype",
    "prototype already exists",
    "canary already exists",
    "existing canary",
    "fixture already exists",
    "existing fixture",
    "lab canary passed",
    "context-reducer-lab canary passed",
)

ACTION_TERMS = (
    "add",
    "automate",
    "build",
    "change",
    "create",
    "debug",
    "deploy",
    "fix",
    "implement",
    "improve",
    "refactor",
    "test",
    "write",
)


class Decision:
    def __init__(self, event_name: str, trigger_class: str, score: int, reason: str, action: str) -> None:
        self.event_name = event_name
        self.trigger_class = trigger_class
        self.score = score
        self.reason = reason
        self.action = action

    def context(self) -> str:
        return (
            "Prototype-first automation hint: "
            f"{self.reason}. Suggested next action: {self.action}. "
            "Skip this hint if the task is truly one-off or a suitable canary already exists."
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        run_self_tests()
        return 0

    try:
        payload = json.load(sys.stdin)
        decision = evaluate_payload(payload)
        if decision is None:
            return 0
        print(
            json.dumps(
                {
                    "continue": True,
                    "suppressOutput": True,
                    "hookSpecificOutput": {
                        "hookEventName": decision.event_name,
                        "additionalContext": decision.context(),
                    },
                },
                separators=(",", ":"),
            )
        )
        return 0
    except Exception:
        # Hooks are advisory. Any hook defect must fail open and stay quiet.
        return 0


def evaluate_payload(payload: dict[str, Any], *, now: float | None = None) -> Decision | None:
    event_name = str(payload.get("hook_event_name") or payload.get("hookEventName") or "")
    if event_name == "UserPromptSubmit":
        return dedupe(payload, score_prompt(payload), now=now)
    if event_name == "PreToolUse":
        return dedupe(payload, score_pre_tool(payload), now=now)
    if event_name == "PostToolUse":
        return dedupe(payload, score_post_tool(payload), now=now)
    return None


def score_prompt(payload: dict[str, Any]) -> Decision | None:
    prompt = str(payload.get("prompt") or "")
    lower = prompt.lower()
    if is_clear_one_off(lower) or is_exact_read_prompt(lower):
        return None

    score = 0
    reasons: list[str] = []
    actions: list[str] = []
    for trigger_class, weight, terms, action in STRONG_GROUPS:
        if any(term in lower for term in terms):
            score += weight
            reasons.append(trigger_class)
            actions.append(action)

    if "go on" in lower or "continue" in lower or "resume" in lower:
        score += 1
        reasons.append("continuation-loop")
    if any(term in lower for term in PROTOTYPE_ALREADY_PRESENT_TERMS):
        score -= 2
    if score < DEFAULT_THRESHOLD:
        return None

    return Decision(
        "UserPromptSubmit",
        reasons[0],
        score,
        f"prompt matches {', '.join(unique(reasons[:3]))} work",
        actions[0] if actions else "make a small rerunnable canary before editing broadly",
    )


def score_pre_tool(payload: dict[str, Any]) -> Decision | None:
    tool_name = str(payload.get("tool_name") or "")
    tool_blob = json_compact(payload.get("tool_input"))
    lower = f"{tool_name}\n{tool_blob}".lower()

    if tool_name in {"spawn_agent", "followup_task"} or "explorer" in lower:
        return Decision(
            "PreToolUse",
            "token-saving",
            6,
            "agent or explorer dispatch can burn context quickly",
            "use first_moves/context scout or reuse an existing agent unless the agent ROI is clearly positive",
        )

    if tool_name == "apply_patch" and any(path.lower() in lower for path in WATCH_PATHS):
        return Decision(
            "PreToolUse",
            "tool-runtime",
            6,
            "patch touches tool, hook, skill, scout, reducer, or automation surfaces",
            "prove the risky behavior with a small fixture or lab canary before broadening the patch",
        )

    command = command_text(payload.get("tool_input"))
    family = command_family(command)
    if family and is_expensive_command(command):
        return Decision(
            "PreToolUse",
            "expensive-verification",
            5,
            f"`{family}` is an expensive verification/deploy command",
            "check whether a focused fixture, smoke, or lab canary can prove the next iteration faster",
        )
    if command and any(path.lower() in command.lower() for path in WATCH_PATHS):
        return Decision(
            "PreToolUse",
            "tool-runtime",
            5,
            "command targets a high-leverage Codex tool or automation surface",
            "prefer a small separate prototype or canary before modifying the main path further",
        )
    return None


def score_post_tool(payload: dict[str, Any]) -> Decision | None:
    command = command_text(payload.get("tool_input"))
    family = command_family(command)
    if not family:
        return None

    state = load_state(payload)
    counts = state.setdefault("command_counts", {})
    counts[family] = int(counts.get(family, 0)) + 1

    failed = tool_response_failed(payload.get("tool_response"))
    failures = state.setdefault("failure_counts", {})
    if failed:
        failures[family] = int(failures.get(family, 0)) + 1

    save_state(payload, state)

    if failed and failures[family] >= FAILURE_THRESHOLD:
        return Decision(
            "PostToolUse",
            "repeated-failure",
            6,
            f"`{family}` failed {failures[family]} times",
            "turn the repro into a tiny script or fixture runner before retrying manually",
        )
    if counts[family] >= REPEAT_THRESHOLD and not is_one_off_family(family):
        return Decision(
            "PostToolUse",
            "repeated-command",
            5,
            f"`{family}` has run {counts[family]} times in this session",
            "promote the repeated sequence into a script, canary, or durable tool if it is still needed",
        )
    return None


def dedupe(payload: dict[str, Any], decision: Decision | None, *, now: float | None = None) -> Decision | None:
    if decision is None:
        return None
    now = time.time() if now is None else now
    state = load_state(payload)
    emitted = state.setdefault("emitted", {})
    turn_id = str(payload.get("turn_id") or "turn")
    key = f"{turn_id}:{decision.trigger_class}"
    if key in emitted:
        return None
    emitted[key] = now
    state["last_seen"] = now
    save_state(payload, state)
    cleanup_state_dir()
    return decision


def load_state(payload: dict[str, Any]) -> dict[str, Any]:
    path = state_path(payload)
    try:
        if path.is_file():
            value = json.loads(path.read_text(encoding="utf-8"))
            if isinstance(value, dict):
                return value
    except Exception:
        return {}
    return {}


def save_state(payload: dict[str, Any], state: dict[str, Any]) -> None:
    path = state_path(payload)
    path.parent.mkdir(parents=True, exist_ok=True)
    state["last_seen"] = time.time()
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(state, sort_keys=True, separators=(",", ":")), encoding="utf-8")
    tmp.replace(path)


def state_path(payload: dict[str, Any]) -> Path:
    root = Path(os.environ.get("PROTOTYPE_FIRST_HOOK_CACHE_DIR", str(DEFAULT_CACHE_DIR)))
    session = str(payload.get("session_id") or payload.get("sessionId") or "session")
    cwd = str(payload.get("cwd") or "")
    digest = hashlib.sha256(f"{session}\n{cwd}".encode("utf-8", errors="ignore")).hexdigest()[:24]
    return root / f"{digest}.json"


def cleanup_state_dir() -> None:
    root = Path(os.environ.get("PROTOTYPE_FIRST_HOOK_CACHE_DIR", str(DEFAULT_CACHE_DIR)))
    try:
        files = sorted(root.glob("*.json"), key=lambda p: p.stat().st_mtime, reverse=True)
        for path in files[MAX_STATE_FILES:]:
            path.unlink(missing_ok=True)
    except Exception:
        return


def command_text(tool_input: Any) -> str:
    if isinstance(tool_input, dict):
        command = tool_input.get("command")
        if isinstance(command, list):
            return " ".join(str(part) for part in command)
        if command is not None:
            return str(command)
        params = tool_input.get("params")
        if isinstance(params, dict):
            return command_text(params)
    if isinstance(tool_input, list):
        return " ".join(str(part) for part in tool_input)
    if tool_input is None:
        return ""
    return str(tool_input)


def command_family(command: str) -> str:
    command = command.strip()
    if not command:
        return ""
    command = re.sub(r"^&\s+", "", command)
    parts = re.findall(r'"[^"]+"|\S+', command)
    if not parts:
        return ""
    first = normalize_exe(parts[0])
    if first in {"powershell", "powershell.exe", "pwsh", "pwsh.exe"} and len(parts) > 1:
        script = next((part for part in parts[1:] if part.lower().endswith((".ps1", ".ps1'"))), "")
        if script:
            return normalize_exe(script)
        return first
    if first in {"cmd", "cmd.exe"} and len(parts) > 2:
        return f"{first} {normalize_exe(parts[2])}"
    if first in {"cargo", "just", "git", "cmake", "ninja", "python", "python.exe", "py"} and len(parts) > 1:
        return f"{first} {strip_quotes(parts[1]).lower()}"
    return first


def normalize_exe(part: str) -> str:
    part = strip_quotes(part)
    part = part.replace("\\", "/")
    name = part.rsplit("/", 1)[-1].lower()
    return name


def strip_quotes(text: str) -> str:
    return text.strip().strip("'\"")


def is_expensive_command(command: str) -> bool:
    lower = command.lower()
    expensive_terms = (
        "build-local-codex.ps1",
        "fastrelease",
        "lowmemrelease",
        "cargo test",
        "cargo build",
        "cargo fix",
        "cmake --build",
        "ctest",
        "ninja",
        "npm test",
        "deploy",
        "wrapper",
    )
    return any(term in lower for term in expensive_terms)


def is_clear_one_off(lower: str) -> bool:
    if any(pattern in lower for pattern in EXACT_ONE_OFF_PATTERNS):
        risk = any(term in lower for _, _, terms, _ in STRONG_GROUPS for term in terms)
        return not risk
    return False


def is_exact_read_prompt(lower: str) -> bool:
    starts_like_read = lower.startswith(("read ", "open ", "show ", "show me ", "get-content "))
    names_exact_path = any(marker in lower for marker in ("/", "\\", ".md", ".txt", ".rs", ".py", ".toml", ".json"))
    asks_for_report_only = any(
        phrase in lower
        for phrase in (
            "tell me",
            "one sentence",
            "summarize",
            "quote",
            "what does",
            "what is",
        )
    )
    asks_for_action = any(term in lower for term in ACTION_TERMS)
    return starts_like_read and names_exact_path and asks_for_report_only and not asks_for_action


def is_one_off_family(family: str) -> bool:
    return family in {"git status", "git diff", "git log", "git show", "rg", "cat", "get-content"}


def tool_response_failed(response: Any) -> bool:
    text = json_compact(response).lower()
    if not text:
        return False
    return any(
        marker in text
        for marker in (
            '"success":false',
            '"exit_code":1',
            '"exitcode":1',
            "exit 1",
            "exit code: 1",
            "failed",
            "error:",
        )
    )


def json_compact(value: Any) -> str:
    try:
        return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    except Exception:
        return str(value)


def unique(values: list[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for value in values:
        if value not in seen:
            out.append(value)
            seen.add(value)
    return out


def run_self_tests() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        old_cache = os.environ.get("PROTOTYPE_FIRST_HOOK_CACHE_DIR")
        os.environ["PROTOTYPE_FIRST_HOOK_CACHE_DIR"] = tmp
        try:
            prompt_expectations = [
                (
                    True,
                    "Fix native DAB and add a visible GUI canary before deploy",
                ),
                (
                    True,
                    "Investigate explorer token burn and improve first_moves/cache routing",
                ),
                (
                    True,
                    "Improve build-local-codex FastRelease workflow and deploy system-wide after tests",
                ),
                (
                    True,
                    "Add a new skill and hook to advise automation before risky tool changes",
                ),
                (
                    True,
                    "Mechanical migration: convert all parser fixtures across many files",
                ),
                (
                    True,
                    "I keep manually comparing current output vs alternative output across many logs; automate it",
                ),
                (
                    True,
                    "We are running the same cargo/status check again and again; make it faster and reusable",
                ),
                (
                    False,
                    "Run exactly this read-only shell command once: git diff --stat. Do not run extra exploration.",
                ),
                (False, "Fix a typo in README.md"),
                (False, "Commit and push the already verified changes"),
                (False, "Read docs/MultiAgentMemo.txt and tell me one sentence from it"),
            ]
            for index, (expected, prompt) in enumerate(prompt_expectations):
                decision = evaluate_payload(
                    {
                        "hook_event_name": "UserPromptSubmit",
                        "session_id": "s1",
                        "turn_id": f"prompt-{index}",
                        "cwd": "C:/repo",
                        "prompt": prompt,
                    }
                )
                assert (decision is not None) == expected, prompt

            pre_tool_expectations = [
                (
                    True,
                    {
                        "tool_name": "spawn_agent",
                        "tool_input": {
                            "agent_type": "explorer",
                            "message": "Explore repo broadly",
                        },
                    },
                ),
                (
                    True,
                    {
                        "tool_name": "apply_patch",
                        "tool_input": "*** Update File: codex-rs/desktop-automation/src/windows.rs\n",
                    },
                ),
                (
                    True,
                    {
                        "tool_name": "Bash",
                        "tool_input": {
                            "command": "powershell -ExecutionPolicy Bypass -File scripts\\build-local-codex.ps1 -Mode FastRelease",
                        },
                    },
                ),
                (
                    False,
                    {
                        "tool_name": "Bash",
                        "tool_input": {"command": "git push origin branch"},
                    },
                ),
                (
                    False,
                    {
                        "tool_name": "Bash",
                        "tool_input": {"command": "rg -n prototype .codex"},
                    },
                ),
            ]
            for index, (expected, tool_case) in enumerate(pre_tool_expectations):
                decision = evaluate_payload(
                    {
                        "hook_event_name": "PreToolUse",
                        "session_id": "s1",
                        "turn_id": f"pre-{index}",
                        "cwd": "C:/repo",
                        **tool_case,
                    }
                )
                assert (decision is not None) == expected, tool_case

            for index in range(REPEAT_THRESHOLD - 1):
                assert evaluate_payload(
                    {
                        "hook_event_name": "PostToolUse",
                        "session_id": "s2",
                        "turn_id": f"r{index}",
                        "cwd": "C:/repo",
                        "tool_name": "Bash",
                        "tool_input": {"command": "cargo test -p codex-agent-policy --release -j 1"},
                        "tool_response": {"success": True},
                    }
                ) is None
            assert evaluate_payload(
                {
                    "hook_event_name": "PostToolUse",
                    "session_id": "s2",
                    "turn_id": "r3",
                    "cwd": "C:/repo",
                    "tool_name": "Bash",
                    "tool_input": {"command": "cargo test -p codex-agent-policy --release -j 1"},
                        "tool_response": {"success": True},
                    }
                )
            assert evaluate_payload(
                {
                    "hook_event_name": "PostToolUse",
                    "session_id": "s3",
                    "turn_id": "failure-1",
                    "cwd": "C:/repo",
                    "tool_name": "Bash",
                    "tool_input": {"command": "cmake --build build"},
                    "tool_response": {"success": False, "exit_code": 1},
                }
            ) is None
            assert evaluate_payload(
                {
                    "hook_event_name": "PostToolUse",
                    "session_id": "s3",
                    "turn_id": "failure-2",
                    "cwd": "C:/repo",
                    "tool_name": "Bash",
                    "tool_input": {"command": "cmake --build build"},
                    "tool_response": {"success": False, "exit_code": 1},
                }
            )
        finally:
            if old_cache is None:
                os.environ.pop("PROTOTYPE_FIRST_HOOK_CACHE_DIR", None)
            else:
                os.environ["PROTOTYPE_FIRST_HOOK_CACHE_DIR"] = old_cache
    print("prototype_first_hint self-test passed")


if __name__ == "__main__":
    raise SystemExit(main())
