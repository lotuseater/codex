param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$Filter = "early_pressure"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$testScript = Join-Path $RepoRoot "scripts\test-local-codex-release.ps1"
if (-not (Test-Path -LiteralPath $testScript)) {
    throw "Missing release test helper: $testScript"
}

& powershell -ExecutionPolicy Bypass -File $testScript `
    -RepoRoot $RepoRoot `
    -Package codex-context-reduction `
    -Filter $Filter
