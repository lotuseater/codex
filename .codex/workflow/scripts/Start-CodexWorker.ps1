[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PromptFile,

    [string]$Repo,

    [string]$Title = "codex-worker",

    [ValidateSet("Interactive", "Exec", "Version")]
    [string]$Mode = "Interactive",

    [string]$MarkerFile,

    [switch]$CurrentWindow
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$runner = Join-Path $scriptDir "Invoke-CodexWorker.ps1"
if (-not (Test-Path -LiteralPath $runner)) {
    throw "Runner script not found: $runner"
}

if (-not $Repo) {
    $Repo = (Resolve-Path -LiteralPath (Join-Path $scriptDir "..\..\..")).Path
}
else {
    $Repo = (Resolve-Path -LiteralPath $Repo).Path
}

$PromptFile = (Resolve-Path -LiteralPath $PromptFile).Path

$runnerArgs = @(
    "-NoLogo",
    "-ExecutionPolicy", "Bypass",
    "-File", $runner,
    "-Repo", $Repo,
    "-PromptFile", $PromptFile,
    "-Mode", $Mode
)

if ($MarkerFile) {
    $runnerArgs += @("-MarkerFile", $MarkerFile)
}

if ($CurrentWindow) {
    & powershell @runnerArgs
    exit $LASTEXITCODE
}

$wt = Get-Command wt.exe -ErrorAction Stop
$wtArgs = @(
    "-w", "0",
    "new-tab",
    "--title", $Title,
    "powershell",
    "-NoExit"
) + $runnerArgs

Start-Process -FilePath $wt.Source -ArgumentList $wtArgs -WindowStyle Normal

[pscustomobject]@{
    status = "launched"
    title = $Title
    mode = $Mode
    prompt_file = $PromptFile
    marker_file = $MarkerFile
}
