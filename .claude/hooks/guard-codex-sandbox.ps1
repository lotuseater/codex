# PreToolUse guard (matcher: Edit|Write|MultiEdit)
# AGENTS.md forbids adding/modifying CODEX_SANDBOX_* sandbox env-var CODE.
# Scope: Rust source (*.rs) only. Docs/scripts that merely *mention* the names
# (this guard, BRIEF.md, handoffs) are fine and must not be blocked.
# Deny when the ADDED text introduces the sandbox network-disabled / sandbox
# env-var constant. Defensive: malformed/empty stdin -> allow (exit 0).

$ErrorActionPreference = 'Stop'

function Deny([string]$reason) {
    $out = @{
        hookSpecificOutput = @{
            hookEventName            = 'PreToolUse'
            permissionDecision       = 'deny'
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

# Scope to Rust source files only — the AGENTS.md rule is about code, not prose.
$fp = ''
try { if ($null -ne $j.tool_input.file_path) { $fp = [string]$j.tool_input.file_path } } catch {}
if ($fp -notmatch '\.rs$') { exit 0 }

# Gather the ADDED text across tool shapes.
$added = New-Object System.Text.StringBuilder
try {
    $ns = $j.tool_input.new_string
    if ($null -ne $ns) { [void]$added.Append([string]$ns).Append("`n") }
} catch {}
try {
    $content = $j.tool_input.content
    if ($null -ne $content) { [void]$added.Append([string]$content).Append("`n") }
} catch {}
try {
    $edits = $j.tool_input.edits
    if ($null -ne $edits) {
        foreach ($e in $edits) {
            if ($null -ne $e.new_string) { [void]$added.Append([string]$e.new_string).Append("`n") }
        }
    }
} catch {}

$text = $added.ToString()
if ([string]::IsNullOrEmpty($text)) { exit 0 }

# One regex matches BOTH sandbox constants without spelling either full name
# contiguously, so editing this guard never trips itself.
if ($text -match 'CODEX_SANDBOX(?:_NETWORK_DISABLED)?_ENV_VAR') {
    Deny 'AGENTS.md forbids adding/modifying CODEX_SANDBOX_* sandbox env-var code (guard scoped to *.rs).'
}

exit 0
