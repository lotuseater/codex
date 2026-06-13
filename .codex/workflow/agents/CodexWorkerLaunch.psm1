Set-StrictMode -Version Latest

function ConvertTo-CodexPowerShellSingleQuotedLiteral {
    param([AllowNull()][string]$Value)

    if ($null -eq $Value) {
        return "''"
    }

    "'" + $Value.Replace("'", "''") + "'"
}

function ConvertTo-CodexPowerShellArrayLiteral {
    param([AllowEmptyCollection()][string[]]$Values)

    "@(" + (($Values | ForEach-Object { ConvertTo-CodexPowerShellSingleQuotedLiteral $_ }) -join ", ") + ")"
}

function ConvertTo-CodexSafeFileName {
    param([string]$Value)

    $invalid = [System.IO.Path]::GetInvalidFileNameChars()
    $safeChars = $Value.ToCharArray() | ForEach-Object {
        if ($invalid -contains $_) { "_" } else { $_ }
    }
    [string]::Concat($safeChars)
}

function New-CodexWorkerRootArgs {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Repo,

        [string]$WorkerModel = "gpt-5.5",

        [string]$WorkerReasoningEffort = "xhigh",

        [string]$ApprovalPolicy = "never",

        [string]$SandboxMode = "danger-full-access"
    )

    @(
        "-c",
        "model=$WorkerModel",
        "-c",
        "model_reasoning_effort=$WorkerReasoningEffort",
        "--cd",
        $Repo,
        "--ask-for-approval",
        $ApprovalPolicy,
        "--sandbox",
        $SandboxMode
    )
}

function New-CodexWorkerExecArgs {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Repo,

        [Parameter(Mandatory = $true)]
        [string]$Prompt,

        [string]$WorkerModel = "gpt-5.5",

        [string]$WorkerReasoningEffort = "xhigh",

        [string]$ApprovalPolicy = "never",

        [string]$SandboxMode = "danger-full-access"
    )

    $args = New-CodexWorkerRootArgs `
        -Repo $Repo `
        -WorkerModel $WorkerModel `
        -WorkerReasoningEffort $WorkerReasoningEffort `
        -ApprovalPolicy $ApprovalPolicy `
        -SandboxMode $SandboxMode
    $args + @("exec", $Prompt)
}

function New-CodexWorkerInteractiveArgs {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Repo,

        [Parameter(Mandatory = $true)]
        [string]$Prompt,

        [string]$WorkerModel = "gpt-5.5",

        [string]$WorkerReasoningEffort = "xhigh",

        [string]$ApprovalPolicy = "never",

        [string]$SandboxMode = "danger-full-access"
    )

    $args = New-CodexWorkerRootArgs `
        -Repo $Repo `
        -WorkerModel $WorkerModel `
        -WorkerReasoningEffort $WorkerReasoningEffort `
        -ApprovalPolicy $ApprovalPolicy `
        -SandboxMode $SandboxMode
    $args + @($Prompt)
}

function New-CodexWorkerResumeArgs {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Repo,

        [Parameter(Mandatory = $true)]
        [string]$ResumeSession,

        [Parameter(Mandatory = $true)]
        [string]$Prompt,

        [string]$WorkerModel = "gpt-5.5",

        [string]$WorkerReasoningEffort = "xhigh",

        [switch]$Loop,

        [string]$LoopMessage,

        [int]$LoopPeriod,

        [string]$ApprovalPolicy = "never",

        [string]$SandboxMode = "danger-full-access"
    )

    $args = New-CodexWorkerRootArgs `
        -Repo $Repo `
        -WorkerModel $WorkerModel `
        -WorkerReasoningEffort $WorkerReasoningEffort `
        -ApprovalPolicy $ApprovalPolicy `
        -SandboxMode $SandboxMode

    $args += "resume"
    if ($Loop) {
        $args += "--loop"
        if (-not [string]::IsNullOrWhiteSpace($LoopMessage)) {
            $args += "--loop-message"
            $args += $LoopMessage
        }
        if ($LoopPeriod -gt 0) {
            $args += "--loop-period"
            $args += [string]$LoopPeriod
        }
    }
    $args += $ResumeSession
    $args += $Prompt
    $args
}

function Test-CodexWorkerCustomBuildPath {
    param([AllowNull()][string]$PathValue)

    if ([string]::IsNullOrWhiteSpace($PathValue)) {
        return $false
    }

    $normalized = $PathValue.Replace("/", "\")
    $normalized.Contains("\.codex\local-builds\") -or $normalized.Contains("codex-custom-")
}

function Get-CodexWorkerCommandHealth {
    param(
        [string]$CodexCommand = "codex",

        [string]$WrapperEnvPath = (Join-Path $HOME ".codex\system-wrapper\system.codex-wrapper.env.json")
    )

    $commandInfo = Get-Command $CodexCommand -ErrorAction Stop
    $source = $commandInfo.Source
    if ([string]::IsNullOrWhiteSpace($source)) {
        $source = $commandInfo.Path
    }
    if ([string]::IsNullOrWhiteSpace($source)) {
        $source = $commandInfo.Definition
    }

    $wrapperRealExe = $null
    $wrapperReadError = $null
    if (-not [string]::IsNullOrWhiteSpace($WrapperEnvPath) -and (Test-Path -LiteralPath $WrapperEnvPath)) {
        try {
            $wrapperEnv = Get-Content -Raw -LiteralPath $WrapperEnvPath | ConvertFrom-Json
            $property = $wrapperEnv.PSObject.Properties["WIZARD_CODEX_REAL_EXE"]
            if ($null -ne $property) {
                $wrapperRealExe = [string]$property.Value
            }
        } catch {
            $wrapperReadError = $_.Exception.Message
        }
    }

    $usesCustomBuild =
        (Test-CodexWorkerCustomBuildPath -PathValue $source) -or
        (Test-CodexWorkerCustomBuildPath -PathValue $wrapperRealExe)

    [pscustomobject]@{
        CodexCommand = $CodexCommand
        Source = $source
        WrapperEnvPath = $WrapperEnvPath
        WrapperRealExe = $wrapperRealExe
        WrapperReadError = $wrapperReadError
        UsesCustomBuild = [bool]$usesCustomBuild
    }
}

function Assert-CodexWorkerCommandHealth {
    param(
        [Parameter(Mandatory = $true)]
        [psobject]$Health,

        [switch]$AllowCustomBuild
    )

    if ($Health.UsesCustomBuild -and -not $AllowCustomBuild) {
        throw "Codex command resolves through a custom build path. Use in-app delegation or rerun with -AllowCustomBuild only after terminal spawn tools are verified. Source=$($Health.Source) WrapperRealExe=$($Health.WrapperRealExe)"
    }
}

function Assert-CodexWorkerArgs {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Args,

        [Parameter(Mandatory = $true)]
        [ValidateSet("Exec", "Interactive", "Resume")]
        [string]$Mode,

        [Parameter(Mandatory = $true)]
        [string]$Repo,

        [Parameter(Mandatory = $true)]
        [string]$Prompt
    )

    $cdIndex = [Array]::IndexOf($Args, "--cd")
    $approvalIndex = [Array]::IndexOf($Args, "--ask-for-approval")
    $sandboxIndex = [Array]::IndexOf($Args, "--sandbox")

    if ($cdIndex -lt 0 -or $Args[$cdIndex + 1] -ne $Repo) {
        throw "Worker args must include --cd followed by the target repo."
    }
    if ($approvalIndex -lt 0 -or $Args[$approvalIndex + 1] -ne "never") {
        throw "Worker args must include --ask-for-approval never."
    }
    if ($sandboxIndex -lt 0 -or $Args[$sandboxIndex + 1] -ne "danger-full-access") {
        throw "Worker args must include --sandbox danger-full-access."
    }
    if ($Args[$Args.Count - 1] -ne $Prompt) {
        throw "Worker prompt must be the final argument so PowerShell quoting cannot swallow later flags."
    }

    $modeToken = if ($Mode -eq "Exec") { "exec" } elseif ($Mode -eq "Resume") { "resume" } else { $null }
    if ($null -ne $modeToken) {
        $modeIndex = [Array]::IndexOf($Args, $modeToken)
        if ($modeIndex -lt 0) {
            throw "Worker args must include the $modeToken subcommand."
        }
        foreach ($flagIndex in @($cdIndex, $approvalIndex, $sandboxIndex)) {
            if ($flagIndex -gt $modeIndex) {
                throw "Worker runtime flags must be root CLI flags before the $modeToken subcommand."
            }
        }
    } elseif ([Array]::IndexOf($Args, "exec") -ge 0) {
        throw "Interactive worker args must not include the exec subcommand."
    }
}

Export-ModuleMember `
    -Function ConvertTo-CodexPowerShellSingleQuotedLiteral, `
        ConvertTo-CodexPowerShellArrayLiteral, `
        ConvertTo-CodexSafeFileName, `
        New-CodexWorkerExecArgs, `
        New-CodexWorkerInteractiveArgs, `
        New-CodexWorkerResumeArgs, `
        Get-CodexWorkerCommandHealth, `
        Assert-CodexWorkerCommandHealth, `
        Assert-CodexWorkerArgs
