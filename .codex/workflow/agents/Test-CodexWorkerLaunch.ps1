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

function Assert-Equal {
    param(
        [Parameter(Mandatory = $true)]
        [AllowNull()]
        [object]$Actual,

        [Parameter(Mandatory = $true)]
        [AllowNull()]
        [object]$Expected,

        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if ($Actual -ne $Expected) {
        throw "$Message Expected=[$Expected] Actual=[$Actual]"
    }
}

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

function Assert-FileDoesNotContain {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string[]]$Needles
    )

    $content = Get-Content -Raw -LiteralPath $Path
    foreach ($needle in $Needles) {
        if ($content.Contains($needle)) {
            throw "$Path contains raw prompt text that should only appear as an encoded argv value: $needle"
        }
    }
}

function Assert-PowerShellParses {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $tokens = $null
    $parseErrors = $null
    [System.Management.Automation.Language.Parser]::ParseFile(
        $Path,
        [ref]$tokens,
        [ref]$parseErrors
    ) | Out-Null
    if ($parseErrors -and $parseErrors.Count -gt 0) {
        throw "$Path has PowerShell parse errors: $($parseErrors | ForEach-Object { $_.Message } | Out-String)"
    }
}

function Assert-LauncherContainsWorkerArgs {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Repo,

        [Parameter(Mandatory = $true)]
        [string]$Prompt
    )

    Assert-FileContains `
        -Path $Path `
        -Needles @(
            "`$codexArgs = @(",
            "FromBase64String",
            (ConvertTo-CodexPowerShellUtf8StringExpression "--cd"),
            (ConvertTo-CodexPowerShellUtf8StringExpression $Repo),
            (ConvertTo-CodexPowerShellUtf8StringExpression "--ask-for-approval"),
            (ConvertTo-CodexPowerShellUtf8StringExpression "never"),
            (ConvertTo-CodexPowerShellUtf8StringExpression "--sandbox"),
            (ConvertTo-CodexPowerShellUtf8StringExpression "danger-full-access"),
            "Remove-Item Env:\CODEX_THREAD_ID, Env:\CODEX_SHELL, Env:\CODEX_INTERNAL_ORIGINATOR_OVERRIDE",
            (ConvertTo-CodexPowerShellUtf8StringExpression $Prompt)
        )
    Assert-FileDoesNotContain `
        -Path $Path `
        -Needles @($Prompt)
    Assert-PowerShellParses -Path $Path
}

$resolvedRepo = (Resolve-Path -LiteralPath $Repo).Path
$prompt = "Task: improve benchmark coverage for the user's general branch-feature research goal while preserving user’s smart quote, a backtick ``, and a second line`nsecond line"

$health = Get-CodexWorkerCommandHealth -CodexCommand $CodexCommand
Assert-CodexWorkerCommandHealth -Health $health -AllowCustomBuild:$AllowCustomBuild

$execArgs = New-CodexWorkerExecArgs `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -WorkerModel $WorkerModel `
    -WorkerReasoningEffort $WorkerReasoningEffort
Assert-CodexWorkerArgs `
    -Args $execArgs `
    -Mode Exec `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -ExpectedWorkerModel $WorkerModel `
    -ExpectedWorkerReasoningEffort $WorkerReasoningEffort

$interactiveArgs = New-CodexWorkerInteractiveArgs `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -WorkerModel $WorkerModel `
    -WorkerReasoningEffort $WorkerReasoningEffort
Assert-CodexWorkerArgs `
    -Args $interactiveArgs `
    -Mode Interactive `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -ExpectedWorkerModel $WorkerModel `
    -ExpectedWorkerReasoningEffort $WorkerReasoningEffort

$resumeArgs = New-CodexWorkerResumeArgs `
    -Repo $resolvedRepo `
    -ResumeSession "00000000-0000-0000-0000-000000000000" `
    -Prompt $prompt `
    -WorkerModel $WorkerModel `
    -WorkerReasoningEffort $WorkerReasoningEffort `
    -Loop `
    -LoopMessage "go on" `
    -LoopPeriod 300
Assert-CodexWorkerArgs `
    -Args $resumeArgs `
    -Mode Resume `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -ExpectedWorkerModel $WorkerModel `
    -ExpectedWorkerReasoningEffort $WorkerReasoningEffort

$nonDefaultArgs = New-CodexWorkerExecArgs `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -WorkerModel $WorkerModel `
    -WorkerReasoningEffort $WorkerReasoningEffort `
    -ApprovalPolicy "on-request" `
    -SandboxMode "workspace-write"
Assert-CodexWorkerArgs `
    -Args $nonDefaultArgs `
    -Mode Exec `
    -Repo $resolvedRepo `
    -Prompt $prompt `
    -ExpectedWorkerModel $WorkerModel `
    -ExpectedWorkerReasoningEffort $WorkerReasoningEffort `
    -ExpectedApprovalPolicy "on-request" `
    -ExpectedSandboxMode "workspace-write"

Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "start-codex-workers.ps1") `
    -Needles @("CodexWorkerLaunch.psm1", "New-CodexWorkerExecArgs", "New-CodexWorkerResumeArgs", "Marker")
Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "start-codex-interactive.ps1") `
    -Needles @("CodexWorkerLaunch.psm1", "New-CodexWorkerInteractiveArgs", "New-CodexWorkerResumeArgs", "MarkerPath")
Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "..\scripts\Invoke-CodexWorker.ps1") `
    -Needles @("CodexWorkerLaunch.psm1", "New-CodexWorkerExecArgs", "New-CodexWorkerInteractiveArgs")
Assert-FileContains `
    -Path (Join-Path $PSScriptRoot "..\scripts\Start-CodexWorker.ps1") `
    -Needles @("-CodexCommand", "-WorkerModel", "-WorkerReasoningEffort")

$workerProbeName = "_worker_launch_canary"
$workerProbePrompt = Join-Path $PSScriptRoot "$workerProbeName.prompt.md"
$workerProbeLauncher = Join-Path $PSScriptRoot "$workerProbeName.exec.launch.ps1"
$workerProbeLog = Join-Path $PSScriptRoot "$workerProbeName.exec.visible.log"
$workerProbeMarker = Join-Path $PSScriptRoot "$workerProbeName.exec.marker.txt"
$workerProbeText = "Task: improve benchmark coverage for the user's general branch-feature research goal from file with user’s smart quote"
try {
    Set-Content -LiteralPath $workerProbePrompt -Value $workerProbeText -NoNewline -Encoding UTF8
    $workerDryRun = @( & (Join-Path $PSScriptRoot "start-codex-workers.ps1") `
        -Repo $resolvedRepo `
        -WorkerNames $workerProbeName `
        -DryRun )
    Assert-Equal -Actual $workerDryRun.Count -Expected 1 -Message "Worker dry-run must return exactly one canary row."
    Assert-Equal -Actual $workerDryRun[0].Prompt -Expected $workerProbePrompt -Message "Worker dry-run must report the prompt file path."
    Assert-Equal -Actual $workerDryRun[0].Launcher -Expected $workerProbeLauncher -Message "Worker dry-run launcher path must be deterministic."
    Assert-Equal -Actual $workerDryRun[0].Log -Expected $workerProbeLog -Message "Worker dry-run log path must be deterministic."
    Assert-Equal -Actual $workerDryRun[0].Marker -Expected $workerProbeMarker -Message "Worker dry-run marker path must be deterministic."
    Assert-CodexWorkerArgs `
        -Args ([string[]]$workerDryRun[0].CodexArgs) `
        -Mode Exec `
        -Repo $resolvedRepo `
        -Prompt $workerProbeText `
        -ExpectedWorkerModel $WorkerModel `
        -ExpectedWorkerReasoningEffort $WorkerReasoningEffort
    Assert-LauncherContainsWorkerArgs `
        -Path $workerProbeLauncher `
        -Repo $resolvedRepo `
        -Prompt $workerProbeText
} finally {
    Remove-Item -LiteralPath $workerProbePrompt, $workerProbeLauncher -Force -ErrorAction SilentlyContinue
}

$interactiveProbeName = "_interactive_launch_canary"
$interactiveProbePrompt = Join-Path $PSScriptRoot "$interactiveProbeName.prompt.md"
$interactiveProbeLauncher = Join-Path $PSScriptRoot "handoffs\$interactiveProbeName.launch.ps1"
$interactiveProbeHandoff = Join-Path $PSScriptRoot "handoffs\$interactiveProbeName.handoff.md"
$interactiveProbeMarker = Join-Path $PSScriptRoot "handoffs\$interactiveProbeName.marker.txt"
$interactiveProbeText = "Task: improve benchmark coverage for the user's general branch-feature research goal in interactive mode with user’s smart quote"
try {
    Set-Content -LiteralPath $interactiveProbePrompt -Value $interactiveProbeText -NoNewline -Encoding UTF8
    $interactiveDryRunJson = & (Join-Path $PSScriptRoot "start-codex-interactive.ps1") `
        -Repo $resolvedRepo `
        -Name $interactiveProbeName `
        -PromptPath $interactiveProbePrompt `
        -DryRun
    $interactiveDryRun = ($interactiveDryRunJson | Out-String) | ConvertFrom-Json
    Assert-Equal -Actual $interactiveDryRun.PromptPath -Expected $interactiveProbePrompt -Message "Interactive dry-run must report the prompt file path."
    Assert-Equal -Actual $interactiveDryRun.HandoffPath -Expected $interactiveProbeHandoff -Message "Interactive handoff path must be deterministic."
    Assert-Equal -Actual $interactiveDryRun.LauncherPath -Expected $interactiveProbeLauncher -Message "Interactive launcher path must be deterministic."
    Assert-Equal -Actual $interactiveDryRun.MarkerPath -Expected $interactiveProbeMarker -Message "Interactive marker path must be deterministic."
    Assert-CodexWorkerArgs `
        -Args ([string[]]$interactiveDryRun.CodexArgs) `
        -Mode Interactive `
        -Repo $resolvedRepo `
        -Prompt $interactiveProbeText `
        -ExpectedWorkerModel $WorkerModel `
        -ExpectedWorkerReasoningEffort $WorkerReasoningEffort
    Assert-LauncherContainsWorkerArgs `
        -Path $interactiveProbeLauncher `
        -Repo $resolvedRepo `
        -Prompt $interactiveProbeText
} finally {
    Remove-Item -LiteralPath $interactiveProbePrompt, $interactiveProbeLauncher -Force -ErrorAction SilentlyContinue
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
