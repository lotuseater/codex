param(
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$PromptPath = (Join-Path $PSScriptRoot "self-review-main.prompt.md"),
    [string]$LogPath = (Join-Path $PSScriptRoot "handoffs\self-review-main.exec.log"),
    [string]$DonePath = (Join-Path $PSScriptRoot "handoffs\self-review-main.exec.done.txt"),
    [string]$CodexCommand = "codex",
    [string]$WorkerModel = "gpt-5.5",
    [string]$WorkerReasoningEffort = "xhigh"
)

$ErrorActionPreference = "Stop"
Set-Location -LiteralPath $Repo

$handoffDir = Split-Path -Parent $LogPath
if (-not (Test-Path -LiteralPath $handoffDir)) {
    New-Item -ItemType Directory -Force -Path $handoffDir | Out-Null
}

"started=$(Get-Date -Format o)" | Set-Content -LiteralPath $DonePath -Encoding UTF8
"repo=$Repo" | Add-Content -LiteralPath $DonePath -Encoding UTF8
"prompt=$PromptPath" | Add-Content -LiteralPath $DonePath -Encoding UTF8
"log=$LogPath" | Add-Content -LiteralPath $DonePath -Encoding UTF8

$env:NO_COLOR = "1"
$env:CODEX_WORKER_COMMAND_POLICY = "hard_command_ban=cargo,rustc,just,bazel,build/test scripts,schema generation,deploy/activation,full workspace tests,release builds"

try {
    $promptText = Get-Content -LiteralPath $PromptPath -Raw
    & $CodexCommand --model $WorkerModel --config "model_reasoning_effort=$WorkerReasoningEffort" --cd $Repo --ask-for-approval never --sandbox danger-full-access exec $promptText 2>&1 |
        Tee-Object -FilePath $LogPath
    $exitCode = $LASTEXITCODE
} catch {
    $_ | Out-String | Add-Content -LiteralPath $LogPath -Encoding UTF8
    $exitCode = 1
}

"finished=$(Get-Date -Format o)" | Add-Content -LiteralPath $DonePath -Encoding UTF8
"exit_code=$exitCode" | Add-Content -LiteralPath $DonePath -Encoding UTF8
exit $exitCode
