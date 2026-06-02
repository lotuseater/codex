<#
.SYNOPSIS
    Preflight/CI gate: find heavily fork-modified .rs files in UPSTREAM-HOT areas that
    lack a seam file, so divergence can be isolated into a *_local.rs / *_fork.rs adapter
    BEFORE the next upstream merge turns them into chronic conflicts.

.DESCRIPTION
    For each .rs file changed on the fork since the merge-base (with -UpstreamRef) that
    lives in an upstream-hot area, count fork commits touching it. A file is flagged as an
    "adapter gap" when:
        ForkCommitCount >= -MinForkCommits  AND  no sibling seam file exists
    where a seam is <stem>_local.rs / <stem>_adapter.rs / <stem>_fork.rs in the same dir.

    SuggestedTarget: *_local.rs for session/tui areas, *_fork.rs for protocol/features.

    Exit code is 1 when any gap is found (usable as a CI/preflight gate), 0 otherwise.
    Read-only; never runs cargo/builds or mutating git. Uses `git -C <RepoRoot>`.
#>
param(
    [string]$UpstreamRef = "upstream/main",
    [int]$MinForkCommits = 3,
    [string]$OutPath,
    [string]$ForkRef = "HEAD",
    [string]$RepoRoot = "C:\Users\Oleh\Documents\GitHub\open_ai\codex"
)

$ErrorActionPreference = "Stop"

function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$GitArgs)
    & git -C $RepoRoot @GitArgs
}

# Upstream-hot areas (prefix -> area label). Order matters: most specific first.
$HotAreas = [ordered]@{
    "codex-rs/core/src/session/"        = "core-session"
    "codex-rs/core/src/"                = "core"
    "codex-rs/tui/src/app/"             = "tui-app"
    "codex-rs/tui/src/bottom_pane/"     = "tui-bottom-pane"
    "codex-rs/protocol/src/"            = "protocol"
    "codex-rs/features/src/"            = "features"
    "codex-rs/app-server-protocol/src/" = "app-server-protocol"
}

function Get-HotAreaLabel {
    param([string]$Path)
    foreach ($prefix in $HotAreas.Keys) {
        if ($Path.StartsWith($prefix)) { return $HotAreas[$prefix] }
    }
    return $null
}

function Get-SuggestedTarget {
    param([string]$Path, [string]$Area)
    $stem = $Path -replace "\.rs$", ""
    # protocol/features -> *_fork.rs ; session/tui (and everything else hot) -> *_local.rs
    if ($Area -in @("protocol", "features", "app-server-protocol")) {
        return "$stem`_fork.rs"
    }
    return "$stem`_local.rs"
}

function Test-SeamExists {
    param([string]$RepoRoot, [string]$RelPath)
    $stem = $RelPath -replace "\.rs$", ""
    foreach ($suffix in @("_local.rs", "_adapter.rs", "_fork.rs")) {
        $candidate = Join-Path $RepoRoot ($stem + $suffix)
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $true }
    }
    return $false
}

# --- Preflight: upstream remote must exist ---
$null = Invoke-Git rev-parse --verify --quiet "$UpstreamRef^{commit}"
if ($LASTEXITCODE -ne 0) {
    Write-Error ("Cannot resolve '$UpstreamRef'. Ensure the 'upstream' remote exists and is fetched, " +
        "or pass -UpstreamRef <ref>.")
    exit 2
}

$mergeBase = (Invoke-Git merge-base $ForkRef $UpstreamRef | Select-Object -First 1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($mergeBase)) {
    Write-Error "Could not compute merge-base of '$ForkRef' and '$UpstreamRef'."
    exit 2
}
$mergeBase = $mergeBase.Trim()

# --- Fork-changed .rs files since merge-base ---
$changed = Invoke-Git diff --name-only "$mergeBase..$ForkRef"

# --- Per-file fork commit counts (batched name-only log over hot files) ---
$forkCommitCount = @{}
$forkLog = Invoke-Git log --no-merges --format="C:%H" "$mergeBase..$ForkRef" --name-only
foreach ($line in $forkLog) {
    if ($null -eq $line -or $line.StartsWith("C:")) { continue }
    $p = $line.Trim()
    if ($p) { $forkCommitCount[$p] = ([int]($forkCommitCount[$p])) + 1 }
}

$gaps = New-Object System.Collections.Generic.List[object]
$seen = [System.Collections.Generic.HashSet[string]]::new()
foreach ($file in $changed) {
    if ([string]::IsNullOrWhiteSpace($file)) { continue }
    $file = $file.Trim()
    if ($file -notmatch "\.rs$") { continue }
    if (-not $seen.Add($file)) { continue }
    # Skip seam files themselves.
    if ($file -match "_(local|adapter|fork)\.rs$") { continue }
    $area = Get-HotAreaLabel $file
    if (-not $area) { continue }
    $fc = [int]($forkCommitCount[$file])
    if ($fc -lt $MinForkCommits) { continue }
    if (Test-SeamExists -RepoRoot $RepoRoot -RelPath $file) { continue }
    $gaps.Add([pscustomobject]@{
        File = $file
        ForkCommits = $fc
        Area = $area
        SuggestedTarget = (Get-SuggestedTarget -Path $file -Area $area)
    })
}

$sorted = $gaps | Sort-Object ForkCommits -Descending

# --- Render table ---
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# Adapter-seam gaps (upstream-hot, fork-heavy, no seam)")
$lines.Add("")
$lines.Add("UpstreamRef: $UpstreamRef  |  merge-base: $mergeBase  |  MinForkCommits: $MinForkCommits")
$lines.Add("")
if ($sorted.Count -eq 0) {
    $lines.Add("No adapter gaps found. (All hot fork-heavy files already have a seam, or none qualify.)")
} else {
    $lines.Add("| File | ForkCommits | Area | SuggestedTarget |")
    $lines.Add("| --- | ---: | --- | --- |")
    foreach ($g in $sorted) {
        $lines.Add("| $($g.File) | $($g.ForkCommits) | $($g.Area) | $($g.SuggestedTarget) |")
    }
}
$text = ($lines -join [Environment]::NewLine)

if ($OutPath) {
    $resolvedOut = if ([System.IO.Path]::IsPathRooted($OutPath)) { $OutPath } else { Join-Path $RepoRoot $OutPath }
    $outDir = Split-Path -Parent $resolvedOut
    if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
        New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    }
    Set-Content -LiteralPath $resolvedOut -Value $text -Encoding UTF8
    Write-Host "Wrote adapter-gap report -> $resolvedOut"
} else {
    Write-Output $text
}

if ($sorted.Count -gt 0) {
    Write-Host ""
    Write-Host "GATE: $($sorted.Count) adapter gap(s) found -> exit 1" -ForegroundColor Yellow
    exit 1
}
exit 0
