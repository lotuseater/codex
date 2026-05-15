param(
    [string]$BaseRef = "origin/main",
    [string]$HeadRef = "HEAD",
    [switch]$IncludeWorkingTree,
    [int]$Top = 40
)

$ErrorActionPreference = "Stop"

function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Args)
    & git @Args
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Args -join ' ') failed with exit code $LASTEXITCODE"
    }
}

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

$range = if ($IncludeWorkingTree) { $BaseRef } else { "$BaseRef..$HeadRef" }
$mergeBase = (Invoke-Git merge-base $BaseRef $HeadRef | Select-Object -First 1).Trim()
$baseShort = (Invoke-Git rev-parse --short $BaseRef | Select-Object -First 1).Trim()
$headShort = if ($IncludeWorkingTree) {
    "$((Invoke-Git rev-parse --short $HeadRef | Select-Object -First 1).Trim())+worktree"
} else {
    (Invoke-Git rev-parse --short $HeadRef | Select-Object -First 1).Trim()
}

$rows = New-Object System.Collections.Generic.List[object]
$numstat = if ($IncludeWorkingTree) {
    Invoke-Git diff --numstat --find-renames $BaseRef
} else {
    Invoke-Git diff --numstat --find-renames "$BaseRef..$HeadRef"
}
foreach ($line in $numstat) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $parts = $line -split "`t"
    if ($parts.Count -lt 3) { continue }
    $added = if ($parts[0] -eq "-") { 0 } else { [int]$parts[0] }
    $deleted = if ($parts[1] -eq "-") { 0 } else { [int]$parts[1] }
    $path = $parts[-1]
    if ($path -match " => ") {
        $path = ($path -replace ".* => ", "") -replace "[{}]", ""
    }
    $area = Get-HotArea $path
    $risk = Get-Risk $area
    $rows.Add([pscustomobject]@{
        Path = $path
        Area = $area
        Added = $added
        Deleted = $deleted
        Churn = $added + $deleted
        Risk = $risk
        Weighted = ($added + $deleted) * $risk
    })
}

if ($IncludeWorkingTree) {
    $trackedPaths = @{}
    foreach ($row in $rows) {
        $trackedPaths[$row.Path] = $true
    }
    $untracked = Invoke-Git ls-files --others --exclude-standard
    foreach ($path in $untracked) {
        if ([string]::IsNullOrWhiteSpace($path) -or $trackedPaths.ContainsKey($path)) {
            continue
        }
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            continue
        }
        try {
            $resolved = (Resolve-Path -LiteralPath $path).Path
            $reader = [System.IO.File]::OpenText($resolved)
            try {
                $added = 0
                while ($null -ne $reader.ReadLine()) {
                    $added++
                }
            } finally {
                $reader.Dispose()
            }
        } catch {
            $added = 0
        }
        $area = Get-HotArea $path
        $risk = Get-Risk $area
        $rows.Add([pscustomobject]@{
            Path = $path
            Area = $area
            Added = $added
            Deleted = 0
            Churn = $added
            Risk = $risk
            Weighted = $added * $risk
        })
    }
}

$summary = $rows |
    Group-Object Area |
    ForEach-Object {
        [pscustomobject]@{
            Area = $_.Name
            Files = $_.Count
            Churn = ($_.Group | Measure-Object Churn -Sum).Sum
            Weighted = ($_.Group | Measure-Object Weighted -Sum).Sum
        }
    } |
    Sort-Object Weighted -Descending

$hotFiles = $rows | Sort-Object Weighted -Descending | Select-Object -First $Top

$result = [pscustomobject]@{
    base_ref = $BaseRef
    head_ref = $HeadRef
    includes_working_tree = [bool]$IncludeWorkingTree
    base_short = $baseShort
    head_short = $headShort
    merge_base = $mergeBase
    file_count = $rows.Count
    total_churn = ($rows | Measure-Object Churn -Sum).Sum
    total_weighted_conflict_surface = ($rows | Measure-Object Weighted -Sum).Sum
    area_summary = @($summary)
    hot_files = @($hotFiles)
}

$result | ConvertTo-Json -Depth 6
