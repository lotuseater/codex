<#
.SYNOPSIS
    Fill in the blank ("to be finalized") columns of a row in docs/merge-metrics.csv
    after a merge completes. Companion to scripts/merge-preflight.ps1 -LogMetrics, which
    SEEDS a row with the up-front measurements (date, upstream_tip, commits_behind,
    conflicts, content_conflicts, modify_delete); this script BACKFILLS the outcome
    columns (slices, buildfix_waves, wallclock_min, result, notes).

.DESCRIPTION
    Selects the target row in docs/merge-metrics.csv and writes the supplied outcome
    values into it, then rewrites the CSV. Row selection:
      * If -UpstreamTip is given, the most-recent row whose upstream_tip matches it.
      * Otherwise, the LAST (most-recent) data row in the file.
    Idempotent: running it again with the same values is a no-op-equivalent (it simply
    overwrites the same fields). Defensive: if the CSV or a matching row is missing it
    WARNS and returns without throwing, so it never aborts a wrapping workflow.

    Read-only except for docs/merge-metrics.csv. Performs no git or build actions.

.PARAMETER Slices
    Number of conflict-resolution slices the merge was partitioned into.

.PARAMETER BuildfixWaves
    Number of post-merge build-fix waves required to make the tree compile.

.PARAMETER WallclockMin
    Total wall-clock minutes spent on the merge (preflight -> green build).

.PARAMETER Result
    Short outcome label, e.g. "merged", "merged+built", "aborted".

.PARAMETER Notes
    Free-text notes (quoted automatically in the CSV).

.PARAMETER UpstreamTip
    Optional. Short upstream tip sha identifying which row to finalize. When omitted,
    the most-recent data row is used.

.PARAMETER CsvPath
    Optional override of the metrics CSV path (defaults to docs/merge-metrics.csv
    relative to the repo root inferred from this script's location).

.EXAMPLE
    ./scripts/merge-metrics-finalize.ps1 -Slices 14 -BuildfixWaves 7 -WallclockMin 240 -Result "merged+built" -Notes "Opus 14-slice workflow"
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][int]$Slices,
    [Parameter(Mandatory = $true)][int]$BuildfixWaves,
    [Parameter(Mandatory = $true)][int]$WallclockMin,
    [Parameter(Mandatory = $true)][string]$Result,
    [string]$Notes = "",
    [string]$UpstreamTip,
    [string]$CsvPath
)

$ErrorActionPreference = "Stop"

if (-not $CsvPath) {
    $repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
    $CsvPath = Join-Path $repoRoot "docs/merge-metrics.csv"
}

if (-not (Test-Path -LiteralPath $CsvPath)) {
    Write-Warning "merge-metrics CSV not found at '$CsvPath' — nothing to finalize."
    return
}

$rows = @(Import-Csv -LiteralPath $CsvPath)
if ($rows.Count -eq 0) {
    Write-Warning "merge-metrics CSV '$CsvPath' has no data rows — nothing to finalize."
    return
}

# --- Select the target row ---
$target = $null
if ($UpstreamTip) {
    # Most-recent row matching the requested upstream tip.
    for ($i = $rows.Count - 1; $i -ge 0; $i--) {
        if ($rows[$i].upstream_tip -eq $UpstreamTip) { $target = $rows[$i]; break }
    }
    if ($null -eq $target) {
        Write-Warning "No row with upstream_tip '$UpstreamTip' in '$CsvPath' — nothing to finalize."
        return
    }
} else {
    $target = $rows[$rows.Count - 1]
}

# --- Backfill the outcome columns (idempotent overwrite) ---
$target.slices         = "$Slices"
$target.buildfix_waves = "$BuildfixWaves"
$target.wallclock_min  = "$WallclockMin"
$target.result         = "$Result"
$target.notes          = "$Notes"

# Export-Csv quotes fields as needed and is RFC-4180 friendly (no #TYPE header by default).
$rows | Export-Csv -LiteralPath $CsvPath -Encoding UTF8

Write-Host ("Finalized merge-metrics row (upstream_tip={0}, date={1}): slices={2}, buildfix_waves={3}, wallclock_min={4}, result={5}" -f `
    $target.upstream_tip, $target.date, $Slices, $BuildfixWaves, $WallclockMin, $Result) -ForegroundColor Cyan
Write-Host "Updated -> $CsvPath"
