param(
    [Parameter(Mandatory = $true)]
    [int]$RootPid,

    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$PromptPath = (Join-Path $PSScriptRoot "..\solid-refactor-director-prompt.md"),
    [string]$LogPath = (Join-Path $PSScriptRoot "solid_refactor_director.exec.visible.log"),
    [string]$MarkerPath = (Join-Path $PSScriptRoot "solid_refactor_director.exec.marker.txt"),
    [string]$Title = "SOLID refactor director - Codex",
    [ValidateSet("Start", "Resume", "Manual")]
    [string]$Mode = "Manual"
)

$ErrorActionPreference = "Stop"

function Resolve-FullPath([string]$Path) {
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
}

if (-not (Get-Process -Id $RootPid -ErrorAction SilentlyContinue)) {
    throw "No live process with PID $RootPid."
}

$StatePath = Resolve-FullPath $StatePath
$Repo = Resolve-FullPath $Repo
$PromptPath = Resolve-FullPath $PromptPath
$LogPath = Resolve-FullPath $LogPath
$MarkerPath = Resolve-FullPath $MarkerPath

[pscustomobject]@{
    rootPid = $RootPid
    title = $Title
    mode = $Mode
    repo = $Repo
    promptPath = $PromptPath
    logPath = $LogPath
    markerPath = $MarkerPath
    rememberedAt = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $StatePath -Encoding UTF8

"remembered $(Get-Date -Format o) pid=$RootPid mode=$Mode title=$Title" |
    Set-Content -LiteralPath $MarkerPath -Encoding UTF8

Get-Content -LiteralPath $StatePath -Raw
