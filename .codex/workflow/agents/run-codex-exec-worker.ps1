param(
    [Parameter(Mandatory = $true)]
    [string]$Worker,

    [string]$Model = "gpt-5.5",

    [string]$ReasoningEffort = "xhigh",

    [string]$CodexCommand = "codex"
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$promptPath = Join-Path $PSScriptRoot "$Worker.prompt.md"
$logPath = Join-Path $PSScriptRoot "$Worker.exec.log"
$donePath = Join-Path $PSScriptRoot "$Worker.exec.done.txt"
$markerPath = Join-Path $PSScriptRoot "$Worker.exec.marker.txt"

$started = Get-Date -Format o
@(
    "$started mode=DirectExecHidden pid=$PID worker=$Worker repo=$repo prompt=$promptPath model=$Model reasoning=$ReasoningEffort",
    "log=$logPath",
    "done=$donePath",
    "policy=no source edits; no commits; no builds/tests; write handoff only"
) | Set-Content -LiteralPath $markerPath -Encoding UTF8

try {
    if (-not (Test-Path -LiteralPath $promptPath)) {
        throw "Prompt not found: $promptPath"
    }

    $prompt = Get-Content -LiteralPath $promptPath -Raw
    $codexArgs = @(
        '-c', "model=$Model",
        '-c', "model_reasoning_effort=$ReasoningEffort",
        '--cd', $repo,
        '--ask-for-approval', 'never',
        '--sandbox', 'danger-full-access',
        'exec',
        '--',
        $prompt
    )

    & $CodexCommand @codexArgs *>&1 | Tee-Object -FilePath $logPath
    $exit = $LASTEXITCODE
    "$(Get-Date -Format o) exit=$exit worker=$Worker" | Set-Content -LiteralPath $donePath -Encoding UTF8
    exit $exit
}
catch {
    "$(Get-Date -Format o) ERROR worker=$Worker $($_.Exception.Message)" | Tee-Object -FilePath $logPath
    "$(Get-Date -Format o) exit=1 worker=$Worker" | Set-Content -LiteralPath $donePath -Encoding UTF8
    exit 1
}
