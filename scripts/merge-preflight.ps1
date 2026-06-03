<#
.SYNOPSIS
    One-shot, READ-ONLY preflight for an `upstream/main -> fork` merge. Replaces the six
    manual pre-merge steps (fetch, ahead/behind, conflict rehearsal, area grouping, hotspot
    map, adapter-gap scan) and writes a single Markdown report. It NEVER performs the merge.

.DESCRIPTION
    Behavior (all read-only — only `git merge-tree`, never `git merge`):
      1. Unless -NoFetch, `git fetch upstream main`.
      2. Report current branch, HEAD sha, -UpstreamRef sha, merge-base, ahead/behind
         (`git rev-list --left-right --count HEAD...<upstream>`), and count of upstream
         commits not in HEAD.
      3. Conflict rehearsal WITHOUT touching the tree: `git merge-tree --write-tree HEAD
         <upstream>` (modern git) for the authoritative conflicted-file list + count; falls
         back to `git merge-tree <base> HEAD <upstream>` if --write-tree is unavailable. The
         legacy form is also run (best-effort) to count inline `<<<<<<<` conflict markers,
         which the --write-tree form does not emit.
      4. Group conflicted files by AREA via path prefixes; print a per-area count table.
      5. Best-effort call scripts/generate-merge-hotspot-map.ps1 and
         scripts/detect-adapter-gaps.ps1 (try/catch; never fails the preflight).
      6. Write a Markdown report to -OutFile (default
         .codex/tmp/merge_preflight_<yyyy-MM-dd_HHmm>.md) and echo a terse summary.
      7. Always exit 0 (informational). Prints a "nothing to merge" banner when 0 behind.

.NOTES
    Read-only. Never runs cargo/builds, `git merge`, or any mutating git other than the
    optional `git fetch`. Uses `git -C <RepoRoot>` so it works from any cwd.
#>
[CmdletBinding()]
param(
    [string]$UpstreamRef = "upstream/main",
    [switch]$NoFetch,
    [string]$OutFile,
    [string]$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"
# git is used heavily with EXPECTED non-zero exits (merge-tree returns 1 when conflicts
# exist; rev-parse --verify --quiet returns 1 for a missing ref). We check $LASTEXITCODE
# ourselves, so don't let pwsh 7.4+ turn those into terminating errors.
$PSNativeCommandUseErrorActionPreference = $false

function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$GitArgs)
    & git -C $RepoRoot @GitArgs
}

# --- Area grouping: path-prefix -> area label. Order matters: most specific first. ---
function Get-MergeArea {
    param([string]$Path)
    switch -Regex ($Path) {
        '^codex-rs/core/src/session/'      { return "core-session" }
        '^codex-rs/core/src/tools/'        { return "core-tools" }
        '^codex-rs/core/'                  { return "core-other" }
        '^codex-rs/protocol/'              { return "protocol" }
        '^codex-rs/app-server-protocol/'   { return "app-server-protocol" }
        '^codex-rs/app-server/'            { return "app-server" }
        '^codex-rs/config/'                { return "config" }
        '^codex-rs/tui-render'             { return "tui" }
        '^codex-rs/tui/'                   { return "tui" }
        '^codex-rs/analytics'              { return "analytics" }
        '(^|/)Cargo\.(lock|toml)$'         { return "manifests" }
        '\.json$'                          { return "manifests" }
        '^\.github/'                       { return "ci-infra" }
        '(^|/)\.bazelrc$'                  { return "ci-infra" }
        '(^|/)(MODULE\.bazel(\.lock)?|BUILD\.bazel|.*\.bazel)$' { return "ci-infra" }
        'bazel'                            { return "ci-infra" }
        default                            { return "other" }
    }
}

# Canonical area order for the table (stable, readable).
$AreaOrder = @(
    "core-session", "core-tools", "core-other", "protocol", "app-server-protocol",
    "app-server", "config", "tui", "analytics", "manifests", "ci-infra", "other"
)

# --- Preflight: upstream ref must resolve ---
$null = Invoke-Git rev-parse --verify --quiet "$UpstreamRef^{commit}"
if ($LASTEXITCODE -ne 0) {
    # Try a fetch first if allowed; otherwise hard-stop with guidance.
    if (-not $NoFetch) {
        Write-Host "[*] '$UpstreamRef' not resolvable yet; attempting fetch..." -ForegroundColor Yellow
    } else {
        Write-Error ("Cannot resolve '$UpstreamRef'. Ensure the 'upstream' remote exists and is " +
            "fetched (git remote add upstream https://github.com/openai/codex.git; git fetch upstream), " +
            "or pass -UpstreamRef <ref>. (-NoFetch was set, so no fetch was attempted.)")
        exit 0
    }
}

# --- Step 1: fetch (unless -NoFetch) ---
$fetchNote = ""
if ($NoFetch) {
    $fetchNote = "skipped (-NoFetch)"
    Write-Host "[*] Fetch skipped (-NoFetch)."
} else {
    Write-Host "[*] git fetch upstream main ..."
    Invoke-Git fetch upstream main 2>&1 | Out-Host
    if ($LASTEXITCODE -ne 0) {
        $fetchNote = "FAILED (exit $LASTEXITCODE) — using whatever ref is already present"
        Write-Warning "git fetch upstream main failed (exit $LASTEXITCODE); continuing with existing refs."
    } else {
        $fetchNote = "ok"
    }
}

# Re-verify upstream ref now that a fetch may have happened.
$null = Invoke-Git rev-parse --verify --quiet "$UpstreamRef^{commit}"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cannot resolve '$UpstreamRef' even after fetch. Aborting preflight."
    exit 0
}

# --- Step 2: branch / sha / merge-base / ahead-behind ---
$currentBranch = (Invoke-Git rev-parse --abbrev-ref HEAD | Select-Object -First 1).Trim()
$headSha       = (Invoke-Git rev-parse HEAD | Select-Object -First 1).Trim()
$headShort     = (Invoke-Git rev-parse --short HEAD | Select-Object -First 1).Trim()
$upstreamSha   = (Invoke-Git rev-parse "$UpstreamRef" | Select-Object -First 1).Trim()
$upstreamShort = (Invoke-Git rev-parse --short "$UpstreamRef" | Select-Object -First 1).Trim()

$mergeBase = (Invoke-Git merge-base HEAD "$UpstreamRef" | Select-Object -First 1)
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($mergeBase)) {
    Write-Error "Could not compute merge-base of HEAD and '$UpstreamRef'."
    exit 0
}
$mergeBase = $mergeBase.Trim()

# rev-list --left-right --count HEAD...UPSTREAM => "<ahead>\t<behind>"
$ahead = 0; $behind = 0
$counts = (Invoke-Git rev-list --left-right --count "HEAD...$UpstreamRef" | Select-Object -First 1)
if (-not [string]::IsNullOrWhiteSpace($counts)) {
    $cparts = $counts.Trim() -split "\s+"
    if ($cparts.Count -ge 2) {
        $ahead  = [int]$cparts[0]
        $behind = [int]$cparts[1]
    }
}
# Upstream commits not in HEAD (== behind, but computed independently as a cross-check).
$upstreamOnly = 0
$cntOut = (Invoke-Git rev-list --count "HEAD..$UpstreamRef" | Select-Object -First 1)
if (-not [string]::IsNullOrWhiteSpace($cntOut)) { $upstreamOnly = [int]$cntOut.Trim() }

$nothingToMerge = ($behind -eq 0)

# --- Step 3: conflict rehearsal via git merge-tree (NO working-tree mutation) ---
$rehearsalMode = "write-tree"
$conflictFiles = New-Object System.Collections.Generic.List[string]
$mergedTreeOid = ""
$conflictMessages = New-Object System.Collections.Generic.List[string]
$conflictTypeCounts = @{}
$markerCount = 0
$rehearsalNote = ""

# 3a. Modern form: `git merge-tree --write-tree HEAD <upstream>`.
#     Exit code != 0 here means "conflicts present", NOT a tool failure.
$wt = Invoke-Git merge-tree --write-tree HEAD "$UpstreamRef" 2>&1
$wtExit = $LASTEXITCODE
$wtLines = @($wt | ForEach-Object { [string]$_ })

# Detect whether --write-tree is supported (older git errors with "unknown option").
$writeTreeUnsupported = ($wtLines -join "`n") -match "unknown (option|switch)|usage: git merge-tree"

if (-not $writeTreeUnsupported -and $wtLines.Count -gt 0) {
    # Section 1 line 1 = merged tree OID. Following info lines:
    #   "<mode> <oid> <stage>\t<path>"  (stage 1/2/3). A blank line ends the info section;
    # the remainder is human-readable messages (CONFLICT (...) / Auto-merging ...).
    if ($wtLines[0] -match '^[0-9a-f]{40}$') { $mergedTreeOid = $wtLines[0] }
    $seen = [System.Collections.Generic.HashSet[string]]::new()
    $inInfo = $true
    for ($i = 1; $i -lt $wtLines.Count; $i++) {
        $line = $wtLines[$i]
        if ($inInfo) {
            if ([string]::IsNullOrWhiteSpace($line)) { $inInfo = $false; continue }
            # "<mode> <oid> <stage>\t<path>"
            if ($line -match '^\d{6}\s+[0-9a-f]{40}\s+[123]\t(.+)$') {
                $p = $matches[1].Trim()
                if ($seen.Add($p)) { [void]$conflictFiles.Add($p) }
            }
            continue
        }
        # Message section.
        if ($line -match '^CONFLICT \(([^)]+)\):') {
            $ctype = $matches[1].Trim()
            $conflictTypeCounts[$ctype] = [int]($conflictTypeCounts[$ctype]) + 1
            [void]$conflictMessages.Add($line.Trim())
        }
    }
} else {
    $writeTreeUnsupported = $true
}

# 3b. Legacy fallback / marker source: `git merge-tree <base> HEAD <upstream>`.
#     This diff-style output carries inline conflict markers (often prefixed `+<<<<<<<`),
#     which --write-tree does NOT emit. We always run it best-effort to count markers; if
#     --write-tree was unsupported, we also derive the conflicted-file list from it.
try {
    $legacy = Invoke-Git merge-tree $mergeBase HEAD "$UpstreamRef" 2>&1
    $legacyLines = @($legacy | ForEach-Object { [string]$_ })
    # Count conflict markers: any line whose content (ignoring a leading +/-/space diff
    # prefix) begins with seven '<'.
    foreach ($ln in $legacyLines) {
        if ($ln -match '^[+\- ]?<<<<<<<') { $markerCount++ }
    }
    if ($writeTreeUnsupported) {
        $rehearsalMode = "legacy"
        # Legacy headers like "changed in both" precede a "  our  100644 ... <path>" block;
        # the simplest robust signal is the inline CONFLICT lines + the section headers.
        $seen2 = [System.Collections.Generic.HashSet[string]]::new()
        foreach ($ln in $legacyLines) {
            if ($ln -match '^CONFLICT \(([^)]+)\):.*\bin\b\s+(.+)$') {
                $ctype = $matches[1].Trim()
                $p = $matches[2].Trim()
                $conflictTypeCounts[$ctype] = [int]($conflictTypeCounts[$ctype]) + 1
                if ($seen2.Add($p)) { [void]$conflictFiles.Add($p) }
            }
        }
        $rehearsalNote = "used legacy `git merge-tree <base> HEAD <upstream>` (--write-tree unavailable)"
    }
} catch {
    $rehearsalNote = "legacy merge-tree marker pass failed: $($_.Exception.Message)"
}

$conflictCount = $conflictFiles.Count

# --- Step 4: group conflicted files by area ---
$areaCounts = [ordered]@{}
foreach ($a in $AreaOrder) { $areaCounts[$a] = 0 }
foreach ($f in $conflictFiles) {
    $a = Get-MergeArea $f
    if (-not $areaCounts.Contains($a)) { $areaCounts[$a] = 0 }
    $areaCounts[$a] = [int]$areaCounts[$a] + 1
}

# --- Step 5: best-effort sibling reports (hotspot map + adapter gaps) ---
$ts = Get-Date
$stamp = $ts.ToString("yyyy-MM-dd_HHmm")
$tmpDir = Join-Path $RepoRoot ".codex/tmp"
if (-not (Test-Path -LiteralPath $tmpDir)) { New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null }

$hotspotScript = Join-Path $RepoRoot "scripts/generate-merge-hotspot-map.ps1"
$adapterScript = Join-Path $RepoRoot "scripts/detect-adapter-gaps.ps1"

$hotspotOut = Join-Path $tmpDir "merge_preflight_hotspots_$stamp.md"
$adapterOut = Join-Path $tmpDir "merge_preflight_adapter_gaps_$stamp.md"
$hotspotNote = ""
$adapterNote = ""
$hotspotTop = New-Object System.Collections.Generic.List[string]
$adapterRows = New-Object System.Collections.Generic.List[string]

if (Test-Path -LiteralPath $hotspotScript) {
    try {
        & pwsh -NoProfile -File $hotspotScript -UpstreamRef $UpstreamRef -OutPath $hotspotOut -Top 15 2>&1 | Out-Null
        if (Test-Path -LiteralPath $hotspotOut) {
            $hotspotNote = "ok -> $hotspotOut"
            # Lift the table rows (markdown lines beginning with '| `codex-rs').
            $hsContent = Get-Content -LiteralPath $hotspotOut
            foreach ($l in $hsContent) {
                if ($l -match '^\|\s*`') { $hotspotTop.Add($l) }
            }
        } else {
            $hotspotNote = "ran but produced no output file"
        }
    } catch {
        $hotspotNote = "ERROR: $($_.Exception.Message)"
    }
} else {
    $hotspotNote = "not present (skipped)"
}

if (Test-Path -LiteralPath $adapterScript) {
    try {
        # detect-adapter-gaps exits 1 when gaps are found; that is NOT an error for us.
        & pwsh -NoProfile -File $adapterScript -UpstreamRef $UpstreamRef -OutPath $adapterOut 2>&1 | Out-Null
        if (Test-Path -LiteralPath $adapterOut) {
            $adapterNote = "ok -> $adapterOut"
            $agContent = Get-Content -LiteralPath $adapterOut
            foreach ($l in $agContent) {
                if ($l -match '^\|') { $adapterRows.Add($l) }
            }
        } else {
            $adapterNote = "ran but produced no output file"
        }
    } catch {
        $adapterNote = "ERROR: $($_.Exception.Message)"
    }
} else {
    $adapterNote = "not present (skipped)"
}

# --- Step 6: write Markdown report ---
if (-not $OutFile) {
    $OutFile = Join-Path $tmpDir "merge_preflight_$stamp.md"
}
$resolvedOut = if ([System.IO.Path]::IsPathRooted($OutFile)) { $OutFile } else { Join-Path $RepoRoot $OutFile }
$outDir = Split-Path -Parent $resolvedOut
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine("# Merge preflight — HEAD <- $UpstreamRef")
[void]$sb.AppendLine()
[void]$sb.AppendLine("_Generated $($ts.ToString('yyyy-MM-dd HH:mm:ss')) by ``scripts/merge-preflight.ps1`` (read-only; no merge performed)._")
[void]$sb.AppendLine()

if ($nothingToMerge) {
    [void]$sb.AppendLine("> **NOTHING TO MERGE** — HEAD is 0 commits behind ``$UpstreamRef``.")
    [void]$sb.AppendLine()
}

[void]$sb.AppendLine("## Position")
[void]$sb.AppendLine()
[void]$sb.AppendLine("| Field | Value |")
[void]$sb.AppendLine("| --- | --- |")
[void]$sb.AppendLine("| Current branch | ``$currentBranch`` |")
[void]$sb.AppendLine("| HEAD | ``$headShort`` ($headSha) |")
[void]$sb.AppendLine("| $UpstreamRef | ``$upstreamShort`` ($upstreamSha) |")
[void]$sb.AppendLine("| merge-base | ``$mergeBase`` |")
[void]$sb.AppendLine("| ahead / behind | **$ahead** ahead / **$behind** behind |")
[void]$sb.AppendLine("| upstream commits not in HEAD | $upstreamOnly |")
[void]$sb.AppendLine("| fetch | $fetchNote |")
[void]$sb.AppendLine()

[void]$sb.AppendLine("## Conflict rehearsal (git merge-tree — no tree mutation)")
[void]$sb.AppendLine()
[void]$sb.AppendLine("- Rehearsal mode: **$rehearsalMode**" + $(if ($rehearsalNote) { " — $rehearsalNote" } else { "" }))
if ($mergedTreeOid) { [void]$sb.AppendLine("- Merged tree OID (would-be result): ``$mergedTreeOid``") }
[void]$sb.AppendLine("- Conflicted files: **$conflictCount**")
[void]$sb.AppendLine("- Total conflict markers (``<<<<<<<``) in rehearsal: **$markerCount**")
if ($conflictTypeCounts.Count -gt 0) {
    $typeStr = ($conflictTypeCounts.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Key): $($_.Value)" }) -join ", "
    [void]$sb.AppendLine("- Conflict types: $typeStr")
}
[void]$sb.AppendLine()

[void]$sb.AppendLine("### Conflicts by area")
[void]$sb.AppendLine()
[void]$sb.AppendLine("| Area | Conflicted files |")
[void]$sb.AppendLine("| --- | ---: |")
foreach ($k in $areaCounts.Keys) {
    if ([int]$areaCounts[$k] -gt 0) {
        [void]$sb.AppendLine("| $k | $($areaCounts[$k]) |")
    }
}
[void]$sb.AppendLine("| **TOTAL** | **$conflictCount** |")
[void]$sb.AppendLine()

[void]$sb.AppendLine("## Top hotspots")
[void]$sb.AppendLine()
[void]$sb.AppendLine("Source: ``scripts/generate-merge-hotspot-map.ps1`` — $hotspotNote")
[void]$sb.AppendLine()
if ($hotspotTop.Count -gt 0) {
    [void]$sb.AppendLine("| File | Area | MergeTouches | UpstreamChurn | ForkCommits | Seam | HotspotScore |")
    [void]$sb.AppendLine("| --- | --- | ---: | ---: | ---: | :---: | ---: |")
    foreach ($r in $hotspotTop) { [void]$sb.AppendLine($r) }
} else {
    [void]$sb.AppendLine("_(no hotspot rows captured)_")
}
[void]$sb.AppendLine()

[void]$sb.AppendLine("## Adapter gaps")
[void]$sb.AppendLine()
[void]$sb.AppendLine("Source: ``scripts/detect-adapter-gaps.ps1`` — $adapterNote")
[void]$sb.AppendLine()
if ($adapterRows.Count -gt 0) {
    foreach ($r in $adapterRows) { [void]$sb.AppendLine($r) }
} else {
    [void]$sb.AppendLine("_(no adapter-gap table captured)_")
}
[void]$sb.AppendLine()

[void]$sb.AppendLine("## Raw conflicted file list ($conflictCount)")
[void]$sb.AppendLine()
if ($conflictFiles.Count -gt 0) {
    foreach ($f in ($conflictFiles | Sort-Object)) {
        [void]$sb.AppendLine("- ``$f``")
    }
} else {
    [void]$sb.AppendLine("_(none — merge-tree rehearsal reported no conflicts)_")
}
[void]$sb.AppendLine()

Set-Content -LiteralPath $resolvedOut -Value $sb.ToString() -Encoding UTF8

# --- Terse stdout summary ---
Write-Host ""
if ($nothingToMerge) {
    Write-Host "================ NOTHING TO MERGE (0 behind $UpstreamRef) ================" -ForegroundColor Green
}
Write-Host "Merge preflight: $currentBranch @ $headShort  <-  $UpstreamRef @ $upstreamShort" -ForegroundColor Cyan
Write-Host "  ahead/behind : $ahead / $behind   (upstream-only commits: $upstreamOnly)"
Write-Host "  rehearsal    : $rehearsalMode   conflicted files: $conflictCount   markers(<<<<<<<): $markerCount"
if ($conflictTypeCounts.Count -gt 0) {
    $typeStr = ($conflictTypeCounts.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join " "
    Write-Host "  conflict types: $typeStr"
}
Write-Host "  by area      :" -NoNewline
$first = $true
foreach ($k in $areaCounts.Keys) {
    if ([int]$areaCounts[$k] -gt 0) {
        Write-Host " $k=$($areaCounts[$k])" -NoNewline
        $first = $false
    }
}
if ($first) { Write-Host " (none)" -NoNewline }
Write-Host ""
Write-Host "  hotspots     : $hotspotNote"
Write-Host "  adapter gaps : $adapterNote"
Write-Host "Report -> $resolvedOut" -ForegroundColor Cyan

# Informational tool: always exit 0.
exit 0
