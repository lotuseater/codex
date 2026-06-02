# PreToolUse guard (matcher: Bash|PowerShell)
# Blocks forbidden debug-build cargo command shapes for the Codex Rust fork.
# Reads tool-call JSON on stdin; prints a permissionDecision:"deny" JSON to deny.
# Defensive: malformed/empty stdin or null command -> allow (exit 0).

$ErrorActionPreference = 'Stop'

function Deny([string]$reason) {
    $out = @{
        hookSpecificOutput = @{
            hookEventName          = 'PreToolUse'
            permissionDecision     = 'deny'
            permissionDecisionReason = $reason
        }
    }
    $out | ConvertTo-Json -Compress -Depth 5
    exit 0
}

try {
    $raw = [Console]::In.ReadToEnd()
} catch {
    exit 0
}
if ([string]::IsNullOrWhiteSpace($raw)) { exit 0 }

try {
    $j = $raw | ConvertFrom-Json
} catch {
    exit 0
}

$cmd = $null
try { $cmd = $j.tool_input.command } catch { $cmd = $null }
if ([string]::IsNullOrWhiteSpace($cmd)) { exit 0 }

# Whitelist: always allow these (return early).
if ($cmd -match 'cargo\s+(fmt|insta|metadata|tree)\b') { exit 0 }

$reason = "Build only release! Use scripts\build-local-codex.ps1 -Mode FastRelease or test-local-codex-release.ps1 -Package <crate>. Debug builds can exhaust C: disk/RAM on this checkout."

# (b) broad debug lanes — block regardless of --release.
if ($cmd -match 'cargo\s+test\s+-p\s+codex-cli\b' -or $cmd -match 'cargo\s+test\s+-p\s+codex-exec\b') {
    Deny $reason
}

# (a) cargo build/test/check/run/clippy/nextest/bench WITHOUT --release.
if ($cmd -match 'cargo\s+(build|test|check|run|clippy|nextest|bench)\b' -and $cmd -notmatch '--release') {
    Deny $reason
}

# (c) anything targeting target\debug (or target/debug) with a cargo/rustc invocation.
# In a single-quoted PS string the regex engine needs \\\\ to match one literal backslash.
if ($cmd -match 'target[\\\\/]debug' -and $cmd -match '(cargo|rustc)\b') {
    Deny $reason
}

# (d) build-local-codex.ps1 -Mode DevRelease.
if ($cmd -match '-Mode\s+DevRelease\b') {
    Deny $reason
}

exit 0
