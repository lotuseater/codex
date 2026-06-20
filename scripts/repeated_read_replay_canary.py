#!/usr/bin/env python3
"""Detect repeated or overlapping source reads/searches in Codex session logs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


DRIVE_PATH_RE = re.compile(r"[A-Za-z]:\\[^'\"\r\n;|]+")
PS_ASSIGN_RE = re.compile(r"\$([A-Za-z_][A-Za-z0-9_]*)\s*=\s*['\"]([^'\"]+)['\"]")
PS_FOR_RANGE_RE = re.compile(
    r"for\s*\(\s*\$[A-Za-z_][A-Za-z0-9_]*\s*=\s*(\d+)\s*;\s*\$[A-Za-z_][A-Za-z0-9_]*\s+-le\s+(\d+)",
    re.I,
)
PS_IF_RANGE_RE = re.compile(
    r"if\s*\(\s*\$[A-Za-z_][A-Za-z0-9_]*\s+-ge\s+(\d+)\s+-and\s+\$[A-Za-z_][A-Za-z0-9_]*\s+-le\s+(\d+)",
    re.I,
)
GET_CONTENT_RE = re.compile(
    r"Get-Content(?:\s+-(?:LiteralPath|Path))?\s+('([^']+)'|\"([^\"]+)\"|(\$[A-Za-z_][A-Za-z0-9_]*)|([^\s;|]+))",
    re.I,
)
GET_CONTENT_FLAG_PATH_RE = re.compile(
    r"Get-Content\b.*?-(?:LiteralPath|Path)\s+('([^']+)'|\"([^\"]+)\"|(\$[A-Za-z_][A-Za-z0-9_]*)|([^\s;|]+))",
    re.I,
)
TOTAL_COUNT_RE = re.compile(r"-TotalCount\s+(\d+)", re.I)
RG_RE = re.compile(r"(?:^|\s)rg(?:\.exe)?\s+(.+)", re.I | re.S)
SELECT_STRING_RE = re.compile(r"Select-String\s+(.+)", re.I | re.S)
TOKEN_RE = re.compile(r"'([^']*)'|\"([^\"]*)\"|(\S+)")
RG_VALUE_FLAGS = {
    "-e",
    "--regexp",
    "-g",
    "--glob",
    "--iglob",
    "-t",
    "--type",
    "-T",
    "--type-not",
}
SELECT_PATH_FLAGS = {"-path", "-literalpath"}
SELECT_PATTERN_FLAGS = {"-pattern"}


@dataclass(frozen=True)
class Access:
    kind: str
    target: str
    detail: str = ""
    start: int | None = None
    end: int | None = None
    source_line: int = 0
    session: str = ""

    @property
    def exact_key(self) -> tuple[str, str, str, int | None, int | None]:
        return (self.kind, self.target.lower(), self.detail, self.start, self.end)

    @property
    def target_key(self) -> tuple[str, str]:
        return (self.kind, self.target.lower())

    def range_text(self) -> str:
        if self.start is None or self.end is None:
            return "all"
        return f"{self.start}-{self.end}"


def decode_arguments(payload: dict[str, Any]) -> dict[str, Any]:
    raw = payload.get("arguments")
    if isinstance(raw, dict):
        return raw
    if not isinstance(raw, str):
        return {}
    try:
        decoded = json.loads(raw)
    except json.JSONDecodeError:
        return {"command": raw}
    return decoded if isinstance(decoded, dict) else {}


def shell_like_strings(event: dict[str, Any]) -> Iterable[str]:
    payload = event.get("payload")
    if not isinstance(payload, dict) or payload.get("type") != "function_call":
        return

    name = str(payload.get("name") or "")
    args = decode_arguments(payload)

    if name.endswith("shell_command") and isinstance(args.get("command"), str):
        yield args["command"]
    elif name.endswith("workflow_batch"):
        yield json.dumps(args, sort_keys=True)


def command_tokens(text: str) -> list[str]:
    return [
        next(group for group in match.groups() if group is not None)
        for match in TOKEN_RE.finditer(text)
    ]


def normalize_path(path: str, assignments: dict[str, str]) -> str:
    path = path.strip().strip("'\"")
    if path.startswith("$"):
        path = assignments.get(path[1:], path)
    return path.replace("/", "\\").rstrip(",)")


def looks_like_path_token(token: str) -> bool:
    if not token or token.startswith("-") or token in {"|", ";"}:
        return False
    return any(char in token for char in ("/", "\\")) or Path(token).suffix != ""


def requested_range(text: str) -> tuple[int | None, int | None]:
    range_match = PS_FOR_RANGE_RE.search(text) or PS_IF_RANGE_RE.search(text)
    if range_match:
        return int(range_match.group(1)), int(range_match.group(2))

    total_match = TOTAL_COUNT_RE.search(text)
    if total_match:
        return 1, int(total_match.group(1))

    return None, None


def extract_get_content(text: str, session: str, source_line: int) -> Iterable[Access]:
    assignments = {name: value for name, value in PS_ASSIGN_RE.findall(text)}
    start, end = requested_range(text)

    flag_path_matches = list(GET_CONTENT_FLAG_PATH_RE.finditer(text))
    if flag_path_matches:
        for match in flag_path_matches:
            raw = next(group for group in match.groups()[1:] if group)
            yield Access(
                "read",
                normalize_path(raw, assignments),
                "",
                start,
                end,
                source_line,
                session,
            )
        return

    for match in GET_CONTENT_RE.finditer(text):
        raw = next(group for group in match.groups()[1:] if group)
        if raw.startswith("-"):
            continue
        target = normalize_path(raw, assignments)
        if target.lower() not in {"$null", "|"}:
            yield Access("read", target, "", start, end, source_line, session)


def extract_select_string(
    text: str, session: str, source_line: int
) -> Iterable[Access]:
    match = SELECT_STRING_RE.search(text)
    if not match:
        return

    pattern = ""
    paths: list[str] = []
    tokens = command_tokens(match.group(1))
    index = 0
    while index < len(tokens):
        lower = tokens[index].lower()
        if lower in SELECT_PATTERN_FLAGS and index + 1 < len(tokens):
            pattern = tokens[index + 1]
            index += 2
        elif lower in SELECT_PATH_FLAGS and index + 1 < len(tokens):
            paths.append(tokens[index + 1])
            index += 2
        else:
            index += 1

    if not paths:
        paths = DRIVE_PATH_RE.findall(match.group(1))

    for path in paths:
        yield Access(
            "search",
            normalize_path(path, {}),
            pattern,
            None,
            None,
            source_line,
            session,
        )


def extract_rg(text: str, session: str, source_line: int) -> Iterable[Access]:
    match = RG_RE.search(text)
    if not match:
        return

    pattern = ""
    paths: list[str] = []
    tokens = command_tokens(match.group(1))
    index = 0
    while index < len(tokens):
        token = tokens[index]
        lower = token.lower()
        if lower in RG_VALUE_FLAGS:
            if lower in {"-e", "--regexp"} and index + 1 < len(tokens):
                pattern = tokens[index + 1]
            index += 2
            continue
        if token.startswith("-"):
            index += 1
            continue
        if not pattern:
            pattern = token
        elif looks_like_path_token(token):
            paths.append(token)
        index += 1

    if not paths:
        paths = [
            path
            for path in DRIVE_PATH_RE.findall(match.group(1))
            if ".jsonl" not in path.lower()
        ]
    if not paths:
        paths = ["(implicit cwd)"]

    for path in paths[:8]:
        if path not in {"rg", "rg.exe"} and not path.startswith("*."):
            yield Access(
                "search",
                normalize_path(path, {}),
                pattern,
                None,
                None,
                source_line,
                session,
            )


def extract_accesses(text: str, session: str, source_line: int) -> Iterable[Access]:
    stripped = text.strip()
    if stripped.startswith("{"):
        try:
            structured = json.loads(stripped)
        except json.JSONDecodeError:
            structured = None
        if isinstance(structured, dict):
            yield from extract_workflow_batch_accesses(structured, session, source_line)

    yield from extract_get_content(text, session, source_line)
    yield from extract_select_string(text, session, source_line)
    yield from extract_rg(text, session, source_line)


def extract_workflow_batch_accesses(
    value: Any, session: str, source_line: int
) -> Iterable[Access]:
    if isinstance(value, dict):
        for key, payload in value.items():
            key_lower = key.lower()
            if key_lower in {"read_file", "read_json"}:
                path = workflow_path(payload)
                if path:
                    yield Access(
                        "read",
                        normalize_path(path, {}),
                        "",
                        None,
                        None,
                        source_line,
                        session,
                    )
            elif key_lower in {"search_text", "scoped_search"}:
                path = workflow_path(payload) or "(implicit cwd)"
                detail = workflow_pattern(payload)
                yield Access(
                    "search",
                    normalize_path(path, {}),
                    detail,
                    None,
                    None,
                    source_line,
                    session,
                )
            else:
                yield from extract_workflow_batch_accesses(
                    payload, session, source_line
                )
    elif isinstance(value, list):
        for item in value:
            yield from extract_workflow_batch_accesses(item, session, source_line)


def workflow_path(payload: Any) -> str | None:
    if isinstance(payload, str):
        return payload
    if not isinstance(payload, dict):
        return None
    for key in ("path", "file", "root", "dir"):
        value = payload.get(key)
        if isinstance(value, str) and value:
            return value
    return None


def workflow_pattern(payload: Any) -> str:
    if not isinstance(payload, dict):
        return ""
    for key in ("pattern", "query", "q"):
        value = payload.get(key)
        if isinstance(value, str):
            return value
    return ""


def read_session(path: Path) -> list[Access]:
    accesses: list[Access] = []
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for line_no, line in enumerate(handle, 1):
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            for text in shell_like_strings(event):
                accesses.extend(extract_accesses(text, str(path), line_no))
    return accesses


def ranges_overlap(left: Access, right: Access) -> bool:
    if left.start is None and left.end is None:
        return right.start is not None or right.end is not None
    if right.start is None and right.end is None:
        return left.start is not None or left.end is not None
    if (
        left.start is None
        or left.end is None
        or right.start is None
        or right.end is None
    ):
        return False
    return max(left.start, right.start) <= min(left.end, right.end)


def analyze(accesses: list[Access]) -> dict[str, Any]:
    exact_counts = Counter(access.exact_key for access in accesses)
    exact_repeats = {key: count for key, count in exact_counts.items() if count > 1}

    by_target: dict[tuple[str, str], list[Access]] = defaultdict(list)
    for access in accesses:
        by_target[access.target_key].append(access)

    overlaps: list[tuple[Access, Access]] = []
    for bucket in by_target.values():
        reads = [access for access in bucket if access.kind == "read"]
        for index, left in enumerate(reads):
            for right in reads[index + 1 :]:
                if left.exact_key != right.exact_key and ranges_overlap(left, right):
                    overlaps.append((left, right))

    return {
        "total_accesses": len(accesses),
        "unique_exact_accesses": len(exact_counts),
        "exact_repeated_accesses": len(exact_repeats),
        "overlapping_range_pairs": len(overlaps),
        "repeated_searches": sum(1 for key in exact_repeats if key[0] == "search"),
        "exact_repeats": exact_repeats,
        "overlaps": overlaps,
        "by_target": by_target,
    }


def print_report(report: dict[str, Any], max_examples: int) -> int:
    print("repeated-read replay canary")
    print(f"total_accesses: {report['total_accesses']}")
    print(f"unique_exact_accesses: {report['unique_exact_accesses']}")
    print(f"exact_repeated_accesses: {report['exact_repeated_accesses']}")
    print(f"overlapping_range_pairs: {report['overlapping_range_pairs']}")
    print(f"repeated_searches: {report['repeated_searches']}")

    if report["exact_repeats"]:
        print("\nexact repeat examples:")
        for key, count in list(report["exact_repeats"].items())[:max_examples]:
            kind, target, detail, start, end = key
            span = "all" if start is None or end is None else f"{start}-{end}"
            suffix = f" pattern={detail!r}" if detail else ""
            print(f"- {count}x {kind} {target} {span}{suffix}")

    if report["overlaps"]:
        print("\noverlap examples:")
        for left, right in report["overlaps"][:max_examples]:
            print(
                f"- {left.target} {left.range_text()} @line {left.source_line} "
                f"overlaps {right.range_text()} @line {right.source_line}"
            )

    return (
        1
        if report["exact_repeated_accesses"] or report["overlapping_range_pairs"]
        else 0
    )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("sessions", nargs="+", type=Path)
    parser.add_argument("--max-examples", type=int, default=12)
    parser.add_argument("--json-out", type=Path)
    args = parser.parse_args(argv)

    accesses: list[Access] = []
    for session in args.sessions:
        accesses.extend(read_session(session))

    report = analyze(accesses)
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        serializable = {
            key: value
            for key, value in report.items()
            if key not in {"exact_repeats", "overlaps", "by_target"}
        }
        serializable["accesses"] = [access.__dict__ for access in accesses]
        args.json_out.write_text(json.dumps(serializable, indent=2), encoding="utf-8")

    return print_report(report, args.max_examples)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
