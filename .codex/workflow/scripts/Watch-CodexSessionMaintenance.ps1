[CmdletBinding()]
param(
    [ValidateSet("Self", "Director")]
    [string]$Profile = "Self",

    [string]$SessionPath,
    [string]$SessionId,
    [string]$SearchText,
    [string]$Project,
    [string]$SessionRoot = (Join-Path $HOME ".codex\sessions"),

    [double]$ThresholdPercent = -1,
    [string]$Reminder,

    [string]$Title,
    [int]$RootPid = 0,
    [long]$WindowHandle = 0,
    [string]$PipeName,
    [int]$PipeTimeoutMs = 5000,
    [switch]$ResolveManagedPipe,
    [string]$ManagedTerminalRoot = (Join-Path $HOME ".codex\wizard_sidecars\managed_terminals"),

    [switch]$DryRun,
    [switch]$Once,
    [int]$PollSeconds = 60,
    [int]$MaxIterations = 0,
    [int]$InspectIntervalMinutes = 0,
    [string]$LogPath = (Join-Path (Split-Path -Parent $PSScriptRoot) "session-maintenance\watch.jsonl")
)

$ErrorActionPreference = "Stop"

Import-Module (Join-Path $PSScriptRoot "CodexSessionMaintenance.psm1") -Force

$logDir = Split-Path -Parent $LogPath
if ($logDir -and -not (Test-Path -LiteralPath $logDir)) {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

function Write-WatchEvent {
    param([object]$Event)

    $Event | ConvertTo-Json -Depth 8 -Compress | Add-Content -LiteralPath $LogPath
    $Event
}

$profileInfo = Get-CodexMaintenanceProfile -Profile $Profile
if ($InspectIntervalMinutes -le 0) {
    $InspectIntervalMinutes = $profileInfo.InspectIntervalMinutes
}

$lastInspectAt = $null
$iteration = 0

while ($true) {
    $iteration += 1
    $now = Get-Date
    $effectivePipeName = $PipeName
    $managedPipe = $null
    $didResolveManagedPipe = $false
    if (-not $effectivePipeName -and $ResolveManagedPipe) {
        $didResolveManagedPipe = $true
        $managedPipe = Resolve-CodexWizardManagedPipe `
            -SessionPath $SessionPath `
            -SessionId $SessionId `
            -SearchText $SearchText `
            -Project $Project `
            -SessionRoot $SessionRoot `
            -ManagedTerminalRoot $ManagedTerminalRoot
        if ($null -eq $managedPipe) {
            throw "Could not resolve a live Wizard-managed Codex pipe. Pass -PipeName explicitly or use a terminal selector."
        }
        $effectivePipeName = $managedPipe.PipeName
    }

    $result = Invoke-CodexMaintenanceOnce `
        -Profile $Profile `
        -SessionPath $SessionPath `
        -SessionId $SessionId `
        -SearchText $SearchText `
        -Project $Project `
        -SessionRoot $SessionRoot `
        -ThresholdPercent $ThresholdPercent `
        -Reminder $Reminder `
        -Title $Title `
        -RootPid $RootPid `
        -WindowHandle $WindowHandle `
        -PipeName $effectivePipeName `
        -PipeTimeoutMs $PipeTimeoutMs `
        -DryRun:$DryRun

    $usage = $result.Before
    $plan = New-CodexMaintenancePlan `
        -Usage $usage `
        -ThresholdPercent $result.Plan.ThresholdPercent `
        -Profile $Profile `
        -Now $now `
        -LastInspectAt $lastInspectAt `
        -InspectIntervalMinutes $InspectIntervalMinutes

    if ($plan.InspectDue) {
        $lastInspectAt = $now
    }

    Write-WatchEvent ([pscustomobject]@{
        timestamp = $now.ToString("o")
        iteration = $iteration
        profile = $Profile
        status = $result.Status
        sessionPath = $result.SessionPath
        usedPercent = $usage.UsedPercent
        thresholdPercent = $plan.ThresholdPercent
        actions = $plan.Actions
        inspectDue = $plan.InspectDue
        dryRun = [bool]$DryRun
        pipeName = $effectivePipeName
        pipeTimeoutMs = $PipeTimeoutMs
        pipeSource = if ($effectivePipeName) {
            if ($null -ne $managedPipe) { "managed" } else { "explicit" }
        } else {
            $null
        }
        resolveManagedPipe = $didResolveManagedPipe
        rootPid = $RootPid
        windowHandle = $WindowHandle
        managedPipeSource = if ($null -ne $managedPipe) { $managedPipe.SourcePath } else { $null }
    }) | Format-List

    if ($Once -or ($MaxIterations -gt 0 -and $iteration -ge $MaxIterations)) {
        break
    }

    Start-Sleep -Seconds $PollSeconds
}
