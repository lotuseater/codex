<#
.SYNOPSIS
    Truth-gate a backgrounded cargo run by SCANNING ITS LOG — never by trusting the
    process exit code. Exists to prevent the documented "false-green exit 0" mistake,
    where a trailing `echo` (or a wrapper) masks cargo's non-zero exit while the log
    still contains `error[...]` lines and `could not compile`.

.DESCRIPTION
    Reads the cargo build/check log at -LogPath and decides PASS/FAIL purely from its
    CONTENT. The build is treated as FAILED (exit 1) when the log contains ANY of:

        * a line matching `^error\[`      (rustc error codes at line start, e.g. error[E0432])
        * an inline ` error[` / `: error[`(real fork logs prefix the path: `core\..rs: error[E0432]`)
        * a line matching `^error:`       (rustc/cargo bare errors)
        * `error: could not compile`      (the canonical cargo failure line)
        * `internal compiler error` / panic (`thread 'main' panicked`, `thread '...' panicked`)
        * `LINK : fatal`                  (MSVC linker failure)
        * `note: build failed`            (cargo build summary)
        * `error: aborting due to`        (rustc abort summary)
        * an explicit `EXITCODE=<n>` token with non-zero n  (wrapper-emitted exit marker)
        * `non-zero exit code: <n>`       (PowerShell NativeCommandExitException form)

    In -Strict mode `warning:` lines also fail the gate (default: warnings never fail).

    Output: a concise PASS/FAIL summary, the total error/signal count, the first ~15
    offending lines (with their 1-based line numbers), and the DISTINCT set of broken
    crates inferred from the log (`could not compile \`<crate>\``, `Compiling/Checking
    <crate>` lines preceding errors, `--> codex-rs/<crate>/...` and `--> <crate>\src\...`
    diagnostic paths, and `codex-rs/<crate>/` path fragments on error lines).

    Exit codes:
        0  no failure signal present (green)
        1  at least one failure signal present (red)
        2  -LogPath missing / unreadable (usage error)

    Read-only. Streams the file (Get-Content -ReadCount) so multi-MB logs do not get
    slurped into a single huge string. Works from anywhere; only needs -LogPath.

.PARAMETER LogPath
    Path to the cargo log to scan. Required. Relative paths resolve against the caller's
    current directory.

.PARAMETER Strict
    Also fail on `warning:` lines (clippy/rustc warnings). Off by default.

.EXAMPLE
    pwsh -File scripts/verify-cargo-log.ps1 -LogPath .codex/workflow/agents/codex_core_check_stderr.txt
    # exit 1, prints the broken crate (codex-core) and the first offending lines.

.EXAMPLE
    pwsh -File scripts/verify-cargo-log.ps1 -LogPath logs/clean-build.txt
    # exit 0, prints a one-line green summary.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$LogPath,

    [switch]$Strict
)

$ErrorActionPreference = "Stop"

# --- Resolve + validate the log path (exit 2 on any usage problem) ---
# NB: emit usage errors to the error stream WITHOUT throwing — under
# $ErrorActionPreference='Stop' a `Write-Error` would terminate before `exit 2`
# runs and the script would wrongly exit 1. `$host.UI.WriteErrorLine` bypasses
# the preference and lets the explicit `exit 2` stand.
$resolved = $null
try {
    $resolved = (Resolve-Path -LiteralPath $LogPath -ErrorAction Stop).Path
} catch {
    $host.UI.WriteErrorLine("verify-cargo-log: log not found: '$LogPath'")
    exit 2
}
if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
    $host.UI.WriteErrorLine("verify-cargo-log: not a file: '$resolved'")
    exit 2
}

# --- Failure-signal patterns. Each is a single regex tested per line. ---
# rustc error codes, either at line start OR prefixed by a path (` error[` / `: error[`).
$reErrorCode = '(^error\[|[: ]error\[)'
# bare rustc/cargo errors at line start, plus the canonical compile-failure line anywhere.
$reBareError = '^error:'
$reCouldNotCompile = 'error: could not compile'
$reAbort = 'error: aborting due to'
$reIce = 'internal compiler error'
$rePanic = "thread '.*' panicked|thread `"main`" panicked"
$reLinkFatal = 'LINK : fatal'
$reBuildFailed = 'note: build failed'
# explicit exit markers; capture the numeric value so 0 is treated as success.
$reExitCode = 'EXITCODE=([0-9]+)'
$reNativeExit = 'non-zero exit code:\s*([0-9]+)'
$reWarning = '(^|[: ])warning:'   # only consulted in -Strict

# A line is a "hard" failure signal if it matches any of these (regardless of -Strict).
$hardSignals = @(
    $reErrorCode, $reBareError, $reCouldNotCompile, $reAbort,
    $reIce, $rePanic, $reLinkFatal, $reBuildFailed
)

# --- Crate-name extraction helpers ---
# Map a diagnostic/compile line to a crate name, or $null.
function Get-CrateFromLine {
    param([string]$Line)

    # 1. `could not compile `codex-core` (lib) due to ...`
    if ($Line -match 'could not compile [`'']([A-Za-z0-9_\-]+)[`'']') { return $Matches[1] }

    # 2. `   Compiling codex-core v0.0.0 (...)` / `   Checking codex-core v0.0.0 (...)`
    if ($Line -match '^\s*(?:Compiling|Checking)\s+([A-Za-z0-9_\-]+)\s+v') { return $Matches[1] }

    # 3. `--> codex-rs/<crate>/...` (forward slash) or `--> codex-rs\<crate>\...`
    if ($Line -match 'codex-rs[\\/]([A-Za-z0-9_\-]+)[\\/]') { return $Matches[1] }

    # 4. Fork error-log form with no `codex-rs/` prefix: `core\src\...` / `tui/src/...`.
    #    The first path segment is the crate directory (core, tui, protocol, ...).
    if ($Line -match '(?:^|[ \t(])([a-z][a-z0-9_\-]*)[\\/]src[\\/]') { return $Matches[1] }

    return $null
}

# --- Stream the log. Get-Content -ReadCount batches lines without building one mega-string. ---
$lineNo = 0
$failCount = 0          # count of lines carrying any (hard, or strict-warning) failure signal
$exitFailSeen = $false  # a non-zero EXITCODE=/exit-code token was seen
$offending = New-Object System.Collections.Generic.List[object]   # first N offending lines
$crates = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
$lastCompilingCrate = $null
$maxOffending = 15

Get-Content -LiteralPath $resolved -ReadCount 512 | ForEach-Object {
    foreach ($line in $_) {
        $lineNo++

        # Track the most-recently-announced crate so a following bare `error:` can be attributed.
        if ($line -match '^\s*(?:Compiling|Checking)\s+([A-Za-z0-9_\-]+)\s+v') {
            $lastCompilingCrate = $Matches[1]
        }

        $isFail = $false

        # Exit-code tokens: only fail when the captured value is non-zero.
        if ($line -match $reExitCode -or $line -match $reNativeExit) {
            if ([int]$Matches[1] -ne 0) { $isFail = $true; $exitFailSeen = $true }
        }

        if (-not $isFail) {
            foreach ($pat in $hardSignals) {
                if ($line -match $pat) { $isFail = $true; break }
            }
        }

        if (-not $isFail -and $Strict -and ($line -match $reWarning)) {
            $isFail = $true
        }

        if ($isFail) {
            $failCount++

            $crate = Get-CrateFromLine -Line $line
            if (-not $crate -and $lastCompilingCrate) { $crate = $lastCompilingCrate }
            if ($crate) { [void]$crates.Add($crate) }

            if ($offending.Count -lt $maxOffending) {
                $offending.Add([pscustomobject]@{
                    Line = $lineNo
                    Text = $line.TrimEnd()
                })
            }
        }
    }
}

$totalLines = $lineNo
$failed = ($failCount -gt 0)
$crateList = @($crates | Sort-Object)

# --- Render report ---
$status = if ($failed) { "FAIL" } else { "PASS" }
$strictNote = if ($Strict) { " (strict: warnings fail)" } else { "" }

Write-Host "verify-cargo-log: $status$strictNote  log=$resolved  lines=$totalLines  signals=$failCount"

if ($failed) {
    if ($crateList.Count -gt 0) {
        Write-Host ("broken crate(s): " + ($crateList -join ", "))
    } else {
        Write-Host "broken crate(s): <none inferred>"
    }
    if ($exitFailSeen) {
        Write-Host "note: non-zero EXITCODE/exit-code marker present (would be a FALSE GREEN if you trusted the exit code)"
    }
    Write-Host ("first {0} offending line(s):" -f ([Math]::Min($maxOffending, $offending.Count)))
    foreach ($o in $offending) {
        Write-Host ("  {0,6}: {1}" -f $o.Line, $o.Text)
    }
    if ($failCount -gt $offending.Count) {
        Write-Host ("  ... and {0} more signal line(s) not shown." -f ($failCount - $offending.Count))
    }
    exit 1
}

Write-Host "GREEN: no rustc/cargo error, ICE, panic, linker, build-failed, or non-zero exit marker found."
exit 0
