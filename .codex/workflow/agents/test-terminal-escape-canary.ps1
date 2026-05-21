param(
    [string]$WorkDir = (Join-Path $PSScriptRoot "..\tmp"),
    [int]$TimeoutMs = 5000
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "terminal-paste-enter.ps1")

$pwsh = Join-Path $PSHOME "pwsh.exe"
if (-not (Test-Path -LiteralPath $pwsh)) {
    $pwsh = Join-Path $PSHOME "powershell.exe"
}
if (-not (Test-Path -LiteralPath $pwsh)) {
    $pwsh = (Get-Command pwsh -ErrorAction SilentlyContinue).Source
}
if (-not $pwsh) {
    $pwsh = (Get-Command powershell.exe -ErrorAction Stop).Source
}

$resolvedWorkDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($WorkDir)
New-Item -ItemType Directory -Force -Path $resolvedWorkDir | Out-Null

$runId = "escape_canary_{0:yyyyMMdd_HHmmss_fff}_{1}" -f (Get-Date), $PID
$title = "SOLID escape canary $runId"
$resultPath = Join-Path $resolvedWorkDir "$runId.result.txt"
$runnerPath = Join-Path $resolvedWorkDir "$runId.runner.ps1"

$runner = @'
param(
    [Parameter(Mandatory = $true)]
    [string]$Title,

    [Parameter(Mandatory = $true)]
    [string]$ResultPath,

    [int]$HoldMs = 250
)

$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = $Title
Write-Host "Escape canary ready: $Title"
$key = [Console]::ReadKey($true)
[System.IO.File]::WriteAllText($ResultPath, $key.Key.ToString(), [System.Text.UTF8Encoding]::new($false))
Start-Sleep -Milliseconds $HoldMs
'@

Set-Content -LiteralPath $runnerPath -Value $runner -Encoding UTF8

$argumentList = @(
    "-NoLogo",
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-File", "`"$runnerPath`"",
    "-Title", "`"$title`"",
    "-ResultPath", "`"$resultPath`""
)

$process = Start-Process -FilePath $pwsh -ArgumentList $argumentList -PassThru -WindowStyle Normal

try {
    $send = Invoke-SolidTerminalSendKeys -Keys "{ESC}" -Title $title -RootPid $process.Id -WaitMs $TimeoutMs
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while ([DateTime]::UtcNow -lt $deadline -and -not (Test-Path -LiteralPath $resultPath)) {
        Start-Sleep -Milliseconds 50
    }

    if (-not (Test-Path -LiteralPath $resultPath)) {
        throw "Canary did not write a result file: $resultPath"
    }

    $actual = Get-Content -LiteralPath $resultPath -Raw
    if ($actual -ne "Escape") {
        throw "Canary mismatch. Expected 'Escape' but received '$actual'."
    }

    [pscustomobject]@{
        Succeeded = $true
        Key = $actual
        RootPid = $process.Id
        Title = $title
        ResultPath = $resultPath
        Activation = $send.Method
    } | Format-List
} finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}
