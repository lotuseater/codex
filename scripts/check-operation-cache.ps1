param(
    [string]$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

$projectRootFull = (Resolve-Path -LiteralPath $ProjectRoot).Path
$codexCacheDb = Join-Path $env:USERPROFILE ".codex\cache\tool_cache.sqlite"
$claudeCacheDb = Join-Path $env:USERPROFILE ".claude\cache\tool_cache.sqlite"
$codexAppsToolsDir = Join-Path $env:USERPROFILE ".codex\cache\codex_apps_tools"
$firstMovesDb = Join-Path $projectRootFull ".first_moves.db"
$wrapperEnv = Join-Path $env:USERPROFILE ".codex\system-wrapper\system.codex-wrapper.env.json"

Write-Output "Project root: $projectRootFull"
Write-Output ""

if (Test-Path -LiteralPath $wrapperEnv) {
    $envJson = Get-Content -LiteralPath $wrapperEnv -Raw | ConvertFrom-Json
    Write-Output "Wrapper real exe: $($envJson.WIZARD_CODEX_REAL_EXE)"
    Write-Output "Wrapper stop hooks: $($envJson.WIZARD_CODEX_STOP_HOOKS)"
    Write-Output "Wrapper config: $($envJson.WIZARD_CODEX_CONFIG)"
    Write-Output "Wrapper operation cache: $($envJson.WIZARD_CODEX_OPERATION_CACHE)"
    Write-Output "Wrapper cache DB dir: $($envJson.WIZARD_TOOL_CACHE_DIR)"
    Write-Output "Wrapper cache bridge: $($envJson.WIZARD_CODEX_CACHE_BRIDGE_PY)"
} else {
    Write-Output "Wrapper env: missing ($wrapperEnv)"
}

Write-Output ""

if (Test-Path -LiteralPath $codexAppsToolsDir) {
    $entries = Get-ChildItem -LiteralPath $codexAppsToolsDir -File -ErrorAction SilentlyContinue
    $bytes = ($entries | Measure-Object -Property Length -Sum).Sum
    Write-Output "Codex MCP tools cache: present ($($entries.Count) file(s), $bytes bytes)"
} else {
    Write-Output "Codex MCP tools cache: missing"
}

foreach ($db in @($codexCacheDb, $claudeCacheDb, $firstMovesDb)) {
    if (Test-Path -LiteralPath $db) {
        $item = Get-Item -LiteralPath $db
        Write-Output "Cache DB: $($item.FullName) ($($item.Length) bytes, modified $($item.LastWriteTime))"
    } else {
        Write-Output "Cache DB: missing ($db)"
    }
}

Write-Output ""

$python = @'
import json
import pathlib
import sqlite3
import sys

project_root = pathlib.Path(sys.argv[1]).resolve()
dbs = [
    pathlib.Path.home() / ".codex" / "cache" / "tool_cache.sqlite",
    pathlib.Path.home() / ".claude" / "cache" / "tool_cache.sqlite",
    project_root / ".first_moves.db",
]

def norm(value: str) -> str:
    return value.replace("\\", "/").lower()

for db in dbs:
    print(f"--- {db}")
    if not db.exists():
        print("missing")
        continue
    try:
        con = sqlite3.connect(db)
        con.row_factory = sqlite3.Row
        tables = [row[0] for row in con.execute("select name from sqlite_master where type='table' order by name")]
        print("tables:", ", ".join(tables))
        if "tool_cache" in tables:
            for row in con.execute("select coalesce(source_agent, 'unknown') as agent, count(*) as rows, coalesce(sum(hit_count), 0) as hits from tool_cache group by coalesce(source_agent, 'unknown') order by rows desc"):
                print(f"tool_cache agent={row['agent']} rows={row['rows']} hits={row['hits']}")
            columns = [row[1] for row in con.execute("pragma table_info(tool_cache)")]
            if "project_dir" in columns:
                project_rows = list(con.execute("select project_dir, count(*) as rows from tool_cache group by project_dir order by rows desc limit 20"))
                matched = [row for row in project_rows if row["project_dir"] and norm(row["project_dir"]) == norm(str(project_root))]
                print(f"tool_cache current-project rows: {sum(row['rows'] for row in matched)}")
        if "project_cache_state" in tables:
            states = list(con.execute("select project_dir, display_name, warm_count, last_agent from project_cache_state order by last_warmed_at desc"))
            for row in states:
                print(f"project_cache_state project={row['project_dir']} display={row['display_name']} warm_count={row['warm_count']} last_agent={row['last_agent']}")
            matched = [row for row in states if row["project_dir"] and norm(row["project_dir"]) == norm(str(project_root))]
            print(f"project_cache_state current-project rows: {len(matched)}")
        con.close()
    except Exception as exc:
        print(f"sqlite error: {exc!r}")
'@

$python | python - $projectRootFull
