param(
    [string]$WorkDir = (Join-Path $PSScriptRoot "..\tmp"),
    [int]$TimeoutMs = 8000
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "terminal-paste-enter.ps1")

New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
$id = [guid]::NewGuid().ToString("N")
$title = "solid-esc-compact-canary-$id"
$resultPath = Join-Path $WorkDir "$title.result.json"
$scriptPath = Join-Path $WorkDir "$title.ps1"

@"
`$Host.UI.RawUI.WindowTitle = "$title"
`$key = `$Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
`$line = Read-Host
[pscustomobject]@{
    VirtualKeyCode = `$key.VirtualKeyCode
    Character = [int][char]`$key.Character
    Line = `$line
} | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath "$resultPath" -Encoding UTF8
"@ | Set-Content -LiteralPath $scriptPath -Encoding UTF8

$process = $null
try {
    $process = Start-Process -FilePath powershell.exe -ArgumentList @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", "`"$scriptPath`""
    ) -PassThru -WindowStyle Normal

    Start-Sleep -Milliseconds 800
    Invoke-SolidTerminalSendKeys -Keys "{ESCAPE}" -Title $title -RootPid $process.Id -WaitMs $TimeoutMs | Out-Null
    Start-Sleep -Milliseconds 300
    Invoke-SolidTerminalPasteEnter -Message "/compact" -Title $title -RootPid $process.Id -WaitMs $TimeoutMs -SubmitRepeat 3 | Out-Null

    $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
    while ((Get-Date) -lt $deadline -and -not (Test-Path -LiteralPath $resultPath)) {
        Start-Sleep -Milliseconds 100
    }

    if (-not (Test-Path -LiteralPath $resultPath)) {
        throw "Canary did not produce result: $resultPath"
    }

    $result = Get-Content -LiteralPath $resultPath -Raw | ConvertFrom-Json
    if ([int]$result.VirtualKeyCode -ne 27) {
        throw "Expected first key Esc (27), got $($result.VirtualKeyCode)"
    }

    if ([string]$result.Line -ne "/compact") {
        throw "Expected submitted line '/compact', got '$($result.Line)'"
    }

    [pscustomobject]@{
        Passed = $true
        ResultPath = $resultPath
        FirstKey = "Esc"
        SubmittedLine = $result.Line
    } | Format-List
} finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}
