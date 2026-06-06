param(
    [string]$StandardCliPath,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$helper = Join-Path $PSScriptRoot 'scripts\switch-ui-codex-runtime.ps1'

if ($StandardCliPath) {
    & $helper -Mode Standard -StandardCliPath $StandardCliPath -DryRun:$DryRun
} else {
    & $helper -Mode Standard -DryRun:$DryRun
}
exit $LASTEXITCODE
