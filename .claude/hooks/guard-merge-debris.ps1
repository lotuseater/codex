# PreToolUse guard (matcher: Bash)
# Only acts on `git add` / `git commit` commands. Denies if merge debris would
# be staged: any path ending in .orig, or matching .codex/diff_*.patch.
# Defensive: malformed/empty stdin or unrelated command -> allow (exit 0).

$ErrorActionPreference = 'SilentlyContinue'

$repo = 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'

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

# Only act on git add / git commit.
if ($cmd -notmatch 'git\s+(commit|add)\b') { exit 0 }

$staged = @()
try { $staged = git -C $repo diff --cached --name-only 2>$null } catch {}
$staged = $staged | Where-Object { $_ -and $_.Trim() -ne '' }
if (-not $staged -or $staged.Count -eq 0) { exit 0 }

$debris = $staged | Where-Object { $_ -match '\.orig$' -or $_ -match '\.codex/diff_.*\.patch$' }
if ($debris -and $debris.Count -gt 0) {
    $list = ($debris -join ', ')
    Deny ("Merge debris staged (*.orig / .codex/diff_*.patch): " + $list + ". Remove before committing.")
}

exit 0
