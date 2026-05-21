param(
    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [string]$WindowTitle = "SOLID refactor director - Codex",
    [int]$WaitMs = 800,
    [int]$Repeat = 1
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "terminal-paste-enter.ps1")

if (-not (Test-Path -LiteralPath $StatePath)) {
    throw "Director state file not found: $StatePath. Start or remember exactly one director first."
}

$state = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
$rootPid = [int]$state.rootPid
if ($state.windowTitle) {
    $WindowTitle = [string]$state.windowTitle
} elseif ($state.title) {
    $WindowTitle = [string]$state.title
}
$windowHandle = 0
if ($state.windowHandle) {
    $windowHandle = [long]$state.windowHandle
}

$send = Invoke-SolidTerminalSendKeys -Keys "{ESC}" -Title $WindowTitle -RootPid $rootPid -WindowHandle $windowHandle -WaitMs $WaitMs -Repeat $Repeat

[pscustomobject]@{
    Interrupted = $true
    RootPid = $rootPid
    WindowTitle = $WindowTitle
    WindowHandle = $send.WindowHandle
    Activation = $send.Method
    Key = "Esc"
    Repeat = $Repeat
} | Format-List
