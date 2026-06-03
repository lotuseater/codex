<#
.SYNOPSIS
    Gate that catches a code-motion/split that silently DROPPED a function body.

.DESCRIPTION
    When functions are extracted/moved out of one file into one or more sibling
    files, no `fn` should disappear: the UNION of `fn` names across the resulting
    (-After) files must be a SUPERSET of the `fn` names in the ORIGINAL file as it
    was before the split.

    This script:
      * reads the ORIGINAL file content at a git ref (`git show <Ref>:<Original>`)
        and extracts the set of Rust function names found there (free functions
        AND methods inside impl/trait blocks are all `fn name`, so one regex finds
        all of them),
      * reads the CURRENT working-tree content of each -After file and builds the
        UNION of their function names,
      * reports LOST functions (present in ORIGINAL, absent from the AFTER union) —
        these are the dangerous silently-dropped bodies — and exits 1 if any,
      * reports ADDED functions (in AFTER, not in ORIGINAL) as informational.

    The `fn`-name regex tolerates visibility / async / const / unsafe / extern and
    generics, e.g. `pub(crate) async unsafe fn foo<T>(...)` -> `foo`. It also skips
    `fn` text inside line comments and string/char literals so doc examples and
    messages do not produce phantom names.

    Known false positive: a function that was legitimately RENAMED or MERGED into
    another shows up as BOTH lost and added. The script prints a hint to eyeball
    any name that looks like a rename of an added one before trusting the gate.

    -Original may be the SAME path as one of -After (the common case: a file kept
    most of its functions and extracted the rest into a sibling). The comparison is
    always ORIGINAL-at-ref  vs  UNION-of-After-now.

    Read-only. Never runs cargo/builds or mutating git. Exit 0 when nothing lost.

.PARAMETER Ref
    Git ref at which to read the ORIGINAL file (e.g. HEAD~5, a commit sha, a branch).

.PARAMETER Original
    Repo-relative path of the file as it existed at -Ref (before the split).

.PARAMETER After
    One or more repo-relative paths whose CURRENT working-tree content should
    collectively contain every function the original had.

.PARAMETER RepoRoot
    Repo root. Defaults to the parent of this script's directory so it works from
    anywhere.

.EXAMPLE
    pwsh -File scripts/split-completeness-check.ps1 -Ref 85d9e93d5c~1 `
        -Original codex-rs/core/src/session/turn.rs `
        -After codex-rs/core/src/session/turn.rs,codex-rs/core/src/session/context_budget_adapter.rs
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Ref,

    [Parameter(Mandatory = $true)]
    [string]$Original,

    [Parameter(Mandatory = $true)]
    [string[]]$After,

    [string]$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

function Invoke-Git {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$GitArgs)
    & git -C $RepoRoot @GitArgs
}

# Extract the set of Rust `fn` names from a block of source text.
# Strips line comments and string/char literals first so `fn` inside a comment or
# a message string does not create a phantom name. Captures the identifier after
# `fn`, tolerating any leading qualifiers (pub/async/const/unsafe/extern "C"/...)
# because the regex only anchors on the `fn` keyword itself.
function Get-FnNames {
    param([string[]]$Lines)
    $names = [System.Collections.Generic.HashSet[string]]::new()
    if ($null -eq $Lines) { return $names }
    foreach ($raw in $Lines) {
        if ($null -eq $raw) { continue }
        $line = $raw
        # Drop // line comments (best-effort; good enough for fn detection).
        $ci = $line.IndexOf("//")
        if ($ci -ge 0) { $line = $line.Substring(0, $ci) }
        # Blank out double-quoted string and single-quoted char literals so an
        # `fn` inside them is not matched.
        $line = [regex]::Replace($line, '"(?:\\.|[^"\\])*"', '""')
        $line = [regex]::Replace($line, "'(?:\\.|[^'\\])'", "''")
        foreach ($m in [regex]::Matches($line, '\bfn\s+([a-zA-Z_][a-zA-Z0-9_]*)')) {
            [void]$names.Add($m.Groups[1].Value)
        }
    }
    return $names
}

# --- Read ORIGINAL at the ref ---
$origText = Invoke-Git show "${Ref}:${Original}"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Could not read '$Original' at ref '$Ref' (git show exit $LASTEXITCODE)."
    exit 2
}
$originalSet = Get-FnNames -Lines $origText

# --- Build UNION of AFTER (current working-tree content) ---
# When invoked via `pwsh -File ... -After a,b`, the comma list arrives as a SINGLE
# string element rather than an array, so split any element that itself contains
# commas. (Native array binding still works for dot-sourced / direct calls.)
$afterPaths = @($After | ForEach-Object { $_ -split ',' })
$afterSet = [System.Collections.Generic.HashSet[string]]::new()
$afterReadCount = 0
foreach ($rel in $afterPaths) {
    if ([string]::IsNullOrWhiteSpace($rel)) { continue }
    $rel = $rel.Trim()
    $full = if ([System.IO.Path]::IsPathRooted($rel)) { $rel } else { Join-Path $RepoRoot $rel }
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
        Write-Error "After file not found in working tree: $rel"
        exit 2
    }
    $afterReadCount++
    $afterLines = Get-Content -LiteralPath $full
    $fns = Get-FnNames -Lines $afterLines
    foreach ($n in $fns) { [void]$afterSet.Add($n) }
}
if ($afterReadCount -eq 0) {
    Write-Error "No valid -After files were provided."
    exit 2
}

# --- Diff the sets ---
$lost = [System.Collections.Generic.List[string]]::new()
foreach ($n in $originalSet) {
    if (-not $afterSet.Contains($n)) { [void]$lost.Add($n) }
}
$added = [System.Collections.Generic.List[string]]::new()
foreach ($n in $afterSet) {
    if (-not $originalSet.Contains($n)) { [void]$added.Add($n) }
}
$lostSorted = @($lost | Sort-Object)
$addedSorted = @($added | Sort-Object)

# --- Report ---
Write-Host "# Split completeness check"
Write-Host ""
Write-Host "ORIGINAL : ${Ref}:${Original}  (fn count: $($originalSet.Count))"
Write-Host "AFTER    : union of $($afterPaths -join ', ')  (fn count: $($afterSet.Count))"
Write-Host ""

if ($lostSorted.Count -eq 0) {
    Write-Host "LOST functions (in ORIGINAL, missing from AFTER union): none" -ForegroundColor Green
} else {
    Write-Host "LOST functions (in ORIGINAL, missing from AFTER union): $($lostSorted.Count)" -ForegroundColor Red
    foreach ($n in $lostSorted) { Write-Host "  - $n" -ForegroundColor Red }
}
Write-Host ""

if ($addedSorted.Count -eq 0) {
    Write-Host "ADDED functions (in AFTER, not in ORIGINAL): none"
} else {
    Write-Host "ADDED functions (in AFTER, not in ORIGINAL): $($addedSorted.Count) [informational]"
    foreach ($n in $addedSorted) { Write-Host "  + $n" }
}
Write-Host ""

if ($lostSorted.Count -gt 0 -and $addedSorted.Count -gt 0) {
    Write-Host ("HINT: when BOTH lists are non-empty, a 'lost' name may simply have been " +
        "RENAMED into an 'added' one (or merged into it). Eyeball the pairs above before " +
        "treating this as a real drop — that is the known false positive.") -ForegroundColor Yellow
    Write-Host ""
}

if ($lostSorted.Count -gt 0) {
    Write-Host "GATE: $($lostSorted.Count) function(s) lost in the split -> exit 1" -ForegroundColor Red
    exit 1
}
Write-Host "GATE: no functions lost -> exit 0" -ForegroundColor Green
exit 0
