param(
    [string]$StatePath = (Join-Path $PSScriptRoot "solid_refactor_director.state.json"),
    [string]$SessionPath,
    [int]$SessionTailLines = 160,
    [int]$InitialWaitSeconds = 5,
    [int]$StableSeconds = 12,
    [int]$MaxWaitSeconds = 180,
    [int]$InterruptDelayMs = 700,
    [switch]$NoInterrupt,
    [switch]$WaitAndRemindOnly,
    [string]$Reminder = "Post-compaction reminder: reread .codex\workflow\solid-refactor-overseer-memo.md, .codex\workflow\solid-refactor-handoff.md, docs\current-project-architecture-solid-refactor-plan.md, docs\current-project-architecture-solid-review.md, and fresh worker handoffs. Continue as director only: spawn real separate visible worker windows via codex-workers, no broad builds/tests/schema/formatters/Bazel/lock/release until architecture refactor is complete, and no broad self-review. Maybe you spawned too few sessions for current broad work; think of more possible subtasks and spawn a broader worker wave according to your handoff."
)

$ErrorActionPreference = "Stop"

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

function Find-DirectorSession {
    param([object]$State)

    if (-not $State -or -not $State.runnerStartedAt) {
        return $null
    }

    try {
        $startedAt = [datetime]$State.runnerStartedAt
    } catch {
        return $null
    }

    $root = Join-Path $env:USERPROFILE ".codex\sessions"
    $dates = @($startedAt.Date, $startedAt.Date.AddDays(-1), $startedAt.Date.AddDays(1)) |
        Sort-Object -Unique
    $candidates = @()

    foreach ($date in $dates) {
        $dir = Join-Path $root (Join-Path $date.ToString("yyyy") (Join-Path $date.ToString("MM") $date.ToString("dd")))
        if (-not (Test-Path -LiteralPath $dir)) {
            continue
        }

        $candidates += Get-ChildItem -LiteralPath $dir -Filter "*.jsonl" -File -ErrorAction SilentlyContinue |
            ForEach-Object {
                $sessionStartedAt = Try-ParseSessionStart $_.Name
                if (-not $sessionStartedAt) {
                    return
                }

                [pscustomobject]@{
                    File = $_
                    DistanceSeconds = [Math]::Abs(($sessionStartedAt - $startedAt).TotalSeconds)
                }
            } |
            Where-Object { $_.DistanceSeconds -le 300 }
    }

    foreach ($candidate in @($candidates | Sort-Object DistanceSeconds | Select-Object -First 12)) {
        $tailText = ""
        try {
            $tailText = (Get-Content -LiteralPath $candidate.File.FullName -Tail $SessionTailLines -ErrorAction Stop) -join "`n"
        } catch {
            continue
        }

        if ($tailText -match 'SOLID refactor director|solid_refactor_wave|solid-refactor-handoff') {
            return $candidate.File.FullName
        }
    }

    $nearest = @($candidates | Sort-Object DistanceSeconds | Select-Object -First 1)
    if ($nearest.Count -gt 0) {
        return $nearest[0].File.FullName
    }

    return $null
}

function Resolve-SessionPath {
    param([object]$State)

    if ($SessionPath) {
        return $SessionPath
    }

    if ($State -and ($State.PSObject.Properties.Name -contains "sessionPath") -and $State.sessionPath) {
        return [string]$State.sessionPath
    }

    return Find-DirectorSession $State
}

function Wait-ForSessionQuiet {
    param([string]$Path)

    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) {
        Start-Sleep -Seconds $InitialWaitSeconds
        return "session not resolved; waited $InitialWaitSeconds seconds"
    }

    $deadline = (Get-Date).AddSeconds($MaxWaitSeconds)
    $lastWrite = (Get-Item -LiteralPath $Path).LastWriteTimeUtc
    $lastChange = Get-Date

    Start-Sleep -Seconds $InitialWaitSeconds
    while ((Get-Date) -lt $deadline) {
        $currentWrite = (Get-Item -LiteralPath $Path).LastWriteTimeUtc
        if ($currentWrite -ne $lastWrite) {
            $lastWrite = $currentWrite
            $lastChange = Get-Date
        } elseif (((Get-Date) - $lastChange).TotalSeconds -ge $StableSeconds) {
            return "session stable for $StableSeconds seconds"
        }

        Start-Sleep -Seconds 2
    }

    return "max wait reached after $MaxWaitSeconds seconds"
}

$state = $null
if (Test-Path -LiteralPath $StatePath) {
    $state = Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
}

$session = Resolve-SessionPath $state
$sendScript = Join-Path $PSScriptRoot "send-solid-refactor-director-followup.ps1"
$interruptScript = Join-Path $PSScriptRoot "interrupt-solid-refactor-director.ps1"

if (-not $WaitAndRemindOnly) {
    if (-not $NoInterrupt) {
        & $interruptScript | Out-Null
        Start-Sleep -Milliseconds $InterruptDelayMs
    }

    & $sendScript -Message "/compact" | Out-Null
}

$waitResult = Wait-ForSessionQuiet $session
& $sendScript -Message $Reminder | Out-Null

[pscustomobject]@{
    InterruptSent = (-not $WaitAndRemindOnly -and -not $NoInterrupt)
    CompactSent = (-not $WaitAndRemindOnly)
    Session = $session
    WaitResult = $waitResult
    ReminderSent = $true
    ReminderLength = $Reminder.Length
} | Format-List
