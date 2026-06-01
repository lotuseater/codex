param(
    [string]$Project = (Get-Location).Path,
    [int]$Limit = 5,
    [int]$RecentDays = 14,
    [int]$TailBytes = 4096,
    [switch]$IncludeLogs,
    [switch]$Live,
    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-FullPath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }

    $expanded = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
    $full = [System.IO.Path]::GetFullPath($expanded).TrimEnd('\', '/')
    if ($full.StartsWith('\\?\UNC\', [System.StringComparison]::OrdinalIgnoreCase)) {
        return '\\' + $full.Substring(8)
    }
    if ($full.StartsWith('\\?\', [System.StringComparison]::OrdinalIgnoreCase)) {
        return $full.Substring(4)
    }
    return $full
}

function Shorten {
    param(
        [string]$Text,
        [int]$Max = 160
    )

    if ($null -eq $Text) {
        return $null
    }

    $flat = ($Text -replace '\s+', ' ').Trim()
    if ($flat.Length -le $Max) {
        return $flat
    }

    return $flat.Substring(0, $Max - 3) + '...'
}

function Unescape-JsonString {
    param([string]$Text)

    if ($null -eq $Text) {
        return $null
    }

    try {
        return ('"' + $Text + '"') | ConvertFrom-Json
    }
    catch {
        return $Text
    }
}

function Read-FirstSessionMeta {
    param([System.IO.FileInfo]$File)

    $stream = [System.IO.File]::Open($File.FullName, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    $reader = [System.IO.StreamReader]::new($stream)
    try {
        $line = $reader.ReadLine()
    }
    finally {
        $reader.Dispose()
    }

    if ([string]::IsNullOrWhiteSpace($line)) {
        return $null
    }

    $head = if ($line.Length -gt 8192) { $line.Substring(0, 8192) } else { $line }
    if ($head -notmatch '"type"\s*:\s*"session_meta"') {
        return $null
    }

    $id = $null
    $cwd = $null
    $originator = $null
    $source = $null

    $match = [regex]::Match($head, '"id"\s*:\s*"((?:\\.|[^"])*)"')
    if ($match.Success) {
        $id = Unescape-JsonString $match.Groups[1].Value
    }

    $match = [regex]::Match($head, '"cwd"\s*:\s*"((?:\\.|[^"])*)"')
    if ($match.Success) {
        $cwd = Unescape-JsonString $match.Groups[1].Value
    }

    $match = [regex]::Match($head, '"originator"\s*:\s*"((?:\\.|[^"])*)"')
    if ($match.Success) {
        $originator = Unescape-JsonString $match.Groups[1].Value
    }

    $match = [regex]::Match($head, '"source"\s*:\s*"((?:\\.|[^"])*)"')
    if ($match.Success) {
        $source = Unescape-JsonString $match.Groups[1].Value
    }
    else {
        $match = [regex]::Match($head, '"source"\s*:\s*(\{[^{}]*\})')
        if ($match.Success) {
            $source = $match.Groups[1].Value
        }
    }

    if ([string]::IsNullOrWhiteSpace($id) -and [string]::IsNullOrWhiteSpace($cwd)) {
        return $null
    }

    return [pscustomobject]@{
        id = $id
        cwd = $cwd
        source = $source
        originator = $originator
    }
}

function Read-TailText {
    param(
        [System.IO.FileInfo]$File,
        [int]$Bytes
    )

    if ($Bytes -le 0 -or $File.Length -le 0) {
        return ''
    }

    $readBytes = [Math]::Min([int64]$Bytes, $File.Length)
    $readLength = [int]$readBytes
    $buffer = [byte[]]::new($readLength)
    $stream = [System.IO.File]::Open($File.FullName, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    try {
        [void]$stream.Seek(-$readBytes, [System.IO.SeekOrigin]::End)
        [void]$stream.Read($buffer, 0, $readLength)
    }
    finally {
        $stream.Dispose()
    }

    return [System.Text.Encoding]::UTF8.GetString($buffer)
}

function Get-CodexHome {
    if (-not [string]::IsNullOrWhiteSpace($env:CODEX_HOME)) {
        return Resolve-FullPath $env:CODEX_HOME
    }

    return Resolve-FullPath (Join-Path $HOME '.codex')
}

function Get-RecentSessionFiles {
    param(
        [string]$CodexHome,
        [int]$Days
    )

    $roots = @(
        Join-Path $CodexHome 'sessions'
        Join-Path $CodexHome 'archived_sessions'
    ) | Where-Object { Test-Path -LiteralPath $_ }

    $files = New-Object 'System.Collections.Generic.List[System.IO.FileInfo]'
    $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $today = Get-Date
    $cutoff = $today.AddDays(-[Math]::Max($Days, 1))

    function Add-SessionFile {
        param([System.IO.FileInfo]$File)

        if ($seen.Add($File.FullName)) {
            [void]$files.Add($File)
        }
    }

    foreach ($root in $roots) {
        if ((Split-Path -Leaf $root) -eq 'sessions') {
            for ($i = 0; $i -lt [Math]::Max($Days, 1); $i++) {
                $day = $today.AddDays(-$i)
                $dayPath = Join-Path $root ($day.ToString('yyyy'))
                $dayPath = Join-Path $dayPath ($day.ToString('MM'))
                $dayPath = Join-Path $dayPath ($day.ToString('dd'))
                if (Test-Path -LiteralPath $dayPath) {
                    Get-ChildItem -LiteralPath $dayPath -File -Filter '*.jsonl' -ErrorAction SilentlyContinue |
                        ForEach-Object { Add-SessionFile $_ }
                }
            }

            Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.jsonl' -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -ge $cutoff } |
                ForEach-Object { Add-SessionFile $_ }
        }
        else {
            Get-ChildItem -LiteralPath $root -File -Filter '*.jsonl' -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -ge $cutoff } |
                ForEach-Object { Add-SessionFile $_ }
        }
    }

    return $files | Sort-Object LastWriteTime -Descending
}

function Get-StateDbSessionSummaries {
    param(
        [string]$CodexHome,
        [string]$ProjectPath,
        [int]$Limit
    )

    $sqlite = Get-Command sqlite3 -ErrorAction SilentlyContinue
    if ($null -eq $sqlite) {
        return @()
    }

    $dbPath = Join-Path $CodexHome 'state_5.sqlite'
    if (-not (Test-Path -LiteralPath $dbPath)) {
        return @()
    }

    $queryLimit = [Math]::Max($Limit * 20, 100)
    $query = @"
select
  id,
  rollout_path as path,
  cwd,
  coalesce(updated_at_ms, updated_at * 1000) as updated_ms,
  tokens_used,
  source,
  title
from threads
order by coalesce(updated_at_ms, updated_at * 1000) desc
limit $queryLimit;
"@

    try {
        $raw = & sqlite3 -readonly -json $dbPath $query 2>$null
        $text = ($raw -join "`n").Trim()
        if ([string]::IsNullOrWhiteSpace($text)) {
            return @()
        }
        $parsed = $text | ConvertFrom-Json
        $rows = @($parsed | ForEach-Object { $_ })
    }
    catch {
        return @()
    }

    $results = New-Object 'System.Collections.Generic.List[object]'
    foreach ($row in $rows) {
        if (-not (Session-MatchesProject -File $null -SessionCwd $row.cwd -ProjectPath $ProjectPath -TailBytes 0)) {
            continue
        }

        $path = [string]$row.path
        $file = $null
        if (-not [string]::IsNullOrWhiteSpace($path)) {
            try {
                if (Test-Path -LiteralPath $path -PathType Leaf) {
                    $file = Get-Item -LiteralPath $path
                }
            }
            catch {
                $file = $null
            }
        }
        if ($null -eq $file) {
            continue
        }
        $bytes = $file.Length

        $updated = $null
        try {
            $updated = [DateTimeOffset]::FromUnixTimeMilliseconds([int64]$row.updated_ms).LocalDateTime
        }
        catch {
            $updated = $null
        }
        if ($null -eq $updated -or $file.LastWriteTime -gt $updated) {
            $updated = $file.LastWriteTime
        }

        [void]$results.Add([pscustomobject]@{
            kind = 'session'
            id = $row.id
            cwd = $row.cwd
            updated = $updated
            bytes = $bytes
            source = $row.source
            originator = 'state-db'
            tokens_used = $row.tokens_used
            path = $path
            title = Shorten -Text $row.title -Max 180
        })

        if ($results.Count -ge $Limit) {
            break
        }
    }

    return $results
}

function Session-MatchesProject {
    param(
        [System.IO.FileInfo]$File,
        [string]$SessionCwd,
        [string]$ProjectPath,
        [int]$TailBytes
    )

    if ([string]::IsNullOrWhiteSpace($ProjectPath)) {
        return $true
    }

    if (-not [string]::IsNullOrWhiteSpace($SessionCwd)) {
        try {
            $resolvedCwd = Resolve-FullPath $SessionCwd
            if ($resolvedCwd.Equals($ProjectPath, [System.StringComparison]::OrdinalIgnoreCase)) {
                return $true
            }
            return $false
        }
        catch {
            if ($SessionCwd.Equals($ProjectPath, [System.StringComparison]::OrdinalIgnoreCase)) {
                return $true
            }
            return $false
        }
    }

    $tail = Read-TailText -File $File -Bytes $TailBytes
    if ([string]::IsNullOrWhiteSpace($tail)) {
        return $false
    }

    $needle = $ProjectPath.ToLowerInvariant()
    $escapedNeedle = ($ProjectPath -replace '\\', '\\').ToLowerInvariant()
    $haystack = $tail.ToLowerInvariant()
    return $haystack.Contains($needle) -or $haystack.Contains($escapedNeedle)
}

function Get-SessionSummaries {
    param(
        [string]$CodexHome,
        [string]$ProjectPath,
        [int]$Days,
        [int]$Limit,
        [int]$TailBytes
    )

    $results = New-Object 'System.Collections.Generic.List[object]'
    foreach ($file in Get-RecentSessionFiles -CodexHome $CodexHome -Days $Days) {
        $meta = Read-FirstSessionMeta -File $file
        if ($null -eq $meta) {
            continue
        }

        $cwd = $meta.cwd

        if (-not (Session-MatchesProject -File $file -SessionCwd $cwd -ProjectPath $ProjectPath -TailBytes $TailBytes)) {
            continue
        }

        [void]$results.Add([pscustomobject]@{
            kind = 'session'
            id = $meta.id
            cwd = $cwd
            updated = $file.LastWriteTime
            bytes = $file.Length
            source = $meta.source
            originator = $meta.originator
            path = $file.FullName
        })

        if ($results.Count -ge $Limit) {
            break
        }
    }

    return $results
}

function Get-ProjectLogs {
    param(
        [string]$ProjectPath,
        [int]$Limit
    )

    if ([string]::IsNullOrWhiteSpace($ProjectPath)) {
        return @()
    }

    $logDir = Join-Path $ProjectPath 'logs'
    if (-not (Test-Path -LiteralPath $logDir)) {
        return @()
    }

    return Get-ChildItem -LiteralPath $logDir -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First $Limit |
        ForEach-Object {
            [pscustomobject]@{
                kind = 'log'
                id = $_.BaseName
                cwd = $ProjectPath
                updated = $_.LastWriteTime
                bytes = $_.Length
                source = 'repo-log'
                originator = $null
                path = $_.FullName
            }
        }
}

function Get-LiveCodexProcesses {
    param(
        [string]$ProjectPath,
        [int]$Limit
    )

    $names = @('codex.exe', 'pwsh.exe', 'powershell.exe', 'cargo.exe', 'rustc.exe', 'link.exe')
    Get-CimInstance Win32_Process |
        Where-Object { $names -contains $_.Name } |
        Sort-Object CreationDate -Descending |
        Select-Object -First $Limit |
        ForEach-Object {
            $cmd = $_.CommandLine
            $created = $null
            try {
                if ($_.CreationDate -is [datetime]) {
                    $created = $_.CreationDate
                }
                elseif (-not [string]::IsNullOrWhiteSpace([string]$_.CreationDate)) {
                    $created = [System.Management.ManagementDateTimeConverter]::ToDateTime($_.CreationDate)
                }
            }
            catch {
                $created = $null
            }

            [pscustomobject]@{
                kind = 'process'
                id = [string]$_.ProcessId
                cwd = $null
                updated = $created
                bytes = $null
                source = $_.Name
                originator = "ppid=$($_.ParentProcessId)"
                path = Shorten -Text $cmd -Max 220
                matches_project = if ([string]::IsNullOrWhiteSpace($ProjectPath) -or [string]::IsNullOrWhiteSpace($cmd)) { $false } else { $cmd.ToLowerInvariant().Contains($ProjectPath.ToLowerInvariant()) }
            }
        }
}

$codexHome = Get-CodexHome
$projectPath = Resolve-FullPath $Project
$items = New-Object 'System.Collections.Generic.List[object]'
$seenPaths = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)

function Add-ResultItem {
    param([object]$Item)

    $key = if (-not [string]::IsNullOrWhiteSpace($Item.path)) { $Item.path } elseif (-not [string]::IsNullOrWhiteSpace($Item.id)) { $Item.id } else { $null }
    if ($null -eq $key -or $seenPaths.Add($key)) {
        [void]$items.Add($Item)
    }
}

Get-StateDbSessionSummaries -CodexHome $codexHome -ProjectPath $projectPath -Limit $Limit |
    ForEach-Object { Add-ResultItem $_ }

if ($items.Count -lt $Limit) {
    Get-SessionSummaries -CodexHome $codexHome -ProjectPath $projectPath -Days $RecentDays -Limit $Limit -TailBytes $TailBytes |
        ForEach-Object { Add-ResultItem $_ }
}

if ($IncludeLogs) {
    Get-ProjectLogs -ProjectPath $projectPath -Limit $Limit |
        ForEach-Object { Add-ResultItem $_ }
}

if ($Live) {
    Get-LiveCodexProcesses -ProjectPath $projectPath -Limit ([Math]::Max($Limit * 2, 8)) |
        ForEach-Object { Add-ResultItem $_ }
}

$ordered = $items | Sort-Object updated -Descending

if ($Json) {
    $ordered | ConvertTo-Json -Depth 5
}
else {
    $ordered |
        Select-Object kind,
            @{ Name = 'id'; Expression = { Shorten -Text $_.id -Max 14 } },
            updated,
            bytes,
            @{ Name = 'src'; Expression = { Shorten -Text $_.source -Max 18 } },
            @{ Name = 'tokens'; Expression = { $_.tokens_used } },
            @{ Name = 'project'; Expression = { if ([string]::IsNullOrWhiteSpace($_.cwd)) { $null } else { Split-Path -Leaf $_.cwd } } },
            @{ Name = 'file_or_command'; Expression = { if ($_.kind -eq 'process') { Shorten -Text $_.path -Max 64 } elseif ([string]::IsNullOrWhiteSpace($_.path)) { $null } else { Shorten -Text (Split-Path -Leaf $_.path) -Max 42 } } } |
        Format-Table -AutoSize -Wrap
}
