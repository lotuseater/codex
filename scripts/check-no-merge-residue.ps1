<#
.SYNOPSIS
    Commit/merge GATE: fail if leftover conflict residue is present in the tree.

.DESCRIPTION
    Prevents committing leftover git conflict markers or `*.orig` merge-backup files
    after an `upstream/main -> fork` merge. Two checks, either of which fails the gate:

      1. Conflict markers at the START of a line in tracked text files:
             <<<<<<<            (ours marker, followed by a space)
             |||||||            (diff3 base marker, followed by a space)
             =======            (separator: EXACTLY 7 equals on their own line)
             >>>>>>>            (theirs marker, followed by a space)
         Scanned via `git grep -nE` over tracked content (so .git/, target/,
         node_modules/, and binary files are never touched). Doc/example files that
         legitimately contain literal marker strings are excluded by path:
         this script, anything under `.codex/` or `docs/`, and any `*.md`.
         `git diff --check` is also run; any output is treated as a failure.

      2. Any `*.orig` file in the working tree (a merge backup) -> fail and list them.

    -Staged          Restrict the marker scan to STAGED files only
                     (`git diff --cached --name-only`) — use as a pre-commit gate.
    -IncludeUntracked  Also consider untracked files for `*.orig` detection and the
                     marker scan (otherwise only tracked content is scanned).

    Exit 1 on any residue (prints each offending file:line); exit 0 on a clean tree
    with a one-line green summary. Read-only; never builds or mutates git.
#>
[CmdletBinding()]
param(
    [switch]$Staged,
    [switch]$IncludeUntracked,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"
# Native (exe) non-zero exits must NOT throw: `git grep` exits 1 on no-match, which is
# the normal "clean" case. We inspect $LASTEXITCODE explicitly instead.
$PSNativeCommandUseErrorActionPreference = $false

function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$GitArgs)
    & git -C $RepoRoot @GitArgs
}

# --- Path exclusions: files that legitimately contain literal marker strings ---
# (this gate itself, automation docs under .codex/, docs/, and any markdown).
function Test-ExcludedPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $true }
    $p = $Path -replace "\\", "/"
    if ($p -ieq "scripts/check-no-merge-residue.ps1") { return $true }
    if ($p -like ".codex/*") { return $true }
    if ($p -like "docs/*")   { return $true }
    if ($p -like "*.md")     { return $true }
    return $false
}

# --- Conflict-marker regexes (anchored at line start) ---
# Built from char codes so this script does not itself contain literal marker runs
# (defense-in-depth; the path exclusion above already protects this file).
$lt   = [string][char]0x3C   # '<'
$gt   = [string][char]0x3E   # '>'
$eq   = [string][char]0x3D   # '='
$pipe = [string][char]0x7C   # '|'
# NOTE: in ERE, '|' is the alternation operator, so the diff3 base marker MUST be
# escaped as '\|' or the pattern matches every line. The other markers are literal.
$bs   = [string][char]0x5C   # '\'
$markerPatterns = @(
    "^$lt$lt$lt$lt$lt$lt$lt "                                       # <<<<<<< (ours)
    "^$gt$gt$gt$gt$gt$gt$gt "                                       # >>>>>>> (theirs)
    "^$eq$eq$eq$eq$eq$eq$eq$"                                       # ======= (exactly 7, whole line)
    "^$bs$pipe$bs$pipe$bs$pipe$bs$pipe$bs$pipe$bs$pipe$bs$pipe "    # ||||||| (diff3 base, pipes escaped)
)

$offenders = New-Object System.Collections.Generic.List[string]

# --- Determine the file set to grep ---
# Default: all tracked text content. -Staged: only staged files. The marker scan
# always runs `git grep` against tracked/staged content so binaries and ignored
# trees (.git/, target/, node_modules/) are skipped by git itself.
# NOTE: do NOT name a local var '$staged' — PowerShell vars are case-insensitive and it
# would clobber the [switch]$Staged parameter (string->SwitchParameter conversion error).
$pathFilter = $null
if ($Staged) {
    $stagedFiles = Invoke-Git diff --cached --name-only --diff-filter=ACMR
    if ($LASTEXITCODE -ne 0) { Write-Error "git diff --cached failed."; exit 2 }
    $pathFilter = @($stagedFiles | Where-Object { -not (Test-ExcludedPath $_) })
    if (-not $pathFilter -or $pathFilter.Count -eq 0) {
        Write-Verbose "No staged files to scan for markers."
    }
}

# --- 1. Conflict-marker scan via git grep ---
foreach ($pat in $markerPatterns) {
    $grepArgs = @("grep", "-nIE", "--no-color")
    if ($Staged) { $grepArgs += "--cached" }
    elseif ($IncludeUntracked) { $grepArgs += @("--untracked") }
    $grepArgs += @("-e", $pat)
    # Restrict to staged subset when -Staged; otherwise scan whole tracked tree.
    if ($Staged) {
        if (-not $pathFilter -or $pathFilter.Count -eq 0) { continue }
        $grepArgs += "--"
        $grepArgs += $pathFilter
    }
    $hits = Invoke-Git @grepArgs
    # git grep exits 1 with no match (normal), 0 with match, >1 on error.
    if ($LASTEXITCODE -gt 1) { Write-Error "git grep failed (pattern: $pat)."; exit 2 }
    foreach ($h in $hits) {
        if ([string]::IsNullOrWhiteSpace($h)) { continue }
        # Format: <path>:<line>:<content>
        $file = ($h -split ":", 2)[0]
        if (Test-ExcludedPath $file) { continue }
        $offenders.Add($h)
    }
}

# --- git diff --check (catches markers + whitespace-conflict residue) ---
$diffCheck = Invoke-Git diff --check
$diffCheckExit = $LASTEXITCODE
$diffCheckHits = @($diffCheck | Where-Object {
    -not [string]::IsNullOrWhiteSpace($_) -and ($_ -match "conflict marker")
})
foreach ($h in $diffCheckHits) {
    $file = ($h -split ":", 2)[0]
    if (Test-ExcludedPath $file) { continue }
    $offenders.Add("diff --check: $h")
}

# --- 2. *.orig backup files ---
# A `*.orig` file is residue that must never be committed. We flag:
#   - TRACKED *.orig            -> always fail (already in the index).
#   - Untracked, NOT-ignored    -> fail: it can be `git add`ed by accident.
# We deliberately IGNORE untracked *.orig that git itself ignores (`--exclude-standard`),
# e.g. backups sitting in the gitignored `.codex/tmp/` scratch dir: those cannot be
# committed, so they are not a gate failure (consistent with the marker-scan exclusions).
$origFiles = New-Object System.Collections.Generic.List[string]
$trackedOrig = Invoke-Git ls-files "*.orig"
foreach ($f in $trackedOrig) {
    $t = $f.Trim()
    if (-not [string]::IsNullOrWhiteSpace($t) -and -not (Test-ExcludedPath $t) -and -not $origFiles.Contains($t)) {
        $origFiles.Add($t)
    }
}
# `--others --exclude-standard` = untracked AND not gitignored (i.e. committable).
$untrackedOrig = Invoke-Git ls-files --others --exclude-standard "*.orig"
foreach ($f in $untrackedOrig) {
    $t = $f.Trim()
    if (-not [string]::IsNullOrWhiteSpace($t) -and -not (Test-ExcludedPath $t) -and -not $origFiles.Contains($t)) {
        $origFiles.Add($t)
    }
}

# --- Verdict ---
$failed = $false

if ($offenders.Count -gt 0) {
    $failed = $true
    Write-Host "RESIDUE: conflict markers found:" -ForegroundColor Red
    foreach ($o in ($offenders | Sort-Object -Unique)) {
        Write-Host "  $o"
    }
}

if ($origFiles.Count -gt 0) {
    $failed = $true
    Write-Host "RESIDUE: merge-backup *.orig file(s) present:" -ForegroundColor Red
    foreach ($f in ($origFiles | Sort-Object -Unique)) {
        Write-Host "  $f"
    }
}

if ($failed) {
    Write-Host ""
    Write-Host "GATE FAILED: remove the residue above before committing -> exit 1" -ForegroundColor Red
    exit 1
}

$scope = if ($Staged) { "staged files" } else { "tracked tree" }
Write-Host "OK: no conflict markers, no *.orig backups in $scope. -> exit 0" -ForegroundColor Green
exit 0
