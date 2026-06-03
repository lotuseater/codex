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

.PARAMETER BlockLevel
    Also run the BLOCK-LEVEL pass (on by default; pass -BlockLevel:$false to skip). The
    file-level pass above misses fork logic that is still inlined INSIDE an upstream-shaped
    function when a sibling seam file ALREADY exists (documented mistake #5). The block-level
    pass scans the upstream-hot .rs sources for the fork-feature markers
    (collaboration_mode, context_budget_mode, personality, SemanticCheckpoint /
    semantic_compact, ForkFeaturesState, fork_features) and reports each occurrence that
    appears OUTSIDE a designated seam file (*_local.rs / *_adapter.rs / *_fork.rs) as a
    "block-level gap" -> file:line: <marker>. Occurrences INSIDE seam files are expected and
    are NOT reported. It also flags the TUI `PersistPersonalitySelection` match arm when its
    body is real inline logic rather than a one-line delegate into event_dispatch_local.rs.

.PARAMETER BlockLevelScope
    Which files the block-level pass scans:
        Named (default) - only the known un-extracted spots (turn.rs, event_dispatch.rs,
                          tasks/mod.rs). This is the actionable list the orchestrator uses to
                          verify Phase B extractions drive the gap count to zero.
        All             - additionally sweep every fork-changed upstream-hot .rs file for the
                          markers. Much noisier (markers like `personality` appear legitimately
                          in upstream struct fields / config plumbing); use for a broad audit,
                          not as a gate.

.PARAMETER FailOnGaps
    By default the block-level pass is ADVISORY: it reports gaps but does not change the exit
    code (so it never breaks the preflight). Pass -FailOnGaps to make the block-level pass
    contribute to a non-zero exit when block-level gaps are found.
#>
param(
    [string]$UpstreamRef = "upstream/main",
    [int]$MinForkCommits = 3,
    [string]$OutPath,
    [string]$ForkRef = "HEAD",
    [string]$RepoRoot = "C:\Users\Oleh\Documents\GitHub\open_ai\codex",
    [switch]$BlockLevel = $true,
    [ValidateSet("Named", "All")]
    [string]$BlockLevelScope = "Named",
    [switch]$FailOnGaps
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

# --- Block-level (intra-function) fork-residue detection ---------------------
# A "seam file" is where fork divergence is SUPPOSED to live. Occurrences of fork
# markers inside these are EXPECTED, not gaps. Any *_local.rs / *_adapter.rs /
# *_fork.rs sibling counts as a seam; the two canonical ones are named explicitly.
function Test-IsSeamFile {
    param([string]$RelPath)
    $p = $RelPath -replace "\\", "/"
    if ($p -match "_(local|adapter|fork)\.rs$") { return $true }
    return $false
}

# Fork-only identifiers (brief). semantic_compact is the snake_case spelling of the
# SemanticCheckpoint feature as it appears inlined in tasks/mod.rs.
$script:ForkMarkerPatterns = [ordered]@{
    "collaboration_mode"  = "collaboration_mode"
    "context_budget_mode" = "context_budget_mode"
    "personality"         = "personality"
    "SemanticCheckpoint"  = "SemanticCheckpoint"
    "semantic_compact"    = "semantic_compact"
    "ForkFeaturesState"   = "ForkFeaturesState"
    "fork_features"       = "fork_features"
}

# Files the block-level pass MUST scan even if they have a sibling seam (the known
# un-extracted spots — documented mistake #5). Relative to RepoRoot, forward slashes.
$script:BlockLevelScanFiles = @(
    "codex-rs/core/src/session/turn.rs"
    "codex-rs/tui/src/app/event_dispatch.rs"
    "codex-rs/core/src/tasks/mod.rs"
)

function Get-BlockLevelMarkerGaps {
    param([string]$RepoRoot, [string[]]$RelFiles)
    $hits = New-Object System.Collections.Generic.List[object]
    foreach ($rel in $RelFiles) {
        if (Test-IsSeamFile $rel) { continue }   # expected home for fork logic
        $full = Join-Path $RepoRoot ($rel -replace "/", [System.IO.Path]::DirectorySeparatorChar)
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { continue }
        $lineNo = 0
        foreach ($line in (Get-Content -LiteralPath $full)) {
            $lineNo++
            $trimmed = $line.Trim()
            # A line that DELEGATES into a seam module (event_dispatch_local::,
            # *_adapter::, *_fork::) is the GOOD pattern (a reference to the seam, not
            # inline fork logic) -> not a gap.
            if ($trimmed -match "_(local|adapter|fork)::") { continue }
            # Pure imports are not inline logic.
            if ($trimmed -match "^\s*use\s") { continue }
            foreach ($name in $script:ForkMarkerPatterns.Keys) {
                $pat = $script:ForkMarkerPatterns[$name]
                # Require the literal token to appear (substring match keeps the family,
                # e.g. personality_label, *_for_semantic_compact).
                if ($line -match [regex]::Escape($pat)) {
                    $hits.Add([pscustomobject]@{
                        File   = $rel
                        Line   = $lineNo
                        Marker = $name
                        Text   = $trimmed
                    })
                }
            }
        }
    }
    return $hits
}

# The TUI PersistPersonalitySelection arm SHOULD be a one-line delegate into
# event_dispatch_local.rs (like PersistContextBudgetModeSelection). Flag it when the
# arm body holds real inline logic instead. Heuristic: locate the arm header and count
# non-trivial body lines until the matching close; a clean delegate has <= 1.
function Get-PersistPersonalityArmGap {
    param([string]$RepoRoot)
    $rel = "codex-rs/tui/src/app/event_dispatch.rs"
    $full = Join-Path $RepoRoot ($rel -replace "/", [System.IO.Path]::DirectorySeparatorChar)
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { return $null }
    $allLines = Get-Content -LiteralPath $full
    for ($i = 0; $i -lt $allLines.Count; $i++) {
        if ($allLines[$i] -match "AppEvent::PersistPersonalitySelection\b") {
            $armLine = $i + 1
            # Walk forward counting brace depth from the arm header; the body ends when
            # depth returns to 0. Count "real" body lines (non-blank, non-brace-only,
            # non-comment) to distinguish a delegate from inlined logic.
            $depth = 0
            $started = $false
            $bodyLines = 0
            for ($j = $i; $j -lt $allLines.Count -and $j -lt ($i + 200); $j++) {
                $l = $allLines[$j]
                $opens = ([regex]::Matches($l, "\{")).Count
                $closes = ([regex]::Matches($l, "\}")).Count
                if ($opens -gt 0) { $started = $true }
                if ($started -and $j -gt $i) {
                    $t = $l.Trim()
                    if ($t -ne "" -and $t -notmatch "^[\{\}]+,?$" -and $t -notmatch "^//") {
                        $bodyLines++
                    }
                }
                $depth += $opens - $closes
                if ($started -and $depth -le 0 -and $j -gt $i) { break }
            }
            if ($bodyLines -gt 1) {
                return [pscustomobject]@{
                    File      = $rel
                    Line      = $armLine
                    BodyLines = $bodyLines
                    Text      = "PersistPersonalitySelection arm has ~$bodyLines lines of inline logic (expected a one-line delegate into event_dispatch_local.rs)"
                }
            }
            return $null
        }
    }
    return $null
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
# --- Block-level pass (intra-function fork residue) --------------------------
$blockGapCount = 0
if ($BlockLevel) {
    # Default scope (Named): only the known un-extracted spots -> a tight, actionable list
    # the orchestrator can drive to zero. Scope=All additionally sweeps every fork-changed
    # upstream-hot .rs file (noisier; broad audit only). Seam files are always excluded.
    $scanSet = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($f in $script:BlockLevelScanFiles) { $null = $scanSet.Add($f) }
    if ($BlockLevelScope -eq "All") {
        foreach ($file in $changed) {
            if ([string]::IsNullOrWhiteSpace($file)) { continue }
            $file = $file.Trim()
            if ($file -notmatch "\.rs$") { continue }
            if (Test-IsSeamFile $file) { continue }
            if (-not (Get-HotAreaLabel $file)) { continue }
            $null = $scanSet.Add($file)
        }
    }
    $scanFiles = @($scanSet) | Sort-Object

    $markerHits = Get-BlockLevelMarkerGaps -RepoRoot $RepoRoot -RelFiles $scanFiles
    $armGap = Get-PersistPersonalityArmGap -RepoRoot $RepoRoot

    $sortedHits = $markerHits | Sort-Object File, Line
    $blockGapCount = $sortedHits.Count + (@($armGap).Where({ $_ }).Count)

    $lines.Add("")
    $lines.Add("## Block-level fork residue (inline markers outside seam files)")
    $lines.Add("")
    $lines.Add("Scope: $BlockLevelScope ($($scanFiles.Count) file(s) scanned)")
    $lines.Add("Markers: " + ($script:ForkMarkerPatterns.Keys -join ", "))
    $lines.Add("Seam files (occurrences there are EXPECTED, not gaps): *_local.rs / *_adapter.rs / *_fork.rs")
    $lines.Add("")
    if ($blockGapCount -eq 0) {
        $lines.Add("No block-level gaps found. (No fork markers inlined in upstream-shaped files.)")
    } else {
        $lines.Add("Block-level gaps: $blockGapCount")
        $lines.Add("")
        $lines.Add("| File:Line | Marker | Snippet |")
        $lines.Add("| --- | --- | --- |")
        foreach ($h in $sortedHits) {
            $snippet = $h.Text
            if ($snippet.Length -gt 80) { $snippet = $snippet.Substring(0, 77) + "..." }
            $snippet = $snippet -replace "\|", "\|"
            $lines.Add("| $($h.File):$($h.Line) | $($h.Marker) | $snippet |")
        }
        if ($armGap) {
            $lines.Add("| $($armGap.File):$($armGap.Line) | PersistPersonalitySelection-arm | $($armGap.Text) |")
        }
        $lines.Add("")
        $lines.Add("Block-level gap list (file:line: marker):")
        foreach ($h in $sortedHits) {
            $lines.Add("  $($h.File):$($h.Line): $($h.Marker)")
        }
        if ($armGap) {
            $lines.Add("  $($armGap.File):$($armGap.Line): PersistPersonalitySelection-arm-inline")
        }
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

if ($BlockLevel) {
    Write-Host ""
    if ($blockGapCount -gt 0) {
        $advisory = if ($FailOnGaps) { "" } else { " (advisory)" }
        Write-Host "BLOCK-LEVEL: $blockGapCount inline fork-residue gap(s) found$advisory" -ForegroundColor Yellow
    } else {
        Write-Host "BLOCK-LEVEL: no inline fork-residue gaps." -ForegroundColor Green
    }
}

# Exit policy:
#   - File-level gaps always gate (exit 1) — that is the script's original CI contract.
#   - Block-level gaps are ADVISORY by default (no effect on exit) so they never break the
#     preflight; they only contribute to a non-zero exit when -FailOnGaps is passed.
if ($sorted.Count -gt 0) {
    Write-Host ""
    Write-Host "GATE: $($sorted.Count) adapter gap(s) found -> exit 1" -ForegroundColor Yellow
    exit 1
}
if ($FailOnGaps -and $blockGapCount -gt 0) {
    Write-Host ""
    Write-Host "GATE: $blockGapCount block-level gap(s) + -FailOnGaps -> exit 1" -ForegroundColor Yellow
    exit 1
}
exit 0
