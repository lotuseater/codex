param(
    [string]$Repo,
    [Parameter(Mandatory = $true)]
    [string]$PromptPath,
    [string]$Name = "codex-interactive-worker",
    [string]$CodexCommand = "codex",
    [string]$HandoffPath,
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

    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return (Resolve-Path -LiteralPath $PathValue).Path
    }

    return (Resolve-Path -LiteralPath (Join-Path $BasePath $PathValue)).Path
}

function Convert-ToSafeFileName {
    param([string]$Value)

    $safe = $Value -replace '[^A-Za-z0-9_.-]', '_'
    if ([string]::IsNullOrWhiteSpace($safe)) {
        return "codex-interactive-worker"
    }
    return $safe
}

$resolvedRepo = Resolve-RepoPath -PathValue $Repo
$resolvedPrompt = Resolve-InputPath -BasePath $resolvedRepo -PathValue $PromptPath

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

if (-not (Test-Path -LiteralPath $HandoffPath)) {
    "# $Name handoff`n`nStarted: $(Get-Date -Format o)`n" | Set-Content -LiteralPath $HandoffPath -Encoding UTF8
}

$bootstrapPrompt = @"
You are a delegated visible interactive Codex worker.

Repository:
$resolvedRepo

Task prompt file:
$resolvedPrompt

Handoff file:
$HandoffPath

Read the task prompt file first, execute that task in this repository, and keep the handoff file concise and current. Do not wait for root unless the prompt names a blocker.
"@

$childCommand = @"
`$Host.UI.RawUI.WindowTitle = 'Codex worker: $Name'
Set-Location -LiteralPath '$($resolvedRepo.Replace("'", "''"))'
`$env:CODEX_WORKER_NAME = '$($Name.Replace("'", "''"))'
`$env:CODEX_WORKER_PROMPT = '$($resolvedPrompt.Replace("'", "''"))'
`$env:CODEX_WORKER_HANDOFF = '$($HandoffPath.Replace("'", "''"))'
`$prompt = @'
$bootstrapPrompt
'@
& '$($CodexCommand.Replace("'", "''"))' --cd '$($resolvedRepo.Replace("'", "''"))' --ask-for-approval never --sandbox danger-full-access `$prompt
"@

$encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($childCommand))
$args = @(
    "-NoExit",
    "-ExecutionPolicy",
    "Bypass",
    "-EncodedCommand",
    $encoded
)

if ($DryRun) {
    [pscustomobject]@{
        Repo = $resolvedRepo
        PromptPath = $resolvedPrompt
        HandoffPath = $HandoffPath
        CodexCommand = $CodexCommand
        WindowTitle = "Codex worker: $Name"
        Launcher = "powershell.exe"
        Arguments = $args -join " "
    } | ConvertTo-Json -Depth 4
    exit 0
}

$process = Start-Process -FilePath "powershell.exe" -ArgumentList $args -WindowStyle Normal -PassThru

[pscustomobject]@{
    ProcessId = $process.Id
    Repo = $resolvedRepo
    PromptPath = $resolvedPrompt
    HandoffPath = $HandoffPath
    WindowTitle = "Codex worker: $Name"
} | ConvertTo-Json -Depth 4
