<#
.SYNOPSIS
    ONE iteration of the post-merge build-fix loop: cargo check -> truth-gate -> triage.

.DESCRIPTION
    Runs a SINGLE cargo check pass and classifies the result. The ORCHESTRATOR drives the
    outer loop — re-invoke this script after fix-workers land their edits, until it exits 0.

    This script does NOT spawn fix-workers and does NOT loop forever. It produces the
    inputs the orchestrator needs to brief the next round of file-disjoint fix-workers:
      * triage_<n>.json   -- machine-readable slice partition
      * triage_<n>.md     -- human summary (one line per slice) for worker prompt authoring

    cfg(test) errors (~1918 deferred) are intentionally excluded: cargo is always invoked
    WITHOUT --tests so test-only code does not gate the merge build-fix loop.

    CRITICAL CWD RULE: cargo MUST run from $Repo/codex-rs/, NOT the repo root.
    codex-rs/rust-toolchain.toml pins rustc 1.95.0; toolchain selection follows CWD.
    Running from the repo root picks the global/default toolchain (1.93.0) and produces
    a TOOLCHAIN TRAP log -- merge-buildfix-triage.ps1 will exit 3 and print the banner.

    EXIT CODES (orchestrator contract):
        0  build is GREEN (verify-cargo-log.ps1 found no error signals)
        1  internal failure (missing sub-script, unreadable log, unexpected exception)
        2  build has errors; triage_<n>.json + triage_<n>.md written; fix-workers needed

    Iteration numbering: <n> is the next integer not already used in $LogDir (check_0.log,
    check_1.log, ...). Never uses timestamps. Safe to re-invoke concurrently in disjoint
    $LogDir paths; NOT safe to re-invoke concurrently with the SAME $LogDir (race on <n>).

.PARAMETER Repo
    Path to the repository root. Defaults to the parent of $PSScriptRoot (i.e. the repo
    root when the script lives in scripts/).

.PARAMETER LogDir
    Directory where check_<n>.log, triage_<n>.json, and triage_<n>.md are written.
    Defaults to $Repo/.codex/tmp/merge_2026-06-20/buildlogs.

.PARAMETER SkipCheck
    Skip the cargo check step and re-triage the MOST RECENT check_<n>.log already in
    $LogDir. Useful when a previous cargo run was interrupted and the log is complete.

.EXAMPLE
    # Iteration 0 (no prior log):
    pwsh -File scripts/merge-buildfix-fanout.ps1
    # Prints: BUILDFIX: 3 error-slices -> triage_0.json ; exits 2.

.EXAMPLE
    # After fix-workers land, run iteration 1:
    pwsh -File scripts/merge-buildfix-fanout.ps1
    # Prints: BUILDFIX: GREEN (log=...check_1.log) ; exits 0.

.EXAMPLE
    # Re-triage the existing latest log without re-running cargo:
    pwsh -File scripts/merge-buildfix-fanout.ps1 -SkipCheck
#>
[CmdletBinding()]
param(
    [string]$Repo     = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
    [string]$LogDir   = "",
    [switch]$SkipCheck
)

$ErrorActionPreference = "Stop"
# Native exe non-zero exits must NOT throw automatically -- we read $LASTEXITCODE explicitly.
$PSNativeCommandUseErrorActionPreference = $false

# ---------------------------------------------------------------------------
# Resolve defaults and locate sibling scripts.
# ---------------------------------------------------------------------------
if (-not $LogDir) {
    $LogDir = Join-Path $Repo ".codex\tmp\merge_2026-06-20\buildlogs"
}

$ScriptsDir   = $PSScriptRoot
$VerifyScript = Join-Path $ScriptsDir "verify-cargo-log.ps1"
$TriageScript = Join-Path $ScriptsDir "merge-buildfix-triage.ps1"

foreach ($s in @($VerifyScript, $TriageScript)) {
    if (-not (Test-Path -LiteralPath $s -PathType Leaf)) {
        Write-Host "BUILDFIX ERROR: required sibling script not found: $s"
        exit 1
    }
}

# ---------------------------------------------------------------------------
# Ensure $LogDir exists.
# ---------------------------------------------------------------------------
if (-not (Test-Path -LiteralPath $LogDir -PathType Container)) {
    $null = New-Item -ItemType Directory -Force -Path $LogDir
    Write-Host "BUILDFIX: created log directory: $LogDir"
}

# ---------------------------------------------------------------------------
# Determine iteration number <n>: next integer not already used.
# Enumerate check_*.log files; find the highest ordinal and add 1.
# ---------------------------------------------------------------------------
$existingLogs = @(Get-ChildItem -LiteralPath $LogDir -Filter "check_*.log" -ErrorAction SilentlyContinue |
    ForEach-Object {
        if ($_.Name -match '^check_(\d+)\.log$') { [int]$Matches[1] }
    } |
    Sort-Object)

$n = if ($existingLogs.Count -gt 0) { ($existingLogs[-1]) + 1 } else { 0 }
$LogPath    = Join-Path $LogDir ("check_{0}.log"   -f $n)
$TriageJson = Join-Path $LogDir ("triage_{0}.json" -f $n)
$TriageMd   = Join-Path $LogDir ("triage_{0}.md"   -f $n)

# ---------------------------------------------------------------------------
# STEP 1: cargo check (unless -SkipCheck).
# ---------------------------------------------------------------------------
if ($SkipCheck) {
    # Re-use the MOST RECENT existing log (highest ordinal already in the dir).
    if ($existingLogs.Count -eq 0) {
        Write-Host "BUILDFIX ERROR: -SkipCheck specified but no check_*.log files found in: $LogDir"
        exit 1
    }
    $latestOrdinal = $existingLogs[-1]
    $LogPath    = Join-Path $LogDir ("check_{0}.log"   -f $latestOrdinal)
    $TriageJson = Join-Path $LogDir ("triage_{0}.json" -f $latestOrdinal)
    $TriageMd   = Join-Path $LogDir ("triage_{0}.md"   -f $latestOrdinal)
    Write-Host "BUILDFIX: -SkipCheck -- re-triaging existing log: $LogPath"
} else {
    # Run cargo from codex-rs/ (toolchain CWD rule -- see header).
    $CargoDir = Join-Path $Repo "codex-rs"
    if (-not (Test-Path -LiteralPath $CargoDir -PathType Container)) {
        Write-Host "BUILDFIX ERROR: codex-rs/ directory not found under repo root: $Repo"
        exit 1
    }

    Write-Host "BUILDFIX: iteration $n -- running cargo check (CWD=$CargoDir) -> $LogPath"
    Write-Host "BUILDFIX: cargo check --workspace --release --keep-going  [cfg(test) deferred]"

    Push-Location $CargoDir
    try {
        # Tee both stdout and stderr into the log.  cargo check writes diagnostics to
        # stderr; 2>&1 merges both streams before Tee-Object so nothing is lost.
        # The trailing "EXITCODE=$LASTEXITCODE" token lets verify-cargo-log.ps1 detect the
        # real exit code even if a wrapper masks it (documented false-green guard).
        cargo check --workspace --release --keep-going 2>&1 |
            Tee-Object -FilePath $LogPath
        $cargoExit = $LASTEXITCODE
        # Append the exit-code marker so the log is self-describing.
        Add-Content -LiteralPath $LogPath -Value ("EXITCODE={0}" -f $cargoExit)
        Write-Host ("BUILDFIX: cargo exited {0} (raw process code; truth is the log scan below)" -f $cargoExit)
    } finally {
        Pop-Location
    }
}

# Confirm the log exists and is non-empty before proceeding.
if (-not (Test-Path -LiteralPath $LogPath -PathType Leaf)) {
    Write-Host "BUILDFIX ERROR: log file was not produced: $LogPath"
    exit 1
}
$logSize = (Get-Item -LiteralPath $LogPath).Length
if ($logSize -eq 0) {
    Write-Host "BUILDFIX ERROR: log file is empty (cargo may have produced no output): $LogPath"
    exit 1
}

# ---------------------------------------------------------------------------
# STEP 2: Truth-gate via verify-cargo-log.ps1 (param: -LogPath).
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "BUILDFIX: scanning log with verify-cargo-log.ps1 ..."
try {
    & pwsh -File $VerifyScript -LogPath $LogPath
    $verifyExit = $LASTEXITCODE
} catch {
    Write-Host ("BUILDFIX ERROR: verify-cargo-log.ps1 threw an exception: {0}" -f $_.Exception.Message)
    exit 1
}

# verify-cargo-log.ps1 exits 0=green, 1=red, 2=usage-error.
if ($verifyExit -eq 0) {
    Write-Host ""
    Write-Host ("BUILDFIX: GREEN (log={0})" -f $LogPath)
    exit 0
}

if ($verifyExit -eq 2) {
    Write-Host ("BUILDFIX ERROR: verify-cargo-log.ps1 reported a usage error (exit 2) for log: $LogPath")
    exit 1
}

# verifyExit -eq 1: the build has errors.  Proceed to triage.

# ---------------------------------------------------------------------------
# STEP 3: Triage via merge-buildfix-triage.ps1 -Json (param: -LogPath, -Json).
# Writes JSON to stdout; we capture it and persist to triage_<n>.json.
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "BUILDFIX: running merge-buildfix-triage.ps1 -Json ..."
$triageRawJson = $null
try {
    $triageRawJson = & pwsh -File $TriageScript -LogPath $LogPath -Json
    $triageExit = $LASTEXITCODE
} catch {
    Write-Host ("BUILDFIX ERROR: merge-buildfix-triage.ps1 threw an exception: {0}" -f $_.Exception.Message)
    exit 1
}

# triage exits 0=clean, 1=errors (normal), 2=usage-error, 3=toolchain-trap.
if ($triageExit -eq 2) {
    Write-Host "BUILDFIX ERROR: merge-buildfix-triage.ps1 reported a usage error (exit 2)."
    exit 1
}
if ($triageExit -eq 3) {
    Write-Host "BUILDFIX ERROR: TOOLCHAIN TRAP detected by triage. Run cargo from codex-rs/, not repo root."
    Write-Host "  Hint: this script pushes to codex-rs/ automatically -- but if -SkipCheck was used,"
    Write-Host "  the existing log may have been produced from the wrong CWD. Re-run without -SkipCheck."
    exit 1
}

# Persist the JSON.
if ($triageRawJson) {
    $triageRawJson | Out-File -LiteralPath $TriageJson -Encoding utf8NoBOM
} else {
    # Triage produced no output (unexpected); write a minimal sentinel so downstream callers don't break.
    '{"status":"no-output","slices":[]}' | Out-File -LiteralPath $TriageJson -Encoding utf8NoBOM
}

# ---------------------------------------------------------------------------
# STEP 4: Parse the JSON and write triage_<n>.md (human summary for worker prompts).
# ---------------------------------------------------------------------------
$sliceCount = 0
try {
    $triageObj = $triageRawJson | ConvertFrom-Json -ErrorAction Stop

    # Build the markdown summary.
    $mdLines = New-Object System.Collections.Generic.List[string]
    $mdLines.Add("# Build-fix triage -- iteration $n")
    $mdLines.Add("")

    $summary = $triageObj.summary
    if ($summary) {
        $mdLines.Add(("total_errors={0}  root_causes={1}  files={2}  suggested_slices={3}" -f
            $summary.total_errors,
            $summary.distinct_root_causes,
            $summary.files_touched,
            $summary.suggested_slices))
        $mdLines.Add("")
    }

    $slices = $triageObj.slices
    if ($slices -and $slices.Count -gt 0) {
        $sliceCount = $slices.Count
        $mdLines.Add("## Slices (file-disjoint; each becomes one fix-worker prompt)")
        $mdLines.Add("")
        foreach ($sl in $slices) {
            $sliceNum  = $sl.slice
            $errCount  = $sl.error_count
            $fileList  = @($sl.files)
            $rootCauses = @($sl.root_causes)
            $rcSummary = if ($rootCauses.Count -gt 0) { $rootCauses -join ', ' } else { '<unclassified>' }
            $mdLines.Add(("### Slice {0}  ({1} error(s), {2} file(s))" -f $sliceNum, $errCount, $fileList.Count))
            $mdLines.Add(("root causes: {0}" -f $rcSummary))
            foreach ($f in $fileList) {
                $mdLines.Add(("  - {0}" -f $f))
            }
            $mdLines.Add("")
        }
    } else {
        $mdLines.Add("(no slices in JSON -- check triage_$n.json for details)")
        $sliceCount = 0
    }

    $mdLines.Add("## Notes")
    $mdLines.Add("- cfg(test) errors are DEFERRED; do not pass --tests to cargo.")
    $mdLines.Add("- cargo MUST run from codex-rs/ (toolchain CWD rule).")
    $mdLines.Add(("- Full JSON partition: {0}" -f $TriageJson))
    $mdLines.Add(("- Log: {0}" -f $LogPath))

    $mdLines | Out-File -LiteralPath $TriageMd -Encoding utf8NoBOM
} catch {
    # JSON parse failure is non-fatal -- the raw JSON is already on disk.
    Write-Host ("BUILDFIX: WARNING -- could not parse triage JSON for .md summary: {0}" -f $_.Exception.Message)
    "# Build-fix triage $n -- JSON parse failed; see triage_$n.json directly." |
        Out-File -LiteralPath $TriageMd -Encoding utf8NoBOM
}

# ---------------------------------------------------------------------------
# STEP 5: Report and exit 2 (errors present; orchestrator must dispatch workers).
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host ("BUILDFIX: {0} error-slices -> {1}" -f $sliceCount, (Split-Path $TriageJson -Leaf))
Write-Host ("  json : {0}" -f $TriageJson)
Write-Host ("  md   : {0}" -f $TriageMd)
Write-Host ("  log  : {0}" -f $LogPath)
Write-Host "BUILDFIX: Dispatch one fix-worker per slice (disjoint files), then re-run this script."
exit 2
