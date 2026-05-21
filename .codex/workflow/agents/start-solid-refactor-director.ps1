param(
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$CodexCommand = "C:\Users\Oleh\bin\codex.ps1",
    [string]$PowerShellCommand = $(if (Test-Path -LiteralPath (Join-Path $PSHOME "pwsh.exe")) { Join-Path $PSHOME "pwsh.exe" } else { Join-Path $PSHOME "powershell.exe" }),
    [string]$PromptPath = (Join-Path $PSScriptRoot "..\solid-refactor-director-prompt.md"),
    [string]$LogPath = (Join-Path $PSScriptRoot "solid_refactor_director.exec.visible.log"),
    [string]$MarkerPath = (Join-Path $PSScriptRoot "solid_refactor_director.exec.marker.txt"),
    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [ValidateSet("Start", "Resume")]
    [string]$Mode = "Start",
    [int]$InitialPromptDelaySeconds = 6,
    [int]$InitialPromptWaitMs = 10000,
    [int]$InitialPromptSubmitDelayMs = 300,
    [switch]$DryRun,
    [switch]$NoStopExisting
)

$ErrorActionPreference = "Stop"

$directorTitle = "SOLID refactor director - Codex"

function Resolve-FullPath([string]$Path) {
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
}

function Resolve-ExecutablePath([string]$Command, [string]$FallbackCommand) {
    $resolved = (Get-Command $Command -ErrorAction SilentlyContinue).Source
    if ($resolved) {
        return $resolved
    }

    if ($FallbackCommand) {
        $fallback = (Get-Command $FallbackCommand -ErrorAction SilentlyContinue).Source
        if ($fallback) {
            return $fallback
        }
    }

    if (Test-Path -LiteralPath $Command) {
        return (Resolve-Path -LiteralPath $Command).Path
    }

    throw "Executable not found: $Command"
}

function Update-DirectorWindowState {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [object]$Window
    )

    $state = [ordered]@{}
    if (Test-Path -LiteralPath $Path) {
        $existing = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
        foreach ($property in $existing.PSObject.Properties) {
            $state[$property.Name] = $property.Value
        }
    }

    $state["windowHandle"] = $Window.Handle.ToInt64()
    $state["windowPid"] = [int]$Window.ProcessId
    $state["windowTitle"] = [string]$Window.Title
    $state["windowRememberedAt"] = (Get-Date).ToString("o")

    [pscustomobject]$state | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $Path -Encoding UTF8
}

$Repo = Resolve-FullPath $Repo
$PromptPath = Resolve-FullPath $PromptPath
$LogPath = Resolve-FullPath $LogPath
$MarkerPath = Resolve-FullPath $MarkerPath
$StatePath = Resolve-FullPath $StatePath
$CodexCommand = Resolve-ExecutablePath $CodexCommand $null
$PowerShellCommand = Resolve-ExecutablePath $PowerShellCommand "powershell.exe"
$runScript = Resolve-FullPath (Join-Path $PSScriptRoot "run-solid-refactor-director.ps1")
$stopScript = Resolve-FullPath (Join-Path $PSScriptRoot "stop-solid-refactor-director.ps1")

if (-not (Test-Path -LiteralPath $PromptPath)) {
    throw "Director prompt not found: $PromptPath"
}

if ($DryRun) {
    [pscustomobject]@{
        Director = "solid_refactor_director"
        Mode = $Mode
        Title = $directorTitle
        Repo = $Repo
        Codex = $CodexCommand
        PowerShell = $PowerShellCommand
        Prompt = $PromptPath
        Log = $LogPath
        Marker = $MarkerPath
        State = $StatePath
        WouldStopExisting = -not $NoStopExisting
        InitialPromptDelaySeconds = $InitialPromptDelaySeconds
        InitialPromptWaitMs = $InitialPromptWaitMs
        InitialPromptSubmitDelayMs = $InitialPromptSubmitDelayMs
    } | Format-List
    exit 0
}

if (-not $NoStopExisting) {
    & $stopScript -StatePath $StatePath -ScanFallback -Quiet
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogPath) | Out-Null
. (Join-Path $PSScriptRoot "terminal-paste-enter.ps1")
$baselineHandles = @(Get-SolidVisibleWindows | ForEach-Object { $_.Handle.ToInt64() })

$argumentList = @(
    "-NoExit",
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-File", "`"$runScript`"",
    "-Repo", "`"$Repo`"",
    "-CodexCommand", "`"$CodexCommand`"",
    "-PromptPath", "`"$PromptPath`"",
    "-LogPath", "`"$LogPath`"",
    "-StatePath", "`"$StatePath`"",
    "-Mode", $Mode
)

$process = Start-Process -FilePath $PowerShellCommand -ArgumentList $argumentList -PassThru -WindowStyle Normal

[pscustomobject]@{
    rootPid = $process.Id
    title = $directorTitle
    mode = $Mode
    repo = $Repo
    promptPath = $PromptPath
    logPath = $LogPath
    markerPath = $MarkerPath
    startedAt = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $StatePath -Encoding UTF8

"started $(Get-Date -Format o) pid=$($process.Id) mode=$Mode title=$directorTitle" |
    Set-Content -LiteralPath $MarkerPath -Encoding UTF8

$targetWindow = $null
if ($Mode -eq "Start") {
    Start-Sleep -Seconds $InitialPromptDelaySeconds
    $targetWindow = Wait-SolidTerminalWindow -Title $directorTitle -RootPid $process.Id -BaselineHandles $baselineHandles -WaitMs $InitialPromptWaitMs
    $windowHandle = 0
    if ($targetWindow) {
        $windowHandle = $targetWindow.Handle.ToInt64()
        Update-DirectorWindowState -Path $StatePath -Window $targetWindow
    }

    $prompt = Get-Content -LiteralPath $PromptPath -Raw
    Invoke-SolidTerminalPasteEnter -Message $prompt -Title $directorTitle -RootPid $process.Id -WindowHandle $windowHandle -WaitMs $InitialPromptWaitMs -SubmitDelayMs $InitialPromptSubmitDelayMs | Out-Null
}

[pscustomobject]@{
    Director = "solid_refactor_director"
    Pid = $process.Id
    Mode = $Mode
    Title = $directorTitle
    WindowTitle = $(if ($targetWindow) { $targetWindow.Title } else { $null })
    WindowHandle = $(if ($targetWindow) { $targetWindow.Handle.ToInt64() } else { $null })
    Visible = $true
    State = $StatePath
} | Format-Table -AutoSize
