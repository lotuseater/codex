[CmdletBinding()]
param(
    [string]$WorkDir = (Join-Path (Split-Path -Parent $PSScriptRoot) "tmp\solid-director-runner-tests")
)

$ErrorActionPreference = "Stop"

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) {
        throw $Message
    }
}

function Assert-ArrayEquals([object[]]$Actual, [object[]]$Expected, [string]$Message) {
    $actualText = [string]::Join("`u{1f}", [string[]]$Actual)
    $expectedText = [string]::Join("`u{1f}", [string[]]$Expected)
    if ($actualText -ne $expectedText) {
        throw "$Message`nExpected: $expectedText`nActual:   $actualText"
    }
}

$runner = Join-Path $PSScriptRoot "run-solid-refactor-director.ps1"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$workDirFull = [IO.Path]::GetFullPath($WorkDir)
New-Item -ItemType Directory -Force -Path $workDirFull | Out-Null

$promptPath = Join-Path $workDirFull "director-prompt.md"
$argLogPath = Join-Path $workDirFull "fake-codex-args.jsonl"
$fakeCodexPath = Join-Path $workDirFull "fake-codex.ps1"
$statePath = Join-Path $workDirFull "director-state.json"
$runnerLogPath = Join-Path $workDirFull "runner.log"

Set-Content -LiteralPath $promptPath -Encoding UTF8 -Value "DIRECTOR PROMPT"
Remove-Item -LiteralPath $argLogPath -Force -ErrorAction SilentlyContinue

@'
[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RemainingArguments
)

[pscustomobject]@{
    args = @($RemainingArguments)
} | ConvertTo-Json -Compress | Add-Content -LiteralPath $env:FAKE_CODEX_ARG_LOG -Encoding UTF8
'@ | Set-Content -LiteralPath $fakeCodexPath -Encoding UTF8

$env:FAKE_CODEX_ARG_LOG = $argLogPath

try {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -Repo $repo `
        -CodexCommand $fakeCodexPath `
        -PromptPath $promptPath `
        -LogPath $runnerLogPath `
        -StatePath $statePath `
        -Mode Resume `
        -ResumeSessionId "session-123"

    & powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -Repo $repo `
        -CodexCommand $fakeCodexPath `
        -PromptPath $promptPath `
        -LogPath $runnerLogPath `
        -StatePath $statePath `
        -Mode Resume

    & powershell -NoProfile -ExecutionPolicy Bypass -File $runner `
        -Repo $repo `
        -CodexCommand $fakeCodexPath `
        -PromptPath $promptPath `
        -LogPath $runnerLogPath `
        -StatePath $statePath `
        -Mode Start
} finally {
    Remove-Item Env:\FAKE_CODEX_ARG_LOG -ErrorAction SilentlyContinue
}

$calls = @(Get-Content -LiteralPath $argLogPath | ForEach-Object { $_ | ConvertFrom-Json })
Assert-True ($calls.Count -eq 3) "expected exactly three fake codex invocations"

Assert-ArrayEquals @($calls[0].args) @(
    "--loop",
    "--cd", $repo,
    "--ask-for-approval", "never",
    "--sandbox", "danger-full-access",
    "resume",
    "session-123"
) "resume with explicit session should avoid the prompt argument"

Assert-ArrayEquals @($calls[1].args) @(
    "--loop",
    "--cd", $repo,
    "--ask-for-approval", "never",
    "--sandbox", "danger-full-access",
    "resume",
    "--last"
) "resume without explicit session should pass --last without a prompt argument"

Assert-ArrayEquals @($calls[2].args) @(
    "--loop",
    "--cd", $repo,
    "--ask-for-approval", "never",
    "--sandbox", "danger-full-access"
) "start mode should keep using the interactive prompt path"

[pscustomobject]@{
    status = "passed"
    workDir = $workDirFull
    calls = $calls.Count
} | Format-List
