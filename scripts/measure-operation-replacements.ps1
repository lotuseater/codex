param(
    [string[]]$Task = @('GitSummary', 'SessionFind'),

    [string]$Project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,

    [int]$RecentDays = 3,

    [int]$Limit = 10,

    [string]$Pattern,

    [string[]]$SearchRoots = @('codex-rs/core/src', 'codex-rs/tools/src', 'scripts', 'docs'),

    [int]$MaxFiles = 20,

    [int]$MaxMatchesPerFile = 5,

    [string]$FilePath = 'codex-rs/core/src/tools/handlers/shell.rs',

    [int]$MaxOutlineItems = 200,

    [string]$CheckCommand,

    [string]$ArtifactRoot = 'logs/operation-replacement-artifacts',

    [int]$MaxDigestLines = 30,

    [string]$OutFile,

    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-FullPath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }

    $full = [System.IO.Path]::GetFullPath(
        $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
    ).TrimEnd('\', '/')
    if ($full.StartsWith('\\?\UNC\', [System.StringComparison]::OrdinalIgnoreCase)) {
        return '\\' + $full.Substring(8)
    }
    if ($full.StartsWith('\\?\', [System.StringComparison]::OrdinalIgnoreCase)) {
        return $full.Substring(4)
    }
    return $full
}

function ConvertTo-ApproxTokens {
    param([string]$Text)

    if ($null -eq $Text) {
        return 0
    }

    return [int][Math]::Ceiling($Text.Length / 4.0)
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

function Count-Items {
    param([object]$Value)

    if ($null -eq $Value) {
        return 0
    }

    $measure = $Value | Measure-Object
    return [int]$measure.Count
}

function Invoke-Measured {
    param(
        [string]$Name,
        [scriptblock]$Script
    )

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $text = ''
    $errorText = $null
    $nativePreference = Get-Variable -Name PSNativeCommandUseErrorActionPreference -Scope Global -ErrorAction SilentlyContinue
    $oldNativePreference = $null
    $oldErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        if ($null -ne $nativePreference) {
            $oldNativePreference = $nativePreference.Value
            Set-Variable -Name PSNativeCommandUseErrorActionPreference -Scope Global -Value $false
        }
        $result = & $Script
        if ($null -ne $result) {
            $text = ($result | Out-String).TrimEnd()
        }
    }
    catch {
        $errorText = $_.Exception.Message
        if ($_.InvocationInfo -and $_.InvocationInfo.ScriptLineNumber) {
            $errorText = "$errorText at line $($_.InvocationInfo.ScriptLineNumber)"
        }
        $text = $errorText
    }
    finally {
        $ErrorActionPreference = $oldErrorActionPreference
        if ($null -ne $nativePreference) {
            Set-Variable -Name PSNativeCommandUseErrorActionPreference -Scope Global -Value $oldNativePreference
        }
        $sw.Stop()
    }

    return [pscustomobject]@{
        name = $Name
        text = $text
        chars = $text.Length
        approx_tokens = ConvertTo-ApproxTokens $text
        wall_ms = [int64]$sw.ElapsedMilliseconds
        error = $errorText
    }
}

function New-BenchRecord {
    param(
        [string]$Operation,
        [object]$Baseline,
        [object]$Candidate,
        [object]$Comparison,
        [string]$Verdict,
        [string]$FallbackReason = $null
    )

    $saved = [Math]::Max(0, $Baseline.approx_tokens - $Candidate.approx_tokens)
    $savingPct = if ($Baseline.approx_tokens -gt 0) {
        [Math]::Round(($saved * 100.0) / $Baseline.approx_tokens, 1)
    }
    else {
        0
    }

    return [pscustomobject]@{
        type = 'replacement_bench'
        operation = $Operation
        baseline = [pscustomobject]@{
            name = $Baseline.name
            model_visible_chars = $Baseline.chars
            model_visible_tokens = $Baseline.approx_tokens
            wall_ms = $Baseline.wall_ms
            error = $Baseline.error
        }
        candidate = [pscustomobject]@{
            name = $Candidate.name
            model_visible_chars = $Candidate.chars
            model_visible_tokens = $Candidate.approx_tokens
            wall_ms = $Candidate.wall_ms
            error = $Candidate.error
        }
        token_savings = [pscustomobject]@{
            approx_tokens = $saved
            percent = $savingPct
        }
        comparison = $Comparison
        fallback_reason = $FallbackReason
        verdict = $Verdict
    }
}

function Resolve-TaskList {
    param([string[]]$Tasks)

    $valid = @('GitSummary', 'SessionFind', 'SearchText', 'FileOutline', 'RunCheck')
    $resolved = New-Object 'System.Collections.Generic.List[string]'
    foreach ($taskItem in $Tasks) {
        foreach ($part in ([string]$taskItem -split ',')) {
            $taskName = $part.Trim()
            if ([string]::IsNullOrWhiteSpace($taskName)) {
                continue
            }
            $match = @($valid | Where-Object { [String]::Equals($_, $taskName, [StringComparison]::OrdinalIgnoreCase) })
            if ($match.Count -eq 0) {
                throw "Unknown task '$taskName'. Valid tasks: $($valid -join ', ')."
            }
            [void]$resolved.Add($match[0])
        }
    }

    if ($resolved.Count -eq 0) {
        throw "No tasks selected. Valid tasks: $($valid -join ', ')."
    }

    return $resolved.ToArray()
}

function Get-GitStatusFacts {
    param([string[]]$Lines)

    $files = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $staged = New-Object 'System.Collections.Generic.List[string]'
    $unstaged = New-Object 'System.Collections.Generic.List[string]'
    $untracked = New-Object 'System.Collections.Generic.List[string]'

    foreach ($line in $Lines) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line.Length -lt 3) {
            continue
        }

        $xy = $line.Substring(0, 2)
        $path = $line.Substring(3).Trim()
        if ($path.Contains(' -> ')) {
            $path = ($path -split ' -> ')[-1].Trim()
        }

        [void]$files.Add($path)

        if ($xy -eq '??') {
            [void]$untracked.Add($path)
            continue
        }

        if ($xy[0] -ne ' ') {
            [void]$staged.Add($path)
        }
        if ($xy[1] -ne ' ') {
            [void]$unstaged.Add($path)
        }
    }

    return [pscustomobject]@{
        files = @($files | Sort-Object)
        staged = @($staged | Sort-Object -Unique)
        unstaged = @($unstaged | Sort-Object -Unique)
        untracked = @($untracked | Sort-Object -Unique)
    }
}

function Get-TopDirs {
    param([string[]]$Files)

    $groups = @{}
    foreach ($file in $Files) {
        $dir = if ($file.Contains('/')) {
            ($file -split '/')[0]
        }
        elseif ($file.Contains('\')) {
            ($file -split '\\')[0]
        }
        else {
            '.'
        }
        if ($groups.ContainsKey($dir)) {
            $groups[$dir] = 1 + $groups[$dir]
        }
        else {
            $groups[$dir] = 1
        }
    }

    return $groups.GetEnumerator() |
        Sort-Object Value -Descending |
        Select-Object -First 8 |
        ForEach-Object { "$($_.Key)=$($_.Value)" }
}

function Invoke-GitSummaryBench {
    param([string]$ProjectPath)

    $baseline = Invoke-Measured 'raw git status/diff' {
        Push-Location $ProjectPath
        try {
            $status = (& git status --short 2>&1) -join "`n"
            $stat = (& git diff --stat 2>&1) -join "`n"
            $names = (& git diff --name-only 2>&1) -join "`n"
            "git status --short`n$status`n`ngit diff --stat`n$stat`n`ngit diff --name-only`n$names"
        }
        finally {
            Pop-Location
        }
    }

    $script:candidateFacts = $null
    $candidate = Invoke-Measured 'git_worktree_summary prototype' {
        Push-Location $ProjectPath
        try {
            $branch = ((& git branch --show-current 2>$null) -join "`n").Trim()
            $statusLines = @(& git -c core.quotepath=false status --porcelain=v1 2>&1)
            $facts = Get-GitStatusFacts $statusLines
            $script:candidateFacts = $facts
            $topDirs = @(Get-TopDirs $facts.files)
            $lines = New-Object 'System.Collections.Generic.List[string]'
            [void]$lines.Add("branch: $branch")
            [void]$lines.Add("changed_files: $($facts.files.Count)")
            [void]$lines.Add("staged: $($facts.staged.Count)")
            [void]$lines.Add("unstaged: $($facts.unstaged.Count)")
            [void]$lines.Add("untracked: $($facts.untracked.Count)")
            if ($topDirs.Count -gt 0) {
                [void]$lines.Add("top_dirs: $($topDirs -join ', ')")
            }
            foreach ($label in @('staged', 'unstaged', 'untracked')) {
                $values = @($facts.$label | Select-Object -First $Limit)
                if ($values.Count -gt 0) {
                    [void]$lines.Add("${label}:")
                    foreach ($value in $values) {
                        [void]$lines.Add("- $value")
                    }
                }
            }
            [void]$lines.Add("raw: git status --short; git diff --stat")
            $lines -join "`n"
        }
        finally {
            Pop-Location
        }
    }
    $candidateFacts = $script:candidateFacts

    Push-Location $ProjectPath
    try {
        $baselineFacts = Get-GitStatusFacts @(& git -c core.quotepath=false status --porcelain=v1 2>&1)
    }
    finally {
        Pop-Location
    }

    $missing = @($baselineFacts.files | Where-Object { $candidateFacts.files -notcontains $_ })
    $verdict = if ($baseline.error -or $candidate.error) {
        'fallback_required'
    }
    elseif ($missing.Count -eq 0) {
        'pass'
    }
    else {
        'fail_quality'
    }
    $fallback = if ($missing.Count -gt 0) { "candidate missed files: $($missing -join ', ')" } else { $null }

    return New-BenchRecord `
        -Operation 'git_worktree_summary' `
        -Baseline $baseline `
        -Candidate $candidate `
        -Comparison ([pscustomobject]@{
            baseline_files = $baselineFacts.files.Count
            candidate_files = $candidateFacts.files.Count
            missing_files = $missing
        }) `
        -Verdict $verdict `
        -FallbackReason $fallback
}

function Get-CodexHome {
    if (-not [string]::IsNullOrWhiteSpace($env:CODEX_HOME)) {
        return Resolve-FullPath $env:CODEX_HOME
    }

    return Resolve-FullPath (Join-Path $HOME '.codex')
}

function Read-SessionMeta {
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

    try {
        $json = $line | ConvertFrom-Json
        if ($json.type -ne 'session_meta') {
            return $null
        }
        $meta = if ($json.payload) { $json.payload } else { $json }
        return [pscustomobject]@{
            id = $meta.id
            cwd = $meta.cwd
            path = $File.FullName
        }
    }
    catch {
        return $null
    }
}

function Read-TailText {
    param(
        [System.IO.FileInfo]$File,
        [int]$Bytes = 4096
    )

    if ($File.Length -le 0) {
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

function Get-BroadSessionMatches {
    param(
        [string]$ProjectPath,
        [int]$Days,
        [int]$Limit
    )

    $codexHome = Get-CodexHome
    $sessionsRoot = Join-Path $codexHome 'sessions'
    if (-not (Test-Path -LiteralPath $sessionsRoot)) {
        return @()
    }

    $cutoff = (Get-Date).AddDays(-[Math]::Max($Days, 1))
    $needle = $ProjectPath.ToLowerInvariant()
    $escapedNeedle = ($ProjectPath -replace '\\', '\\').ToLowerInvariant()

    $sessionMatches = New-Object 'System.Collections.Generic.List[object]'
    $files = Get-ChildItem -LiteralPath $sessionsRoot -Recurse -File -Filter '*.jsonl' -ErrorAction SilentlyContinue |
        Where-Object { $_.LastWriteTime -ge $cutoff } |
        Sort-Object LastWriteTime -Descending

    foreach ($file in $files) {
        $meta = Read-SessionMeta $file
        $isMatch = $false
        if ($null -ne $meta -and -not [string]::IsNullOrWhiteSpace($meta.cwd)) {
            try {
                $resolvedMetaCwd = Resolve-FullPath $meta.cwd
                $isMatch = [String]::Equals($resolvedMetaCwd, $ProjectPath, [StringComparison]::OrdinalIgnoreCase)
            }
            catch {
                $isMatch = [String]::Equals($meta.cwd, $ProjectPath, [StringComparison]::OrdinalIgnoreCase)
            }
        }
        else {
            $tail = (Read-TailText $file).ToLowerInvariant()
            $isMatch = $tail.Contains($needle) -or $tail.Contains($escapedNeedle)
        }

        if ($isMatch) {
            [void]$sessionMatches.Add([pscustomobject]@{
                id = if ($null -ne $meta) { $meta.id } else { $null }
                path = $file.FullName
                last_write_time = $file.LastWriteTime
            })
        }
        if ($sessionMatches.Count -ge $Limit) {
            break
        }
    }

    return $sessionMatches.ToArray()
}

function Get-SessionCompareKey {
    param([object]$Session)

    if ($Session.path) {
        $path = [string]$Session.path
        try {
            return (Resolve-FullPath $path).ToLowerInvariant()
        }
        catch {
            return $path.Trim().ToLowerInvariant()
        }
    }

    if ($Session.id) {
        return ([string]$Session.id).Trim().ToLowerInvariant()
    }

    return $null
}

function Format-SessionTimestamp {
    param([object]$Value)

    if ($Value -is [datetime]) {
        return $Value.ToString('s')
    }

    $text = [string]$Value
    if ($text -match '/Date\((\d+)\)/') {
        try {
            return [DateTimeOffset]::FromUnixTimeMilliseconds([int64]$matches[1]).LocalDateTime.ToString('s')
        }
        catch {
            return $text
        }
    }

    return $text
}

function Invoke-SessionFindBench {
    param(
        [string]$ProjectPath,
        [int]$Days,
        [int]$Limit
    )

    $script:baselineMatches = @()
    $baseline = Invoke-Measured 'broad recursive session scan' {
        $script:baselineMatches = @(Get-BroadSessionMatches -ProjectPath $ProjectPath -Days $Days -Limit $Limit)
        $script:baselineMatches |
            ForEach-Object { "$($_.last_write_time.ToString('s')) $($_.id) $($_.path)" }
    }

    $script:candidateObjects = @()
    $candidate = Invoke-Measured 'session_find prototype' {
        $scriptPath = Join-Path $PSScriptRoot 'find-codex-sessions.ps1'
        $raw = & powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath -Project $ProjectPath -RecentDays $Days -Limit $Limit -Json
        $text = ($raw -join "`n").Trim()
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            $parsed = $text | ConvertFrom-Json
            $script:candidateObjects = @($parsed | ForEach-Object { $_ })
        }

        $lines = New-Object 'System.Collections.Generic.List[string]'
        foreach ($item in $script:candidateObjects) {
            $id = Shorten -Text $item.id -Max 14
            $updated = Format-SessionTimestamp $item.updated
            [void]$lines.Add("$updated id=$id bytes=$($item.bytes) path=$($item.path)")
        }
        $lines -join "`n"
    }

    $baselineMatches = @($script:baselineMatches)
    $candidateObjects = @($script:candidateObjects)
    $baselineKeys = @($baselineMatches | ForEach-Object { Get-SessionCompareKey $_ } | Where-Object { $_ } | Select-Object -Unique)
    $candidateKeys = @($candidateObjects | ForEach-Object { Get-SessionCompareKey $_ } | Where-Object { $_ } | Select-Object -Unique)
    $missing = @($baselineKeys | Where-Object { $candidateKeys -notcontains $_ })
    $extra = @($candidateKeys | Where-Object { $baselineKeys -notcontains $_ })

    $verdict = if ($baseline.error -or $candidate.error) {
        'fallback_required'
    }
    elseif ($candidateKeys.Count -eq 0) {
        'fallback_required'
    }
    elseif ($missing.Count -eq 0) {
        'pass'
    }
    elseif ($extra.Count -gt 0 -and ($candidateKeys.Count -ge $baselineKeys.Count -or $candidateKeys.Count -ge $Limit)) {
        'needs_human_review'
    }
    else {
        'fail_quality'
    }
    $fallback = if ($candidateKeys.Count -eq 0) {
        'candidate returned no sessions'
    }
    elseif ($missing.Count -gt 0 -and $verdict -eq 'needs_human_review') {
        "baseline/candidate disagreement; candidate found $($extra.Count) additional sessions and omitted $($missing.Count) baseline sessions"
    }
    elseif ($missing.Count -gt 0) {
        "candidate missed sessions: $($missing -join ', ')"
    }
    else {
        $null
    }

    return New-BenchRecord `
        -Operation 'session_find' `
        -Baseline $baseline `
        -Candidate $candidate `
        -Comparison ([pscustomobject]@{
            baseline_sessions = $baselineKeys.Count
            candidate_sessions = $candidateKeys.Count
            missing_sessions = $missing
            extra_sessions = $extra
        }) `
        -Verdict $verdict `
        -FallbackReason $fallback
}

function Invoke-SearchTextBench {
    param(
        [string]$ProjectPath,
        [string]$Pattern,
        [string[]]$Roots,
        [int]$MaxFiles,
        [int]$MaxMatchesPerFile
    )

    if ([string]::IsNullOrWhiteSpace($Pattern)) {
        throw 'SearchText requires -Pattern.'
    }

    $existingRoots = @()
    foreach ($root in $Roots) {
        $full = Join-Path $ProjectPath $root
        if (Test-Path -LiteralPath $full) {
            $existingRoots += $root
        }
    }
    if ($existingRoots.Count -eq 0) {
        throw 'SearchText has no existing search roots.'
    }

    $script:baselineLines = @()
    $script:candidateLines = New-Object 'System.Collections.Generic.List[string]'
    $baseline = Invoke-Measured 'raw rg search' {
        Push-Location $ProjectPath
        try {
            $script:baselineLines = @(& rg -n --hidden --glob '!target' --glob '!.git' -- $Pattern @existingRoots 2>&1)
            $script:baselineLines -join "`n"
        }
        finally {
            Pop-Location
        }
    }

    $candidate = Invoke-Measured 'search_text prototype' {
        $byFile = [ordered]@{}
        foreach ($line in $script:baselineLines) {
            if ($line -notmatch '^([^:]+):(\d+):(.*)$') {
                continue
            }
            $file = $matches[1]
            if (-not $byFile.Contains($file)) {
                if ($byFile.Count -ge $MaxFiles) {
                    continue
                }
                $byFile[$file] = New-Object 'System.Collections.Generic.List[string]'
            }
            if ($byFile[$file].Count -lt $MaxMatchesPerFile) {
                [void]$byFile[$file].Add("$($matches[2]):$(Shorten $matches[3] 120)")
            }
        }

        $out = New-Object 'System.Collections.Generic.List[string]'
        [void]$out.Add("pattern: $Pattern")
        [void]$out.Add("files_returned: $($byFile.Count)")
        foreach ($entry in $byFile.GetEnumerator()) {
            [void]$out.Add("")
            [void]$out.Add($entry.Key)
            foreach ($matchLine in $entry.Value) {
                [void]$out.Add("- $matchLine")
                [void]$script:candidateLines.Add("$($entry.Key):$matchLine")
            }
        }
        $out -join "`n"
    }

    $baselineLines = @($script:baselineLines)
    $candidateLines = @($script:candidateLines)
    $baselineFiles = @($baselineLines |
        Where-Object { $_ -match '^([^:]+):(\d+):(.*)$' } |
        ForEach-Object { if ($_ -match '^([^:]+):') { $matches[1] } } |
        Select-Object -Unique)
    $candidateFiles = @($candidateLines |
        ForEach-Object { if ($_ -match '^([^:]+):') { $matches[1] } } |
        Select-Object -Unique)
    $omittedFiles = @($baselineFiles | Where-Object { $candidateFiles -notcontains $_ })

    $verdict = if ($baseline.error -or $candidate.error) {
        'fallback_required'
    }
    elseif ($omittedFiles.Count -eq 0) {
        'pass'
    }
    else {
        'fallback_required'
    }
    $fallback = if ($omittedFiles.Count -gt 0) {
        "candidate omitted files due caps: $($omittedFiles.Count)"
    }
    else {
        $null
    }

    return New-BenchRecord `
        -Operation 'search_text' `
        -Baseline $baseline `
        -Candidate $candidate `
        -Comparison ([pscustomobject]@{
            baseline_files = $baselineFiles.Count
            candidate_files = $candidateFiles.Count
            omitted_files = @($omittedFiles | Select-Object -First 20)
            omitted_file_count = $omittedFiles.Count
        }) `
        -Verdict $verdict `
        -FallbackReason $fallback
}

function Get-FileOutlineFacts {
    param(
        [string]$Path,
        [int]$MaxItems
    )

    $lines = [System.IO.File]::ReadAllLines($Path)
    $definitions = New-Object 'System.Collections.Generic.List[object]'
    $imports = New-Object 'System.Collections.Generic.List[object]'

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        $lineNumber = $i + 1
        $trimmed = $line.Trim()

        if ($trimmed -match '^(use|mod)\s+(.+?);$') {
            if ($imports.Count -lt 40) {
                [void]$imports.Add([pscustomobject]@{
                    line = $lineNumber
                    text = $trimmed
                })
            }
            continue
        }

        if ($trimmed -match '^(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(fn|struct|enum|trait|type|const|static|mod)\s+([A-Za-z_][A-Za-z0-9_]*)') {
            [void]$definitions.Add([pscustomobject]@{
                line = $lineNumber
                kind = $matches[1]
                name = $matches[2]
                text = $trimmed
                key = "$($matches[1]):$($matches[2]):$lineNumber"
            })
            continue
        }

        if ($trimmed -match '^(?:unsafe\s+)?impl(?:<[^>]+>)?\s+(.+?)\s*(?:\{|where\b)') {
            $name = Shorten -Text $matches[1] -Max 80
            [void]$definitions.Add([pscustomobject]@{
                line = $lineNumber
                kind = 'impl'
                name = $name
                text = $trimmed
                key = "impl:${name}:$lineNumber"
            })
        }
    }

    $definitionTotal = [int]$definitions.Count
    $maxItemCount = [int]$MaxItems
    $definitionReturnCount = [Math]::Min($definitionTotal, $maxItemCount)

    return [pscustomobject]@{
        line_count = [int]$lines.Count
        import_count = [int]$imports.Count
        definition_count = $definitionReturnCount
        total_definition_count = $definitionTotal
        imports = @($imports | ForEach-Object { $_ })
        definitions = @($definitions | Select-Object -First $maxItemCount)
        all_definition_keys = @($definitions | ForEach-Object { $_.key })
        omitted_definition_count = [Math]::Max(0, $definitionTotal - $maxItemCount)
    }
}

function Invoke-FileOutlineBench {
    param(
        [string]$ProjectPath,
        [string]$RelativeOrFullPath,
        [int]$MaxItems
    )

    $fullPath = if ([System.IO.Path]::IsPathRooted($RelativeOrFullPath)) {
        Resolve-FullPath $RelativeOrFullPath
    }
    else {
        Resolve-FullPath (Join-Path $ProjectPath $RelativeOrFullPath)
    }

    if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        throw "FileOutline path does not exist: $RelativeOrFullPath"
    }

    $script:outlineFacts = $null
    $baseline = Invoke-Measured 'raw whole-file read' {
        Get-Content -Raw -LiteralPath $fullPath
    }

    $candidate = Invoke-Measured 'file_outline prototype' {
        $facts = Get-FileOutlineFacts -Path $fullPath -MaxItems $MaxItems
        $script:outlineFacts = $facts
        $relative = try {
            [System.IO.Path]::GetRelativePath($ProjectPath, $fullPath)
        }
        catch {
            $fullPath
        }

        $out = New-Object 'System.Collections.Generic.List[string]'
        [void]$out.Add("path: $relative")
        [void]$out.Add("lines: $($facts.line_count)")
        [void]$out.Add("imports_returned: $($facts.import_count)")
        [void]$out.Add("definitions_returned: $($facts.definition_count)")
        [void]$out.Add("definitions_omitted: $($facts.omitted_definition_count)")
        if ($facts.import_count -gt 0) {
            [void]$out.Add("")
            [void]$out.Add("imports:")
            foreach ($import in $facts.imports) {
                [void]$out.Add("- L$($import.line) $($import.text)")
            }
        }
        if ($facts.definition_count -gt 0) {
            [void]$out.Add("")
            [void]$out.Add("definitions:")
            foreach ($definition in $facts.definitions) {
                [void]$out.Add("- L$($definition.line) $($definition.kind) $($definition.name)")
            }
        }
        $out -join "`n"
    }

    $facts = $script:outlineFacts
    $baselineKeys = if ($null -ne $facts) { @($facts.all_definition_keys) } else { @() }
    $candidateKeys = if ($null -ne $facts) { @($facts.definitions | ForEach-Object { $_.key }) } else { @() }
    $omitted = @($baselineKeys | Where-Object { $candidateKeys -notcontains $_ })
    $omittedCount = Count-Items $omitted

    $verdict = if ($baseline.error -or $candidate.error) {
        'fallback_required'
    }
    elseif ($null -eq $facts) {
        'fallback_required'
    }
    elseif ($omittedCount -eq 0) {
        'pass'
    }
    else {
        'fallback_required'
    }
    $fallback = if ($omittedCount -gt 0) {
        "outline omitted definitions due cap: $omittedCount"
    }
    elseif ($null -eq $facts) {
        'outline did not produce facts'
    }
    else {
        $null
    }

    return New-BenchRecord `
        -Operation 'file_outline' `
        -Baseline $baseline `
        -Candidate $candidate `
        -Comparison ([pscustomobject]@{
            file = $fullPath
            line_count = if ($null -ne $facts) { $facts.line_count } else { 0 }
            imports_returned = if ($null -ne $facts) { $facts.import_count } else { 0 }
            baseline_definitions = if ($null -ne $facts) { $facts.total_definition_count } else { 0 }
            candidate_definitions = if ($null -ne $facts) { $facts.definition_count } else { 0 }
            omitted_definition_count = $omittedCount
            omitted_definitions = @($omitted | Select-Object -First 20)
        }) `
        -Verdict $verdict `
        -FallbackReason $fallback
}

function Get-SimulatedCheckLog {
    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add('running release check prototype')
    for ($i = 1; $i -le 350; $i++) {
        [void]$lines.Add("compiled crate_$i in release profile")
    }
    [void]$lines.Add('warning: context layer prompt prefix changed; cache miss expected')
    for ($i = 351; $i -le 520; $i++) {
        [void]$lines.Add("compiled crate_$i in release profile")
    }
    [void]$lines.Add('error: first actionable failure: linker exited with code 1120')
    [void]$lines.Add('note: full command output preserved in artifact')
    $lines -join "`n"
}

function Get-DiagnosticLines {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return @()
    }

    $lines = $Text -split "`r?`n"
    return @($lines | Where-Object { $_ -match '(?i)\b(error|failed|failure|panic|exception|warning|warn)\b' })
}

function Invoke-RunCheckBench {
    param(
        [string]$ProjectPath,
        [string]$Command,
        [string]$ArtifactRoot,
        [int]$MaxDigestLines
    )

    $baseline = Invoke-Measured 'raw check output' {
        if ([string]::IsNullOrWhiteSpace($Command)) {
            Get-SimulatedCheckLog
        }
        else {
            Push-Location $ProjectPath
            try {
                & powershell -NoProfile -ExecutionPolicy Bypass -Command $Command 2>&1
            }
            finally {
                Pop-Location
            }
        }
    }

    $candidateDiagnostics = @()
    $script:runCheckArtifactPath = $null
    $candidate = Invoke-Measured 'run_check digest prototype' {
        $artifactDir = if ([System.IO.Path]::IsPathRooted($ArtifactRoot)) {
            Resolve-FullPath $ArtifactRoot
        }
        else {
            Resolve-FullPath (Join-Path $ProjectPath $ArtifactRoot)
        }
        New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null

        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($baseline.text)
            $hash = ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join ''
        }
        finally {
            $sha.Dispose()
        }

        $stamp = (Get-Date).ToString('yyyyMMdd-HHmmss')
        $script:runCheckArtifactPath = Join-Path $artifactDir "run-check-$stamp-$($hash.Substring(0, 12)).log"
        Set-Content -LiteralPath $script:runCheckArtifactPath -Value $baseline.text -Encoding UTF8

        $diagnostics = @(Get-DiagnosticLines $baseline.text)
        $script:runCheckDiagnostics = @($diagnostics | Select-Object -First $MaxDigestLines)
        $omitted = [Math]::Max(0, $diagnostics.Count - $MaxDigestLines)

        $out = New-Object 'System.Collections.Generic.List[string]'
        [void]$out.Add('status: failed_or_needs_review')
        [void]$out.Add("artifact: $script:runCheckArtifactPath")
        [void]$out.Add("raw_chars: $($baseline.chars)")
        [void]$out.Add("raw_tokens: $($baseline.approx_tokens)")
        [void]$out.Add("diagnostics_returned: $($script:runCheckDiagnostics.Count)")
        [void]$out.Add("diagnostics_omitted: $omitted")
        foreach ($line in $script:runCheckDiagnostics) {
            [void]$out.Add("- $(Shorten -Text $line -Max 180)")
        }
        $out -join "`n"
    }

    $baselineDiagnostics = @(Get-DiagnosticLines $baseline.text)
    $candidateDiagnostics = @($script:runCheckDiagnostics)
    $omittedDiagnostics = @($baselineDiagnostics | Where-Object { $candidateDiagnostics -notcontains $_ })
    $omittedCritical = @($omittedDiagnostics | Where-Object { $_ -match '(?i)\b(error|failed|failure|panic|exception)\b' })

    $verdict = if ($baseline.error -or $candidate.error) {
        'fallback_required'
    }
    elseif ($omittedCritical.Count -eq 0) {
        'pass'
    }
    else {
        'fallback_required'
    }
    $fallback = if ($omittedCritical.Count -gt 0) {
        "digest omitted critical diagnostics: $($omittedCritical.Count)"
    }
    else {
        $null
    }

    return New-BenchRecord `
        -Operation 'run_check_digest' `
        -Baseline $baseline `
        -Candidate $candidate `
        -Comparison ([pscustomobject]@{
            artifact_path = $script:runCheckArtifactPath
            baseline_diagnostics = $baselineDiagnostics.Count
            candidate_diagnostics = $candidateDiagnostics.Count
            omitted_diagnostic_count = $omittedDiagnostics.Count
            omitted_critical_count = $omittedCritical.Count
            omitted_diagnostics = @($omittedDiagnostics | Select-Object -First 20)
        }) `
        -Verdict $verdict `
        -FallbackReason $fallback
}

$Task = Resolve-TaskList $Task
$projectPath = Resolve-FullPath $Project
$records = New-Object 'System.Collections.Generic.List[object]'

foreach ($item in $Task) {
    switch ($item) {
        'GitSummary' {
            [void]$records.Add((Invoke-GitSummaryBench -ProjectPath $projectPath))
        }
        'SessionFind' {
            [void]$records.Add((Invoke-SessionFindBench -ProjectPath $projectPath -Days $RecentDays -Limit $Limit))
        }
        'SearchText' {
            [void]$records.Add((Invoke-SearchTextBench -ProjectPath $projectPath -Pattern $Pattern -Roots $SearchRoots -MaxFiles $MaxFiles -MaxMatchesPerFile $MaxMatchesPerFile))
        }
        'FileOutline' {
            [void]$records.Add((Invoke-FileOutlineBench -ProjectPath $projectPath -RelativeOrFullPath $FilePath -MaxItems $MaxOutlineItems))
        }
        'RunCheck' {
            [void]$records.Add((Invoke-RunCheckBench -ProjectPath $projectPath -Command $CheckCommand -ArtifactRoot $ArtifactRoot -MaxDigestLines $MaxDigestLines))
        }
    }
}

if ($Json) {
    $output = $records | ConvertTo-Json -Depth 8
}
else {
    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add("# Operation Replacement Benchmark")
    [void]$lines.Add("")
    [void]$lines.Add("project: $projectPath")
    [void]$lines.Add("generated_at: $((Get-Date).ToString('s'))")
    foreach ($record in $records) {
        [void]$lines.Add("")
        [void]$lines.Add("## $($record.operation)")
        [void]$lines.Add("")
        [void]$lines.Add("- verdict: $($record.verdict)")
        if ($record.fallback_reason) {
            [void]$lines.Add("- fallback_reason: $($record.fallback_reason)")
        }
        [void]$lines.Add("- baseline_tokens: $($record.baseline.model_visible_tokens)")
        [void]$lines.Add("- candidate_tokens: $($record.candidate.model_visible_tokens)")
        [void]$lines.Add("- token_savings: $($record.token_savings.approx_tokens) ($($record.token_savings.percent)%)")
        [void]$lines.Add("- baseline_wall_ms: $($record.baseline.wall_ms)")
        [void]$lines.Add("- candidate_wall_ms: $($record.candidate.wall_ms)")
    }
    $output = $lines -join "`n"
}

if (-not [string]::IsNullOrWhiteSpace($OutFile)) {
    $outPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutFile)
    $outDir = Split-Path -Parent $outPath
    if (-not [string]::IsNullOrWhiteSpace($outDir)) {
        New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    }
    Set-Content -LiteralPath $outPath -Value $output -Encoding UTF8
}

$output
