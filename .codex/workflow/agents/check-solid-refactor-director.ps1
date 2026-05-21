param(
    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [string]$WorkflowHandoffPath = (Join-Path (Split-Path $PSScriptRoot -Parent) "solid-refactor-handoff.md"),
    [string]$SessionPath,
    [int]$RecentMinutes = 20,
    [int]$MaxItems = 5,
    [int]$SessionTailLines = 240,
    [int]$RecentTalkItems = 3,
    [int]$MaxTalkChars = 180,
    [double]$CompactWarnPercent = 25.0,
    [double]$CompactNowPercent = 30.0
)

$ErrorActionPreference = "Stop"

$now = Get-Date

function Convert-ToLocalTime {
    param([datetime]$Time)

    if ($Time.Kind -eq [DateTimeKind]::Utc) {
        return $Time.ToLocalTime()
    }

    return $Time
}

function Format-Age {
    param([datetime]$Time)

    $localTime = Convert-ToLocalTime $Time
    $age = $now - $localTime
    if ($age.TotalMinutes -lt 60) {
        return ("{0:N1}m" -f $age.TotalMinutes)
    }

    return ("{0:N1}h" -f $age.TotalHours)
}

function Format-ClockTime {
    param([datetime]$Time)

    return (Convert-ToLocalTime $Time).ToString("HH:mm:ss")
}

function Format-Items {
    param([object[]]$Items)

    if (-not $Items -or $Items.Count -eq 0) {
        return "(none)"
    }

    return (($Items | ForEach-Object {
        "{0} ({1})" -f $_.Name, (Format-Age $_.LastWriteTime)
    }) -join "; ")
}

function Recent-Items {
    param([string]$Filter)

    @(Get-ChildItem -LiteralPath $PSScriptRoot -Filter $Filter -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First $MaxItems)
}

function Normalize-OneLine {
    param([string]$Text)

    if (-not $Text) {
        return ""
    }

    $normalized = $Text -replace "\s+", " "
    return $normalized.Trim()
}

function Shorten-Text {
    param(
        [string]$Text,
        [int]$MaxChars
    )

    $normalized = Normalize-OneLine $Text
    if ($normalized.Length -le $MaxChars) {
        return $normalized
    }

    return $normalized.Substring(0, [Math]::Max(0, $MaxChars - 3)) + "..."
}

function Try-ParseSessionStart {
    param([string]$Name)

    if ($Name -notmatch '^rollout-(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})-(\d{2})-') {
        return $null
    }

    $stamp = "{0}T{1}:{2}:{3}" -f $Matches[1], $Matches[2], $Matches[3], $Matches[4]
    try {
        return [datetime]::ParseExact(
            $stamp,
            "yyyy-MM-ddTHH:mm:ss",
            [Globalization.CultureInfo]::InvariantCulture
        )
    } catch {
        return $null
    }
}

function Session-DateDirectories {
    param([datetime]$StartedAt)

    $root = Join-Path $env:USERPROFILE ".codex\sessions"
    if (-not (Test-Path -LiteralPath $root)) {
        return @()
    }

    $dates = @($StartedAt.Date, $StartedAt.Date.AddDays(-1), $StartedAt.Date.AddDays(1)) |
        Sort-Object -Unique

    @($dates | ForEach-Object {
        Join-Path $root (Join-Path $_.ToString("yyyy") (Join-Path $_.ToString("MM") $_.ToString("dd")))
    } | Where-Object { Test-Path -LiteralPath $_ })
}

function Find-DirectorSessionFromState {
    param(
        [object]$State,
        [int]$TailLines
    )

    if (-not $State -or -not $State.runnerStartedAt) {
        return $null
    }

    try {
        $startedAt = [datetime]$State.runnerStartedAt
    } catch {
        return $null
    }

    $candidates = @()
    foreach ($dir in (Session-DateDirectories $startedAt)) {
        $candidates += Get-ChildItem -LiteralPath $dir -Filter "*.jsonl" -File -ErrorAction SilentlyContinue |
            ForEach-Object {
                $sessionStartedAt = Try-ParseSessionStart $_.Name
                if (-not $sessionStartedAt) {
                    return
                }

                $distanceSeconds = [Math]::Abs(($sessionStartedAt - $startedAt).TotalSeconds)
                [pscustomobject]@{
                    File = $_
                    DistanceSeconds = $distanceSeconds
                }
            } |
            Where-Object { $_.DistanceSeconds -le 300 }
    }

    $nearest = @($candidates | Sort-Object DistanceSeconds | Select-Object -First 12)
    foreach ($candidate in $nearest) {
        $tailText = ""
        try {
            $tailText = (Get-Content -LiteralPath $candidate.File.FullName -Tail $TailLines -ErrorAction Stop) -join "`n"
        } catch {
            continue
        }

        if ($tailText -match 'SOLID refactor director|solid_refactor_wave|solid-refactor-handoff') {
            return [pscustomobject]@{
                Path = $candidate.File.FullName
                Source = "session file near director start"
            }
        }
    }

    if ($nearest.Count -gt 0) {
        return [pscustomobject]@{
            Path = $nearest[0].File.FullName
            Source = "nearest session file to director start"
        }
    }

    return $null
}

function Resolve-DirectorSession {
    param(
        [object]$State,
        [string]$ExplicitPath,
        [int]$TailLines
    )

    if ($ExplicitPath) {
        return [pscustomobject]@{ Path = $ExplicitPath; Source = "argument" }
    }

    if ($State -and ($State.PSObject.Properties.Name -contains "sessionPath") -and $State.sessionPath) {
        return [pscustomobject]@{ Path = [string]$State.sessionPath; Source = "state" }
    }

    return Find-DirectorSessionFromState $State $TailLines
}

function Read-SessionTail {
    param(
        [string]$Path,
        [int]$TailLines
    )

    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) {
        return @()
    }

    @(Get-Content -LiteralPath $Path -Tail $TailLines -ErrorAction Stop)
}

function Read-JsonLine {
    param([string]$Line)

    try {
        return $Line | ConvertFrom-Json -ErrorAction Stop
    } catch {
        return $null
    }
}

function Get-TokenSnapshot {
    param([string[]]$Lines)

    for ($index = $Lines.Count - 1; $index -ge 0; $index--) {
        $line = $Lines[$index]
        if ($line -notmatch '"token_count"') {
            continue
        }

        $record = Read-JsonLine $line
        if (-not $record -or $record.type -ne "event_msg" -or $record.payload.type -ne "token_count") {
            continue
        }

        $info = $record.payload.info
        $window = [double]$info.model_context_window
        $used = [double]$info.last_token_usage.total_tokens
        if ($used -le 0) {
            $used = [double]$info.last_token_usage.input_tokens
        }

        $percent = $null
        if ($window -gt 0 -and $used -gt 0) {
            $percent = [Math]::Round(($used / $window) * 100.0, 1)
        }

        return [pscustomobject]@{
            Percent = $percent
            Used = [int64]$used
            Window = [int64]$window
            Timestamp = [datetime]$record.timestamp
        }
    }

    return $null
}

function Format-TokenSnapshot {
    param([object]$Snapshot)

    if (-not $Snapshot) {
        return "(no token_count in tail)"
    }

    if ($null -eq $Snapshot.Percent) {
        return ("unknown percent ({0:N0}/{1:N0}, {2})" -f $Snapshot.Used, $Snapshot.Window, (Format-Age $Snapshot.Timestamp))
    }

    return ("{0:N1}% ({1:N0}/{2:N0}, {3})" -f $Snapshot.Percent, $Snapshot.Used, $Snapshot.Window, (Format-Age $Snapshot.Timestamp))
}

function Get-RecentDirectorTalk {
    param(
        [string[]]$Lines,
        [int]$Take,
        [int]$MaxChars
    )

    $messages = @()
    foreach ($line in $Lines) {
        if ($line -notmatch '"type":"message"') {
            continue
        }

        $record = Read-JsonLine $line
        if (-not $record -or $record.type -ne "response_item") {
            continue
        }

        $payload = $record.payload
        if ($payload.type -ne "message" -or $payload.role -ne "assistant") {
            continue
        }

        $text = (($payload.content | ForEach-Object { $_.text }) -join " ")
        $text = Shorten-Text $text $MaxChars
        if ($text) {
            $messages += [pscustomobject]@{
                Timestamp = [datetime]$record.timestamp
                Text = $text
            }
        }
    }

    @($messages | Select-Object -Last $Take)
}

$state = $null
$stateSummary = "(missing)"
if (Test-Path -LiteralPath $StatePath) {
    try {
        $state = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
        $stateSummary = "rootPid={0} hwnd={1} remembered={2}" -f $state.rootPid, $state.windowHandle, $state.windowRememberedAt
    } catch {
        $stateSummary = "unreadable: $($_.Exception.Message)"
    }
}

$workflowHandoff = $null
if (Test-Path -LiteralPath $WorkflowHandoffPath) {
    $workflowHandoff = Get-Item -LiteralPath $WorkflowHandoffPath
}

$workerLogs = Recent-Items "solid_refactor_wave*.exec.visible.log"
$workerHandoffs = Recent-Items "solid_refactor_wave*.handoff.md"

$recentCutoff = $now.AddMinutes(-1 * $RecentMinutes)
$recentLogCount = @($workerLogs | Where-Object { $_.LastWriteTime -ge $recentCutoff }).Count
$recentHandoffCount = @($workerHandoffs | Where-Object { $_.LastWriteTime -ge $recentCutoff }).Count

$session = Resolve-DirectorSession $state $SessionPath $SessionTailLines
$sessionTail = @()
if ($session -and (Test-Path -LiteralPath $session.Path)) {
    $sessionTail = Read-SessionTail $session.Path $SessionTailLines
}

$tokenSnapshot = Get-TokenSnapshot $sessionTail
$recentTalk = Get-RecentDirectorTalk $sessionTail $RecentTalkItems $MaxTalkChars
$recentTalkText = (($recentTalk | ForEach-Object { $_.Text }) -join " ")
$possibleUnderDelegation = $recentTalkText -match '(?i)\b(single|one|two|2)\s+(follow-up\s+)?(worker|session)s?\b|\bonly\s+\d+\s+(worker|session)s?\b'

if ($tokenSnapshot -and $null -ne $tokenSnapshot.Percent -and $tokenSnapshot.Percent -ge $CompactNowPercent) {
    $action = "director context >= $CompactNowPercent%; run .codex\workflow\agents\compact-solid-refactor-director.ps1"
} elseif ($tokenSnapshot -and $null -ne $tokenSnapshot.Percent -and $tokenSnapshot.Percent -ge $CompactWarnPercent) {
    $action = "director context >= $CompactWarnPercent%; ask director to compact at the next safe handoff checkpoint"
} elseif ($recentHandoffCount -gt 0) {
    $action = "read fresh worker handoffs only; then decide whether one short director follow-up is needed"
} elseif ($recentLogCount -gt 0) {
    $action = "wait; worker logs changed recently"
} else {
    $action = "send one short director follow-up or ask it to update handoff before compact"
}

"SOLID director checkpoint: {0:o}" -f $now
"state: {0}" -f $stateSummary
if ($workflowHandoff) {
    "solid handoff: {0} ({1})" -f $workflowHandoff.Name, (Format-Age $workflowHandoff.LastWriteTime)
} else {
    "solid handoff: (missing)"
}
if ($session -and (Test-Path -LiteralPath $session.Path)) {
    "session: {0} ({1}, tail={2})" -f (Split-Path $session.Path -Leaf), $session.Source, $SessionTailLines
} else {
    "session: (not resolved; pass -SessionPath or relaunch with current scripts)"
}
"context: {0}" -f (Format-TokenSnapshot $tokenSnapshot)
if ($recentTalk.Count -gt 0) {
    "recent director talk:"
    $recentTalk | ForEach-Object {
        "- {0} {1}" -f (Format-ClockTime $_.Timestamp), $_.Text
    }
} else {
    "recent director talk: (none in tail)"
}
if ($possibleUnderDelegation) {
    "parallelism: recent director talk suggests possible under-delegation; include broader-wave note in the next reminder"
} else {
    "parallelism: no under-delegation signal in recent talk"
}
"worker logs: {0}" -f (Format-Items $workerLogs)
"worker handoffs: {0}" -f (Format-Items $workerHandoffs)
"action: {0}" -f $action
"note: normal checkpoints read only the session tail and skip singleton/process scans."
