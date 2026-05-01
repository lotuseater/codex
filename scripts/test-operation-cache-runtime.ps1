param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,

    [string]$LogDir = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")).Path "logs"),

    [string]$CacheDb = (Join-Path $HOME ".claude\cache\tool_cache.sqlite"),

    [string]$CacheBridgePy = ""
)

$ErrorActionPreference = "Stop"

$repoRootFull = (Resolve-Path -LiteralPath $RepoRoot).Path
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

if (-not $CacheBridgePy) {
    if ($env:WIZARD_CODEX_CACHE_BRIDGE_PY) {
        $CacheBridgePy = $env:WIZARD_CODEX_CACHE_BRIDGE_PY
    } else {
        $CacheBridgePy = "C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus\src\mcp\hooks\codex_cache_bridge_cli.py"
    }
}

if (-not (Test-Path -LiteralPath $CacheDb)) {
    throw "Operation cache DB not found: $CacheDb"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$canaryDir = Join-Path $LogDir "cache-canaries"
New-Item -ItemType Directory -Force -Path $canaryDir | Out-Null
$canaryPath = Join-Path $canaryDir "operation-cache-runtime-$stamp.txt"
$canaryText = "operation-cache-runtime-$stamp " + ("x" * 240)
Set-Content -LiteralPath $canaryPath -Value $canaryText -Encoding UTF8
$missingPath = Join-Path $canaryDir "operation-cache-runtime-missing-$stamp.txt"

$run1Log = Join-Path $LogDir "operation-cache-runtime-$stamp-run1.log"
$run2Log = Join-Path $LogDir "operation-cache-runtime-$stamp-run2.log"
$failedLog = Join-Path $LogDir "operation-cache-runtime-$stamp-failed-read.log"
$canaryName = Split-Path -Leaf $canaryPath
$missingName = Split-Path -Leaf $missingPath

$python = @'
import pathlib
import sqlite3
import sys

db_path = pathlib.Path(sys.argv[1])
needle = sys.argv[2].lower()
if not db_path.exists():
    print("0 0 0")
    raise SystemExit

con = sqlite3.connect(db_path)
con.row_factory = sqlite3.Row
rows = list(
    con.execute(
        """
        select canonical_input, hit_count
        from tool_cache
        where coalesce(source_agent, '') = 'codex'
          and lower(canonical_input) like ?
        """,
        (f"%{needle}%",),
    )
)
print(
    len(rows),
    sum(int(row["hit_count"] or 0) for row in rows),
    sum(1 for row in rows if int(row["hit_count"] or 0) > 0),
)
con.close()
'@

$cleanupPython = @'
import os
import pathlib
import sqlite3
import sys

db_path = pathlib.Path(sys.argv[1])
bridge_py = pathlib.Path(sys.argv[2])
paths = [pathlib.Path(path) for path in sys.argv[3:] if path]
needles = [path.name.lower() for path in paths]
if not db_path.exists() or not paths:
    print("0 0")
    raise SystemExit

def canonical_keys() -> set[str]:
    os.environ["WIZARD_TOOL_CACHE_DIR"] = str(db_path.parent)
    hooks_dir = bridge_py.parent
    if str(hooks_dir) not in sys.path:
        sys.path.insert(0, str(hooks_dir))
    try:
        import codex_cache_bridge  # type: ignore
    except Exception:
        return set()

    keys: set[str] = set()
    for path in paths:
        event = {
            "tool_name": "shell_command",
            "tool_input": {
                "command": f"Get-Content -LiteralPath '{path}' -Raw",
            },
        }
        try:
            key = codex_cache_bridge.canonical_event_key(event)
        except Exception:
            key = None
        if key:
            keys.add(str(key))
    return keys

def key_fingerprints(keys: set[str]) -> set[str]:
    if not keys:
        return set()
    os.environ["WIZARD_TOOL_CACHE_DIR"] = str(db_path.parent)
    hooks_dir = bridge_py.parent
    if str(hooks_dir) not in sys.path:
        sys.path.insert(0, str(hooks_dir))
    try:
        import tool_cache  # type: ignore
    except Exception:
        return set()

    fingerprints: set[str] = set()
    for key in keys:
        try:
            fingerprint = tool_cache._key_fingerprint(key=key)
        except Exception:
            fingerprint = None
        if fingerprint:
            fingerprints.add(str(fingerprint))
    return fingerprints

con = sqlite3.connect(db_path)
try:
    keys = canonical_keys()
    for needle in needles:
        rows = con.execute(
            """
            select key
            from tool_cache
            where coalesce(source_agent, '') = 'codex'
              and lower(canonical_input) like ?
            """,
            (f"%{needle}%",),
        ).fetchall()
        keys.update(str(row[0]) for row in rows)

    unique_keys = sorted(set(keys))
    key_hashes = sorted(key_fingerprints(set(unique_keys)))
    deleted_cache_rows = 0
    deleted_miss_rows = 0
    for key in unique_keys:
        con.execute("delete from cache_deps where cache_key = ?", (key,))
        cursor = con.execute("delete from tool_cache where key = ?", (key,))
        deleted_cache_rows += cursor.rowcount if cursor.rowcount > 0 else 0
    for key_hash in key_hashes:
        cursor = con.execute("delete from cache_miss_reasons where key_hash = ?", (key_hash,))
        deleted_miss_rows += cursor.rowcount if cursor.rowcount > 0 else 0
    con.commit()
    print(deleted_cache_rows, deleted_miss_rows)
finally:
    con.close()
'@

$missStatsPython = @'
import os
import pathlib
import sqlite3
import sys

db_path = pathlib.Path(sys.argv[1])
bridge_py = pathlib.Path(sys.argv[2])
paths = [pathlib.Path(path) for path in sys.argv[3:] if path]
if not db_path.exists() or not paths:
    print("0 0")
    raise SystemExit

os.environ["WIZARD_TOOL_CACHE_DIR"] = str(db_path.parent)
hooks_dir = bridge_py.parent
if str(hooks_dir) not in sys.path:
    sys.path.insert(0, str(hooks_dir))

fingerprints: set[str] = set()
try:
    import codex_cache_bridge  # type: ignore
    import tool_cache  # type: ignore
    for path in paths:
        event = {
            "tool_name": "shell_command",
            "tool_input": {
                "command": f"Get-Content -LiteralPath '{path}' -Raw",
            },
        }
        key = codex_cache_bridge.canonical_event_key(event)
        if key:
            fingerprint = tool_cache._key_fingerprint(key=key)
            if fingerprint:
                fingerprints.add(str(fingerprint))
except Exception:
    fingerprints = set()

if not fingerprints:
    print("0 0")
    raise SystemExit

placeholders = ",".join("?" for _ in fingerprints)
con = sqlite3.connect(db_path)
try:
    row = con.execute(
        f"select count(*) from cache_miss_reasons where key_hash in ({placeholders})",
        sorted(fingerprints),
    ).fetchone()
    print(int((row or (0,))[0] or 0), len(fingerprints))
finally:
    con.close()
'@

function Get-CanaryCacheStats {
    param([string]$Needle)

    $result = $python | python - $CacheDb $Needle
    if ($LASTEXITCODE -ne 0) {
        throw "SQLite cache stats probe failed with exit code $LASTEXITCODE"
    }
    $parts = $result.Trim().Split(" ")
    [pscustomobject]@{
        Rows = [int]$parts[0]
        Hits = [int]$parts[1]
        HitRows = [int]$parts[2]
    }
}

function Remove-CanaryCacheRows {
    $result = $cleanupPython | python - $CacheDb $CacheBridgePy $canaryPath $missingPath
    if ($LASTEXITCODE -ne 0) {
        throw "SQLite canary cleanup failed with exit code $LASTEXITCODE"
    }
    $parts = $result.Trim().Split(" ", [System.StringSplitOptions]::RemoveEmptyEntries)
    [pscustomobject]@{
        CacheRows = [int]$parts[0]
        MissRows = [int]$parts[1]
    }
}

function Get-CanaryMissTelemetryStats {
    $result = $missStatsPython | python - $CacheDb $CacheBridgePy $canaryPath $missingPath
    if ($LASTEXITCODE -ne 0) {
        throw "SQLite canary miss telemetry probe failed with exit code $LASTEXITCODE"
    }
    $parts = $result.Trim().Split(" ", [System.StringSplitOptions]::RemoveEmptyEntries)
    [pscustomobject]@{
        Rows = [int]$parts[0]
        Fingerprints = [int]$parts[1]
    }
}

function Assert-LogMatches {
    param(
        [string]$Path,
        [string]$Pattern,
        [string]$Description
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing log for $Description`: $Path"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    if ($content -notmatch $Pattern) {
        throw "Runtime canary log did not show $Description`: $Path"
    }
}

$before = Get-CanaryCacheStats -Needle $canaryName
$missTelemetryBefore = Get-CanaryMissTelemetryStats
$prompt = "Use shell_command to run exactly this PowerShell command and return only its output: Get-Content -LiteralPath '$canaryPath' -Raw"
$failedBefore = Get-CanaryCacheStats -Needle $missingName
$failedPrompt = "Use shell_command to run exactly this PowerShell command and return only its output: Get-Content -LiteralPath '$missingPath' -Raw"

$oldNativeCommandPreference = $PSNativeCommandUseErrorActionPreference
$PSNativeCommandUseErrorActionPreference = $false
try {
    codex -a never --sandbox danger-full-access --cd $repoRootFull exec $prompt *> $run1Log
    $exit1 = $LASTEXITCODE
    $afterFirst = Get-CanaryCacheStats -Needle $canaryName

    codex -a never --sandbox danger-full-access --cd $repoRootFull exec $prompt *> $run2Log
    $exit2 = $LASTEXITCODE
    $afterSecond = Get-CanaryCacheStats -Needle $canaryName

    codex -a never --sandbox danger-full-access --cd $repoRootFull exec $failedPrompt *> $failedLog
    $failedExit = $LASTEXITCODE
    $failedAfter = Get-CanaryCacheStats -Needle $missingName
}
finally {
    $PSNativeCommandUseErrorActionPreference = $oldNativeCommandPreference
    Remove-Item -LiteralPath $canaryPath -Force -ErrorAction SilentlyContinue
    if ((Test-Path -LiteralPath $canaryDir) -and -not (Get-ChildItem -LiteralPath $canaryDir -Force)) {
        Remove-Item -LiteralPath $canaryDir -Force -ErrorAction SilentlyContinue
    }
}

$summary = [ordered]@{
    canary_path = $canaryPath
    run1_exit = $exit1
    run2_exit = $exit2
    rows_before = $before.Rows
    rows_after_first = $afterFirst.Rows
    rows_after_second = $afterSecond.Rows
    hits_before = $before.Hits
    hits_after_first = $afterFirst.Hits
    hits_after_second = $afterSecond.Hits
    hit_rows_after_second = $afterSecond.HitRows
    failed_read_exit = $failedExit
    failed_read_rows_before = $failedBefore.Rows
    failed_read_rows_after = $failedAfter.Rows
    miss_telemetry_rows_before = $missTelemetryBefore.Rows
    run1_log = $run1Log
    run2_log = $run2Log
    failed_read_log = $failedLog
}

$summary | ConvertTo-Json -Depth 4

if ($exit1 -ne 0 -or $exit2 -ne 0) {
    throw "Codex runtime cache canary failed: run1=$exit1 run2=$exit2"
}
if ($afterFirst.Rows -le $before.Rows) {
    throw "Operation cache did not store the canary read after the first run"
}
if ($afterSecond.Hits -le $afterFirst.Hits) {
    throw "Operation cache hit count did not increase after the second run"
}
Assert-LogMatches `
    -Path $run1Log `
    -Pattern "(?m)^exec\r?$" `
    -Description "the first read executing the shell tool"
Assert-LogMatches `
    -Path $run1Log `
    -Pattern ([regex]::Escape($canaryText)) `
    -Description "the first read returning the canary text"
Assert-LogMatches `
    -Path $failedLog `
    -Pattern "(?m)^exec\r?$" `
    -Description "the failed read executing the shell tool"
Assert-LogMatches `
    -Path $failedLog `
    -Pattern "exited 1" `
    -Description "the failed read exiting nonzero"
Assert-LogMatches `
    -Path $failedLog `
    -Pattern "Cannot find path" `
    -Description "the failed read reporting the missing file"
if ($failedAfter.Rows -ne $failedBefore.Rows) {
    throw "Failed cacheable read unexpectedly created an operation-cache row"
}

$missTelemetryBeforeCleanup = Get-CanaryMissTelemetryStats
$removedRows = Remove-CanaryCacheRows
$afterCleanup = Get-CanaryCacheStats -Needle $canaryName
if ($afterCleanup.Rows -ne $before.Rows) {
    throw "Runtime canary cleanup left cache rows behind: before=$($before.Rows) after=$($afterCleanup.Rows) removed=$($removedRows.CacheRows)"
}
$missTelemetryAfterCleanup = Get-CanaryMissTelemetryStats
if ($missTelemetryAfterCleanup.Rows -ne $missTelemetryBefore.Rows) {
    throw "Runtime canary cleanup left miss telemetry behind: before=$($missTelemetryBefore.Rows) before_cleanup=$($missTelemetryBeforeCleanup.Rows) after=$($missTelemetryAfterCleanup.Rows) removed=$($removedRows.MissRows)"
}
Write-Output "Removed $($removedRows.CacheRows) runtime canary cache row(s) and $($removedRows.MissRows) miss telemetry row(s)."
