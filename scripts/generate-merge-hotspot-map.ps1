<#
.SYNOPSIS
    Generate a merge-hotspot map for the Codex fork: per-.rs-file pressure scoring
    that blends historical merge-conflict frequency, upstream churn, and fork churn.

.DESCRIPTION
    For every .rs file under codex-rs/ that has REAL divergence (changed on either the
    upstream side or the fork side since the merge-base), compute:
      * MergeTouchCount     - over recent merge commits on -ForkRef, how many touched
                              this file on BOTH parents (a real conflict opportunity).
      * UpstreamChurnLines  - added+deleted lines in <merge-base>..-UpstreamRef.
      * ForkCommitCount     - fork commits touching the file since merge-base.
      * SeamExists          - a sibling <stem>_local.rs / _adapter.rs / _fork.rs exists.
      * Area / RiskWeight    - area->risk mapping kept IDENTICAL to
                              scripts\analyze-branch-conflict-surface.ps1 (Get-Risk).
      * HotspotScore = (MergeTouchCount+1) * (UpstreamChurnLines+1) * RiskWeight
        (the +1s keep new-pressure-only files from zeroing out).

    Emits a markdown table sorted desc by HotspotScore (top -Top) and, optionally, JSON.

.NOTES
    Read-only. Never runs cargo/builds or mutating git. Uses `git -C <RepoRoot>` so it
    works from any cwd.
#>
param(
    [string]$UpstreamRef = "upstream/main",
    [string]$ForkRef = "HEAD",
    [string]$OutPath = "docs/merge-hotspot-map.md",
    [string]$JsonOut,
    [int]$Top = 60,
    [int]$MaxMerges = 25,
    [string]$RepoRoot = "C:\Users\Oleh\Documents\GitHub\open_ai\codex"
)

$ErrorActionPreference = "Stop"

function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$GitArgs)
    & git -C $RepoRoot @GitArgs
}

# --- Area + risk mapping: kept IDENTICAL to analyze-branch-conflict-surface.ps1 ---
# (Source of truth for these weights is Get-HotArea / Get-Risk in that script.)
function Get-HotArea {
    param([string]$Path)
    if ($Path -match "^(MODULE\.bazel(\.lock)?|codex-rs/Cargo\.(toml|lock))$") {
        return "generated-or-workspace-locks"
    }
    if ($Path -match "^codex-rs/core/") { return "codex-core" }
    if ($Path -match "^codex-rs/tui/") { return "codex-tui" }
    if ($Path -match "^codex-rs/protocol/") { return "codex-protocol" }
    if ($Path -match "^codex-rs/app-server-protocol/") { return "app-server-protocol" }
    if ($Path -match "^scripts/") { return "shared-scripts" }
    if ($Path -match "^codex-rs/[^/]+/") { return "owner-crates" }
    return "other"
}

function Get-Risk {
    param([string]$Area)
    switch ($Area) {
        "codex-core" { 5 }
        "codex-tui" { 5 }
        "codex-protocol" { 5 }
        "app-server-protocol" { 4 }
        "generated-or-workspace-locks" { 4 }
        "shared-scripts" { 3 }
        "other" { 2 }
        default { 1 }
    }
}

function Test-SeamExists {
    param([string]$RepoRoot, [string]$RelPath)
    if ($RelPath -notmatch "\.rs$") { return $false }
    $stem = $RelPath -replace "\.rs$", ""
    foreach ($suffix in @("_local.rs", "_adapter.rs", "_fork.rs")) {
        $candidate = Join-Path $RepoRoot ($stem + $suffix)
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $true }
    }
    return $false
}

function Get-FileLoc {
    # Line count for a repo-relative path. Defensive: returns 0 on missing/unreadable
    # files (e.g. deleted upstream-only paths or binaries we cannot enumerate).
    param([string]$RepoRoot, [string]$RelPath)
    $full = Join-Path $RepoRoot $RelPath
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { return 0 }
    try {
        return (Get-Content -LiteralPath $full -ErrorAction Stop | Measure-Object -Line).Lines
    }
    catch {
        return 0
    }
}

# --- Preflight: upstream remote must exist ---
$null = Invoke-Git rev-parse --verify --quiet "$UpstreamRef^{commit}"
if ($LASTEXITCODE -ne 0) {
    Write-Error ("Cannot resolve '$UpstreamRef'. Ensure the 'upstream' remote exists and is fetched " +
        "(e.g. `git remote add upstream https://github.com/openai/codex.git; git fetch upstream`), " +
        "or pass -UpstreamRef <ref>.")
    exit 1
}

$mergeBase = (Invoke-Git merge-base $ForkRef $UpstreamRef | Select-Object -First 1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($mergeBase)) {
    Write-Error "Could not compute merge-base of '$ForkRef' and '$UpstreamRef'."
    exit 1
}
$mergeBase = $mergeBase.Trim()
$upstreamSha = (Invoke-Git rev-parse --short $UpstreamRef | Select-Object -First 1).Trim()
$forkSha = (Invoke-Git rev-parse --short $ForkRef | Select-Object -First 1).Trim()

# --- Candidate file set: .rs files with REAL divergence on either side ---
$candidates = [System.Collections.Generic.HashSet[string]]::new()
foreach ($side in @("$mergeBase..$UpstreamRef", "$mergeBase..$ForkRef")) {
    $names = Invoke-Git diff --name-only $side
    foreach ($n in $names) {
        if ([string]::IsNullOrWhiteSpace($n)) { continue }
        if ($n -notmatch "^codex-rs/.*\.rs$") { continue }
        [void]$candidates.Add($n.Trim())
    }
}

if ($candidates.Count -eq 0) {
    Write-Warning "No diverged .rs files found under codex-rs/ between merge-base and either ref."
}

# --- Precompute upstream churn (numstat once for the whole upstream range) ---
$upstreamChurn = @{}
$numstat = Invoke-Git diff --numstat "$mergeBase..$UpstreamRef"
foreach ($line in $numstat) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $parts = $line -split "`t"
    if ($parts.Count -lt 3) { continue }
    $added = if ($parts[0] -eq "-") { 0 } else { [int]$parts[0] }
    $deleted = if ($parts[1] -eq "-") { 0 } else { [int]$parts[1] }
    $path = $parts[-1]
    if ($path -match " => ") { $path = ($path -replace ".* => ", "") -replace "[{}]", "" }
    $upstreamChurn[$path.Trim()] = $added + $deleted
}

# --- Precompute per-file merge-touch counts over recent merges ---
# For each merge commit M (on -ForkRef), a file is "touched on both parents" if it
# appears in `git diff --name-only M^1 M^2`. We tally that across the last -MaxMerges merges.
$mergeTouch = @{}
$mergeCommits = Invoke-Git log --merges --format=%H $ForkRef | Where-Object { $_ -and $_.Trim() } | Select-Object -First $MaxMerges
foreach ($m in $mergeCommits) {
    $m = $m.Trim()
    # Two-parent diff; skip octopus/other merges that lack ^2.
    $touched = Invoke-Git diff --name-only "$m^1" "$m^2" 2>$null
    if ($LASTEXITCODE -ne 0) { continue }
    foreach ($t in $touched) {
        $t = if ($t) { $t.Trim() } else { "" }
        if ($t) { $mergeTouch[$t] = ([int]($mergeTouch[$t])) + 1 }
    }
}

# --- Precompute fork commit counts per file (one batched name-only log) ---
$forkCommitCount = @{}
$forkLog = Invoke-Git log --no-merges --format="C:%H" "$mergeBase..$ForkRef" --name-only
$current = $null
foreach ($line in $forkLog) {
    if ($null -eq $line) { continue }
    if ($line.StartsWith("C:")) { $current = $line; continue }
    $p = $line.Trim()
    if ($p) { $forkCommitCount[$p] = ([int]($forkCommitCount[$p])) + 1 }
}

# --- Build rows ---
$rows = New-Object System.Collections.Generic.List[object]
foreach ($file in $candidates) {
    $area = Get-HotArea $file
    $risk = Get-Risk $area
    $mt = [int]($mergeTouch[$file])
    $uc = [int]($upstreamChurn[$file])
    $fc = [int]($forkCommitCount[$file])
    $seam = Test-SeamExists -RepoRoot $RepoRoot -RelPath $file
    $loc = Get-FileLoc -RepoRoot $RepoRoot -RelPath $file
    $score = ($mt + 1) * ($uc + 1) * $risk
    # SizeChurnScore targets SRP-splits: big files that are ALSO frequently conflicted.
    # Reuses the existing merge-touch metric ($mt / MergeTouches), no parallel signal.
    $sizeChurn = $loc * $mt
    $rows.Add([pscustomobject]@{
        File = $file
        Area = $area
        Loc = $loc
        MergeTouches = $mt
        UpstreamChurn = $uc
        ForkCommits = $fc
        Seam = $seam
        RiskWeight = $risk
        HotspotScore = $score
        SizeChurnScore = $sizeChurn
    })
}

$sorted = $rows | Sort-Object HotspotScore -Descending
$topRows = $sorted | Select-Object -First $Top

# --- Markdown output ---
$timestamp = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine("# Merge hotspot map")
[void]$sb.AppendLine()
[void]$sb.AppendLine("_Generated $timestamp (regenerate with ``scripts/generate-merge-hotspot-map.ps1``)._")
[void]$sb.AppendLine()
[void]$sb.AppendLine("- UpstreamRef: ``$UpstreamRef`` @ ``$upstreamSha``")
[void]$sb.AppendLine("- ForkRef: ``$ForkRef`` @ ``$forkSha``")
[void]$sb.AppendLine("- merge-base: ``$mergeBase``")
[void]$sb.AppendLine("- merges scanned: up to $MaxMerges  |  diverged .rs files: $($rows.Count)  |  showing top $Top")
[void]$sb.AppendLine()
[void]$sb.AppendLine("HotspotScore = (MergeTouches+1) * (UpstreamChurn+1) * RiskWeight.")
[void]$sb.AppendLine()
[void]$sb.AppendLine("| File | Area | LOC | MergeTouches | UpstreamChurn | ForkCommits | Seam | HotspotScore |")
[void]$sb.AppendLine("| --- | --- | ---: | ---: | ---: | ---: | :---: | ---: |")
foreach ($r in $topRows) {
    $seamCell = if ($r.Seam) { "yes" } else { "-" }
    [void]$sb.AppendLine("| ``$($r.File)`` | $($r.Area) | $($r.Loc) | $($r.MergeTouches) | $($r.UpstreamChurn) | $($r.ForkCommits) | $seamCell | $($r.HotspotScore) |")
}

# --- SRP-split candidate section: big AND frequently-conflicted files ---
# SizeChurnScore = LOC * MergeTouches. Splitting these reduces future conflicts most.
$srpRows = $rows |
    Where-Object { $_.SizeChurnScore -gt 0 } |
    Sort-Object SizeChurnScore -Descending |
    Select-Object -First 15
[void]$sb.AppendLine()
[void]$sb.AppendLine("## Top SRP-split candidates (LOC * merge-touch)")
[void]$sb.AppendLine()
[void]$sb.AppendLine("Files that are both large and frequently conflicted; splitting them by responsibility reduces future merge conflicts the most. SizeChurnScore = LOC * MergeTouches.")
[void]$sb.AppendLine()
[void]$sb.AppendLine("| File | LOC | merge-touch | SizeChurnScore |")
[void]$sb.AppendLine("| --- | ---: | ---: | ---: |")
if (@($srpRows).Count -eq 0) {
    [void]$sb.AppendLine("| _(no file has both LOC and merge-touch &gt; 0)_ | - | - | - |")
}
else {
    foreach ($r in $srpRows) {
        [void]$sb.AppendLine("| ``$($r.File)`` | $($r.Loc) | $($r.MergeTouches) | $($r.SizeChurnScore) |")
    }
}

$resolvedOut = if ([System.IO.Path]::IsPathRooted($OutPath)) { $OutPath } else { Join-Path $RepoRoot $OutPath }
$outDir = Split-Path -Parent $resolvedOut
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}
Set-Content -LiteralPath $resolvedOut -Value $sb.ToString() -Encoding UTF8
Write-Host "Wrote markdown hotspot map -> $resolvedOut ($($rows.Count) files, top $Top shown)"

# --- Optional JSON output ---
if ($JsonOut) {
    $payload = [pscustomobject]@{
        generated_at = $timestamp
        upstream_ref = $UpstreamRef
        upstream_sha = $upstreamSha
        fork_ref = $ForkRef
        fork_sha = $forkSha
        merge_base = $mergeBase
        merges_scanned = $MaxMerges
        file_count = $rows.Count
        top = $Top
        hotspots = @($topRows)
    }
    $resolvedJson = if ([System.IO.Path]::IsPathRooted($JsonOut)) { $JsonOut } else { Join-Path $RepoRoot $JsonOut }
    $jsonDir = Split-Path -Parent $resolvedJson
    if ($jsonDir -and -not (Test-Path -LiteralPath $jsonDir)) {
        New-Item -ItemType Directory -Force -Path $jsonDir | Out-Null
    }
    $payload | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $resolvedJson -Encoding UTF8
    Write-Host "Wrote JSON hotspot map -> $resolvedJson"
}
