param(
    [string]$CustomCliPath,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$helper = Join-Path $PSScriptRoot 'scripts\switch-ui-codex-runtime.ps1'

if ($CustomCliPath) {
    & $helper -Mode Custom -CustomCliPath $CustomCliPath -DryRun:$DryRun
} else {
    & $helper -Mode Custom -DryRun:$DryRun
}
exit $LASTEXITCODE
