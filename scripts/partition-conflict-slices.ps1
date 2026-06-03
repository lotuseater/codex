<#
.SYNOPSIS
    Partition the unmerged (conflicted) files of an in-progress merge into DISJOINT
    AREA slices, one slice per resolver worker, so concurrent merge-conflict resolvers
    never touch the same file.

.DESCRIPTION
    During an `upstream/main -> fork` merge, `git` leaves a set of conflicted files.
    This script maps each file to exactly ONE area slice (by path prefix, the same
    taxonomy used by the merge-preflight / hotspot tooling) and emits a JSON object
        { "<slice>": ["file", ...], ... }
    plus a human-readable count table. A file is NEVER split across slices, and each
    slice is intended for exactly ONE resolver worker -- adjacent edits by different
    workers are the main conflict source, so co-locating an area in one worker avoids it.

    Source of the file list:
        -FromFile <path>  : read one path per line from a file (for testing / scripting)
        (default)         : `git diff --name-only --diff-filter=U` (live unmerged set)

    Read-only: never mutates git or runs cargo/builds.

.PARAMETER FromFile
    Read the conflicted file list from this file (one path per line) instead of asking
    git. Lines that are blank or start with `#` are ignored.

.PARAMETER OutFile
    Where to write the JSON. Default: `.codex/tmp/conflict_slices.json` under repo root.

.PARAMETER RepoRoot
    Repo root. Defaults to $PSScriptRoot/.. so the script works from anywhere.

.EXAMPLE
    pwsh -File scripts/partition-conflict-slices.ps1
    Partition the current merge's unmerged files into slices.

.EXAMPLE
    pwsh -File scripts/partition-conflict-slices.ps1 -FromFile .codex/tmp/sample_unmerged.txt
    Partition a fixed list (e.g. for testing or dry planning).
#>
[CmdletBinding()]
param(
    [string]$FromFile,
    [string]$OutFile,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

if (-not $OutFile) { $OutFile = Join-Path $RepoRoot ".codex/tmp/conflict_slices.json" }

# --- Area taxonomy ----------------------------------------------------------
# Map a repo-relative path to ONE slice. Order matters: most specific first.
# Mirrors the merge-preflight / hotspot-map / adapter-gap taxonomy.
function Get-Slice {
    param([string]$Path)
    $p = $Path -replace "\\", "/"

    # CI / build infra
    if ($p -match "^(\.github/|\.bazelrc$|BUILD\.bazel$|MODULE\.bazel(\.lock)?$)" -or
        $p -match "(^|/)BUILD\.bazel$" -or $p -match "\.bazel$" -or $p -match "(^|/)WORKSPACE$") {
        return "ci-infra"
    }
    # Manifests + generated lock/schema artifacts
    if ($p -match "(^|/)Cargo\.(toml|lock)$" -or $p -match "(^|/)schema/.*\.(json|ts)$" -or
        $p -match "(^|/)config\.schema\.json$") {
        return "manifests"
    }

    # core/session vs core/tools vs core/other
    if ($p -match "^codex-rs/core/src/session/") { return "core-session" }
    if ($p -match "^codex-rs/core/src/tools/")   { return "core-tools" }
    if ($p -match "^codex-rs/core/")             { return "core-other" }

    # protocol crates
    if ($p -match "^codex-rs/app-server-protocol/") { return "app-server-protocol" }
    if ($p -match "^codex-rs/protocol/")            { return "protocol" }

    # app-server (non-protocol)
    if ($p -match "^codex-rs/app-server/") { return "app-server" }

    # config crate
    if ($p -match "^codex-rs/config/") { return "config" }

    # tui crate
    if ($p -match "^codex-rs/tui/") { return "tui" }

    # analytics crates
    if ($p -match "^codex-rs/analytics") { return "analytics" }

    return "other"
}

# Canonical slice order (empty slices are dropped from output).
$sliceOrder = @(
    "core-session", "core-tools", "core-other",
    "protocol", "app-server-protocol", "app-server",
    "config", "tui", "analytics",
    "manifests", "ci-infra", "other"
)

# --- Gather the conflicted file list ----------------------------------------
$files = New-Object System.Collections.Generic.List[string]
if ($FromFile) {
    $resolvedFrom = if ([System.IO.Path]::IsPathRooted($FromFile)) { $FromFile } else { Join-Path $RepoRoot $FromFile }
    if (-not (Test-Path -LiteralPath $resolvedFrom -PathType Leaf)) {
        Write-Error "Conflict-list file not found: $resolvedFrom"
        exit 1
    }
    foreach ($line in Get-Content -LiteralPath $resolvedFrom) {
        $t = $line.Trim()
        if (-not $t -or $t.StartsWith("#")) { continue }
        $files.Add($t)
    }
} else {
    $raw = & git -C $RepoRoot diff --name-only --diff-filter=U
    if ($LASTEXITCODE -ne 0) {
        Write-Error "git diff --diff-filter=U failed (exit $LASTEXITCODE). Are you mid-merge?"
        exit 1
    }
    foreach ($line in $raw) {
        $t = ($line | Out-String).Trim()
        if ($t) { $files.Add($t) }
    }
}

# --- Partition (dedupe; each file in exactly one slice) ---------------------
$slices = [ordered]@{}
foreach ($s in $sliceOrder) { $slices[$s] = New-Object System.Collections.Generic.List[string] }

$seen = [System.Collections.Generic.HashSet[string]]::new()
foreach ($file in $files) {
    if ([string]::IsNullOrWhiteSpace($file)) { continue }
    $file = $file.Trim()
    if (-not $seen.Add($file)) { continue }
    $slice = Get-Slice $file
    $slices[$slice].Add($file)
}

# --- Build output object (only non-empty slices) ----------------------------
$out = [ordered]@{}
foreach ($s in $sliceOrder) {
    if ($slices[$s].Count -gt 0) { $out[$s] = @($slices[$s]) }
}

$jsonText = ([pscustomobject]$out | ConvertTo-Json -Depth 5)
# Single-element arrays must still serialize as arrays.
if ($out.Count -eq 0) { $jsonText = "{}" }

$outDir = Split-Path -Parent $OutFile
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}
Set-Content -LiteralPath $OutFile -Value $jsonText -Encoding UTF8

# --- Human-readable count table ---------------------------------------------
$nonEmpty = @($out.Keys)
$totalFiles = ($files | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique).Count

Write-Host ""
Write-Host "Conflict slices (each slice = ONE resolver worker):" -ForegroundColor Cyan
Write-Host ""
Write-Host ("{0,-22} {1,5}" -f "Slice", "Files")
Write-Host ("{0,-22} {1,5}" -f ("-" * 22), "-----")
foreach ($s in $nonEmpty) {
    Write-Host ("{0,-22} {1,5}" -f $s, $out[$s].Count)
}
Write-Host ("{0,-22} {1,5}" -f ("-" * 22), "-----")
Write-Host ("{0,-22} {1,5}" -f "TOTAL", $totalFiles)
Write-Host ""
Write-Host "Suggested resolver worker count: $($nonEmpty.Count)  (= number of non-empty slices)" -ForegroundColor Green
Write-Host "JSON written -> $OutFile"
Write-Host ""
Write-Output $jsonText

exit 0
