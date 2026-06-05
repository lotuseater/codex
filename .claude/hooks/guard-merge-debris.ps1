# PreToolUse guard (matcher: Bash)
# Only acts on `git add` / `git commit` commands. Denies if merge debris would
# be staged: any path ending in .orig, or matching .codex/diff_*.patch. Also
# denies if the STAGED tracked content contains real inline conflict markers.
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

# --- Staged inline conflict-marker scan ---
# Block committing real conflict markers that survived a merge. Primary signal is
# `git diff --cached --check` (git's own conflict-marker detector over the staged
# diff: ignores binaries and most false positives). A regex over staged blob
# content is the backstop. Both restrict to STAGED tracked content only.
#
# Path exclusions: files that legitimately contain literal marker strings -- this
# hook, the residue gate, automation docs under .codex/, docs/, and any *.md.
function Test-ExcludedPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $true }
    $p = $Path -replace '\\', '/'
    if ($p -match 'check-no-merge-residue') { return $true }
    if ($p -match 'guard-merge-debris')     { return $true }
    if ($p -like '.codex/*')                { return $true }
    if ($p -like 'docs/*')                  { return $true }
    if ($p -like '*.md')                    { return $true }
    return $false
}

# Staged files to scan (added/copied/modified/renamed -- not deletions).
$markerFiles = @()
try {
    $markerFiles = git -C $repo diff --cached --name-only --diff-filter=ACMR 2>$null
} catch {}
$markerFiles = @($markerFiles | Where-Object { $_ -and $_.Trim() -ne '' -and -not (Test-ExcludedPath $_) })

if ($markerFiles.Count -gt 0) {
    $markerHits = New-Object System.Collections.Generic.List[string]

    # Primary: git diff --cached --check (lines containing "conflict marker").
    $check = $null
    try { $check = git -C $repo diff --cached --check 2>$null } catch {}
    foreach ($line in @($check)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line -notmatch 'conflict marker') { continue }
        $f = ($line -split ':', 2)[0]
        if (Test-ExcludedPath $f) { continue }
        $markerHits.Add($f) | Out-Null
    }

    # Backstop: regex over each staged blob (`git show :<file>`). Markers anchored
    # at line start: <<<<<<< (ours, +space), >>>>>>> (theirs, +space), ======= (a
    # whole line of exactly 7 '='), ||||||| (diff3 base, +space). Built from char
    # codes so this file holds no literal marker runs. Only REAL 7-char markers --
    # shorter '=' rows (markdown rules) or '>>>' in code do not match.
    # NOTE: '|' is the regex alternation operator, so each pipe MUST be escaped
    # (\|) or the diff3 pattern collapses to "match anything". '<', '>', '=' are
    # not regex-special and stay literal.
    $lt    = [string][char]0x3C
    $gt    = [string][char]0x3E
    $eq    = [string][char]0x3D
    $pipeE = [string][char]0x5C + [string][char]0x7C   # '\|' (escaped pipe)
    $markerRegexes = @(
        ('^' + ($lt * 7) + ' ')
        ('^' + ($gt * 7) + ' ')
        ('^' + ($eq * 7) + '$')
        ('^' + ($pipeE * 7) + ' ')
    )
    foreach ($f in $markerFiles) {
        if ($markerHits.Contains($f)) { continue }
        $blob = $null
        try { $blob = git -C $repo show (":" + $f) 2>$null } catch {}
        if (-not $blob) { continue }
        # Skip binary blobs (NUL byte present).
        if (($blob -join "`n") -match "`0") { continue }
        foreach ($ln in @($blob)) {
            $matched = $false
            foreach ($rx in $markerRegexes) {
                if ($ln -match $rx) { $matched = $true; break }
            }
            if ($matched) { $markerHits.Add($f) | Out-Null; break }
        }
    }

    if ($markerHits.Count -gt 0) {
        $list = (($markerHits | Sort-Object -Unique) -join ', ')
        Deny ("Inline merge conflict markers in staged content: " + $list + ". Resolve before committing.")
    }
}

exit 0
