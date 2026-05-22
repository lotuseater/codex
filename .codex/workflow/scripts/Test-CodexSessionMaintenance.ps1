[CmdletBinding()]
param(
    [string]$WorkDir = (Join-Path (Split-Path -Parent $PSScriptRoot) "tmp\session-maintenance-tests")
)

$ErrorActionPreference = "Stop"

Import-Module (Join-Path $PSScriptRoot "CodexSessionMaintenance.psm1") -Force

$script:Assertions = 0

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    $script:Assertions += 1
    if (-not $Condition) {
        throw "ASSERT_TRUE failed: $Message"
    }
}

function Assert-Equal {
    param(
        [object]$Actual,
        [object]$Expected,
        [string]$Message
    )

    $script:Assertions += 1
    if ($Actual -ne $Expected) {
        throw "ASSERT_EQUAL failed: $Message. Expected '$Expected', got '$Actual'"
    }
}

if (Test-Path -LiteralPath $WorkDir) {
    Remove-Item -LiteralPath $WorkDir -Recurse -Force
}
New-Item -ItemType Directory -Path $WorkDir | Out-Null

$sessionPath = Join-Path $WorkDir "rollout-test.jsonl"
$sessionId = "019e-test-session"
$lines = @(
    (@{
        timestamp = "2026-05-22T00:00:00.000Z"
        type = "session_meta"
        payload = @{
            id = $sessionId
            cwd = "C:\repo"
        }
    } | ConvertTo-Json -Depth 8 -Compress),
    (@{
        timestamp = "2026-05-22T00:01:00.000Z"
        type = "event_msg"
        payload = @{
            token_count = @{
                total_token_usage = @{
                    input_tokens = 250
                    cached_input_tokens = 10
                    output_tokens = 50
                    reasoning_output_tokens = 0
                    total_tokens = 300
                }
                last_token_usage = @{
                    input_tokens = 25
                    cached_input_tokens = 0
                    output_tokens = 5
                    reasoning_output_tokens = 0
                    total_tokens = 30
                }
                model_context_window = 1000
            }
        }
    } | ConvertTo-Json -Depth 8 -Compress)
)
$lines | Set-Content -LiteralPath $sessionPath -Encoding UTF8

$usage = Get-CodexSessionTokenUsage -SessionPath $sessionPath
Assert-Equal $usage.TotalTokens 300 "token_count total usage should be parsed"
Assert-Equal $usage.ContextWindow 1000 "context window should be parsed"
Assert-Equal $usage.UsedPercent 30 "used percent should be computed"
Assert-True (Test-CodexTokenThreshold -Usage $usage -ThresholdPercent 25) "self threshold should trip at 30%"
Assert-True (Test-CodexTokenThreshold -Usage $usage -ThresholdPercent 30) "director threshold should trip at 30%"

$resolved = Resolve-CodexSessionPath -SessionId $sessionId -SessionRoot $WorkDir
Assert-Equal $resolved $sessionPath "session id should resolve to fixture path"
Assert-Equal (Get-CodexSessionIdFromPath -SessionPath $sessionPath) $sessionId "session id should be read from session_meta"

$plan = New-CodexMaintenancePlan -Usage $usage -ThresholdPercent 30 -Profile Director -Now ([datetime]"2026-05-22T00:02:00Z") -InspectIntervalMinutes 10
Assert-True $plan.ThresholdReached "director plan should reach threshold"
Assert-True ($plan.Actions -contains "interrupt_active_action") "plan should interrupt before compact"
Assert-True ($plan.Actions -contains "submit_compact") "plan should compact"
Assert-True ($plan.Actions -contains "verify_compaction_reduction") "plan should verify compaction"
Assert-True ($plan.Actions -contains "send_profile_reminder") "plan should send reminder"
Assert-True ($plan.Actions -contains "inspect_director_recent_talk") "director plan should request periodic inspection"

$belowPayload = @{
    total_token_usage = @{
        input_tokens = 100
        output_tokens = 20
        total_tokens = 120
    }
    model_context_window = 1000
}
$below = Get-CodexTokenUsageFromObject -Payload ([pscustomobject]$belowPayload) -SessionPath "inline" -Timestamp "now"
Assert-Equal $below.UsedPercent 12 "direct token payload should parse"
$belowPlan = New-CodexMaintenancePlan -Usage $below -ThresholdPercent 25 -Profile Self
Assert-True (-not $belowPlan.ThresholdReached) "below-threshold plan should not compact"
Assert-True ($belowPlan.Actions -contains "observe") "below-threshold plan should observe"

$afterGood = [pscustomobject]@{ UsedPercent = 18.0 }
$afterBad = [pscustomobject]@{ UsedPercent = 27.0 }
$goodReduction = Test-CodexCompactionReduction -Before $usage -After $afterGood -MinReductionPercent 30 -MinReductionPoints 5
$badReduction = Test-CodexCompactionReduction -Before $usage -After $afterBad -MinReductionPercent 30 -MinReductionPoints 5
Assert-True $goodReduction.Succeeded "large compaction reduction should pass"
Assert-True (-not $badReduction.Succeeded) "small compaction reduction should fail"

$command = New-CodexLoopResumeCommand -CodexCommand "codex" -SessionId $sessionId -Prompt "go on"
Assert-Equal $command.Arguments[0] "codex" "command starts with codex"
Assert-Equal $command.Arguments[1] "--loop" "--loop must be immediately after codex command"
Assert-Equal $command.Arguments[2] "resume" "resume subcommand should follow --loop"
Assert-Equal $command.Arguments[3] $sessionId "session id should follow resume"
Assert-Equal $command.Arguments[4] "go on" "prompt should be included"

$dryRun = Invoke-CodexMaintenanceOnce -Profile Director -SessionPath $sessionPath -DryRun
Assert-Equal $dryRun.Status "dry_run_threshold_reached" "dry-run should report threshold without controlling terminal"
Assert-True ($dryRun.Plan.Actions -contains "submit_compact") "dry-run should expose compact action"
Assert-True ($dryRun.Reminder -match "SOLID refactor director") "director reminder should be used"

$writePayload = New-CodexPwshPipePayload -Command "write" -Text "/compact" -Submit $true
Assert-Equal $writePayload.command "write" "write payload should carry command"
Assert-Equal $writePayload.text "/compact" "write payload should carry text"
Assert-Equal $writePayload.submit $true "write payload should request submit"
$interruptPayload = New-CodexPwshPipePayload -Command "interrupt"
Assert-Equal $interruptPayload.command "interrupt" "interrupt payload should carry command"
$keysPayload = New-CodexPwshPipePayload -Command "send_keys" -Keys "{ESCAPE}"
Assert-Equal $keysPayload.command "keys" "send_keys alias should use the Wizard pipe keys verb"
Assert-Equal $keysPayload.keys "{ESCAPE}" "keys payload should carry keys"

$errorPipeName = "codex-maintenance-error-test-$([guid]::NewGuid().ToString("N"))"
$errorPipeJob = Start-Job -ArgumentList $errorPipeName -ScriptBlock {
    param($PipeName)

    $ErrorActionPreference = "Stop"
    $encoding = [System.Text.UTF8Encoding]::new($false)
    $server = [System.IO.Pipes.NamedPipeServerStream]::new(
        $PipeName,
        [System.IO.Pipes.PipeDirection]::InOut,
        1,
        [System.IO.Pipes.PipeTransmissionMode]::Byte,
        [System.IO.Pipes.PipeOptions]::None
    )
    try {
        $server.WaitForConnection()
        $reader = [System.IO.StreamReader]::new($server, $encoding)
        $writer = [System.IO.StreamWriter]::new($server, $encoding)
        try {
            $writer.AutoFlush = $true
            $reader.ReadLine() | Out-Null
            $writer.WriteLine((@{ status = "error"; error = "forced_failure" } | ConvertTo-Json -Compress))
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
            $server.Dispose()
        }
        catch {
        }
    }
}
try {
    Start-Sleep -Milliseconds 500
    $pipeErrorThrown = $false
    try {
        Invoke-CodexPwshPipeRequest `
            -PipeName $errorPipeName `
            -Payload (New-CodexPwshPipePayload -Command "write" -Text "/compact" -Submit $true) `
            -TimeoutMs 5000 | Out-Null
    }
    catch {
        $pipeErrorThrown = $_.Exception.Message -match "forced_failure"
    }
    Assert-True $pipeErrorThrown "pipe error response should fail the request"
    Wait-Job -Job $errorPipeJob -Timeout 10 | Out-Null
    if ($errorPipeJob.State -ne "Completed") {
        throw "fake error pipe server did not complete; state=$($errorPipeJob.State)"
    }
    Receive-Job -Job $errorPipeJob -ErrorAction Stop | Out-Null
}
finally {
    if ($errorPipeJob.State -eq "Running") {
        Stop-Job -Job $errorPipeJob
    }
    Remove-Job -Job $errorPipeJob -Force -ErrorAction SilentlyContinue
}

$pipeName = "codex-maintenance-test-$([guid]::NewGuid().ToString("N"))"
$pipeLogPath = Join-Path $WorkDir "pipe-requests.jsonl"
$pipeSessionPath = Join-Path $WorkDir "pipe-session.jsonl"
$pipeSessionId = "019e-pipe-session"
@(
    (@{
        timestamp = "2026-05-22T00:00:00.000Z"
        type = "session_meta"
        payload = @{
            id = $pipeSessionId
            cwd = "C:\repo"
        }
    } | ConvertTo-Json -Depth 8 -Compress),
    (@{
        timestamp = "2026-05-22T00:01:00.000Z"
        type = "event_msg"
        payload = @{
            token_count = @{
                total_token_usage = @{
                    input_tokens = 250
                    output_tokens = 50
                    total_tokens = 300
                }
                model_context_window = 1000
            }
        }
    } | ConvertTo-Json -Depth 8 -Compress)
) | Set-Content -LiteralPath $pipeSessionPath -Encoding UTF8

$managedTerminalRoot = Join-Path $WorkDir "managed-terminals"
New-Item -ItemType Directory -Path $managedTerminalRoot | Out-Null
$managedSidecarPath = Join-Path $managedTerminalRoot "codex-live.json"
@{
    session_id = "managed-codex-live"
    provider = "codex"
    command_name = "codex"
    cwd = "C:\repo"
    loop_target_pwsh_pipe = $pipeName
    loop_target_pwsh_pid = $PID
    process_pid = $PID
} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $managedSidecarPath -Encoding UTF8

$resolvedManagedPipe = Resolve-CodexWizardManagedPipe -SessionPath $pipeSessionPath -ManagedTerminalRoot $managedTerminalRoot
Assert-Equal $resolvedManagedPipe.PipeName $pipeName "managed pipe should resolve from session cwd"
Assert-Equal $resolvedManagedPipe.SourcePath $managedSidecarPath "managed pipe should report sidecar source"
$resolvedProjectPipe = Resolve-CodexWizardManagedPipe -Project "C:\repo" -ManagedTerminalRoot $managedTerminalRoot
Assert-Equal $resolvedProjectPipe.PipeName $pipeName "managed pipe should resolve from explicit project"
$wrongProjectPipe = Resolve-CodexWizardManagedPipe -Project "C:\other" -ManagedTerminalRoot $managedTerminalRoot
Assert-Equal $wrongProjectPipe $null "managed pipe should not guess across projects"

$staleManagedRoot = Join-Path $WorkDir "managed-terminals-stale"
New-Item -ItemType Directory -Path $staleManagedRoot | Out-Null
@{
    session_id = "managed-codex-stale"
    provider = "codex"
    command_name = "codex"
    cwd = "C:\repo"
    loop_target_pwsh_pipe = "stale-pipe"
    loop_target_pwsh_pid = 999999
    process_pid = 999999
} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $staleManagedRoot "codex-stale.json") -Encoding UTF8
$stalePipe = Resolve-CodexWizardManagedPipe -SessionPath $pipeSessionPath -ManagedTerminalRoot $staleManagedRoot
Assert-Equal $stalePipe $null "managed pipe should ignore stale process metadata"

$watchPipeLogPath = Join-Path $WorkDir "watch-pipe.jsonl"
& (Join-Path $PSScriptRoot "Watch-CodexSessionMaintenance.ps1") `
    -Profile Director `
    -SessionPath $pipeSessionPath `
    -DryRun `
    -Once `
    -ResolveManagedPipe `
    -ManagedTerminalRoot $managedTerminalRoot `
    -LogPath $watchPipeLogPath | Out-Null
$watchPipeEvent = Get-Content -LiteralPath $watchPipeLogPath | Select-Object -Last 1 | ConvertFrom-Json
Assert-Equal $watchPipeEvent.status "dry_run_threshold_reached" "watch dry-run should keep maintenance status"
Assert-Equal $watchPipeEvent.pipeName $pipeName "watch should log resolved managed pipe"
Assert-Equal $watchPipeEvent.managedPipeSource $managedSidecarPath "watch should log managed sidecar source"

$serverJob = Start-Job -ArgumentList $pipeName, $pipeLogPath, $pipeSessionPath -ScriptBlock {
    param($PipeName, $PipeLogPath, $PipeSessionPath)

    $ErrorActionPreference = "Stop"
    $encoding = [System.Text.UTF8Encoding]::new($false)

    for ($i = 0; $i -lt 3; $i++) {
        $server = [System.IO.Pipes.NamedPipeServerStream]::new(
            $PipeName,
            [System.IO.Pipes.PipeDirection]::InOut,
            1,
            [System.IO.Pipes.PipeTransmissionMode]::Byte,
            [System.IO.Pipes.PipeOptions]::None
        )
        try {
            $server.WaitForConnection()
            $reader = [System.IO.StreamReader]::new($server, $encoding)
            $writer = [System.IO.StreamWriter]::new($server, $encoding)
            try {
                $writer.AutoFlush = $true
                $line = $reader.ReadLine()
                Add-Content -LiteralPath $PipeLogPath -Value $line
                $request = $line | ConvertFrom-Json -ErrorAction Stop
                if ($request.command -eq "write" -and $request.text -eq "/compact") {
                    (@{
                        timestamp = "2026-05-22T00:02:00.000Z"
                        type = "event_msg"
                        payload = @{
                            token_count = @{
                                total_token_usage = @{
                                    input_tokens = 150
                                    output_tokens = 30
                                    total_tokens = 180
                                }
                                model_context_window = 1000
                            }
                        }
                    } | ConvertTo-Json -Depth 8 -Compress) | Add-Content -LiteralPath $PipeSessionPath
                }
                $writer.WriteLine((@{ status = "ok" } | ConvertTo-Json -Compress))
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
                $server.Dispose()
            }
            catch {
            }
        }
    }
}

try {
    Start-Sleep -Milliseconds 500
    $pipeResult = Invoke-CodexMaintenanceOnce `
        -Profile Director `
        -SessionPath $pipeSessionPath `
        -PipeName $pipeName `
        -PipeTimeoutMs 5000 `
        -WaitAfterCompactSeconds 0 `
        -ThresholdPercent 30
    Assert-Equal $pipeResult.Status "compaction_verified_reminder_sent" "pipe maintenance should compact, verify, and send reminder"
    Assert-True ($pipeResult.Plan.Actions -contains "submit_compact") "live pipe result should keep plan for watcher logging"

    Wait-Job -Job $serverJob -Timeout 10 | Out-Null
    if ($serverJob.State -ne "Completed") {
        throw "fake pipe server did not complete; state=$($serverJob.State)"
    }
    Receive-Job -Job $serverJob -ErrorAction Stop | Out-Null

    $pipeRequests = Get-Content -LiteralPath $pipeLogPath | ConvertFrom-Json
    Assert-Equal $pipeRequests[0].command "interrupt" "pipe path should interrupt first"
    Assert-Equal $pipeRequests[1].command "write" "pipe path should write compact second"
    Assert-Equal $pipeRequests[1].text "/compact" "pipe path should submit /compact"
    Assert-Equal $pipeRequests[2].command "write" "pipe path should write reminder third"
    Assert-True ($pipeRequests[2].text -match "SOLID refactor director") "pipe path should send director reminder"
}
finally {
    if ($serverJob.State -eq "Running") {
        Stop-Job -Job $serverJob
    }
    Remove-Job -Job $serverJob -Force -ErrorAction SilentlyContinue
}

$startPlan = & (Join-Path $PSScriptRoot "Start-CodexDirectorLoop.ps1") -SessionPath $sessionPath -SessionRoot $WorkDir -DryRun
Assert-Equal $startPlan.arguments[0] "codex" "director start should use codex"
Assert-Equal $startPlan.arguments[1] "--loop" "director start should put --loop after codex"
Assert-Equal $startPlan.arguments[2] "resume" "director start should resume existing session"
Assert-Equal $startPlan.sessionId $sessionId "director start should resolve session id"
Assert-True ($startPlan.commandLine -match "Automatic Director post-compaction") "director start should send stale reminder"

$watchLogPath = Join-Path $WorkDir "watch-dry-run.jsonl"
Remove-Item -LiteralPath $watchLogPath -Force -ErrorAction SilentlyContinue
& (Join-Path $PSScriptRoot "Watch-CodexSessionMaintenance.ps1") `
    -Profile Director `
    -SessionPath $sessionPath `
    -PipeName "fake-pipe-for-test" `
    -ResolveManagedPipe `
    -PipeTimeoutMs 1234 `
    -DryRun `
    -Once `
    -LogPath $watchLogPath | Out-Null
$watchEvent = Get-Content -Raw -LiteralPath $watchLogPath | ConvertFrom-Json
Assert-Equal $watchEvent.status "dry_run_threshold_reached" "watcher dry-run should record threshold event"
Assert-Equal $watchEvent.pipeName "fake-pipe-for-test" "watcher log should keep explicit fake pipe name"
Assert-Equal $watchEvent.pipeTimeoutMs 1234 "watcher log should keep pipe timeout"
Assert-Equal $watchEvent.pipeSource "explicit" "watcher log should mark explicit pipe source"
Assert-Equal $watchEvent.resolveManagedPipe $false "watcher log should mark managed-pipe resolution state"
Assert-Equal $watchEvent.managedPipeSource $null "watcher log should not invent managed source for explicit pipe"

$agentsRoot = Join-Path (Split-Path -Parent $PSScriptRoot) "agents"
foreach ($canaryName in @("test-terminal-escape-canary.ps1", "test-terminal-esc-compact-canary.ps1")) {
    $canaryText = Get-Content -Raw -LiteralPath (Join-Path $agentsRoot $canaryName)
    Assert-True ($canaryText -match 'Invoke-SolidTerminalSendKeys\s+-Keys\s+"\{ESCAPE\}"') "$canaryName should cancel with ESCAPE"
    Assert-True (-not ($canaryText -match '\^C|CTRL\+C|ControlC|VK_CONTROL')) "$canaryName should not use control-c key sequences"
}

[pscustomobject]@{
    status = "passed"
    workDir = $WorkDir
    assertions = $script:Assertions
} | Format-List
