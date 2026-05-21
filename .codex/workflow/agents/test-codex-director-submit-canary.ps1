param(
    [string]$WorkDir = (Join-Path $PSScriptRoot "..\tmp"),
    [int]$InitialPromptDelaySeconds = 7,
    [int]$InitialPromptWaitMs = 12000,
    [int]$FollowupWaitMs = 5000,
    [int]$SettleSeconds = 8
)

$ErrorActionPreference = "Stop"

$resolvedWorkDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($WorkDir)
New-Item -ItemType Directory -Force -Path $resolvedWorkDir | Out-Null

$runId = "director_submit_canary_{0:yyyyMMdd_HHmmss_fff}_{1}" -f (Get-Date), $PID
$initialMarker = "DIRECTOR_INITIAL_SUBMIT_$runId"
$followupMarker = "DIRECTOR_FOLLOWUP_SUBMIT_$runId"
$promptPath = Join-Path $resolvedWorkDir "$runId.prompt.md"

@"
You are only a director submit canary. Reply exactly: $initialMarker.
Do not inspect files, launch workers, edit files, run commands, or continue refactoring.
Then wait for follow-up.
"@ | Set-Content -LiteralPath $promptPath -Encoding UTF8

$startScript = Join-Path $PSScriptRoot "start-solid-refactor-director.ps1"
$sendScript = Join-Path $PSScriptRoot "send-solid-refactor-director-followup.ps1"
$interruptScript = Join-Path $PSScriptRoot "interrupt-solid-refactor-director.ps1"
$stopScript = Join-Path $PSScriptRoot "stop-solid-refactor-director.ps1"

function Find-SessionMarker {
    param([Parameter(Mandatory = $true)][string]$Marker)

    $sessionsRoot = Join-Path $env:USERPROFILE ".codex\sessions"
    if (-not (Test-Path -LiteralPath $sessionsRoot)) {
        return @()
    }

    @(rg --fixed-strings --files-with-matches --glob "*.jsonl" $Marker $sessionsRoot 2>$null)
}

try {
    & $startScript -PromptPath $promptPath -InitialPromptDelaySeconds $InitialPromptDelaySeconds -InitialPromptWaitMs $InitialPromptWaitMs | Out-Host
    Start-Sleep -Seconds $SettleSeconds

    & $sendScript -Message "Director submit follow-up canary. Reply exactly: $followupMarker. Do not inspect files, run commands, launch workers, or edit files." -WaitMs $FollowupWaitMs | Out-Host
    Start-Sleep -Seconds $SettleSeconds

    $initialHits = @(Find-SessionMarker -Marker $initialMarker)
    $followupHits = @(Find-SessionMarker -Marker $followupMarker)

    if ($initialHits.Count -eq 0) {
        throw "Initial prompt marker was not found in Codex session logs: $initialMarker"
    }

    if ($followupHits.Count -eq 0) {
        throw "Follow-up marker was not found in Codex session logs: $followupMarker"
    }

    & $interruptScript -WaitMs 1000 | Out-Host

    [pscustomobject]@{
        Succeeded = $true
        InitialMarker = $initialMarker
        InitialSession = $initialHits[0]
        FollowupMarker = $followupMarker
        FollowupSession = $followupHits[0]
    } | Format-List
} finally {
    & $stopScript -ScanFallback | Out-Host
}
