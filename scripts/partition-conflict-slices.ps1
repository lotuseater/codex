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

.PARAMETER EmitBriefs
    After writing the JSON, ALSO generate one ready-to-paste resolver brief per non-empty
    slice at `.codex/tmp/slice_<area>_brief.md`. Each brief lists the slice's files by
    ABSOLUTE path, the fork features at risk (parsed from `docs/fork-feature-inventory.md`),
    any modify/delete conflicts (when a live merge is in progress), and a HANDOFF CONTRACT
    skeleton (union-preserve policy, leave edits unstaged). Opt-in; off by default so the
    existing JSON-only behavior is unchanged.

.PARAMETER InventoryFile
    Path to the fork-feature inventory used to build the path/crate -> feature mapping for
    `-EmitBriefs`. Default: `docs/fork-feature-inventory.md` under repo root. If missing,
    brief generation warns and still emits paths (with a keyword-fallback feature scan).

.EXAMPLE
    pwsh -File scripts/partition-conflict-slices.ps1 -FromFile .codex/tmp/sample_conflicts.txt -EmitBriefs
    Partition a fixed list AND emit one resolver brief per non-empty slice.
#>
[CmdletBinding()]
param(
    [string]$FromFile,
    [string]$OutFile,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
    [switch]$EmitBriefs,
    [string]$InventoryFile
)

$ErrorActionPreference = "Stop"

if (-not $OutFile) { $OutFile = Join-Path $RepoRoot ".codex/tmp/conflict_slices.json" }
if (-not $InventoryFile) { $InventoryFile = Join-Path $RepoRoot "docs/fork-feature-inventory.md" }

# Absolute repo-root prefix used when rendering ABSOLUTE paths into briefs.
$repoRootAbs = $RepoRoot.TrimEnd('\', '/')

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

# --- Brief generation: fork-feature mapping + emitter -----------------------
# These functions are only exercised when -EmitBriefs is supplied. They are kept
# defensive: a missing/loose inventory never throws -- briefs still emit paths.

# Build a list of { Feature; Tokens=[substring,...] } rules by scanning the
# fork-feature inventory's "Owner surfaces" column + Modularization Contract,
# plus a fixed set of known fork markers as a keyword fallback.
function Get-ForkFeatureRules {
    param([string]$InventoryPath)

    $rules = New-Object System.Collections.Generic.List[object]

    # Always-on keyword fallback: known fork markers -> matched as path/crate substrings.
    # (Lowercased substrings; matched case-insensitively against the lowercased path.)
    $fallback = [ordered]@{
        "collaboration_mode"  = @("collaboration_mode", "collaborationmode")
        "context_budget_mode" = @("context_budget", "context-budget", "contextbudget", "slow-context", "context-pack")
        "personality"         = @("personality")
        "fork_features"       = @("fork_features", "fork-local", "codex-rs/features")
        "semantic checkpoint" = @("semantic", "context-ops", "context_ops", "replacement-shadow", "replacement_shadow")
        "blackboard"          = @("blackboard")
        "first_moves"         = @("first-moves", "first_moves")
        "repo_context_scout"  = @("repo-context-scout", "repo_context_scout", "context-scout")
        "multi_agent_v2"      = @("multi_agents_v2", "multi-agents", "multi_agents", "multiagent")
        "task_memory"         = @("task-memory", "task_memory")
        "guardian"            = @("guardian", "review_session")
        "turn-diff"           = @("turn-diff", "turn_diff")
        "analytics session_id"= @("analytics")
        "self-review"         = @("self-review", "self_review")
        "operation-cache"     = @("operation-cache", "operation_cache")
        "session limit footer"= @("session_limit_footer")
        "cognos-ops"          = @("cognos-ops", "cognos_ops")
        "desktop-automation"  = @("desktop-automation", "desktop_automation")
        "reasoning-logic"     = @("reasoning-logic", "reasoning_logic")
    }
    foreach ($k in $fallback.Keys) {
        $rules.Add([pscustomobject]@{ Feature = $k; Tokens = @($fallback[$k]) })
    }

    if (-not (Test-Path -LiteralPath $InventoryPath -PathType Leaf)) {
        Write-Warning "Fork-feature inventory not found: $InventoryPath (using keyword fallback only)."
        return $rules
    }

    # Parse the inventory's feature-family table rows of the form:
    #   | Feature family | commits | Owner surfaces | health checks |
    # and pull crate/path-like tokens out of the "Owner surfaces" cell, plus any
    # owner-crate names from the "Current Modularization Contract" bullet list.
    $inv = Get-Content -LiteralPath $InventoryPath -ErrorAction SilentlyContinue
    if (-not $inv) { return $rules }

    # Regex for a path/crate-ish token (backticked or bare): codex-rs/..., codex-xxx, scripts/..., *.rs files.
    $tokenRx = [regex]'`?((?:codex-rs/[^`,\s|]+)|(?:codex-[a-z0-9\-]+)|(?:scripts/[^`,\s|]+)|(?:\.codex/[^`,\s|]+)|(?:[a-z0-9_\-]+/[a-z0-9_\-/]+\.rs)|(?:config-types|permission-types|git-types|thread-config-remote|model-provider-info))`?'

    foreach ($line in $inv) {
        $cells = $line -split '\|'
        if ($cells.Count -lt 4) { continue }
        $feature = $cells[1].Trim()
        if (-not $feature -or $feature -match '^-+$' -or $feature -eq "Feature family") { continue }
        # Owner surfaces is the 4th pipe cell (index 3) in the 4-column table.
        $owner = $cells[3]
        $toks = New-Object System.Collections.Generic.List[string]
        foreach ($m in $tokenRx.Matches($owner)) {
            $tok = $m.Groups[1].Value.Trim().TrimEnd('.', ',').ToLowerInvariant()
            # Strip a trailing /* glob and the leading codex-rs/ so "codex-rs/core/src/x" also matches the crate dir.
            $tok = $tok -replace '/\*$', ''
            if ($tok.Length -ge 4 -and -not $toks.Contains($tok)) { $toks.Add($tok) }
        }
        if ($toks.Count -gt 0) {
            $rules.Add([pscustomobject]@{ Feature = $feature; Tokens = @($toks) })
        }
    }

    return $rules
}

# Given a repo-relative path and the rules, return the distinct matched feature names.
function Get-FeaturesForFile {
    param([string]$Path, [System.Collections.Generic.List[object]]$Rules)
    $pl = ($Path -replace '\\', '/').ToLowerInvariant()
    $hits = New-Object System.Collections.Generic.List[string]
    foreach ($rule in $Rules) {
        foreach ($t in $rule.Tokens) {
            if (-not $t) { continue }
            if ($pl.Contains($t)) {
                if (-not $hits.Contains($rule.Feature)) { $hits.Add($rule.Feature) }
                break
            }
        }
    }
    return @($hits)
}

# Read the HANDOFF CONTRACT block from the merge-conflict-resolver agent file if it
# has one; otherwise return a concise built-in skeleton.
function Get-HandoffSkeleton {
    param([string]$RepoRootPath)
    $agent = Join-Path $RepoRootPath ".claude/agents/merge-conflict-resolver.md"
    if (Test-Path -LiteralPath $agent -PathType Leaf) {
        $text = Get-Content -LiteralPath $agent -Raw -ErrorAction SilentlyContinue
        if ($text) {
            # Pull the first fenced block that contains HANDOFF_STATUS.
            $rx = [regex]'(?s)```\s*(HANDOFF_STATUS:.*?)```'
            $m = $rx.Match($text)
            if ($m.Success) {
                return $m.Groups[1].Value.Trim()
            }
        }
    }
    # Fallback concise 6-line skeleton.
    return @"
HANDOFF_STATUS: success | partial | blocked
FILES_RESOLVED:
  - <path> | <strategy: union|take-fork|take-upstream|structural> | fork_feature_preserved: <name|none>
FILES_UNCERTAIN:
  - <path> | <why>
MARKERS_REMAINING: <int>   # must be 0 for success
"@
}

# Emit one markdown brief per non-empty slice.
function Write-SliceBriefs {
    param(
        [System.Collections.Specialized.OrderedDictionary]$Out,   # slice -> string[] (non-empty slices)
        [string]$RepoRootPath,
        [string]$RepoRootAbs,
        [string]$InventoryPath,
        [string]$TmpDir,
        [hashtable]$ModDelFlags   # repo-rel path -> $true if MODIFY/DELETE
    )

    $rules = Get-ForkFeatureRules -InventoryPath $InventoryPath
    $handoff = Get-HandoffSkeleton -RepoRootPath $RepoRootPath
    $invShort = if (Test-Path -LiteralPath $InventoryPath -PathType Leaf) { "docs/fork-feature-inventory.md" } else { "(inventory missing -- keyword fallback used)" }

    if (-not (Test-Path -LiteralPath $TmpDir)) {
        New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null
    }

    $written = New-Object System.Collections.Generic.List[string]
    foreach ($slice in $Out.Keys) {
        $sliceFiles = @($Out[$slice])
        if ($sliceFiles.Count -eq 0) { continue }

        $sb = New-Object System.Text.StringBuilder
        [void]$sb.AppendLine("# Merge-conflict resolver brief: slice ``$slice``")
        [void]$sb.AppendLine("")
        [void]$sb.AppendLine("- Slice: **$slice**")
        [void]$sb.AppendLine("- Files in slice: **$($sliceFiles.Count)**")
        [void]$sb.AppendLine("- Fork-feature inventory: $invShort")
        [void]$sb.AppendLine("- Resolution policy: **union-preserve** (keep BOTH sides; never drop fork features).")
        [void]$sb.AppendLine("")
        [void]$sb.AppendLine("## Files (ABSOLUTE paths) + fork-features-at-risk")
        [void]$sb.AppendLine("")

        foreach ($rel in $sliceFiles) {
            $relNorm = $rel -replace '/', '\'
            $abs = Join-Path $RepoRootAbs $relNorm
            $flag = ""
            if ($ModDelFlags -and $ModDelFlags.ContainsKey(($rel -replace '\\', '/'))) {
                $flag = "  **[MODIFY/DELETE -> decide RESTORE vs REMOVE]**"
            }
            [void]$sb.AppendLine("- ``$abs``$flag")
            $feats = Get-FeaturesForFile -Path $rel -Rules $rules
            if ($feats.Count -gt 0) {
                [void]$sb.AppendLine("    - fork features at risk: " + (($feats | ForEach-Object { "**$_**" }) -join ", "))
            } else {
                [void]$sb.AppendLine("    - fork features at risk: (no inventory match -- inspect manually)")
            }
        }

        [void]$sb.AppendLine("")
        [void]$sb.AppendLine("## HANDOFF CONTRACT")
        [void]$sb.AppendLine("")
        [void]$sb.AppendLine("Own ONLY the files above. Remove every ``<<<<<<<`` / ``=======`` / ``>>>>>>>`` marker.")
        [void]$sb.AppendLine("UNION by default; PRESERVE every fork feature whose owner surface you touch.")
        [void]$sb.AppendLine("Leave all edits **UNSTAGED** -- the orchestrator stages and commits. Run NO git mutations and NO build.")
        [void]$sb.AppendLine("Report success | partial | blocked, and list any uncertain files under FILES_UNCERTAIN.")
        [void]$sb.AppendLine("")
        [void]$sb.AppendLine('```')
        [void]$sb.AppendLine($handoff)
        [void]$sb.AppendLine('```')

        $briefPath = Join-Path $TmpDir "slice_${slice}_brief.md"
        Set-Content -LiteralPath $briefPath -Value ($sb.ToString()) -Encoding UTF8
        $written.Add($briefPath)
    }
    return $written
}

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

# --- Optional: emit ready-to-paste resolver briefs --------------------------
if ($EmitBriefs) {
    # Compute MODIFY/DELETE flags from the live merge (skip gracefully when -FromFile / no merge).
    $modDelFlags = @{}
    if (-not $FromFile) {
        foreach ($filterArg in @("DU", "UD")) {
            $dd = & git -C $RepoRoot diff --name-only --diff-filter=$filterArg 2>$null
            if ($LASTEXITCODE -eq 0 -and $dd) {
                foreach ($line in $dd) {
                    $t = ($line | Out-String).Trim()
                    if ($t) { $modDelFlags[($t -replace '\\', '/')] = $true }
                }
            }
        }
    }

    $tmpDir = Split-Path -Parent $OutFile
    $briefs = Write-SliceBriefs -Out $out -RepoRootPath $RepoRoot -RepoRootAbs $repoRootAbs `
        -InventoryPath $InventoryFile -TmpDir $tmpDir -ModDelFlags $modDelFlags

    Write-Host "Resolver briefs emitted: $($briefs.Count)" -ForegroundColor Cyan
    foreach ($b in $briefs) { Write-Host "  -> $b" }
    if ($modDelFlags.Count -gt 0) {
        Write-Host "MODIFY/DELETE conflicts flagged in briefs: $($modDelFlags.Count)" -ForegroundColor Yellow
    }
    Write-Host ""
}

Write-Output $jsonText

exit 0
