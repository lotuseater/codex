param(
    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [string]$WorkflowHandoffPath = (Join-Path (Split-Path $PSScriptRoot -Parent) "solid-refactor-handoff.md"),
    [int]$RecentMinutes = 20,
    [int]$MaxItems = 5
)

$ErrorActionPreference = "Stop"

$now = Get-Date

function Format-Age {
    param([datetime]$Time)

    $age = $now - $Time
    if ($age.TotalMinutes -lt 60) {
        return ("{0:N1}m" -f $age.TotalMinutes)
    }

    return ("{0:N1}h" -f $age.TotalHours)
}

function Format-Items {
    param([object[]]$Items)

    if (-not $Items -or $Items.Count -eq 0) {
        return "(none)"
    }

    return (($Items | ForEach-Object {
        "{0} ({1})" -f $_.Name, (Format-Age $_.LastWriteTime)
    }) -join "; ")
}

function Recent-Items {
    param([string]$Filter)

    @(Get-ChildItem -LiteralPath $PSScriptRoot -Filter $Filter -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First $MaxItems)
}

$state = $null
$stateSummary = "(missing)"
if (Test-Path -LiteralPath $StatePath) {
    try {
        $state = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
        $stateSummary = "rootPid={0} hwnd={1} remembered={2}" -f $state.rootPid, $state.windowHandle, $state.windowRememberedAt
    } catch {
        $stateSummary = "unreadable: $($_.Exception.Message)"
    }
}

$workflowHandoff = $null
if (Test-Path -LiteralPath $WorkflowHandoffPath) {
    $workflowHandoff = Get-Item -LiteralPath $WorkflowHandoffPath
}

$workerLogs = Recent-Items "solid_refactor_wave*.exec.visible.log"
$workerHandoffs = Recent-Items "solid_refactor_wave*.handoff.md"

$recentCutoff = $now.AddMinutes(-1 * $RecentMinutes)
$recentLogCount = @($workerLogs | Where-Object { $_.LastWriteTime -ge $recentCutoff }).Count
$recentHandoffCount = @($workerHandoffs | Where-Object { $_.LastWriteTime -ge $recentCutoff }).Count

if ($recentHandoffCount -gt 0) {
    $action = "read fresh worker handoffs only; then decide whether a short director follow-up is needed"
} elseif ($recentLogCount -gt 0) {
    $action = "wait; worker logs changed recently"
} else {
    $action = "send one short director follow-up or ask it to update handoff before compact"
}

"SOLID director checkpoint: {0:o}" -f $now
"state: {0}" -f $stateSummary
if ($workflowHandoff) {
    "solid handoff: {0} ({1})" -f $workflowHandoff.Name, (Format-Age $workflowHandoff.LastWriteTime)
} else {
    "solid handoff: (missing)"
}
"worker logs: {0}" -f (Format-Items $workerLogs)
"worker handoffs: {0}" -f (Format-Items $workerHandoffs)
"action: {0}" -f $action
"note: normal checkpoints skip singleton/process scans; start/stop/relaunch scripts own that."
