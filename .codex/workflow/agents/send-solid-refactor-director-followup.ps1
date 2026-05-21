param(
    [Parameter(Mandatory = $true)]
    [string]$Message,

    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [string]$WindowTitle = "SOLID refactor director - Codex",
    [int]$WaitMs = 5000,
    [int]$SubmitDelayMs = 300,
    [int]$SubmitRepeat = 1
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

$processAlive = $false
if ($rootPid -gt 0) {
    $processAlive = [bool](Get-Process -Id $rootPid -ErrorAction SilentlyContinue)
}

$send = Invoke-SolidTerminalPasteEnter -Message $Message -Title $WindowTitle -RootPid $rootPid -WindowHandle $windowHandle -WaitMs $WaitMs -SubmitDelayMs $SubmitDelayMs -SubmitRepeat $SubmitRepeat

[pscustomobject]@{
    Sent = $true
    RootPid = $rootPid
    RootProcessAlive = $processAlive
    WindowTitle = $WindowTitle
    WindowHandle = $send.WindowHandle
    Activation = $send.Method
    SubmitKey = $send.SubmitKey
    SubmitRepeat = $send.SubmitRepeat
    MessageLength = $Message.Length
} | Format-List
