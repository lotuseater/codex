$ErrorActionPreference = 'Stop'
Set-Location 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$env:NO_COLOR = '1'
if ([string]::IsNullOrWhiteSpace($env:NO_PROXY)) {
    $env:NO_PROXY = 'localhost,127.0.0.1,::1'
} elseif ($env:NO_PROXY -notmatch '(^|,)localhost(,|$)') {
    $env:NO_PROXY = $env:NO_PROXY + ',localhost,127.0.0.1,::1'
}
& .\scripts\build-local-codex.ps1 -Mode FastRelease
$code = $LASTEXITCODE
Set-Content -LiteralPath '.codex\workflow\agents\final_build_deploy\build.exit.txt' -Value $code -Encoding ascii
exit $code
