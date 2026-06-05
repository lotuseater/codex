<#
.SYNOPSIS
    Triage a post-merge `cargo check` log: group rustc errors by ROOT CAUSE
    (missing symbol + error-code) and by FILE, then suggest a file-disjoint
    fix-slice partition that clusters each chokepoint with its call sites.

.DESCRIPTION
    This automates the MECHANICAL half of the post-merge "Phase D" compile-error
    loop documented in .codex/tmp/merge_buildfix_lessons.md. After an upstream
    merge, `cargo check --workspace --release --keep-going` surfaces errors in
    WAVES, and the dominant failure pattern is: upstream dropped/renamed a member
    (field / method / variant / fn / import) but its USAGE survived, so one root
    cause explodes into many call-site errors across many files.

    Given a cargo log, this script:
      1. Parses every `error[EXXXX]: <msg>` (and bare `error: <msg>`) plus the
         FIRST following `--> <file>:<line>:<col>` location.
      2. Extracts the offending SYMBOL from the message via the regexes for the
         patterns the lessons doc lists (no field / no method / cannot find /
         unresolved import / not a member of trait / has no field named / takes N
         arguments / missing field / no variant / cannot find type|value|fn ...).
      3. Infers the owning CRATE from the file path (`codex-rs\<crate>\src\...`
         or the leading `<crate>\src\...` segment; `core\src` -> codex-core).
      4. Groups BY ROOT CAUSE (symbol+code -> count + distinct files) and BY FILE
         (file -> its errors), printing both as tables.
      5. Suggests a STARTING-POINT file-disjoint partition: each erroring file is
         assigned to exactly ONE slice; files sharing a dominant symbol cluster
         into the same slice so a chokepoint + its call sites land together. This
         is a hint for a recon worker / human, NOT gospel.

    Special-case banners (instead of a misleading triage):
      * TOOLCHAIN TRAP  (exit 3) - the log is the rust-toolchain-CWD bail
        ("requires rustc 1.9x" / "is not supported by the following package").
        Fix is operational: run cargo from inside codex-rs/, not the repo root.
      * FALSE-GREEN warn       - no `EXITCODE=0` line AND zero parsed errors:
        the log may be truncated / cargo masked; check EXITCODE before trusting.
      * CLEAN  (exit 0)        - zero errors and an explicit `EXITCODE=0`.

    Exit codes:
        0  clean (no errors, EXITCODE=0)
        1  errors found (normal triage)
        2  -LogPath missing / unreadable (usage error)
        3  toolchain-CWD trap detected (not a code error)

    Pairs with scripts/verify-cargo-log.ps1 (the pass/FAIL truth-gate). Run the
    gate first to know IF the build failed; run THIS to know WHAT to fix and HOW
    to slice it. Because the loop uses `--keep-going`, each pass reveals errors in
    a NEW wave (a crate whose dep failed is skipped) - so re-run this every
    iteration on the fresh log; the partition is per-wave.

    Pure PowerShell: no external dependencies (regex + hashtables only). Read-only;
    never runs cargo or git.

.PARAMETER LogPath
    Path to the cargo check/build log to triage. Required. Relative paths resolve
    against the caller's current directory.

.PARAMETER Json
    Emit a machine-readable JSON object (to stdout) instead of the human tables, so
    a recon worker can consume the breakdown + suggested partition programmatically.

.PARAMETER Top
    Limit the number of groups shown in each table (root-cause and file). Default 40.
    Does not affect the JSON payload (which is always complete) or the partition.

.EXAMPLE
    pwsh -File scripts/merge-buildfix-triage.ps1 -LogPath logs/merge-check-release-iter7.log
    # human triage: summary header + by-root-cause table + by-file table + slices.

.EXAMPLE
    pwsh -File scripts/merge-buildfix-triage.ps1 -LogPath logs/merge-check-release-iter7.log -Json
    # JSON for a recon worker: { summary, root_causes[], files[], slices[], unclassified[] }.

.EXAMPLE
    pwsh -File scripts/merge-buildfix-triage.ps1 -LogPath logs/iter8.log -Top 15
    # show only the top 15 root-cause groups and top 15 files.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$LogPath,

    [switch]$Json,

    [int]$Top = 40
)

$ErrorActionPreference = "Stop"

# --- Resolve + validate the log path (exit 2 on any usage problem). ---
# Mirror verify-cargo-log.ps1: emit usage errors WITHOUT throwing so the explicit
# `exit 2` stands even under $ErrorActionPreference='Stop'.
$resolved = $null
try {
    $resolved = (Resolve-Path -LiteralPath $LogPath -ErrorAction Stop).Path
} catch {
    $host.UI.WriteErrorLine("merge-buildfix-triage: log not found: '$LogPath'")
    exit 2
}
if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
    $host.UI.WriteErrorLine("merge-buildfix-triage: not a file: '$resolved'")
    exit 2
}

# ---------------------------------------------------------------------------
# Symbol extraction. Each entry: { Code (label) ; Regex with a 'sym' capture }.
# Ordered most-specific first; first match wins. Patterns mirror the lessons doc
# "DOMINANT FAILURE PATTERN" variants. The optional [`'] alternation tolerates
# either backtick-quoted (rustc default) or plain-quoted symbols.
# ---------------------------------------------------------------------------
$symbolRules = @(
    # struct `...` has no field named `X`         (E0560)
    @{ Kind = 'no-field-named'; Regex = "has no field named [``'](?<sym>[^``']+)[``']" }
    # no field `X` on type ...                    (E0609)
    @{ Kind = 'no-field'; Regex = "no field [``'](?<sym>[^``']+)[``'] on type" }
    # no method named `X` found ...               (E0599)
    @{ Kind = 'no-method'; Regex = "no method named [``'](?<sym>[^``']+)[``']" }
    # no function or associated item named `X` ... (E0599, associated-fn form)
    @{ Kind = 'no-assoc-item'; Regex = "no (?:function|associated item|associated function|associated constant) (?:or associated item )?named [``'](?<sym>[^``']+)[``']" }
    # no associated item / no variant ... named `X` (E0599 variant form)
    @{ Kind = 'no-variant'; Regex = "no variant(?:\s+or associated item)?[^``']*named [``'](?<sym>[^``']+)[``']" }
    # method `X` is not a member of trait `Y`     (E0407)  -> sym = method, extra = trait
    @{ Kind = 'not-in-trait'; Regex = "method [``'](?<sym>[^``']+)[``'] is not a member of trait [``'](?<trait>[^``']+)[``']" }
    # `X` is not a member of trait `Y`            (generic not-in-trait)
    @{ Kind = 'not-in-trait'; Regex = "[``'](?<sym>[^``']+)[``'] is not a member of trait [``'](?<trait>[^``']+)[``']" }
    # unresolved import `a::b::X`                  (E0432) -> sym = LAST path segment
    @{ Kind = 'unresolved-import'; Regex = "unresolved import [``'](?<sym>[^``']+)[``']" }
    # missing field `X` in initializer ...        (E0063)
    @{ Kind = 'missing-field'; Regex = "missing field [``'](?<sym>[^``']+)[``'] in initializer" }
    # non-exhaustive patterns: `X` not covered    (E0004) -> sym = uncovered variant
    @{ Kind = 'non-exhaustive'; Regex = "non-exhaustive patterns: [``'](?<sym>[^``']+)[``'] not covered" }
    # cannot find value|type|function|... `X` in this scope|in `...`  (E0425/E0433/E0412/E0531...)
    @{ Kind = 'cannot-find'; Regex = "cannot find (?:value|type|function|macro|trait|attribute|derive macro|item|method|variant or associated item) [``'](?<sym>[^``']+)[``']" }
    # could not find `X` in `...`                  (sub-message of E0432, fallback)
    @{ Kind = 'cannot-find'; Regex = "could not find [``'](?<sym>[^``']+)[``'] in" }
    # no `X` in `...` / no `X` in the root         (E0432 sub-message)
    @{ Kind = 'cannot-find'; Regex = "\bno [``'](?<sym>[^``']+)[``'] in (?:the root|[``'])" }
)

# Arity-mismatch errors have no named symbol; bucket them by a synthetic label so
# they still cluster (they usually trace to ONE changed fn/method signature).
$reArity = "this (?<what>function|method) takes (?<exp>\d+) arguments? but (?<got>\d+) (?:argument|were|was)"
# A generic trailing "X has a `&self` declaration ..." (E0186) carries the method name.
$reSelfDecl = "method [``'](?<sym>[^``']+)[``'] has a [``']&?(?:mut )?self[``'] declaration"

# Special-case detectors.
$reToolchain = "is not supported by the following package|requires rustc 1\.9|rust-toolchain|can't find crate for [``']std[``']|toolchain '[^']*' is not installed"
$reExitCodeZero = "EXITCODE=0\b"
$reExitCodeAny = "EXITCODE=(?<n>\d+)"

# ---------------------------------------------------------------------------
# Crate inference from a file path. Handles both `codex-rs\<crate>\src\...` and the
# fork-log form `<crate>\src\...` (no codex-rs prefix). `core` -> codex-core etc.
# ---------------------------------------------------------------------------
function Get-CrateFromPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $null }
    $p = $Path -replace '/', '\'
    # codex-rs\<crate>\...  (prefer the segment after codex-rs)
    if ($p -match 'codex-rs\\([A-Za-z0-9_\-]+)\\') {
        return (Normalize-CrateName $Matches[1])
    }
    # leading <crate>\src\... or <crate>\<...>\src
    if ($p -match '(?:^|\\)([a-z][a-z0-9_\-]*)\\src\\') {
        return (Normalize-CrateName $Matches[1])
    }
    # bare first segment fallback
    if ($p -match '^([a-z][a-z0-9_\-]+)\\') {
        return (Normalize-CrateName $Matches[1])
    }
    return $null
}

function Normalize-CrateName {
    param([string]$Dir)
    if ([string]::IsNullOrWhiteSpace($Dir)) { return $null }
    # Crate dirs under codex-rs/ are bare (core, tui, protocol); the cargo crate
    # name is codex-<dir> for most. Keep the dir name but tag the common one.
    switch ($Dir) {
        'core' { return 'core' }
        default { return $Dir }
    }
}

# Pull the symbol (and optional trait/extra) out of a single error message.
function Resolve-Symbol {
    param([string]$Message)

    foreach ($rule in $symbolRules) {
        if ($Message -match $rule.Regex) {
            $sym = $Matches['sym']
            # unresolved import: keep the LAST path segment as the symbol; that is
            # the actually-missing item (`crate::tools::tool_search_entry` -> tool_search_entry).
            if ($rule.Kind -eq 'unresolved-import' -and $sym -match '::') {
                $sym = ($sym -split '::')[-1]
            }
            $extra = $null
            if ($Matches.ContainsKey('trait')) { $extra = $Matches['trait'] }
            return [pscustomobject]@{ Symbol = $sym; Kind = $rule.Kind; Extra = $extra }
        }
    }
    if ($Message -match $reSelfDecl) {
        return [pscustomobject]@{ Symbol = $Matches['sym']; Kind = 'self-decl'; Extra = $null }
    }
    if ($Message -match $reArity) {
        # Synthetic, stable label so all "takes N args" cluster by arity shape.
        $label = "<arity {0}->{1}>" -f $Matches['exp'], $Matches['got']
        return [pscustomobject]@{ Symbol = $label; Kind = 'arity'; Extra = $Matches['what'] }
    }
    return $null
}

# ---------------------------------------------------------------------------
# Parse the log into error records. State machine: when we see an error header,
# remember it; the FIRST following `-->` location fills file/line/col, then we
# emit the record. A new error header before a `-->` flushes the previous header
# with an unknown location (rare; e.g. errors that print no span).
# ---------------------------------------------------------------------------
# A REAL error header is either at column 0 (`error[E0432]:` / `error:`) OR prefixed
# by a diagnostic PATH in the fork-log form (`core\..rs: error[E0432]:`). It must NOT
# match an `error:`/`error[` substring inside a rustc SOURCE SNIPPET (those lines start
# with the gutter `<digits> |`, `   |`, or `   = `, and the L406 `pub(crate) error:
# TurnCodexError,` false-positive proved a loose `.*?error:` pattern over-matches).
$reErrorHeader = "^(?:[^|=]*?\.rs:\s*)?error(?:\[(?<code>E\d{4})\])?:\s*(?<msg>.+?)\s*$"
# Reject rustc gutter / snippet lines outright (defense in depth against snippet matches).
$reGutter = "^\s*(?:\d+\s*\||\||=|\.\.\.)"
# Exclude the cargo summary lines that also start with `error:` but are not triagable.
$reSummaryNoise = "^error:\s*(?:could not compile|aborting due to|build failed)"
$reLoc = "^\s*-->\s*(?<file>[^:]+):(?<line>\d+):(?<col>\d+)\s*$"

$records = New-Object System.Collections.Generic.List[object]
$unclassified = New-Object System.Collections.Generic.List[object]
$toolchainHit = $false
$exitCode = $null
$lineNo = 0
$pending = $null   # error header awaiting its --> location

function Flush-Pending {
    param($Pending, [string]$File, [int]$Line, [int]$Col)
    if ($null -eq $Pending) { return }
    $resv = Resolve-Symbol -Message $Pending.Msg
    $crate = if ($File) { Get-CrateFromPath -Path $File } else { $null }
    $rec = [pscustomobject]@{
        Code    = $Pending.Code
        Message = $Pending.Msg
        Symbol  = if ($resv) { $resv.Symbol } else { $null }
        Kind    = if ($resv) { $resv.Kind } else { 'unclassified' }
        Extra   = if ($resv) { $resv.Extra } else { $null }
        File    = $File
        Line    = $Line
        Col     = $Col
        Crate   = $crate
        HdrLine = $Pending.HdrLine
    }
    $records.Add($rec)
    if (-not $resv) {
        $unclassified.Add([pscustomobject]@{ Code = $Pending.Code; Message = $Pending.Msg; File = $File; Line = $Pending.HdrLine })
    }
}

Get-Content -LiteralPath $resolved -ReadCount 512 | ForEach-Object {
    foreach ($line in $_) {
        $lineNo++

        if (-not $toolchainHit -and ($line -match $reToolchain)) { $toolchainHit = $true }

        if ($line -match $reExitCodeAny) { $exitCode = [int]$Matches['n'] }

        # Error header? (a gutter/snippet line can never be one)
        if (($line -notmatch $reGutter) -and ($line -match $reErrorHeader) -and ($line -notmatch $reSummaryNoise)) {
            # A header arriving while one is pending means the previous had no span.
            if ($null -ne $pending) {
                Flush-Pending -Pending $pending -File $null -Line 0 -Col 0
            }
            $pending = [pscustomobject]@{
                Code    = $Matches['code']
                Msg     = $Matches['msg']
                HdrLine = $lineNo
            }
            continue
        }

        # Location line that closes the pending header.
        if ($null -ne $pending -and ($line -match $reLoc)) {
            Flush-Pending -Pending $pending -File ($Matches['file'].Trim()) -Line ([int]$Matches['line']) -Col ([int]$Matches['col'])
            $pending = $null
            continue
        }
    }
}
# Trailing header with no span.
if ($null -ne $pending) {
    Flush-Pending -Pending $pending -File $null -Line 0 -Col 0
    $pending = $null
}

$totalErrors = $records.Count

# ---------------------------------------------------------------------------
# SPECIAL CASES (decide before rendering normal triage).
# ---------------------------------------------------------------------------

# Toolchain-CWD trap: the rust-toolchain signature is present AND the log is short.
# The real bail IS an `error:` line ("...requires rustc 1.94.0"), so do NOT gate on a
# zero error count - gate on the signature + brevity (a genuine code-error wave is long
# and has many `-->` spans; the toolchain bail has ~none).
$isShortLog = ($lineNo -le 60)
$distinctSpanFiles = @($records | Where-Object { $_.File } | ForEach-Object { $_.File } | Sort-Object -Unique)
$fewSpans = ($distinctSpanFiles.Count -le 1)
if ($toolchainHit -and $isShortLog -and $fewSpans) {
    if ($Json) {
        [pscustomobject]@{
            status = 'toolchain-trap'
            log    = $resolved
            message = 'rust-toolchain CWD trap: run cargo from inside codex-rs/, not the repo root (rust-toolchain.toml pins 1.95.0). Not a code error.'
        } | ConvertTo-Json -Depth 4
    } else {
        Write-Host ""
        Write-Host "================ TOOLCHAIN TRAP ================" -ForegroundColor Yellow
        Write-Host "This log is the rust-toolchain CWD bail, NOT a code error." -ForegroundColor Yellow
        Write-Host "Run cargo from INSIDE codex-rs/, not the repo root" -ForegroundColor Yellow
        Write-Host "(codex-rs/rust-toolchain.toml pins channel 1.95.0; toolchain selection follows CWD, not --manifest-path)." -ForegroundColor Yellow
        Write-Host "===============================================" -ForegroundColor Yellow
    }
    exit 3
}

# Clean: zero errors and an explicit EXITCODE=0.
if ($totalErrors -eq 0 -and $null -ne $exitCode -and $exitCode -eq 0) {
    if ($Json) {
        [pscustomobject]@{ status = 'clean'; log = $resolved; exit_code = $exitCode } | ConvertTo-Json -Depth 3
    } else {
        Write-Host "CLEAN: no rustc errors parsed and EXITCODE=0. Nothing to triage." -ForegroundColor Green
    }
    exit 0
}

# False-green: zero parsed errors but NO EXITCODE=0 line either -> log may be truncated/masked.
$falseGreenWarn = ($totalErrors -eq 0 -and -not ($null -ne $exitCode -and $exitCode -eq 0))

# ---------------------------------------------------------------------------
# GROUPING.
#   Root cause = (Symbol, Code). Count + distinct files.
#   By file    = file -> its error records.
# ---------------------------------------------------------------------------
$byRoot = @{}    # key "symbol||code" -> aggregate
foreach ($r in $records) {
    $sym = if ($r.Symbol) { $r.Symbol } else { '<unclassified>' }
    $key = "$sym||$($r.Code)"
    if (-not $byRoot.ContainsKey($key)) {
        $byRoot[$key] = [pscustomobject]@{
            Symbol = $sym
            Code   = $r.Code
            Kind   = $r.Kind
            Count  = 0
            Files  = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
            Crates = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
            Sample = $r.Message
        }
    }
    $g = $byRoot[$key]
    $g.Count++
    if ($r.File) { [void]$g.Files.Add($r.File) }
    if ($r.Crate) { [void]$g.Crates.Add($r.Crate) }
}
$rootGroups = @($byRoot.Values | Sort-Object @{ E = { $_.Count }; Descending = $true }, @{ E = { $_.Symbol } })

$byFile = @{}
foreach ($r in $records) {
    $f = if ($r.File) { $r.File } else { '<no-span>' }
    if (-not $byFile.ContainsKey($f)) {
        $byFile[$f] = [pscustomobject]@{
            File    = $f
            Crate   = $r.Crate
            Count   = 0
            Symbols = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
            Codes   = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
        }
    }
    $fg = $byFile[$f]
    $fg.Count++
    if ($r.Symbol) { [void]$fg.Symbols.Add($r.Symbol) }
    if ($r.Code) { [void]$fg.Codes.Add($r.Code) }
}
$fileGroups = @($byFile.Values | Sort-Object @{ E = { $_.Count }; Descending = $true }, @{ E = { $_.File } })

$distinctCrates = @($records | Where-Object { $_.Crate } | ForEach-Object { $_.Crate } | Sort-Object -Unique)

# ---------------------------------------------------------------------------
# FILE-DISJOINT PARTITION (starting point).
#   Each file -> exactly one slice. Cluster files that share a DOMINANT symbol so
#   a chokepoint + its call sites land together. Approach: union-find over files,
#   joined when they share a symbol. Each connected component becomes a slice.
#   This keeps a "one dropped member -> many call sites" cluster intact.
# ---------------------------------------------------------------------------
# Map symbol -> set of files carrying it.
$symToFiles = @{}
foreach ($r in $records) {
    if (-not $r.File -or -not $r.Symbol) { continue }
    if (-not $symToFiles.ContainsKey($r.Symbol)) {
        $symToFiles[$r.Symbol] = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    }
    [void]$symToFiles[$r.Symbol].Add($r.File)
}

# Union-find over the set of erroring files.
$allFiles = @($records | Where-Object { $_.File } | ForEach-Object { $_.File } | Sort-Object -Unique)
$parent = @{}
foreach ($f in $allFiles) { $parent[$f] = $f }
function Find-Root {
    param([string]$X)
    $root = $X
    while ($parent[$root] -ne $root) { $root = $parent[$root] }
    # path compression
    $cur = $X
    while ($parent[$cur] -ne $root) { $n = $parent[$cur]; $parent[$cur] = $root; $cur = $n }
    return $root
}
function Union-Files {
    param([string]$A, [string]$B)
    $ra = Find-Root $A; $rb = Find-Root $B
    if ($ra -ne $rb) { $parent[$ra] = $rb }
}
foreach ($sym in $symToFiles.Keys) {
    $files = @($symToFiles[$sym])
    for ($i = 1; $i -lt $files.Count; $i++) { Union-Files $files[0] $files[$i] }
}

# Gather components.
$components = @{}
foreach ($f in $allFiles) {
    $root = Find-Root $f
    if (-not $components.ContainsKey($root)) {
        $components[$root] = New-Object System.Collections.Generic.List[string]
    }
    $components[$root].Add($f)
}

# Build slice objects with their root-cause symbols + total error counts; sort by load.
$fileErrorCount = @{}
foreach ($r in $records) { if ($r.File) { $fileErrorCount[$r.File] = [int]($fileErrorCount[$r.File]) + 1 } }
$fileSymbols = @{}
foreach ($r in $records) {
    if (-not $r.File -or -not $r.Symbol) { continue }
    if (-not $fileSymbols.ContainsKey($r.File)) {
        $fileSymbols[$r.File] = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    }
    [void]$fileSymbols[$r.File].Add($r.Symbol)
}

$sliceList = New-Object System.Collections.Generic.List[object]
foreach ($root in $components.Keys) {
    $files = @($components[$root] | Sort-Object)
    $syms = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $errs = 0
    foreach ($f in $files) {
        $errs += [int]$fileErrorCount[$f]
        if ($fileSymbols.ContainsKey($f)) {
            foreach ($s in $fileSymbols[$f]) { [void]$syms.Add($s) }
        }
    }
    $sliceList.Add([pscustomobject]@{
        Files      = $files
        Symbols    = @($syms | Sort-Object)
        ErrorCount = $errs
    })
}
$slices = @($sliceList | Sort-Object @{ E = { $_.ErrorCount }; Descending = $true })

# ---------------------------------------------------------------------------
# OUTPUT.
# ---------------------------------------------------------------------------
if ($Json) {
    $payload = [pscustomobject]@{
        status = if ($falseGreenWarn) { 'false-green-warning' } else { 'errors' }
        log    = $resolved
        exit_code = $exitCode
        summary = [pscustomobject]@{
            total_errors        = $totalErrors
            distinct_root_causes = $rootGroups.Count
            crates_touched      = $distinctCrates
            files_touched       = $allFiles.Count
            unclassified        = $unclassified.Count
            suggested_slices    = $slices.Count
        }
        root_causes = @($rootGroups | ForEach-Object {
            [pscustomobject]@{
                symbol = $_.Symbol
                code   = $_.Code
                kind   = $_.Kind
                count  = $_.Count
                files  = @($_.Files | Sort-Object)
                crates = @($_.Crates | Sort-Object)
                sample_message = $_.Sample
            }
        })
        files = @($fileGroups | ForEach-Object {
            [pscustomobject]@{
                file    = $_.File
                crate   = $_.Crate
                count   = $_.Count
                symbols = @($_.Symbols | Sort-Object)
                codes   = @($_.Codes | Sort-Object)
            }
        })
        slices = @(for ($i = 0; $i -lt $slices.Count; $i++) {
            [pscustomobject]@{
                slice       = ($i + 1)
                files       = $slices[$i].Files
                root_causes = $slices[$i].Symbols
                error_count = $slices[$i].ErrorCount
            }
        })
        unclassified = @($unclassified | ForEach-Object {
            [pscustomobject]@{ code = $_.Code; message = $_.Message; file = $_.File; line = $_.Line }
        })
    }
    $payload | ConvertTo-Json -Depth 8
    if ($falseGreenWarn) { exit 1 }
    if ($totalErrors -eq 0) { exit 0 }
    exit 1
}

# --- Human report ---
Write-Host ""
if ($falseGreenWarn) {
    Write-Host "WARNING: 0 errors parsed AND no EXITCODE=0 line found." -ForegroundColor Yellow
    Write-Host "         The log may be truncated or cargo's failure was masked (false-green risk)." -ForegroundColor Yellow
    Write-Host "         Verify with scripts/verify-cargo-log.ps1 and check the EXITCODE marker." -ForegroundColor Yellow
    Write-Host ""
    exit 1
}

$crateSummary = if ($distinctCrates.Count -gt 0) { $distinctCrates -join ', ' } else { '<none inferred>' }
Write-Host ("merge-buildfix-triage: {0} error(s) | {1} distinct root cause(s) | {2} file(s) | crates: {3}" -f `
        $totalErrors, $rootGroups.Count, $allFiles.Count, $crateSummary)
if ($unclassified.Count -gt 0) {
    Write-Host ("  ({0} error(s) had no symbol extracted - see UNCLASSIFIED below)" -f $unclassified.Count) -ForegroundColor DarkYellow
}
if ($null -ne $exitCode) { Write-Host ("  log EXITCODE={0}" -f $exitCode) }
Write-Host ""

# --- Table 1: by root cause ---
Write-Host "==== BY ROOT CAUSE (symbol + code -> count, distinct files) ====" -ForegroundColor Cyan
$shownRoots = $rootGroups | Select-Object -First $Top
$rootTable = $shownRoots | ForEach-Object {
    [pscustomobject]@{
        Count  = $_.Count
        Code   = if ($_.Code) { $_.Code } else { '-' }
        Symbol = $_.Symbol
        Kind   = $_.Kind
        Files  = $_.Files.Count
        Crates = (@($_.Crates | Sort-Object) -join ',')
    }
}
$rootTable | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
if ($rootGroups.Count -gt $Top) {
    Write-Host ("  ... and {0} more root-cause group(s) (raise -Top to see them)." -f ($rootGroups.Count - $Top))
    Write-Host ""
}

# --- Table 2: by file ---
Write-Host "==== BY FILE (file -> error count, symbols) ====" -ForegroundColor Cyan
$shownFiles = $fileGroups | Select-Object -First $Top
$fileTable = $shownFiles | ForEach-Object {
    $symList = @($_.Symbols | Sort-Object)
    $symStr = if ($symList.Count -gt 4) { (($symList | Select-Object -First 4) -join ',') + ",+$($symList.Count - 4)" } else { $symList -join ',' }
    [pscustomobject]@{
        Errs    = $_.Count
        Crate   = if ($_.Crate) { $_.Crate } else { '-' }
        File    = $_.File
        Symbols = $symStr
    }
}
$fileTable | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
if ($fileGroups.Count -gt $Top) {
    Write-Host ("  ... and {0} more file(s) (raise -Top to see them)." -f ($fileGroups.Count - $Top))
    Write-Host ""
}

# --- Suggested partition ---
Write-Host "==== SUGGESTED FILE-DISJOINT PARTITION (starting point, NOT gospel) ====" -ForegroundColor Cyan
Write-Host "Each file is in exactly ONE slice; files sharing a dominant symbol are clustered" -ForegroundColor DarkGray
Write-Host "so a chokepoint + its call sites land together. A recon worker should still make" -ForegroundColor DarkGray
Write-Host "the per-root-cause RESTORE / ADOPT-RENAME / REMOVE decision once, globally." -ForegroundColor DarkGray
Write-Host ""
for ($i = 0; $i -lt $slices.Count; $i++) {
    $s = $slices[$i]
    Write-Host ("Slice {0}  ({1} error(s), {2} file(s))" -f ($i + 1), $s.ErrorCount, $s.Files.Count) -ForegroundColor White
    Write-Host ("  root causes: {0}" -f ($s.Symbols -join ', '))
    foreach ($f in $s.Files) {
        Write-Host ("    - {0}  [{1} err]" -f $f, [int]$fileErrorCount[$f])
    }
    Write-Host ""
}

# --- Unclassified coverage gap ---
if ($unclassified.Count -gt 0) {
    Write-Host "==== UNCLASSIFIED (no symbol extracted - coverage gap) ====" -ForegroundColor DarkYellow
    foreach ($u in ($unclassified | Select-Object -First $Top)) {
        $codeStr = if ($u.Code) { $u.Code } else { '-' }
        Write-Host ("  L{0} [{1}] {2}  ({3})" -f $u.Line, $codeStr, $u.Message, $u.File)
    }
    if ($unclassified.Count -gt $Top) {
        Write-Host ("  ... and {0} more." -f ($unclassified.Count - $Top))
    }
    Write-Host ""
}

exit 1
