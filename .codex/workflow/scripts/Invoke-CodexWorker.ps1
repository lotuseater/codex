[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Repo,

    [Parameter(Mandatory = $true)]
    [string]$PromptFile,

    [ValidateSet("Interactive", "Exec", "Version")]
    [string]$Mode = "Interactive",

    [string]$MarkerFile,

    [string]$CodexCommand = "codex",

    [string]$WorkerModel = "gpt-5.5",

    [string]$WorkerReasoningEffort = "xhigh"
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$launchModule = Join-Path $scriptDir "..\agents\CodexWorkerLaunch.psm1"
Import-Module $launchModule -Force -DisableNameChecking

function Write-Marker {
    param([string]$Message)

    if (-not $MarkerFile) {
        return
    }

    $markerParent = Split-Path -Parent $MarkerFile
    if ($markerParent) {
        New-Item -ItemType Directory -Force -Path $markerParent | Out-Null
    }
    Add-Content -LiteralPath $MarkerFile -Value "$(Get-Date -Format o) $Message"
}

$repoPath = (Resolve-Path -LiteralPath $Repo).Path
$promptPath = (Resolve-Path -LiteralPath $PromptFile).Path

Set-Location -LiteralPath $repoPath
$commandHealth = Get-CodexWorkerCommandHealth -CodexCommand $CodexCommand

Write-Marker "starting mode=$Mode repo=$repoPath prompt=$promptPath codex=$($commandHealth.Source) wrapper_real_exe=$($commandHealth.WrapperRealExe)"

try {
    switch ($Mode) {
        "Version" {
            & $CodexCommand --version
            $exitCode = $LASTEXITCODE
        }
        "Exec" {
            $prompt = Get-Content -Raw -LiteralPath $promptPath
            $codexArgs = New-CodexWorkerExecArgs `
                -Repo $repoPath `
                -Prompt $prompt `
                -WorkerModel $WorkerModel `
                -WorkerReasoningEffort $WorkerReasoningEffort
            & $CodexCommand @codexArgs
            $exitCode = $LASTEXITCODE
        }
        "Interactive" {
            $prompt = Get-Content -Raw -LiteralPath $promptPath
            $codexArgs = New-CodexWorkerInteractiveArgs `
                -Repo $repoPath `
                -Prompt $prompt `
                -WorkerModel $WorkerModel `
                -WorkerReasoningEffort $WorkerReasoningEffort
            & $CodexCommand @codexArgs
            $exitCode = $LASTEXITCODE
        }
    }
}
catch {
    Write-Marker "failed error=$($_.Exception.Message)"
    throw
}

Write-Marker "completed mode=$Mode exit=$exitCode"
exit $exitCode
