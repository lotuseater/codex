[CmdletBinding()]
param(
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,

    [string]$CodexCommand = "codex",

    [string]$WorkerModel = "gpt-5.5",

    [string]$WorkerReasoningEffort = "xhigh",

    [switch]$AllowCustomBuild
)

$ErrorActionPreference = "Stop"

$launchModule = Join-Path $PSScriptRoot "CodexWorkerLaunch.psm1"
Import-Module $launchModule -Force -DisableNameChecking

function Assert-FileContains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string[]]$Needles
    )

    $content = Get-Content -Raw -LiteralPath $Path
    foreach ($needle in $Needles) {
        if (-not $content.Contains($needle)) {
            throw "$Path is missing required launcher wiring token: $needle"
        }
    }
}

$resolvedRepo = (Resolve-Path -LiteralPath $Repo).Path
$prompt = "spawn worker launch canary prompt"

$health = Get-CodexWorkerCommandHealth -CodexCommand $CodexCommand
Assert-CodexWorkerCommandHealth -Health $health -AllowCustomBuild:$AllowCustomBuild

$execArgs = New-CodexWorkerExecArgs `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -WorkerModel $WorkerModel `
    -WorkerReasoningEffort $WorkerReasoningEffort
Assert-CodexWorkerArgs -Args $execArgs -Mode Exec -Repo $resolvedRepo -Prompt $prompt

$interactiveArgs = New-CodexWorkerInteractiveArgs `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -WorkerModel $WorkerModel `
    -WorkerReasoningEffort $WorkerReasoningEffort
Assert-CodexWorkerArgs -Args $interactiveArgs -Mode Interactive -Repo $resolvedRepo -Prompt $prompt

$resumeArgs = New-CodexWorkerResumeArgs `
    -Repo $resolvedRepo `
    -ResumeSession "00000000-0000-0000-0000-000000000000" `
    -Prompt $prompt `
    -WorkerModel $WorkerModel `
    -WorkerReasoningEffort $WorkerReasoningEffort `
    -Loop `
    -LoopMessage "go on" `
    -LoopPeriod 300
Assert-CodexWorkerArgs -Args $resumeArgs -Mode Resume -Repo $resolvedRepo -Prompt $prompt

Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "start-codex-workers.ps1") `
    -Needles @("CodexWorkerLaunch.psm1", "New-CodexWorkerExecArgs", "New-CodexWorkerResumeArgs")
Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "start-codex-interactive.ps1") `
    -Needles @("CodexWorkerLaunch.psm1", "New-CodexWorkerInteractiveArgs", "New-CodexWorkerResumeArgs")
Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "..\scripts\Invoke-CodexWorker.ps1") `
    -Needles @("CodexWorkerLaunch.psm1", "New-CodexWorkerExecArgs", "New-CodexWorkerInteractiveArgs")
Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "..\scripts\Start-CodexWorker.ps1") `
    -Needles @("-CodexCommand", "-WorkerModel", "-WorkerReasoningEffort")

$workerProbeName = "_worker_launch_canary"
$workerProbeLauncher = Join-Path $PSScriptRoot "$workerProbeName.exec.launch.ps1"
try {
    & (Join-Path $PSScriptRoot "start-codex-workers.ps1") `
        -Repo $resolvedRepo `
        -WorkerNames $workerProbeName `
        -Prompt $prompt `
        -DryRun | Out-Null
} finally {
    Remove-Item -LiteralPath $workerProbeLauncher -Force -ErrorAction SilentlyContinue
}

$interactiveProbeName = "_interactive_launch_canary"
$interactiveProbeLauncher = Join-Path $PSScriptRoot "handoffs\$interactiveProbeName.launch.ps1"
try {
    & (Join-Path $PSScriptRoot "start-codex-interactive.ps1") `
        -Repo $resolvedRepo `
        -Name $interactiveProbeName `
        -Prompt $prompt `
        -DryRun | Out-Null
} finally {
    Remove-Item -LiteralPath $interactiveProbeLauncher -Force -ErrorAction SilentlyContinue
}

[pscustomobject]@{
    Status = "ok"
    Repo = $resolvedRepo
    CodexCommand = $CodexCommand
    UsesCustomBuild = $health.UsesCustomBuild
    CodexSource = $health.Source
    WrapperRealExe = $health.WrapperRealExe
    ExecArgs = $execArgs
    InteractiveArgs = $interactiveArgs
    ResumeArgs = $resumeArgs
} | ConvertTo-Json -Depth 6
