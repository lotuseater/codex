param(
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,

    [string]$CodexCommand = "codex",

    [string]$WorkerModel = "gpt-5.5",

    [string]$WorkerReasoningEffort = "xhigh",

    [string]$Pattern = "solid_refactor_wave3_*.prompt.md",

    [string[]]$WorkerNames,

    [Alias("ResumeSessions")]
    [string[]]$Resume,

    [string]$Prompt,

    [switch]$Loop,

    [string]$LoopMessage,

    [int]$LoopPeriod,

    [switch]$Hidden,

    [string]$CommandPolicy = "hard_command_ban=cargo,rustc,just,bazel,build/test scripts,schema generation,deploy/activation",

    [switch]$List,

    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$agentsDir = $PSScriptRoot
$launchModule = Join-Path $agentsDir "CodexWorkerLaunch.psm1"
Import-Module $launchModule -Force -DisableNameChecking

if ($Loop -and (-not $Resume -or $Resume.Count -eq 0)) {
    throw "-Loop is only supported with -Resume."
}

if ($Resume -and $Resume.Count -gt 0) {
    if (-not $WorkerNames -or $WorkerNames.Count -eq 0) {
        $WorkerNames = for ($i = 0; $i -lt $Resume.Count; $i++) {
            $session = $Resume[$i]
            $prefixLength = [Math]::Min(8, $session.Length)
            "resume_$($i + 1)_$($session.Substring(0, $prefixLength))"
        }
    } elseif ($WorkerNames.Count -ne $Resume.Count) {
        throw "-WorkerNames count must match -Resume count."
    }
} elseif (-not $WorkerNames -or $WorkerNames.Count -eq 0) {
    $WorkerNames = Get-ChildItem -LiteralPath $agentsDir -Filter $Pattern |
        Sort-Object Name |
        ForEach-Object { $_.BaseName -replace "\.prompt$", "" }
}

if ($List) {
    $items = for ($i = 0; $i -lt $WorkerNames.Count; $i++) {
        $name = $WorkerNames[$i]
        [pscustomobject]@{
            Worker = $name
            Prompt = if ($Resume -and $Resume.Count -gt 0) { $Prompt } else { Join-Path $agentsDir "$name.prompt.md" }
            Handoff = Join-Path $agentsDir "$name.handoff.md"
            Resume = if ($Resume -and $Resume.Count -gt 0) { $Resume[$i] } else { $null }
            Loop = [bool]$Loop
            WorkerModel = $WorkerModel
            WorkerReasoningEffort = $WorkerReasoningEffort
        }
    }
    $items | Format-Table -AutoSize
    exit 0
}

$launched = for ($i = 0; $i -lt $WorkerNames.Count; $i++) {
    $name = $WorkerNames[$i]
    $resumeSession = if ($Resume -and $Resume.Count -gt 0) { $Resume[$i] } else { $null }
    $promptPath = Join-Path $agentsDir "$name.prompt.md"

    if (
        [string]::IsNullOrWhiteSpace($resumeSession) -and
        [string]::IsNullOrWhiteSpace($Prompt) -and
        -not (Test-Path -LiteralPath $promptPath)
    ) {
        throw "Prompt file not found: $promptPath"
    }

    $promptText = $Prompt
    if ([string]::IsNullOrWhiteSpace($promptText)) {
        if (Test-Path -LiteralPath $promptPath) {
            $promptText = Get-Content -Raw -LiteralPath $promptPath
        } else {
            throw "Either -Prompt or a prompt file is required."
        }
    }

    $safeName = ConvertTo-CodexSafeFileName -Value $name
    $logPath = Join-Path $agentsDir "$safeName.exec.visible.log"
    $markerPath = Join-Path $agentsDir "$safeName.exec.marker.txt"
    $launcherPath = Join-Path $agentsDir "$safeName.exec.launch.ps1"

    if (-not $DryRun -and (Test-Path -LiteralPath $logPath)) {
        Remove-Item -LiteralPath $logPath -Force
    }

    if ([string]::IsNullOrWhiteSpace($resumeSession)) {
        $codexArgs = New-CodexWorkerExecArgs `
            -Repo $Repo `
            -Prompt $promptText `
            -WorkerModel $WorkerModel `
            -WorkerReasoningEffort $WorkerReasoningEffort
        $mode = "ExecFullPromptVisible"
        $redirectToLog = $true
    } else {
        $codexArgs = New-CodexWorkerResumeArgs `
            -Repo $Repo `
            -ResumeSession $resumeSession `
            -Prompt $promptText `
            -WorkerModel $WorkerModel `
            -WorkerReasoningEffort $WorkerReasoningEffort `
            -Loop:$Loop `
            -LoopMessage $LoopMessage `
            -LoopPeriod $LoopPeriod
        $mode = if ($Loop) { "ResumeLoop" } else { "Resume" }
        $redirectToLog = $false
    }

    $redirectLiteral = if ($redirectToLog) { '$true' } else { '$false' }
    $childCommand = @"
`$ErrorActionPreference = "Stop"
`$Host.UI.RawUI.WindowTitle = $(ConvertTo-CodexPowerShellSingleQuotedLiteral "Codex worker: $name")
Set-Location -LiteralPath $(ConvertTo-CodexPowerShellSingleQuotedLiteral $Repo)
`$codexArgs = $(ConvertTo-CodexPowerShellArrayLiteral $codexArgs)
`$redirectToLog = $redirectLiteral
if (`$redirectToLog) {
    & $(ConvertTo-CodexPowerShellSingleQuotedLiteral $CodexCommand) @codexArgs *>&1 | Tee-Object -FilePath $(ConvertTo-CodexPowerShellSingleQuotedLiteral $logPath)
} else {
    & $(ConvertTo-CodexPowerShellSingleQuotedLiteral $CodexCommand) @codexArgs
}
"@

    $childCommand | Set-Content -LiteralPath $launcherPath -Encoding UTF8

    $startArgs = @{
        FilePath = "powershell.exe"
        ArgumentList = @("-NoExit", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $launcherPath)
        PassThru = $true
    }

    if ($Hidden) {
        $startArgs.WindowStyle = "Hidden"
    } else {
        $startArgs.WindowStyle = "Normal"
    }

    if ($DryRun) {
        [pscustomobject]@{
            Worker = $name
            Pid = $null
            Visible = -not $Hidden
            Prompt = if ([string]::IsNullOrWhiteSpace($resumeSession)) { $promptPath } else { $promptText }
            Log = if ($redirectToLog) { $logPath } else { $null }
            Resume = $resumeSession
            Loop = [bool]$Loop
            LoopMessage = $LoopMessage
            Mode = $mode
            WorkerModel = $WorkerModel
            WorkerReasoningEffort = $WorkerReasoningEffort
            Launcher = $launcherPath
            Marker = $markerPath
            CodexArgs = $codexArgs
            DryRun = $true
        }
        continue
    }

    $process = Start-Process @startArgs
    @(
        "$(Get-Date -Format o) mode=$mode pid=$($process.Id) repo=$Repo prompt=$promptPath resume=$resumeSession launcher=$launcherPath codex=$CodexCommand model=$WorkerModel reasoning=$WorkerReasoningEffort",
        "visible=$(-not $Hidden)",
        "log=$(if ($redirectToLog) { $logPath } else { '' })",
        "loop=$([bool]$Loop)",
        "loop_message=$LoopMessage",
        $CommandPolicy,
        "commit_policy=prompt-specific; designated workers may commit focused verified slices"
    ) | Set-Content -LiteralPath $markerPath

    [pscustomobject]@{
        Worker = $name
        Pid = $process.Id
        Visible = -not $Hidden
        Prompt = if ([string]::IsNullOrWhiteSpace($resumeSession)) { $promptPath } else { $promptText }
        Log = if ($redirectToLog) { $logPath } else { $null }
        Resume = $resumeSession
        Loop = [bool]$Loop
        Mode = $mode
        WorkerModel = $WorkerModel
        WorkerReasoningEffort = $WorkerReasoningEffort
        Launcher = $launcherPath
        Marker = $markerPath
    }
}

if ($DryRun) {
    $launched
} else {
    $launched | Format-Table -AutoSize
}
