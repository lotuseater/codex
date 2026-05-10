$ErrorActionPreference = 'Continue'
$start = Get-Date
$preFreeGB = (Get-PSDrive C).Free / 1GB
Write-Host '### Slice2 timed cold build start ###'
Write-Host ("Pre-build free C: {0:F2} GB; start: {1}" -f $preFreeGB, $start.ToString('s'))
$exit = 0
try {
    & "$PSScriptRoot/build-local-codex.ps1" -Mode LowMemRelease
    $exit = $LASTEXITCODE
} catch {
    $exit = 1
    Write-Host "build threw: $_"
}
$end = Get-Date
$elapsed = $end - $start
$postFreeGB = (Get-PSDrive C).Free / 1GB
$rel = "$PSScriptRoot/../codex-rs/target/release"
$postTargetSize = 0
if (Test-Path $rel) {
    $postTargetSize = (Get-ChildItem $rel -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1GB
}
$dbg = "$PSScriptRoot/../codex-rs/target/debug"
if (Test-Path $dbg) {
    $dbgSize = (Get-ChildItem $dbg -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1MB
    Remove-Item -Recurse -Force $dbg -ErrorAction SilentlyContinue
    Write-Host ("target/debug existed ({0:F1} MB); cleaned" -f $dbgSize)
} else {
    Write-Host 'target/debug absent (good)'
}
Write-Host '### Slice2 timed cold build end ###'
Write-Host ("Wall time: {0:F1} min ({1:F0} s)" -f $elapsed.TotalMinutes, $elapsed.TotalSeconds)
Write-Host ("Post-build free C: {0:F2} GB; target/release: {1:F2} GB" -f $postFreeGB, $postTargetSize)
exit $exit
