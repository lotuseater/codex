param(
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,

    [string]$CodexCommand = "C:\Users\Oleh\bin\codex.ps1",

    [string]$Pattern = "solid_refactor_wave3_*.prompt.md",

    [string[]]$WorkerNames,

    [switch]$Hidden,

    [string]$CommandPolicy = "hard_command_ban=cargo,rustc,just,bazel,build/test scripts,schema generation,deploy/activation",

    [switch]$List,

    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$agentsDir = $PSScriptRoot
$runner = Join-Path $agentsDir "launch-solid-refactor-worker.ps1"

if (-not $WorkerNames -or $WorkerNames.Count -eq 0) {
    $WorkerNames = Get-ChildItem -LiteralPath $agentsDir -Filter $Pattern |
        Sort-Object Name |
        ForEach-Object { $_.BaseName -replace "\.prompt$", "" }
}

if ($List) {
    $WorkerNames | ForEach-Object {
        [pscustomobject]@{
            Worker = $_
            Prompt = Join-Path $agentsDir "$_.prompt.md"
            Handoff = Join-Path $agentsDir "$_.handoff.md"
        }
    } | Format-Table -AutoSize
    exit 0
}

$launched = foreach ($name in $WorkerNames) {
    $promptPath = Join-Path $agentsDir "$name.prompt.md"
    if (-not (Test-Path -LiteralPath $promptPath)) {
        throw "Prompt file not found: $promptPath"
    }

    $logPath = Join-Path $agentsDir "$name.exec.visible.log"
    $markerPath = Join-Path $agentsDir "$name.exec.marker.txt"

    if ($DryRun) {
        [pscustomobject]@{
            Worker = $name
            Pid = $null
            Visible = -not $Hidden
            Prompt = $promptPath
            Log = $logPath
            DryRun = $true
        }
        continue
    }

    if (Test-Path -LiteralPath $logPath) {
        Remove-Item -LiteralPath $logPath -Force
    }

    $args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-NoExit",
        "-File", $runner,
        "-Repo", $Repo,
        "-PromptPath", $promptPath,
        "-CodexCommand", $CodexCommand,
        "-LogPath", $logPath
    )

    $startArgs = @{
        FilePath = "powershell.exe"
        ArgumentList = $args
        WorkingDirectory = $Repo
        PassThru = $true
    }

    if ($Hidden) {
        $startArgs.WindowStyle = "Hidden"
    } else {
        $startArgs.WindowStyle = "Normal"
    }

    $process = Start-Process @startArgs
    @(
        "$(Get-Date -Format o) mode=ExecFullPromptVisible pid=$($process.Id) repo=$Repo prompt=$promptPath launcher=$runner codex=$CodexCommand",
        "visible=$(-not $Hidden)",
        "log=$logPath",
        $CommandPolicy,
        "commit_policy=prompt-specific; designated workers may commit focused verified slices"
    ) | Set-Content -LiteralPath $markerPath

    [pscustomobject]@{
        Worker = $name
        Pid = $process.Id
        Visible = -not $Hidden
        Prompt = $promptPath
        Log = $logPath
    }
}

$launched | Format-Table -AutoSize
