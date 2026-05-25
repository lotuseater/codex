param(
    [string]$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,

    [string]$CodexCommand = "codex",

    [string]$WorkerModel = "gpt-5.3-codex",

    [string]$WorkerReasoningEffort = "high",

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

function ConvertTo-PowerShellSingleQuotedLiteral {
    param([string]$Value)

    "'" + $Value.Replace("'", "''") + "'"
}

function ConvertTo-PowerShellArrayLiteral {
    param([string[]]$Values)

    "@(" + (($Values | ForEach-Object { ConvertTo-PowerShellSingleQuotedLiteral $_ }) -join ", ") + ")"
}

function Convert-ToSafeFileName {
    param([string]$Value)

    $invalid = [System.IO.Path]::GetInvalidFileNameChars()
    $chars = $Value.ToCharArray() | ForEach-Object {
        if ($invalid -contains $_) { "_" } else { $_ }
    }
    -join $chars
}

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

    if ([string]::IsNullOrWhiteSpace($resumeSession) -and -not (Test-Path -LiteralPath $promptPath)) {
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

    $safeName = Convert-ToSafeFileName -Value $name
    $logPath = Join-Path $agentsDir "$safeName.exec.visible.log"
    $markerPath = Join-Path $agentsDir "$safeName.exec.marker.txt"
    $launcherPath = Join-Path $agentsDir "$safeName.exec.launch.ps1"

    if (Test-Path -LiteralPath $logPath) {
        Remove-Item -LiteralPath $logPath -Force
    }

    if ([string]::IsNullOrWhiteSpace($resumeSession)) {
        $codexArgs = @(
            "-c",
            "model=$WorkerModel",
            "-c",
            "model_reasoning_effort=$WorkerReasoningEffort",
            "--cd",
            $Repo,
            "--ask-for-approval",
            "never",
            "--sandbox",
            "danger-full-access",
            "exec",
            $promptText
        )
        $mode = "ExecFullPromptVisible"
        $redirectToLog = $true
    } else {
        $codexArgs = @(
            "-c",
            "model=$WorkerModel",
            "-c",
            "model_reasoning_effort=$WorkerReasoningEffort",
            "--cd",
            $Repo,
            "--ask-for-approval",
            "never",
            "--sandbox",
            "danger-full-access"
        )
        $codexArgs += "resume"
        if ($Loop) {
            $codexArgs += "--loop"
            if (-not [string]::IsNullOrWhiteSpace($LoopMessage)) {
                $codexArgs += "--loop-message"
                $codexArgs += $LoopMessage
            }
            if ($LoopPeriod -gt 0) {
                $codexArgs += "--loop-period"
                $codexArgs += [string]$LoopPeriod
            }
        }
        $codexArgs += $resumeSession
        $codexArgs += $promptText
        $mode = if ($Loop) { "ResumeLoop" } else { "Resume" }
        $redirectToLog = $false
    }

    $redirectLiteral = if ($redirectToLog) { '$true' } else { '$false' }
    $childCommand = @"
`$ErrorActionPreference = "Stop"
`$Host.UI.RawUI.WindowTitle = $(ConvertTo-PowerShellSingleQuotedLiteral "Codex worker: $name")
Set-Location -LiteralPath $(ConvertTo-PowerShellSingleQuotedLiteral $Repo)
`$codexArgs = $(ConvertTo-PowerShellArrayLiteral $codexArgs)
`$redirectToLog = $redirectLiteral
if (`$redirectToLog) {
    & $(ConvertTo-PowerShellSingleQuotedLiteral $CodexCommand) @codexArgs *>&1 | Tee-Object -FilePath $(ConvertTo-PowerShellSingleQuotedLiteral $logPath)
} else {
    & $(ConvertTo-PowerShellSingleQuotedLiteral $CodexCommand) @codexArgs
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
    }
}

$launched | Format-Table -AutoSize
