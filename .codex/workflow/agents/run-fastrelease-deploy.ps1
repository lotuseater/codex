param(
    [Parameter(Mandatory=$true)]
    [string]$RepoRoot,
    [Parameter(Mandatory=$true)]
    [string]$LogPath,
    [Parameter(Mandatory=$true)]
    [string]$ExitPath
)

Set-Location -LiteralPath $RepoRoot
$ErrorActionPreference = 'Continue'
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $RepoRoot 'scripts\build-local-codex.ps1') -Mode FastRelease -Timings *>&1 | Tee-Object -FilePath $LogPath
$exitCode = if ($null -ne $LASTEXITCODE) { [int]$LASTEXITCODE } else { 0 }
Set-Content -LiteralPath $ExitPath -Value $exitCode -Encoding ascii
exit $exitCode
