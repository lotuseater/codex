Set-StrictMode -Version Latest

function Get-CodexPropertyValue {
    param(
        [object]$Object,
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    if ($null -eq $Object) {
        return $null
    }

    if ($Object -is [System.Collections.IDictionary]) {
        if ($Object.Contains($Name)) {
            return $Object[$Name]
        }
        return $null
    }

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }

    return $property.Value
}

function ConvertTo-CodexInt64OrNull {
    param([object]$Value)

    if ($null -eq $Value -or $Value -eq "") {
        return $null
    }

    try {
        return [int64]$Value
    }
    catch {
        return $null
    }
}

function ConvertTo-CodexDoubleOrNull {
    param([object]$Value)

    if ($null -eq $Value -or $Value -eq "") {
        return $null
    }

    try {
        return [double]$Value
    }
    catch {
        return $null
    }
}

function Get-CodexSharedFileLines {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $share = [System.IO.FileShare]::ReadWrite -bor [System.IO.FileShare]::Delete
    $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, $share)
    try {
        $reader = [System.IO.StreamReader]::new($stream)
        try {
            while (-not $reader.EndOfStream) {
                $reader.ReadLine()
            }
        }
        finally {
            $reader.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

function Test-CodexSharedFileContains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Text
    )

    foreach ($line in Get-CodexSharedFileLines -Path $Path) {
        if ($line.Contains($Text)) {
            return $true
        }
    }

    return $false
}

function ConvertTo-CodexComparablePath {
    param([string]$Path)

    if (-not $Path) {
        return $null
    }

    $trimmed = $Path.Trim()
    if ($trimmed -eq "") {
        return $null
    }

    try {
        $normalized = [System.IO.Path]::GetFullPath($trimmed)
    }
    catch {
        $normalized = $trimmed
    }

    $normalized.TrimEnd([char[]]@("\", "/")).ToLowerInvariant()
}

function Get-CodexTokenUsageFromObject {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Payload,

        [string]$SessionPath,

        [string]$Timestamp
    )

    $tokenContainer = Get-CodexPropertyValue -Object $Payload -Name "token_count"
    if ($null -eq $tokenContainer) {
        $tokenContainer = $Payload
    }

    $totalUsage = Get-CodexPropertyValue -Object $tokenContainer -Name "total_token_usage"
    $lastUsage = Get-CodexPropertyValue -Object $tokenContainer -Name "last_token_usage"

    if ($null -eq $totalUsage) {
        $totalUsage = Get-CodexPropertyValue -Object $Payload -Name "total_token_usage"
    }
    if ($null -eq $lastUsage) {
        $lastUsage = Get-CodexPropertyValue -Object $Payload -Name "last_token_usage"
    }

    $sourceUsage = $totalUsage
    if ($null -eq $sourceUsage) {
        $sourceUsage = $lastUsage
    }
    if ($null -eq $sourceUsage) {
        $sourceUsage = $tokenContainer
    }

    $inputTokens = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sourceUsage -Name "input_tokens")
    $cachedInputTokens = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sourceUsage -Name "cached_input_tokens")
    $outputTokens = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sourceUsage -Name "output_tokens")
    $reasoningOutputTokens = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sourceUsage -Name "reasoning_output_tokens")
    $totalTokens = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sourceUsage -Name "total_tokens")

    if ($null -eq $totalTokens) {
        $totalTokens = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sourceUsage -Name "tokens_used")
    }
    if ($null -eq $totalTokens) {
        $payloadTokensUsed = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $Payload -Name "tokens_used")
        if ($null -ne $payloadTokensUsed) {
            $totalTokens = $payloadTokensUsed
        }
    }
    if ($null -eq $totalTokens -and ($null -ne $inputTokens -or $null -ne $outputTokens)) {
        $totalTokens = 0
        if ($null -ne $inputTokens) {
            $totalTokens += $inputTokens
        }
        if ($null -ne $outputTokens) {
            $totalTokens += $outputTokens
        }
    }

    $contextWindow = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $tokenContainer -Name "model_context_window")
    if ($null -eq $contextWindow) {
        $contextWindow = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $Payload -Name "model_context_window")
    }
    if ($null -eq $contextWindow) {
        $contextWindow = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $Payload -Name "context_window")
    }

    if ($null -eq $totalTokens) {
        return $null
    }

    $usedPercent = $null
    if ($null -ne $contextWindow -and $contextWindow -gt 0) {
        $usedPercent = [math]::Round(($totalTokens * 100.0) / $contextWindow, 2)
    }

    [pscustomobject]@{
        SessionPath = $SessionPath
        Timestamp = $Timestamp
        InputTokens = $inputTokens
        CachedInputTokens = $cachedInputTokens
        OutputTokens = $outputTokens
        ReasoningOutputTokens = $reasoningOutputTokens
        TotalTokens = $totalTokens
        ContextWindow = $contextWindow
        UsedPercent = $usedPercent
    }
}

function Get-CodexSessionTokenUsage {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$SessionPath
    )

    $resolvedPath = (Resolve-Path -LiteralPath $SessionPath -ErrorAction Stop).Path
    $latest = $null

    foreach ($line in Get-CodexSharedFileLines -Path $resolvedPath) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }

        if ($line -notmatch "token_count|total_token_usage|last_token_usage|tokens_used|total_tokens|model_context_window") {
            continue
        }

        try {
            $item = $line | ConvertFrom-Json -ErrorAction Stop
        }
        catch {
            continue
        }

        $payload = Get-CodexPropertyValue -Object $item -Name "payload"
        if ($null -eq $payload) {
            $payload = $item
        }

        $record = Get-CodexTokenUsageFromObject -Payload $payload -SessionPath $resolvedPath -Timestamp (Get-CodexPropertyValue -Object $item -Name "timestamp")
        if ($null -ne $record) {
            $latest = $record
        }
    }

    return $latest
}

function Get-CodexSessionIdFromPath {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$SessionPath
    )

    $resolvedPath = (Resolve-Path -LiteralPath $SessionPath -ErrorAction Stop).Path
    foreach ($line in Get-CodexSharedFileLines -Path $resolvedPath) {
        if ($line -notmatch '"session_meta"') {
            continue
        }

        try {
            $item = $line | ConvertFrom-Json -ErrorAction Stop
        }
        catch {
            continue
        }

        if ((Get-CodexPropertyValue -Object $item -Name "type") -ne "session_meta") {
            continue
        }

        $payload = Get-CodexPropertyValue -Object $item -Name "payload"
        $id = Get-CodexPropertyValue -Object $payload -Name "id"
        if ($null -ne $id -and $id -ne "") {
            return [string]$id
        }
    }

    return $null
}

function Get-CodexSessionCwdFromPath {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$SessionPath
    )

    $resolvedPath = (Resolve-Path -LiteralPath $SessionPath -ErrorAction Stop).Path
    foreach ($line in Get-CodexSharedFileLines -Path $resolvedPath) {
        if ($line -notmatch '"session_meta"') {
            continue
        }

        try {
            $item = $line | ConvertFrom-Json -ErrorAction Stop
        }
        catch {
            continue
        }

        if ((Get-CodexPropertyValue -Object $item -Name "type") -ne "session_meta") {
            continue
        }

        $payload = Get-CodexPropertyValue -Object $item -Name "payload"
        $cwd = Get-CodexPropertyValue -Object $payload -Name "cwd"
        if ($null -ne $cwd -and $cwd -ne "") {
            return [string]$cwd
        }
    }

    return $null
}

function Resolve-CodexSessionPath {
    [CmdletBinding()]
    param(
        [string]$SessionPath,
        [string]$SessionId,
        [string]$SearchText,
        [string]$Project,
        [string]$SessionRoot = (Join-Path $HOME ".codex\sessions"),
        [int]$MaxCandidates = 300
    )

    if ($SessionPath) {
        return (Resolve-Path -LiteralPath $SessionPath -ErrorAction Stop).Path
    }

    if (-not (Test-Path -LiteralPath $SessionRoot)) {
        throw "Session root not found: $SessionRoot"
    }

    $candidates = Get-ChildItem -LiteralPath $SessionRoot -Recurse -Filter *.jsonl -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First $MaxCandidates

    foreach ($candidate in $candidates) {
        if ($SessionId) {
            $needle = '"id":"' + $SessionId + '"'
            if (Test-CodexSharedFileContains -Path $candidate.FullName -Text $needle) {
                return $candidate.FullName
            }
        }

        if ($SearchText) {
            if (Test-CodexSharedFileContains -Path $candidate.FullName -Text $SearchText) {
                return $candidate.FullName
            }
        }

        if ($Project) {
            if (Test-CodexSharedFileContains -Path $candidate.FullName -Text $Project) {
                return $candidate.FullName
            }
        }
    }

    if ($SessionId) {
        throw "Could not resolve Codex session id: $SessionId"
    }
    if ($SearchText) {
        throw "Could not resolve Codex session containing text: $SearchText"
    }
    if ($Project) {
        throw "Could not resolve Codex session for project: $Project"
    }

    throw "Provide SessionPath, SessionId, SearchText, or Project."
}

function Resolve-CodexWizardManagedPipe {
    [CmdletBinding()]
    param(
        [string]$SessionPath,
        [string]$SessionId,
        [string]$SearchText,
        [string]$Project,
        [string]$SessionRoot = (Join-Path $HOME ".codex\sessions"),
        [string]$ManagedTerminalRoot = (Join-Path $HOME ".codex\wizard_sidecars\managed_terminals"),
        [int]$MaxCandidates = 100,
        [bool]$RequireLiveProcess = $true
    )

    $targetProject = $Project
    if (-not $targetProject -and ($SessionPath -or $SessionId -or $SearchText)) {
        $resolvedSessionPath = Resolve-CodexSessionPath `
            -SessionPath $SessionPath `
            -SessionId $SessionId `
            -SearchText $SearchText `
            -SessionRoot $SessionRoot
        $targetProject = Get-CodexSessionCwdFromPath -SessionPath $resolvedSessionPath
    }

    $targetProjectKey = ConvertTo-CodexComparablePath -Path $targetProject
    if (-not $targetProjectKey) {
        return $null
    }
    if (-not (Test-Path -LiteralPath $ManagedTerminalRoot)) {
        return $null
    }

    $pipeMatches = @()
    $sidecars = Get-ChildItem -LiteralPath $ManagedTerminalRoot -Filter *.json -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First $MaxCandidates
    foreach ($sidecarPath in $sidecars) {
        try {
            $sidecar = Get-Content -Raw -LiteralPath $sidecarPath.FullName | ConvertFrom-Json -ErrorAction Stop
        }
        catch {
            continue
        }

        $provider = Get-CodexPropertyValue -Object $sidecar -Name "provider"
        $commandName = Get-CodexPropertyValue -Object $sidecar -Name "command_name"
        $commandBaseName = [System.IO.Path]::GetFileNameWithoutExtension([string]$commandName)
        if ($provider -ne "codex" -and $commandBaseName -ne "codex") {
            continue
        }

        $pipeName = Get-CodexPropertyValue -Object $sidecar -Name "loop_target_pwsh_pipe"
        if (-not $pipeName) {
            continue
        }

        $sidecarProject = Get-CodexPropertyValue -Object $sidecar -Name "cwd"
        if ((ConvertTo-CodexComparablePath -Path $sidecarProject) -ne $targetProjectKey) {
            continue
        }

        $processId = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sidecar -Name "loop_target_pwsh_pid")
        if ($null -eq $processId) {
            $processId = ConvertTo-CodexInt64OrNull (Get-CodexPropertyValue -Object $sidecar -Name "process_pid")
        }
        if ($RequireLiveProcess) {
            if ($null -eq $processId) {
                continue
            }
            if (-not (Get-Process -Id $processId -ErrorAction SilentlyContinue)) {
                continue
            }
        }

        $pipeMatches += [pscustomobject]@{
            PipeName = [string]$pipeName
            SourcePath = $sidecarPath.FullName
            Project = [string]$sidecarProject
            ProcessId = $processId
            SessionId = [string](Get-CodexPropertyValue -Object $sidecar -Name "session_id")
            UpdatedAt = Get-CodexPropertyValue -Object $sidecar -Name "updated_at"
        }
    }

    if ($pipeMatches.Count -eq 0) {
        return $null
    }
    if ($pipeMatches.Count -gt 1) {
        $sources = ($pipeMatches | ForEach-Object { $_.SourcePath }) -join ", "
        throw "Multiple Wizard-managed Codex pipes match project '$targetProject': $sources"
    }

    $pipeMatches[0]
}

function Get-CodexMaintenanceProfile {
    [CmdletBinding()]
    param(
        [ValidateSet("Self", "Director")]
        [string]$Profile = "Self"
    )

    if ($Profile -eq "Director") {
        return [pscustomobject]@{
            Profile = "Director"
            ThresholdPercent = 30.0
            InspectIntervalMinutes = 10
            Reminder = "Automatic Director post-compaction/resume reminder: go on`n`nYou are the SOLID refactor director. Continue in --loop from the latest verified state. Reread .codex/workflow/solid-refactor-handoff.md and .codex/workflow/solid-refactor-director-prompt.md if stale. Do not implement source changes; coordinate workers, keep handoffs compact, and report blockers clearly."
        }
    }

    [pscustomobject]@{
        Profile = "Self"
        ThresholdPercent = 25.0
        InspectIntervalMinutes = 15
        Reminder = "Automatic post-compaction loop continuation: go on`n`nLoop mode is on. Verify compaction really reduced token use, then continue the current implementation and verification path from the latest repo state. Keep handoff concise before future compaction."
    }
}

function Test-CodexTokenThreshold {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [object]$Usage,

        [Parameter(Mandatory = $true)]
        [double]$ThresholdPercent
    )

    $usedPercent = ConvertTo-CodexDoubleOrNull (Get-CodexPropertyValue -Object $Usage -Name "UsedPercent")
    if ($null -eq $usedPercent) {
        return $false
    }

    return $usedPercent -ge $ThresholdPercent
}

function Test-CodexCompactionReduction {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [object]$Before,

        [Parameter(Mandatory = $true)]
        [object]$After,

        [double]$MinReductionPercent = 30.0,

        [double]$MinReductionPoints = 5.0
    )

    $beforePercent = ConvertTo-CodexDoubleOrNull (Get-CodexPropertyValue -Object $Before -Name "UsedPercent")
    $afterPercent = ConvertTo-CodexDoubleOrNull (Get-CodexPropertyValue -Object $After -Name "UsedPercent")
    if ($null -eq $beforePercent -or $null -eq $afterPercent -or $beforePercent -le 0) {
        return [pscustomobject]@{
            Succeeded = $false
            BeforePercent = $beforePercent
            AfterPercent = $afterPercent
            ReductionPoints = $null
            ReductionPercent = $null
            Reason = "missing_percent"
        }
    }

    $reductionPoints = [math]::Round($beforePercent - $afterPercent, 2)
    $reductionPercent = [math]::Round(($reductionPoints * 100.0) / $beforePercent, 2)
    $succeeded = $reductionPoints -ge $MinReductionPoints -and $reductionPercent -ge $MinReductionPercent

    [pscustomobject]@{
        Succeeded = $succeeded
        BeforePercent = $beforePercent
        AfterPercent = $afterPercent
        ReductionPoints = $reductionPoints
        ReductionPercent = $reductionPercent
        Reason = if ($succeeded) { "ok" } else { "insufficient_reduction" }
    }
}

function New-CodexMaintenancePlan {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [object]$Usage,

        [Parameter(Mandatory = $true)]
        [double]$ThresholdPercent,

        [ValidateSet("Self", "Director")]
        [string]$Profile = "Self",

        [datetime]$Now = (Get-Date),

        [Nullable[datetime]]$LastInspectAt,

        [int]$InspectIntervalMinutes = 15
    )

    $usedPercent = ConvertTo-CodexDoubleOrNull (Get-CodexPropertyValue -Object $Usage -Name "UsedPercent")
    $actions = New-Object System.Collections.Generic.List[string]

    if ($null -eq $usedPercent) {
        $actions.Add("token_percent_unavailable")
    }
    elseif ($usedPercent -ge $ThresholdPercent) {
        $actions.Add("interrupt_active_action")
        $actions.Add("submit_compact")
        $actions.Add("verify_compaction_reduction")
        $actions.Add("send_profile_reminder")
    }
    else {
        $actions.Add("observe")
    }

    $inspectDue = $false
    if ($Profile -eq "Director") {
        if ($null -eq $LastInspectAt) {
            $inspectDue = $true
        }
        else {
            $inspectDue = $Now -ge $LastInspectAt.Value.AddMinutes($InspectIntervalMinutes)
        }
        if ($inspectDue) {
            $actions.Add("inspect_director_recent_talk")
        }
    }

    [pscustomobject]@{
        Profile = $Profile
        UsedPercent = $usedPercent
        ThresholdPercent = $ThresholdPercent
        ThresholdReached = ($null -ne $usedPercent -and $usedPercent -ge $ThresholdPercent)
        InspectDue = $inspectDue
        Actions = @($actions)
    }
}

function Join-CodexCommandLine {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Argument
    )

    ($Argument | ForEach-Object {
        if ($_ -match "^[A-Za-z0-9_./:=-]+$") {
            $_
        }
        else {
            "'" + ($_.Replace("'", "''")) + "'"
        }
    }) -join " "
}

function New-CodexLoopResumeCommand {
    [CmdletBinding()]
    param(
        [string]$CodexCommand = "codex",
        [string]$SessionId,
        [string]$Prompt
    )

    $arguments = New-Object System.Collections.Generic.List[string]
    $arguments.Add($CodexCommand)
    $arguments.Add("--loop")

    if ($SessionId) {
        $arguments.Add("resume")
        $arguments.Add($SessionId)
    }

    if ($Prompt) {
        $arguments.Add($Prompt)
    }

    [pscustomobject]@{
        Arguments = [string[]]$arguments
        CommandLine = Join-CodexCommandLine -Argument ([string[]]$arguments)
    }
}

function New-CodexPwshPipePayload {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("interrupt", "write", "keys", "send_keys")]
        [string]$Command,

        [string]$Text,
        [string]$Keys,
        [bool]$Submit = $true
    )

    $wireCommand = $Command
    if ($wireCommand -eq "send_keys") {
        $wireCommand = "keys"
    }

    $payload = [ordered]@{
        command = $wireCommand
    }

    if ($wireCommand -eq "write") {
        $payload.text = [string]$Text
        $payload.submit = [bool]$Submit
    }
    elseif ($wireCommand -eq "keys") {
        $payload.keys = [string]$Keys
    }

    [pscustomobject]$payload
}

function Invoke-CodexPwshPipeRequest {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$PipeName,

        [Parameter(Mandatory = $true)]
        [object]$Payload,

        [int]$TimeoutMs = 5000
    )

    $stream = [System.IO.Pipes.NamedPipeClientStream]::new(
        ".",
        $PipeName,
        [System.IO.Pipes.PipeDirection]::InOut,
        [System.IO.Pipes.PipeOptions]::None
    )

    try {
        $stream.Connect($TimeoutMs)
        $encoding = [System.Text.UTF8Encoding]::new($false)
        $writer = [System.IO.StreamWriter]::new($stream, $encoding)
        $reader = [System.IO.StreamReader]::new($stream, $encoding)
        try {
            $writer.AutoFlush = $true
            $writer.WriteLine(($Payload | ConvertTo-Json -Depth 8 -Compress))
            $line = $reader.ReadLine()
            if ($null -eq $line -or $line -eq "") {
                return $null
            }
            $response = $line | ConvertFrom-Json -ErrorAction Stop
            $status = Get-CodexPropertyValue -Object $response -Name "status"
            $ok = Get-CodexPropertyValue -Object $response -Name "ok"
            $command = Get-CodexPropertyValue -Object $Payload -Name "command"

            if ($null -ne $status) {
                $statusText = ([string]$status).ToLowerInvariant()
                if ($statusText -notin @("ok", "success")) {
                    throw "Pipe request '$command' failed: $line"
                }
            }
            if ($ok -is [bool] -and -not $ok) {
                throw "Pipe request '$command' failed: $line"
            }

            return $response
        }
        finally {
            try {
                $writer.Dispose()
            }
            catch {
            }
            try {
                $reader.Dispose()
            }
            catch {
            }
        }
    }
    finally {
        try {
            $stream.Dispose()
        }
        catch {
        }
    }
}

function Invoke-CodexMaintenanceOnce {
    [CmdletBinding()]
    param(
        [ValidateSet("Self", "Director")]
        [string]$Profile = "Self",

        [string]$SessionPath,
        [string]$SessionId,
        [string]$SearchText,
        [string]$Project,
        [string]$SessionRoot = (Join-Path $HOME ".codex\sessions"),

        [double]$ThresholdPercent = -1,
        [string]$Reminder,

        [string]$Title,
        [int]$RootPid = 0,
        [long]$WindowHandle = 0,
        [string]$PipeName,
        [int]$PipeTimeoutMs = 5000,
        [string]$TerminalControlScript = (Join-Path (Split-Path -Parent $PSScriptRoot) "agents\terminal-paste-enter.ps1"),

        [switch]$DryRun,
        [int]$WaitAfterCompactSeconds = 60,
        [double]$MinReductionPercent = 30.0,
        [double]$MinReductionPoints = 5.0
    )

    $profileInfo = Get-CodexMaintenanceProfile -Profile $Profile
    if ($ThresholdPercent -lt 0) {
        $ThresholdPercent = $profileInfo.ThresholdPercent
    }
    if (-not $Reminder) {
        $Reminder = $profileInfo.Reminder
    }

    $resolvedSessionPath = Resolve-CodexSessionPath -SessionPath $SessionPath -SessionId $SessionId -SearchText $SearchText -Project $Project -SessionRoot $SessionRoot
    $before = Get-CodexSessionTokenUsage -SessionPath $resolvedSessionPath
    if ($null -eq $before) {
        throw "Could not find token usage in session: $resolvedSessionPath"
    }

    $plan = New-CodexMaintenancePlan -Usage $before -ThresholdPercent $ThresholdPercent -Profile $Profile -InspectIntervalMinutes $profileInfo.InspectIntervalMinutes
    if (-not $plan.ThresholdReached) {
        return [pscustomobject]@{
            Status = "below_threshold"
            SessionPath = $resolvedSessionPath
            Before = $before
            Plan = $plan
            DryRun = [bool]$DryRun
        }
    }

    if ($DryRun) {
        return [pscustomobject]@{
            Status = "dry_run_threshold_reached"
            SessionPath = $resolvedSessionPath
            Before = $before
            Plan = $plan
            Reminder = $Reminder
            DryRun = $true
        }
    }

    if ($PipeName) {
        Invoke-CodexPwshPipeRequest -PipeName $PipeName -TimeoutMs $PipeTimeoutMs -Payload (New-CodexPwshPipePayload -Command "interrupt") | Out-Null
        Invoke-CodexPwshPipeRequest -PipeName $PipeName -TimeoutMs $PipeTimeoutMs -Payload (New-CodexPwshPipePayload -Command "write" -Text "/compact" -Submit $true) | Out-Null
    }
    else {
        if (-not (Test-Path -LiteralPath $TerminalControlScript)) {
            throw "Terminal control script not found: $TerminalControlScript"
        }
        if (-not $Title -and $RootPid -le 0 -and $WindowHandle -le 0) {
            throw "Live compaction requires PipeName, Title, RootPid, or WindowHandle so the script does not target the wrong terminal."
        }

        . $TerminalControlScript

        Invoke-SolidTerminalSendKeys -Keys "{ESCAPE}" -Title $Title -RootPid $RootPid -WindowHandle $WindowHandle -Repeat 1 | Out-Null
        Start-Sleep -Milliseconds 800
        Invoke-SolidTerminalPasteEnter -Message "/compact" -Title $Title -RootPid $RootPid -WindowHandle $WindowHandle | Out-Null
    }
    Start-Sleep -Seconds $WaitAfterCompactSeconds

    $after = Get-CodexSessionTokenUsage -SessionPath $resolvedSessionPath
    if ($null -eq $after) {
        throw "Could not read token usage after compaction: $resolvedSessionPath"
    }

    $reduction = Test-CodexCompactionReduction -Before $before -After $after -MinReductionPercent $MinReductionPercent -MinReductionPoints $MinReductionPoints
    if (-not $reduction.Succeeded) {
        return [pscustomobject]@{
            Status = "compaction_verification_failed"
            SessionPath = $resolvedSessionPath
            Before = $before
            After = $after
            Plan = $plan
            Reduction = $reduction
            DryRun = $false
        }
    }

    if ($PipeName) {
        Invoke-CodexPwshPipeRequest -PipeName $PipeName -TimeoutMs $PipeTimeoutMs -Payload (New-CodexPwshPipePayload -Command "write" -Text $Reminder -Submit $true) | Out-Null
    }
    else {
        Invoke-SolidTerminalPasteEnter -Message $Reminder -Title $Title -RootPid $RootPid -WindowHandle $WindowHandle | Out-Null
    }

    [pscustomobject]@{
        Status = "compaction_verified_reminder_sent"
        SessionPath = $resolvedSessionPath
        Before = $before
        After = $after
        Plan = $plan
        Reduction = $reduction
        DryRun = $false
    }
}

Export-ModuleMember -Function @(
    "Get-CodexTokenUsageFromObject",
    "Get-CodexSessionTokenUsage",
    "Get-CodexSessionIdFromPath",
    "Resolve-CodexSessionPath",
    "Resolve-CodexWizardManagedPipe",
    "Get-CodexMaintenanceProfile",
    "Test-CodexTokenThreshold",
    "Test-CodexCompactionReduction",
    "New-CodexMaintenancePlan",
    "Join-CodexCommandLine",
    "New-CodexLoopResumeCommand",
    "New-CodexPwshPipePayload",
    "Invoke-CodexPwshPipeRequest",
    "Invoke-CodexMaintenanceOnce"
)
