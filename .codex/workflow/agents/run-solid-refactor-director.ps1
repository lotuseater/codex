param(
    [Parameter(Mandatory = $true)]
    [string]$Repo,

    [Parameter(Mandatory = $true)]
    [string]$CodexCommand,

    [Parameter(Mandatory = $true)]
    [string]$PromptPath,

    [Parameter(Mandatory = $true)]
    [string]$LogPath,

    [Parameter(Mandatory = $true)]
    [string]$StatePath,

    [ValidateSet("Start", "Resume")]
    [string]$Mode = "Start",

    [string]$ResumeSessionId
)

$ErrorActionPreference = "Stop"
$directorTitle = "SOLID refactor director - Codex"
$Host.UI.RawUI.WindowTitle = $directorTitle
Set-Location -LiteralPath $Repo

$state = [pscustomobject]@{
    rootPid = $PID
    title = $directorTitle
    mode = $Mode
    repo = $Repo
    promptPath = $PromptPath
    logPath = $LogPath
    markerPath = (Join-Path (Split-Path -Parent $StatePath) "solid_refactor_director.exec.marker.txt")
    sessionId = $(if ($ResumeSessionId) { $ResumeSessionId } else { $null })
    runnerStartedAt = (Get-Date).ToString("o")
}
$state | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $StatePath -Encoding UTF8

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogPath) | Out-Null
@(
    "started $(Get-Date -Format o)"
    "pid=$PID"
    "mode=$Mode"
    "repo=$Repo"
    "prompt=$PromptPath"
) | Add-Content -LiteralPath $LogPath -Encoding UTF8

try {
    Write-Host "SOLID refactor director mode: $Mode"
    Write-Host "Repo: $Repo"
    Write-Host "Prompt: $PromptPath"

    if ($Mode -eq "Resume") {
        if ($ResumeSessionId) {
            & $CodexCommand --loop --cd $Repo --ask-for-approval never --sandbox danger-full-access resume $ResumeSessionId
        } else {
            & $CodexCommand --loop --cd $Repo --ask-for-approval never --sandbox danger-full-access resume --last
        }
    } else {
        & $CodexCommand --loop --cd $Repo --ask-for-approval never --sandbox danger-full-access
    }
} finally {
    "ended $(Get-Date -Format o) pid=$PID exit=$LASTEXITCODE" | Add-Content -LiteralPath $LogPath -Encoding UTF8
}
