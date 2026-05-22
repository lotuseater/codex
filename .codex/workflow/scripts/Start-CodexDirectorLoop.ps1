[CmdletBinding()]
param(
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$CodexCommand = "codex",
    [string]$SessionPath,
    [string]$SessionId,
    [string]$SearchText = "You are the SOLID refactor director",
    [string]$SessionRoot = (Join-Path $HOME ".codex\sessions"),
    [string]$PromptFile = (Join-Path (Split-Path -Parent $PSScriptRoot) "solid-refactor-director-prompt.md"),
    [string]$Title = "Codex SOLID Director",
    [switch]$FreshStart,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

Import-Module (Join-Path $PSScriptRoot "CodexSessionMaintenance.psm1") -Force

$profile = Get-CodexMaintenanceProfile -Profile Director
$reminder = $profile.Reminder
$resolvedSessionPath = $null

if (-not $FreshStart) {
    $resolvedSessionPath = Resolve-CodexSessionPath -SessionPath $SessionPath -SessionId $SessionId -SearchText $SearchText -SessionRoot $SessionRoot
    if (-not $SessionId) {
        $SessionId = Get-CodexSessionIdFromPath -SessionPath $resolvedSessionPath
    }
    if (-not $SessionId) {
        throw "Could not read session id from: $resolvedSessionPath"
    }
}

$prompt = $reminder
if ($FreshStart) {
    if (-not (Test-Path -LiteralPath $PromptFile)) {
        throw "Director prompt file not found: $PromptFile"
    }
    $prompt = (Get-Content -Raw -LiteralPath $PromptFile).TrimEnd() + "`n`n" + $reminder
}

$command = New-CodexLoopResumeCommand -CodexCommand $CodexCommand -SessionId $SessionId -Prompt $prompt

$plan = [pscustomobject]@{
    status = if ($FreshStart) { "fresh_start_planned" } else { "resume_planned" }
    title = $Title
    repo = $Repo
    sessionPath = $resolvedSessionPath
    sessionId = $SessionId
    commandLine = $command.CommandLine
    arguments = $command.Arguments
    dryRun = [bool]$DryRun
}

if ($DryRun) {
    return $plan
}

$tmpDir = Join-Path (Split-Path -Parent $PSScriptRoot) "tmp"
if (-not (Test-Path -LiteralPath $tmpDir)) {
    New-Item -ItemType Directory -Path $tmpDir | Out-Null
}

$runnerPath = Join-Path $tmpDir ("director-loop-{0}.ps1" -f (Get-Date -Format "yyyyMMdd-HHmmss"))
$encodedArgs = $command.Arguments | ForEach-Object {
    "'" + ($_.Replace("'", "''")) + "'"
}

@"
Set-Location '$($Repo.Replace("'", "''"))'
& $($encodedArgs -join " ")
if (`$LASTEXITCODE -ne `$null) {
    exit `$LASTEXITCODE
}
"@ | Set-Content -LiteralPath $runnerPath -Encoding UTF8

$wt = Get-Command wt.exe -ErrorAction SilentlyContinue
if ($null -eq $wt) {
    throw "Windows Terminal wt.exe was not found."
}

$wtArgs = @(
    "new-tab",
    "--title", $Title,
    "powershell",
    "-NoExit",
    "-ExecutionPolicy", "Bypass",
    "-File", $runnerPath
)

$process = Start-Process -FilePath $wt.Source -ArgumentList $wtArgs -WindowStyle Normal -PassThru

[pscustomobject]@{
    status = "launched"
    title = $Title
    repo = $Repo
    sessionPath = $resolvedSessionPath
    sessionId = $SessionId
    runnerPath = $runnerPath
    pid = $process.Id
    commandLine = $command.CommandLine
}
