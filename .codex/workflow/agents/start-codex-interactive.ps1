param(
    [string]$Repo,
    [string]$PromptPath,
    [string]$Prompt,
    [string]$Name = "codex-interactive-worker",
    [string]$CodexCommand = "codex",
    [string]$WorkerModel = "gpt-5.3-codex",
    [string]$WorkerReasoningEffort = "high",
    [string]$HandoffPath,
    [Alias("ResumeSession")]
    [string]$Resume,
    [switch]$Loop,
    [string]$LoopMessage,
    [int]$LoopPeriod,
    [switch]$Hidden,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Resolve-RepoPath {
    param([string]$PathValue)

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        return (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..\..\..")).Path
    }

    return (Resolve-Path -LiteralPath $PathValue).Path
}

function Resolve-InputPath {
    param(
        [string]$BasePath,
        [string]$PathValue
    )

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        return $null
    }

    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return (Resolve-Path -LiteralPath $PathValue).Path
    }

    return (Resolve-Path -LiteralPath (Join-Path $BasePath $PathValue)).Path
}

function Convert-ToSafeFileName {
    param([string]$Value)

    $invalid = [System.IO.Path]::GetInvalidFileNameChars()
    $chars = $Value.ToCharArray() | ForEach-Object {
        if ($invalid -contains $_) { "_" } else { $_ }
    }
    -join $chars
}

function ConvertTo-PowerShellSingleQuotedLiteral {
    param([string]$Value)

    "'" + $Value.Replace("'", "''") + "'"
}

function ConvertTo-PowerShellArrayLiteral {
    param([string[]]$Values)

    "@(" + (($Values | ForEach-Object { ConvertTo-PowerShellSingleQuotedLiteral $_ }) -join ", ") + ")"
}

$resolvedRepo = Resolve-RepoPath -PathValue $Repo
$resolvedPrompt = Resolve-InputPath -BasePath $resolvedRepo -PathValue $PromptPath

if ([string]::IsNullOrWhiteSpace($Prompt) -and -not [string]::IsNullOrWhiteSpace($resolvedPrompt)) {
    $Prompt = Get-Content -Raw -LiteralPath $resolvedPrompt
}

if ([string]::IsNullOrWhiteSpace($Prompt)) {
    throw "Either -Prompt or -PromptPath is required."
}

if ($Loop -and [string]::IsNullOrWhiteSpace($Resume)) {
    throw "-Loop is only supported with -Resume."
}

$handoffDir = Join-Path $resolvedRepo ".codex\workflow\agents\handoffs"
New-Item -ItemType Directory -Force -Path $handoffDir | Out-Null

if ([string]::IsNullOrWhiteSpace($HandoffPath)) {
    $safeName = Convert-ToSafeFileName -Value $Name
    $HandoffPath = Join-Path $handoffDir "$safeName.handoff.md"
} elseif (-not [System.IO.Path]::IsPathRooted($HandoffPath)) {
    $HandoffPath = Join-Path $resolvedRepo $HandoffPath
}

$resolvedHandoffParent = Split-Path -Parent $HandoffPath
if (-not [string]::IsNullOrWhiteSpace($resolvedHandoffParent)) {
    New-Item -ItemType Directory -Force -Path $resolvedHandoffParent | Out-Null
}

$codexArgs = @(
    "-c",
    "model=$WorkerModel",
    "-c",
    "model_reasoning_effort=$WorkerReasoningEffort",
    "--cd",
    $resolvedRepo,
        "--ask-for-approval",
    "never",
    "--sandbox",
    "danger-full-access"
)

if ([string]::IsNullOrWhiteSpace($Resume)) {
    $codexArgs += $Prompt
    $mode = "NewInteractivePrompt"
} else {
    $codexArgs += "resume"
    if ($Loop) {
        $codexArgs += "--loop"
        if (-not [string]::IsNullOrWhiteSpace($LoopMessage)) {
            $codexArgs += "--loop-message"
            $codexArgs += $LoopMessage
        }
        if ($LoopPeriod -gt 0) {
            $codexArgs += "--loop-period"
            $codexArgs += [string]$LoopPeriod
        }
    }
    $codexArgs += $Resume
    $codexArgs += $Prompt
    $mode = if ($Loop) { "ResumeLoop" } else { "Resume" }
}

$safeNameForFile = Convert-ToSafeFileName -Value $Name
$launcherPath = Join-Path $handoffDir "$safeNameForFile.launch.ps1"
$markerPath = Join-Path $handoffDir "$safeNameForFile.marker.txt"

$childCommand = @"
`$ErrorActionPreference = "Stop"
`$Host.UI.RawUI.WindowTitle = $(ConvertTo-PowerShellSingleQuotedLiteral "Codex worker: $Name")
Set-Location -LiteralPath $(ConvertTo-PowerShellSingleQuotedLiteral $resolvedRepo)
`$codexArgs = $(ConvertTo-PowerShellArrayLiteral $codexArgs)
& $(ConvertTo-PowerShellSingleQuotedLiteral $CodexCommand) @codexArgs
"@

$childCommand | Set-Content -LiteralPath $launcherPath -Encoding UTF8

$args = @(
    "-NoExit",
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    $launcherPath
)

if ($DryRun) {
    [pscustomobject]@{
        Repo = $resolvedRepo
        PromptPath = $resolvedPrompt
        Prompt = $Prompt
        HandoffPath = $HandoffPath
        Resume = $Resume
        Loop = [bool]$Loop
        LoopMessage = $LoopMessage
        LoopPeriod = $LoopPeriod
        CodexCommand = $CodexCommand
        WorkerModel = $WorkerModel
        WorkerReasoningEffort = $WorkerReasoningEffort
        Mode = $mode
        WindowTitle = "Codex worker: $Name"
        Launcher = "powershell.exe"
        LauncherPath = $launcherPath
        Arguments = $args -join " "
    } | ConvertTo-Json -Depth 4
    exit 0
}

$startArgs = @{
    FilePath = "powershell.exe"
    ArgumentList = $args
    WindowStyle = if ($Hidden) { "Hidden" } else { "Normal" }
    PassThru = $true
}

$process = Start-Process @startArgs

@(
    "$(Get-Date -Format o) mode=$mode pid=$($process.Id) repo=$resolvedRepo resume=$Resume",
    "visible=$(-not $Hidden)",
    "prompt=$Prompt",
    "loop=$([bool]$Loop)",
    "loop_message=$LoopMessage",
    "launcher=$launcherPath"
) | Set-Content -LiteralPath $markerPath

[pscustomobject]@{
    ProcessId = $process.Id
    Repo = $resolvedRepo
    PromptPath = $resolvedPrompt
    HandoffPath = $HandoffPath
    Resume = $Resume
    Loop = [bool]$Loop
    Mode = $mode
    WindowTitle = "Codex worker: $Name"
    LauncherPath = $launcherPath
} | ConvertTo-Json -Depth 4
