param(
    [ValidateSet('Build', 'Scout', 'Status', 'Bench')]
    [string]$Mode = 'Scout',

    [string]$Project = (Get-Location).Path,

    [string]$Prompt = 'review the current repo changes and find the relevant implementation files',

    [string]$CacheRoot = (Join-Path (Join-Path $PSScriptRoot '..') 'logs/context-scout-shadow'),

    [int]$MaxFiles = 2500,

    [int]$MaxFileBytes = 200000,

    [int]$MaxAnchorsPerFile = 25,

    [int]$MaxOutputItems = 12,

    [int]$MaxOutputTokens = 2000,

    [switch]$SkipExternalTools,

    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-FullPath {
    param([string]$Path)

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

function ConvertTo-ApproxTokens {
    param([string]$Text)

    if ($null -eq $Text) {
        return 0
    }
    return [int][Math]::Ceiling($Text.Length / 4.0)
}

function Get-OutputTokenLimit {
    if ($MaxOutputTokens -le 0) {
        return [int]::MaxValue
    }
    return $MaxOutputTokens
}

function Get-ShortHash {
    param([string]$Text)

    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $sha = [System.Security.Cryptography.SHA1]::Create()
    try {
        return (($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join '').Substring(0, 12)
    }
    finally {
        $sha.Dispose()
    }
}

function ConvertTo-SafeName {
    param([string]$Text)

    $safe = ($Text -replace '[^A-Za-z0-9._-]+', '-').Trim('-')
    if ([string]::IsNullOrWhiteSpace($safe)) {
        return 'repo'
    }
    return $safe
}

function Resolve-ProcessCommand {
    param(
        [string]$FileName,
        [string[]]$Arguments
    )

    $command = Get-Command $FileName -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $command) {
        return [pscustomobject]@{
            file_name = $FileName
            arguments = @($Arguments)
        }
    }

    $source = [string]$command.Source
    $extension = [System.IO.Path]::GetExtension($source).ToLowerInvariant()
    if ($command.CommandType -eq 'ExternalScript' -or $extension -eq '.ps1') {
        return [pscustomobject]@{
            file_name = Resolve-PowerShellExecutable
            arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $source) + @($Arguments)
        }
    }
    if ($extension -eq '.cmd' -or $extension -eq '.bat') {
        return [pscustomobject]@{
            file_name = $env:ComSpec
            arguments = @('/d', '/c', 'call', $source) + @($Arguments)
        }
    }

    return [pscustomobject]@{
        file_name = $source
        arguments = @($Arguments)
    }
}

function Resolve-PowerShellExecutable {
    $repoPwsh = 'C:\Users\Oleh\Documents\GitHub\PowerShell\pwsh.exe'
    if (Test-Path -LiteralPath $repoPwsh) {
        return $repoPwsh
    }
    $pwsh = Get-Command pwsh -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $pwsh) {
        return [string]$pwsh.Source
    }
    return 'powershell'
}

function Invoke-CapturedCommand {
    param(
        [string]$Name,
        [string]$FileName,
        [string[]]$Arguments,
        [string]$WorkingDirectory,
        [string]$ArtifactDirectory,
        [string]$StandardInput = $null,
        [int]$TimeoutMs = 120000
    )

    New-Item -ItemType Directory -Path $ArtifactDirectory -Force | Out-Null
    $safeName = ConvertTo-SafeName $Name
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
    $stdoutPath = Join-Path $ArtifactDirectory "$stamp-$safeName.stdout.txt"
    $stderrPath = Join-Path $ArtifactDirectory "$stamp-$safeName.stderr.txt"

    $resolved = Resolve-ProcessCommand -FileName $FileName -Arguments $Arguments
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $resolved.file_name
    foreach ($arg in @($resolved.arguments)) {
        [void]$psi.ArgumentList.Add($arg)
    }
    $psi.WorkingDirectory = $WorkingDirectory
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.RedirectStandardInput = $null -ne $StandardInput
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $psi
    $startError = $null
    try {
        [void]$process.Start()
        if ($null -ne $StandardInput) {
            $process.StandardInput.Write($StandardInput)
            $process.StandardInput.Close()
        }
        if (-not $process.WaitForExit($TimeoutMs)) {
            try {
                $process.Kill($true)
            }
            catch {
                $process.Kill()
            }
            $startError = "timed out after $TimeoutMs ms"
        }
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $exitCode = if ($null -eq $startError) { $process.ExitCode } else { -1 }
    }
    catch {
        $stdout = ''
        $stderr = $_.Exception.Message
        $exitCode = -1
        $startError = $_.Exception.Message
    }
    finally {
        $sw.Stop()
        $process.Dispose()
    }

    Set-Content -LiteralPath $stdoutPath -Value $stdout -Encoding UTF8
    Set-Content -LiteralPath $stderrPath -Value $stderr -Encoding UTF8

    $visible = if ([string]::IsNullOrWhiteSpace($stderr)) { $stdout } else { ($stdout.TrimEnd() + "`n" + $stderr.TrimEnd()).Trim() }
    return [pscustomobject]@{
        name = $Name
        command = $FileName
        args = $Arguments
        exit_code = $exitCode
        wall_ms = [int64]$sw.ElapsedMilliseconds
        stdout_path = $stdoutPath
        stderr_path = $stderrPath
        stdout_chars = $stdout.Length
        stderr_chars = $stderr.Length
        model_visible_chars = $visible.Length
        model_visible_tokens = ConvertTo-ApproxTokens $visible
        output = $visible
        error = $startError
    }
}

function Invoke-GitText {
    param(
        [string]$RepoRoot,
        [string[]]$GitArgs
    )

    $oldPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        Push-Location $RepoRoot
        try {
            $output = & git @GitArgs 2>$null
        }
        finally {
            Pop-Location
        }
        if ($null -eq $output) {
            return @()
        }
        return @($output)
    }
    catch {
        return @()
    }
    finally {
        $ErrorActionPreference = $oldPreference
    }
}

function Resolve-RepoRoot {
    param([string]$ProjectPath)

    $full = Resolve-FullPath $ProjectPath
    if (-not (Test-Path -LiteralPath $full)) {
        throw "Project path not found: $full"
    }

    $root = @(Invoke-GitText -RepoRoot $full -GitArgs @('rev-parse', '--show-toplevel'))
    if ($root.Count -gt 0 -and -not [string]::IsNullOrWhiteSpace($root[0])) {
        return Resolve-FullPath $root[0]
    }
    return $full
}

function Get-RelativePath {
    param(
        [string]$Root,
        [string]$Path
    )

    $rootUri = [System.Uri]::new(($Root.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar))
    $pathUri = [System.Uri]::new($Path)
    return [System.Uri]::UnescapeDataString($rootUri.MakeRelativeUri($pathUri).ToString()).Replace('/', '\')
}

function ConvertTo-SlashPath {
    param([string]$Path)

    return $Path.Replace('\', '/')
}

function Get-RepoKey {
    param([string]$RepoRoot)

    $leaf = Split-Path -Leaf $RepoRoot
    return "$(ConvertTo-SafeName $leaf)-$(Get-ShortHash $RepoRoot)"
}

function Get-CachePaths {
    param(
        [string]$RepoRoot,
        [string]$Root
    )

    $cacheRootFull = Resolve-FullPath $Root
    $repoKey = Get-RepoKey $RepoRoot
    $repoCache = Join-Path $cacheRootFull $repoKey
    $artifacts = Join-Path $repoCache 'artifacts'
    New-Item -ItemType Directory -Path $artifacts -Force | Out-Null
    return [pscustomobject]@{
        repo_key = $repoKey
        cache_root = $repoCache
        artifacts = $artifacts
        index = Join-Path $repoCache 'repo-context-index.json'
        changed = Join-Path $repoCache 'changed-areas.json'
        bench = Join-Path $repoCache ("bench-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.json')
    }
}

function Test-SkippedPath {
    param([string]$RelPath)

    $slash = ConvertTo-SlashPath $RelPath
    return $slash -match '(^|/)(\.git|\.hg|\.svn|target|node_modules|dist|build|coverage|\.cache|\.pytest_cache|__pycache__|\.venv|logs|graphify-out|repomix-output|\.gsd)(/|$)'
}

function Get-Language {
    param([string]$RelPath)

    $ext = [System.IO.Path]::GetExtension($RelPath).ToLowerInvariant()
    switch ($ext) {
        '.rs' { 'rust'; break }
        '.toml' { 'toml'; break }
        '.md' { 'markdown'; break }
        '.json' { 'json'; break }
        '.yaml' { 'yaml'; break }
        '.yml' { 'yaml'; break }
        '.ts' { 'typescript'; break }
        '.tsx' { 'typescript'; break }
        '.js' { 'javascript'; break }
        '.jsx' { 'javascript'; break }
        '.py' { 'python'; break }
        '.ps1' { 'powershell'; break }
        '.sh' { 'shell'; break }
        '.cpp' { 'cpp'; break }
        '.hpp' { 'cpp'; break }
        '.h' { 'c'; break }
        '.c' { 'c'; break }
        default { if ($RelPath -match '(^|/)(Cargo\.toml|package\.json|AGENTS\.md|README\.md)$') { 'manifest' } else { 'other' } }
    }
}

function Get-AnchorPattern {
    param([string]$Language)

    switch ($Language) {
        'rust' { '^\s*(pub\s+)?((async|const|unsafe)\s+)*fn\s+([A-Za-z_][A-Za-z0-9_]*)|^\s*(pub\s+)?(struct|enum|trait|mod|impl)\s+([A-Za-z_][A-Za-z0-9_]*)' }
        'typescript' { '^\s*(export\s+)?(async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)|^\s*(export\s+)?(class|interface|type|enum)\s+([A-Za-z_$][A-Za-z0-9_$]*)|^\s*(export\s+)?const\s+([A-Za-z_$][A-Za-z0-9_$]*)' }
        'javascript' { '^\s*(export\s+)?(async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)|^\s*(export\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)|^\s*(export\s+)?const\s+([A-Za-z_$][A-Za-z0-9_$]*)' }
        'python' { '^\s*(class|def|async\s+def)\s+([A-Za-z_][A-Za-z0-9_]*)' }
        'powershell' { '^\s*function\s+([A-Za-z0-9_-]+)|^\s*param\s*\(' }
        'markdown' { '^\s{0,3}#{1,4}\s+(.+)$' }
        'toml' { '^\s*\[[^\]]+\]' }
        'json' { '^\s*"[^"]+"\s*:' }
        default { $null }
    }
}

function Get-FileAnchors {
    param(
        [string]$Path,
        [string]$Language,
        [int]$Limit
    )

    $pattern = Get-AnchorPattern $Language
    if ($null -eq $pattern) {
        return @()
    }

    try {
        $lines = [System.IO.File]::ReadLines($Path)
        $anchors = New-Object 'System.Collections.Generic.List[object]'
        $lineNumber = 0
        foreach ($line in $lines) {
            $lineNumber++
            if ($line -match $pattern) {
                $text = ($line -replace '\s+', ' ').Trim()
                if ($text.Length -gt 160) {
                    $text = $text.Substring(0, 157) + '...'
                }
                [void]$anchors.Add([pscustomobject]@{
                    line = $lineNumber
                    text = $text
                })
                if ($anchors.Count -ge $Limit) {
                    break
                }
            }
        }
        return $anchors.ToArray()
    }
    catch {
        return @()
    }
}

function Count-FileLines {
    param(
        [string]$Path,
        [int64]$Size,
        [int]$MaxBytes
    )

    if ($Size -gt $MaxBytes) {
        return $null
    }
    try {
        $count = 0
        foreach ($line in [System.IO.File]::ReadLines($Path)) {
            $count++
        }
        return $count
    }
    catch {
        return $null
    }
}

function Get-FileRecord {
    param(
        [string]$RepoRoot,
        [string]$RelPath,
        [int]$BytesLimit,
        [int]$AnchorLimit
    )

    $slash = ConvertTo-SlashPath $RelPath
    if ([string]::IsNullOrWhiteSpace($slash) -or (Test-SkippedPath $slash)) {
        return $null
    }
    $full = Join-Path $RepoRoot $slash
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
        return $null
    }

    $info = Get-Item -LiteralPath $full -ErrorAction SilentlyContinue
    if ($null -eq $info) {
        return $null
    }

    $language = Get-Language $slash
    $lineCount = Count-FileLines -Path $full -Size $info.Length -MaxBytes $BytesLimit
    $anchors = if ($info.Length -le $BytesLimit) {
        Get-FileAnchors -Path $full -Language $language -Limit $AnchorLimit
    }
    else {
        @()
    }

    return [pscustomobject]@{
        path = $slash
        language = $language
        extension = [System.IO.Path]::GetExtension($slash).ToLowerInvariant()
        bytes = [int64]$info.Length
        mtime_utc = $info.LastWriteTimeUtc.ToString('o')
        lines = $lineCount
        anchors = $anchors
    }
}

function Get-RepoFiles {
    param(
        [string]$RepoRoot,
        [int]$Limit
    )

    $paths = @()
    if (Get-Command rg -ErrorAction SilentlyContinue) {
        $old = Get-Location
        try {
            Set-Location $RepoRoot
            $paths = @(& rg --files --hidden 2>$null)
        }
        finally {
            Set-Location $old
        }
    }
    if ($paths.Count -eq 0) {
        $paths = Get-ChildItem -LiteralPath $RepoRoot -Recurse -File -Force -ErrorAction SilentlyContinue |
            ForEach-Object { Get-RelativePath -Root $RepoRoot -Path $_.FullName }
    }
    return @($paths |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and -not (Test-SkippedPath $_) } |
        Select-Object -First $Limit)
}

function Get-ToolAvailability {
    $names = @('git', 'rg', 'sqlite3', 'graphify', 'serena', 'repomix', 'gsd', 'gsd_exec', 'gsd_exec_search', 'gsd_resume')
    $items = foreach ($name in $names) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue | Select-Object -First 1
        [pscustomobject]@{
            name = $name
            available = $null -ne $cmd
            source = if ($cmd) { $cmd.Source } else { $null }
        }
    }
    return $items
}

function Get-ChangedAreas {
    param([string]$RepoRoot)

    $status = @(Invoke-GitText -RepoRoot $RepoRoot -GitArgs @('status', '--short', '--branch'))
    $diff = @(Invoke-GitText -RepoRoot $RepoRoot -GitArgs @('diff', '--name-only'))
    $staged = @(Invoke-GitText -RepoRoot $RepoRoot -GitArgs @('diff', '--name-only', '--cached'))
    $untrackedFiles = @(Invoke-GitText -RepoRoot $RepoRoot -GitArgs @('ls-files', '--others', '--exclude-standard'))
    $untracked = New-Object 'System.Collections.Generic.List[string]'
    $changed = New-Object 'System.Collections.Generic.List[string]'

    foreach ($line in $status) {
        if ($line.StartsWith('##')) {
            continue
        }
        if ($line.Length -lt 4) {
            continue
        }
        $path = $line.Substring(3).Trim()
        if ($path -match ' -> ') {
            $path = ($path -split ' -> ')[-1]
        }
        if ($line.StartsWith('??')) {
            if (-not $path.EndsWith('/') -and -not $path.EndsWith('\')) {
                [void]$untracked.Add($path)
                [void]$changed.Add($path)
            }
            continue
        }
        [void]$changed.Add($path)
    }
    foreach ($path in $diff + $staged + $untrackedFiles) {
        if (-not [string]::IsNullOrWhiteSpace($path)) {
            [void]$changed.Add($path)
        }
    }
    foreach ($path in $untrackedFiles) {
        if (-not [string]::IsNullOrWhiteSpace($path)) {
            [void]$untracked.Add($path)
        }
    }

    $unique = @($changed | Sort-Object -Unique)
    $dirs = @($unique |
        ForEach-Object {
            $slash = ConvertTo-SlashPath $_
            if ($slash -match '/') { $slash.Substring(0, $slash.LastIndexOf('/')) } else { '.' }
        } |
        Group-Object |
        Sort-Object Count -Descending |
        Select-Object -First 20 |
        ForEach-Object { [pscustomobject]@{ path = $_.Name; count = $_.Count } })

    return [pscustomobject]@{
        branch_line = @($status | Where-Object { $_.StartsWith('##') } | Select-Object -First 1)[0]
        changed_paths = $unique
        untracked_paths = @($untracked | Sort-Object -Unique)
        unstaged_paths = @($diff | Sort-Object -Unique)
        staged_paths = @($staged | Sort-Object -Unique)
        changed_dirs = $dirs
        status_text = ($status -join "`n")
    }
}

function Build-Index {
    param(
        [string]$RepoRoot,
        [object]$Cache,
        [int]$FileLimit,
        [int]$BytesLimit,
        [int]$AnchorLimit
    )

    $head = @(Invoke-GitText -RepoRoot $RepoRoot -GitArgs @('rev-parse', 'HEAD'))
    $files = Get-RepoFiles -RepoRoot $RepoRoot -Limit $FileLimit
    $inventory = New-Object 'System.Collections.Generic.List[object]'
    foreach ($rel in $files) {
        $record = Get-FileRecord -RepoRoot $RepoRoot -RelPath $rel -BytesLimit $BytesLimit -AnchorLimit $AnchorLimit
        if ($null -ne $record) {
            [void]$inventory.Add($record)
        }
    }

    $dirs = @($inventory |
        ForEach-Object {
            $p = $_.path
            if ($p -match '/') { $p.Substring(0, $p.IndexOf('/')) } else { '.' }
        } |
        Group-Object |
        Sort-Object Count -Descending |
        Select-Object -First 40 |
        ForEach-Object { [pscustomobject]@{ path = $_.Name; files = $_.Count } })

    $languages = @($inventory |
        Group-Object language |
        Sort-Object Count -Descending |
        ForEach-Object { [pscustomobject]@{ language = $_.Name; files = $_.Count } })

    $index = [pscustomobject]@{
        schema = 'repo_context_scout.v1'
        repo_root = $RepoRoot
        repo_key = $Cache.repo_key
        generated_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        git_head = if ($head.Count -gt 0) { $head[0] } else { $null }
        file_limit = $FileLimit
        indexed_files = $inventory.Count
        directories = $dirs
        languages = $languages
        tools = Get-ToolAvailability
        files = $inventory.ToArray()
    }

    New-Item -ItemType Directory -Path $Cache.cache_root -Force | Out-Null
    $index | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Cache.index -Encoding UTF8
    return $index
}

function Read-Index {
    param(
        [string]$RepoRoot,
        [object]$Cache
    )

    if (-not (Test-Path -LiteralPath $Cache.index)) {
        return Build-Index -RepoRoot $RepoRoot -Cache $Cache -FileLimit $MaxFiles -BytesLimit $MaxFileBytes -AnchorLimit $MaxAnchorsPerFile
    }
    $index = Get-Content -LiteralPath $Cache.index -Raw | ConvertFrom-Json
    $fileLimit = Get-ScoutProperty -InputObject $index -Name 'file_limit'
    if ($null -eq $fileLimit -or [int]$fileLimit -lt $MaxFiles) {
        return Build-Index -RepoRoot $RepoRoot -Cache $Cache -FileLimit $MaxFiles -BytesLimit $MaxFileBytes -AnchorLimit $MaxAnchorsPerFile
    }
    return $index
}

function Get-PromptTerms {
    param([string]$Text)

    if ($null -eq $Text) {
        $Text = ''
    }
    $stop = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($word in @('the', 'and', 'for', 'with', 'from', 'that', 'this', 'have', 'into', 'task', 'work', 'please', 'repo', 'code', 'file', 'files')) {
        [void]$stop.Add($word)
    }
    return @(([regex]::Matches($Text.ToLowerInvariant(), '[a-z0-9_][a-z0-9_-]{2,}') |
        ForEach-Object { $_.Value.Trim('-') } |
        Where-Object { -not $stop.Contains($_) } |
        Sort-Object -Unique) | Select-Object -First 30)
}

function Get-ScoutProperty {
    param(
        [object]$InputObject,
        [string]$Name
    )

    if ($null -eq $InputObject) {
        return $null
    }
    if ($InputObject -is [System.Collections.IDictionary]) {
        if ($InputObject.Contains($Name)) {
            return $InputObject[$Name]
        }
        return $null
    }

    $property = $InputObject.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Get-AnchorText {
    param([object]$Anchor)

    $text = Get-ScoutProperty -InputObject $Anchor -Name 'text'
    if ($null -eq $text) {
        return ''
    }
    return [string]$text
}

function Get-AnchorLine {
    param([object]$Anchor)

    $line = Get-ScoutProperty -InputObject $Anchor -Name 'line'
    if ($null -eq $line) {
        return '?'
    }
    return [string]$line
}

function Format-ReasonSummary {
    param([object[]]$Reasons)

    $summary = New-Object 'System.Collections.Generic.List[string]'
    foreach ($reason in @($Reasons)) {
        $text = [string]$reason
        if ([string]::IsNullOrWhiteSpace($text)) {
            continue
        }
        $short = switch -Regex ($text) {
            '^changed path$' { 'changed'; break }
            "^near changed dir (.+)$" { "near:$($Matches[1])"; break }
            "^path matches '(.+)'$" { "path:$($Matches[1])"; break }
            "^anchor matches '(.+)'$" { "anchor:$($Matches[1])"; break }
            '^project entrypoint$' { 'entry'; break }
            '^test area$' { 'tests'; break }
            '^first_moves history' { 'memory'; break }
            default { $text; break }
        }
        if (-not $summary.Contains($short)) {
            [void]$summary.Add($short)
        }
        if ($summary.Count -ge 5) {
            break
        }
    }
    return ($summary.ToArray() -join ',')
}

function Add-ChangedOverlayToIndex {
    param(
        [object]$Index,
        [object]$Changed
    )

    $known = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in @($Index.files)) {
        $path = Get-ScoutProperty -InputObject $file -Name 'path'
        if (-not [string]::IsNullOrWhiteSpace([string]$path)) {
            [void]$known.Add([string]$path)
        }
    }

    $overlays = New-Object 'System.Collections.Generic.List[object]'
    foreach ($changedPath in @($Changed.changed_paths)) {
        $slash = ConvertTo-SlashPath $changedPath
        if ($known.Contains($slash)) {
            continue
        }
        $record = Get-FileRecord -RepoRoot $Index.repo_root -RelPath $slash -BytesLimit $MaxFileBytes -AnchorLimit $MaxAnchorsPerFile
        if ($null -eq $record) {
            continue
        }
        [void]$overlays.Add($record)
        [void]$known.Add($slash)
    }

    if ($overlays.Count -eq 0) {
        return [pscustomobject]@{
            index = $Index
            overlay_paths = @()
        }
    }

    $properties = [ordered]@{}
    foreach ($property in $Index.PSObject.Properties) {
        $properties[$property.Name] = $property.Value
    }
    $fileList = New-Object 'System.Collections.Generic.List[object]'
    foreach ($file in @($Index.files)) {
        [void]$fileList.Add($file)
    }
    foreach ($file in $overlays.ToArray()) {
        [void]$fileList.Add($file)
    }
    $files = $fileList.ToArray()
    $properties['files'] = $files
    $properties['indexed_files'] = $files.Count

    return [pscustomobject]@{
        index = [pscustomobject]$properties
        overlay_paths = @($overlays.ToArray() | ForEach-Object { $_.path })
    }
}

function Score-Files {
    param(
        [object]$Index,
        [object]$Changed,
        [string[]]$Terms,
        [string]$Variant
    )

    $changedSet = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($p in @($Changed.changed_paths)) {
        [void]$changedSet.Add((ConvertTo-SlashPath $p))
    }
    $changedDirs = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($d in @($Changed.changed_dirs)) {
        [void]$changedDirs.Add([string]$d.path)
    }

    $scored = foreach ($file in @($Index.files)) {
        $score = 0.0
        $reasons = New-Object 'System.Collections.Generic.List[string]'
        $path = [string]$file.path
        $lowerPath = $path.ToLowerInvariant()

        if ($changedSet.Contains($path)) {
            $score += 120
            [void]$reasons.Add('changed path')
        }
        foreach ($dir in $changedDirs) {
            if ($dir -ne '.' -and $lowerPath.StartsWith($dir.ToLowerInvariant() + '/')) {
                $score += 30
                [void]$reasons.Add("near changed dir $dir")
                break
            }
        }
        foreach ($term in $Terms) {
            if ($lowerPath.Contains($term)) {
                $score += 18
                [void]$reasons.Add("path matches '$term'")
            }
            foreach ($anchor in @($file.anchors)) {
                $anchorText = Get-AnchorText $anchor
                if (-not [string]::IsNullOrWhiteSpace($anchorText) -and $anchorText.ToLowerInvariant().Contains($term)) {
                    $score += 14
                    [void]$reasons.Add("anchor matches '$term'")
                    break
                }
            }
        }
        if ($path -match '(^|/)(AGENTS\.md|README\.md|Cargo\.toml|package\.json|pyproject\.toml|justfile|Justfile)$') {
            $score += 8
            [void]$reasons.Add('project entrypoint')
        }
        if ($path -match '(^|/)(test|tests|__tests__|fixtures?)(/|$)' -and $Prompt -match '(test|verify|review|bug|fix)') {
            $score += 12
            [void]$reasons.Add('test area')
        }

        switch ($Variant) {
            'changed_area_scout' {
                if (-not $changedSet.Contains($path) -and $reasons.Count -eq 0) {
                    $score -= 20
                }
            }
            'topic_catalog_scout' {
                if ($reasons.Count -eq 0) {
                    $score -= 10
                }
                if ($path -match '(^|/)(docs?|README|AGENTS|Cargo|package|scripts?)(/|\.|$)') {
                    $score += 10
                }
            }
            'symbol_graph_lite_scout' {
                if (@($file.anchors).Count -eq 0) {
                    $score -= 15
                }
                else {
                    $score += [Math]::Min(15, @($file.anchors).Count)
                }
            }
        }

        if ($score -gt 0) {
            [pscustomobject]@{
                path = $path
                score = [Math]::Round($score, 2)
                language = $file.language
                bytes = $file.bytes
                lines = $file.lines
                anchors = @($file.anchors | Select-Object -First 5)
                reasons = @($reasons | Select-Object -Unique)
            }
        }
    }

    return @($scored | Sort-Object score -Descending | Select-Object -First $MaxOutputItems)
}

function Format-ScoutPacket {
    param(
        [string]$Name,
        [object]$Index,
        [object]$Changed,
        [object[]]$Candidates,
        [string[]]$Terms,
        [string[]]$Warnings
    )

    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add("<repo_context_scout variant=`"$Name`">")
    [void]$lines.Add("repo: $($Index.repo_key)")
    [void]$lines.Add("generated_at_utc: $($Index.generated_at_utc)")
    [void]$lines.Add("prompt_terms: $($Terms -join ', ')")
    $changedDirText = @($Changed.changed_dirs |
        Select-Object -First 6 |
        ForEach-Object { "$($_.path):$($_.count)" }) -join ', '
    if (@($Changed.changed_dirs).Count -gt 6) {
        $changedDirText = "$changedDirText, +$(@($Changed.changed_dirs).Count - 6)"
    }
    [void]$lines.Add("changed_paths: $(@($Changed.changed_paths).Count); changed_dirs: $changedDirText")
    if ($Warnings.Count -gt 0) {
        $visibleWarnings = @($Warnings | Select-Object -First 4)
        if ($Warnings.Count -gt $visibleWarnings.Count) {
            $visibleWarnings += "omitted $($Warnings.Count - $visibleWarnings.Count) more warnings"
        }
        [void]$lines.Add("warnings: $($visibleWarnings -join '; ')")
    }
    [void]$lines.Add("likely_files:")
    $renderedPaths = New-Object 'System.Collections.Generic.List[string]'
    $omittedCandidates = 0
    foreach ($candidate in $Candidates) {
        $candidatePath = [string](Get-ScoutProperty -InputObject $candidate -Name 'path')
        if ([string]::IsNullOrWhiteSpace($candidatePath)) {
            continue
        }
        $candidateScore = Get-ScoutProperty -InputObject $candidate -Name 'score'
        $candidateReasons = @(Get-ScoutProperty -InputObject $candidate -Name 'reasons')
        $candidateLines = New-Object 'System.Collections.Generic.List[string]'
        [void]$candidateLines.Add("- $candidatePath score=$candidateScore why=$(Format-ReasonSummary $candidateReasons)")
        foreach ($anchor in @((Get-ScoutProperty -InputObject $candidate -Name 'anchors') | Select-Object -First 2)) {
            $anchorText = Get-AnchorText $anchor
            if ([string]::IsNullOrWhiteSpace($anchorText)) {
                continue
            }
            [void]$candidateLines.Add("  anchor:$(Get-AnchorLine $anchor): $anchorText")
        }

        $trial = @($lines.ToArray()) + @($candidateLines.ToArray())
        $renderLimit = Get-OutputTokenLimit
        if ($renderLimit -lt [int]::MaxValue) {
            if ((ConvertTo-ApproxTokens ($trial -join "`n")) -gt [Math]::Max(200, $renderLimit - 160)) {
                $omittedCandidates++
                continue
            }
        }
        foreach ($line in $candidateLines.ToArray()) {
            [void]$lines.Add($line)
        }
        [void]$renderedPaths.Add($candidatePath)
    }
    if ($omittedCandidates -gt 0) {
        [void]$lines.Add("omitted_likely_files: $omittedCandidates due output budget")
    }
    [void]$lines.Add("suggested_first_reads:")
    foreach ($path in @($renderedPaths.ToArray() | Select-Object -First 6)) {
        [void]$lines.Add("- Get-Content -LiteralPath `"$path`" -TotalCount 220")
    }
    [void]$lines.Add("</repo_context_scout>")
    return ($lines -join "`n")
}

function New-ScoutRecord {
    param(
        [string]$Name,
        [object]$Index,
        [object]$Changed,
        [string[]]$Terms,
        [object[]]$Candidates,
    [string[]]$Warnings
    )

    $cleanCandidates = @($Candidates | Where-Object {
        $candidatePath = Get-ScoutProperty -InputObject $_ -Name 'path'
        -not [string]::IsNullOrWhiteSpace([string]$candidatePath)
    })
    $allCandidatePaths = @($cleanCandidates | ForEach-Object {
        ConvertTo-SlashPath ([string](Get-ScoutProperty -InputObject $_ -Name 'path'))
    })
    $changedPaths = @($Changed.changed_paths | ForEach-Object { ConvertTo-SlashPath $_ })
    $packet = Format-ScoutPacket -Name $Name -Index $Index -Changed $Changed -Candidates $cleanCandidates -Terms $Terms -Warnings $Warnings
    $visibleCandidatePaths = @([regex]::Matches($packet, '(?m)^- (?<path>.+?) score=') | ForEach-Object {
        ConvertTo-SlashPath $_.Groups['path'].Value
    })
    return [pscustomobject]@{
        name = $Name
        model_visible_chars = $packet.Length
        model_visible_tokens = ConvertTo-ApproxTokens $packet
        candidate_paths = $visibleCandidatePaths
        all_candidate_paths = $allCandidatePaths
        changed_paths_represented = @($changedPaths | Where-Object { $visibleCandidatePaths -contains $_ }).Count
        warnings = $Warnings
        packet = $packet
    }
}

function Get-SessionMemoryScout {
    param(
        [string]$RepoRoot,
        [object]$Index
    )

    $db = Join-Path $RepoRoot '.first_moves.db'
    if (-not (Test-Path -LiteralPath $db) -or -not (Get-Command sqlite3 -ErrorAction SilentlyContinue)) {
        return @()
    }
    $rows = @(& sqlite3 $db "select path, observed, hit_count from path_freq order by hit_count desc, observed desc limit 20;" 2>$null)
    $known = [System.Collections.Generic.Dictionary[string, object]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in @($Index.files)) {
        $known[[string]$file.path] = $file
    }
    $items = New-Object 'System.Collections.Generic.List[object]'
    foreach ($row in $rows) {
        $parts = $row -split '\|'
        if ($parts.Count -lt 1) {
            continue
        }
        $path = ConvertTo-SlashPath $parts[0]
        if (-not $known.ContainsKey($path)) {
            continue
        }
        $file = $known[$path]
        $observed = if ($parts.Count -gt 1) { [int]$parts[1] } else { 0 }
        $hits = if ($parts.Count -gt 2) { [int]$parts[2] } else { 0 }
        [void]$items.Add([pscustomobject]@{
            path = $path
            score = [Math]::Round(20 + $hits * 10 + $observed, 2)
            language = $file.language
            bytes = $file.bytes
            lines = $file.lines
            anchors = @($file.anchors | Select-Object -First 5)
            reasons = @("first_moves history observed=$observed hits=$hits")
        })
    }
    return @($items | Sort-Object score -Descending | Select-Object -First $MaxOutputItems)
}

function New-RawExplorationRecord {
    param(
        [string]$RepoRoot,
        [object]$Changed,
        [object]$Index
    )

    $files = @($Index.files | Select-Object -First 120 | ForEach-Object { $_.path })
    $entry = @($Index.files |
        Where-Object { $_.path -match '(^|/)(AGENTS\.md|README\.md|Cargo\.toml|package\.json|pyproject\.toml|justfile|Justfile)$' } |
        Select-Object -First 20 |
        ForEach-Object { $_.path })
    $lines = @(
        'RAW_EXPLORATION_BASELINE'
        'git status --short --branch:'
        $Changed.status_text
        ''
        'rg --files sample:'
        ($files -join "`n")
        ''
        'root manifests/docs:'
        ($entry -join "`n")
    )
    $text = ($lines -join "`n")
    return [pscustomobject]@{
        name = 'raw_exploration_pack'
        model_visible_chars = $text.Length
        model_visible_tokens = ConvertTo-ApproxTokens $text
        candidate_paths = $files
        packet = $text
    }
}

function Write-ChangedAreas {
    param(
        [object]$Changed,
        [object]$Cache
    )

    $Changed | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $Cache.changed -Encoding UTF8
}

function Get-ScoutRecords {
    param(
        [object]$Index,
        [object]$Changed
    )

    $overlay = Add-ChangedOverlayToIndex -Index $Index -Changed $Changed
    $Index = $overlay.index
    $terms = Get-PromptTerms $Prompt
    $warnings = New-Object 'System.Collections.Generic.List[string]'
    if (@($overlay.overlay_paths).Count -gt 0) {
        [void]$warnings.Add("added changed-file overlay: $(@($overlay.overlay_paths).Count) paths")
    }
    $head = @(Invoke-GitText -RepoRoot $Index.repo_root -GitArgs @('rev-parse', 'HEAD'))
    if ($head.Count -gt 0 -and $Index.git_head -and $head[0] -ne $Index.git_head) {
        [void]$warnings.Add('index HEAD differs from current HEAD')
    }
    $indexedPaths = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in @($Index.files)) {
        if ($file.path) {
            [void]$indexedPaths.Add([string]$file.path)
        }
    }
    foreach ($path in @($Changed.changed_paths)) {
        $slash = ConvertTo-SlashPath $path
        if (-not $indexedPaths.Contains($slash)) {
            [void]$warnings.Add("changed path missing from index: $slash")
        }
    }

    $records = New-Object 'System.Collections.Generic.List[object]'
    foreach ($variant in @('inventory_anchor_scout', 'changed_area_scout', 'topic_catalog_scout', 'symbol_graph_lite_scout')) {
        $candidates = Score-Files -Index $Index -Changed $Changed -Terms $terms -Variant $variant
        [void]$records.Add((New-ScoutRecord -Name $variant -Index $Index -Changed $Changed -Terms $terms -Candidates $candidates -Warnings $warnings.ToArray()))
    }

    $sessionCandidates = Get-SessionMemoryScout -RepoRoot $Index.repo_root -Index $Index
    [void]$records.Add((New-ScoutRecord -Name 'session_memory_scout' -Index $Index -Changed $Changed -Terms $terms -Candidates $sessionCandidates -Warnings $warnings.ToArray()))

    $hybrid = @(
        (Score-Files -Index $Index -Changed $Changed -Terms $terms -Variant 'changed_area_scout') +
        (Score-Files -Index $Index -Changed $Changed -Terms $terms -Variant 'symbol_graph_lite_scout') +
        $sessionCandidates
    ) |
        Group-Object path |
        ForEach-Object {
            $first = $_.Group[0]
            $score = ($_.Group | Measure-Object score -Sum).Sum
            [pscustomobject]@{
                path = $first.path
                score = [Math]::Round($score, 2)
                language = $first.language
                bytes = $first.bytes
                lines = $first.lines
                anchors = $first.anchors
                reasons = @($_.Group | ForEach-Object { $_.reasons } | Select-Object -Unique)
            }
        } |
        Sort-Object score -Descending |
        Select-Object -First $MaxOutputItems
    [void]$records.Add((New-ScoutRecord -Name 'hybrid_ranked_scout' -Index $Index -Changed $Changed -Terms $terms -Candidates $hybrid -Warnings $warnings.ToArray()))

    return $records.ToArray()
}

function New-BenchComparison {
    param(
        [object]$Baseline,
        [object]$Candidate,
        [object]$Changed
    )

    $saved = [Math]::Max(0, $Baseline.model_visible_tokens - $Candidate.model_visible_tokens)
    $percent = if ($Baseline.model_visible_tokens -gt 0) {
        [Math]::Round(($saved * 100.0) / $Baseline.model_visible_tokens, 1)
    }
    else {
        0
    }
    $candidatePaths = @($Candidate.candidate_paths)
    $changedPaths = @($Changed.changed_paths | ForEach-Object { ConvertTo-SlashPath $_ })
    $represented = @($changedPaths | Where-Object { $candidatePaths -contains $_ })
    $tool = Get-ScoutProperty -InputObject $Candidate -Name 'tool'
    $toolExit = Get-ScoutProperty -InputObject $tool -Name 'exit_code'
    $candidateError = Get-ScoutProperty -InputObject $Candidate -Name 'error'
    $usageRole = [string](Get-ScoutProperty -InputObject $Candidate -Name 'usage_role')
    $outputLimit = Get-OutputTokenLimit
    $verdict = if ($null -ne $toolExit -and [int]$toolExit -ne 0) {
        'fail_tool'
    }
    elseif (-not [string]::IsNullOrWhiteSpace([string]$candidateError)) {
        'fail_tool'
    }
    elseif ($usageRole -match '_prompt$' -and $candidatePaths.Count -gt 0 -and $Candidate.model_visible_tokens -le $outputLimit) {
        'pass_support_prompt'
    }
    elseif ($usageRole -match '_prompt$') {
        'needs_more_data'
    }
    elseif ($Candidate.name -match '_scout$' -and $candidatePaths.Count -eq 0) {
        'fail_quality'
    }
    elseif ($outputLimit -lt [int]::MaxValue -and $Candidate.model_visible_tokens -gt $outputLimit) {
        'fail_tokens'
    }
    elseif ($changedPaths.Count -gt 0 -and $represented.Count -eq 0 -and $Candidate.name -notmatch 'raw|gsd|graphify|repomix|serena') {
        'fail_quality'
    }
    elseif ($percent -ge 50 -and $candidatePaths.Count -gt 0) {
        'pass_shadow_candidate'
    }
    else {
        'needs_more_data'
    }

    return [pscustomobject]@{
        name = $Candidate.name
        baseline_tokens = $Baseline.model_visible_tokens
        candidate_tokens = $Candidate.model_visible_tokens
        saved_tokens = $saved
        savings_percent = $percent
        changed_paths = $changedPaths.Count
        changed_paths_represented = $represented.Count
        usage_role = if ([string]::IsNullOrWhiteSpace($usageRole)) { 'direct_context_candidate' } else { $usageRole }
        verdict = $verdict
    }
}

function Copy-GraphifySample {
    param(
        [string]$RepoRoot,
        [object[]]$Candidates,
        [string]$ArtifactRoot
    )

    $sample = Join-Path $ArtifactRoot 'graphify-sample'
    if (Test-Path -LiteralPath $sample) {
        Remove-Item -LiteralPath $sample -Recurse -Force
    }
    New-Item -ItemType Directory -Path $sample -Force | Out-Null
    foreach ($candidate in @($Candidates | Select-Object -First 30)) {
        $rel = [string]$candidate.path
        $src = Join-Path $RepoRoot $rel
        if (-not (Test-Path -LiteralPath $src -PathType Leaf)) {
            continue
        }
        $dest = Join-Path $sample $rel
        New-Item -ItemType Directory -Path (Split-Path -Parent $dest) -Force | Out-Null
        Copy-Item -LiteralPath $src -Destination $dest -Force
    }
    return $sample
}

function Get-GsdStdoutArtifactPath {
    param([string]$GsdOutput)

    if ([string]::IsNullOrWhiteSpace($GsdOutput)) {
        return $null
    }
    $match = [regex]::Match($GsdOutput, 'stdout:\s+\d+B\s+.*?(?<path>[A-Za-z]:\\[^\r\n]+\.stdout)')
    if ($match.Success) {
        return $match.Groups['path'].Value.Trim()
    }
    return $null
}

function Get-ExistingPathHintsFromText {
    param(
        [string]$RepoRoot,
        [string]$Text,
        [int]$Limit = 40
    )

    $seen = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $paths = New-Object 'System.Collections.Generic.List[string]'
    foreach ($rawLine in @($Text -split "`r?`n")) {
        $line = $rawLine.Trim()
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        if ($line -match '^(gsd_exec\[|stdout:|stderr:|--- digest ---|git status|rg --files|## )') {
            continue
        }

        $candidate = $line
        if ($candidate -match '^(.{1,2})\s+(?<path>.+)$') {
            $candidate = $Matches['path']
        }
        if ($candidate -match ' -> ') {
            $candidate = ($candidate -split ' -> ')[-1]
        }
        $candidate = $candidate.Trim().Trim('"')
        if ([string]::IsNullOrWhiteSpace($candidate)) {
            continue
        }

        $slash = ConvertTo-SlashPath $candidate
        if (Test-SkippedPath $slash) {
            continue
        }
        $full = Join-Path $RepoRoot $slash
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
            continue
        }
        if ($seen.Add($slash)) {
            [void]$paths.Add($slash)
            if ($paths.Count -ge $Limit) {
                break
            }
        }
    }
    return $paths.ToArray()
}

function New-GsdExplorationPromptRecord {
    param(
        [string]$RepoRoot,
        [string]$TaskPrompt,
        [object]$Result
    )

    $stdoutArtifact = Get-GsdStdoutArtifactPath -GsdOutput $Result.output
    $artifactText = ''
    if (-not [string]::IsNullOrWhiteSpace($stdoutArtifact) -and (Test-Path -LiteralPath $stdoutArtifact)) {
        $artifactText = Get-Content -LiteralPath $stdoutArtifact -Raw
    }
    $pathHints = Get-ExistingPathHintsFromText -RepoRoot $RepoRoot -Text (($Result.output, $artifactText) -join "`n") -Limit 24
    $handle = if ($Result.output -match '^gsd_exec\[(?<id>[^\]]+)\]') { $Matches['id'] } else { $null }
    $digestLines = @($Result.output -split "`r?`n" | Select-Object -First 12)
    $shortPrompt = if ($TaskPrompt.Length -gt 220) { $TaskPrompt.Substring(0, 217) + '...' } else { $TaskPrompt }

    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add('<gsd2_artifact_exploration_prompt>')
    [void]$lines.Add('role: artifact-backed exploration aid')
    [void]$lines.Add("task_prompt: $shortPrompt")
    if (-not [string]::IsNullOrWhiteSpace($handle)) {
        [void]$lines.Add("gsd_exec_id: $handle")
    }
    if (-not [string]::IsNullOrWhiteSpace($stdoutArtifact)) {
        [void]$lines.Add("stdout_artifact: $stdoutArtifact")
    }
    [void]$lines.Add('use_before_rescan:')
    [void]$lines.Add('- Treat digest paths as weak hints, not a ranked repo map.')
    [void]$lines.Add('- Inspect the artifact or search prior GSD2 exec output before rerunning broad git status or rg --files exploration.')
    [void]$lines.Add('- Combine these hints with repo_context_scout paths; prefer changed paths and prompt-matching anchors.')
    [void]$lines.Add('visible_digest:')
    foreach ($line in $digestLines) {
        [void]$lines.Add("> $line")
    }
    if ($pathHints.Count -gt 0) {
        [void]$lines.Add("path_hints: $($pathHints.Count) shown")
        foreach ($path in $pathHints) {
            [void]$lines.Add("- $path")
        }
    }
    else {
        [void]$lines.Add('path_hints: none parsed from visible digest or artifact')
    }
    [void]$lines.Add('</gsd2_artifact_exploration_prompt>')
    $packet = $lines -join "`n"

    return [pscustomobject]@{
        name = 'gsd2_artifact_exploration_prompt'
        model_visible_chars = $packet.Length
        model_visible_tokens = ConvertTo-ApproxTokens $packet
        candidate_paths = $pathHints
        packet = $packet
        tool = $Result
        artifact_path = $stdoutArtifact
        usage_role = 'artifact_exploration_prompt'
        note = 'GSD2 is evaluated as exploration prompt support, not as a standalone context selector.'
    }
}

function Get-GraphifyPathHintsFromText {
    param(
        [string]$RepoRoot,
        [string]$Text,
        [int]$Limit = 40
    )

    $seen = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $paths = New-Object 'System.Collections.Generic.List[string]'
    foreach ($match in [regex]::Matches($Text, '\bsrc=(?<path>[^\]\s]+)')) {
        $path = ConvertTo-SlashPath $match.Groups['path'].Value.Trim().Trim('"')
        if ([string]::IsNullOrWhiteSpace($path) -or (Test-SkippedPath $path)) {
            continue
        }
        if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot $path) -PathType Leaf)) {
            continue
        }
        if ($seen.Add($path)) {
            [void]$paths.Add($path)
            if ($paths.Count -ge $Limit) {
                break
            }
        }
    }
    return $paths.ToArray()
}

function New-GraphifyTopologyPromptRecord {
    param(
        [string]$RepoRoot,
        [string]$TaskPrompt,
        [object]$QueryResult,
        [string]$GraphPath,
        [string]$SamplePath
    )

    $pathHints = Get-GraphifyPathHintsFromText -RepoRoot $RepoRoot -Text $QueryResult.output -Limit 24
    $queryLines = @($QueryResult.output -split "`r?`n" | Select-Object -First 36)
    $shortPrompt = if ($TaskPrompt.Length -gt 220) { $TaskPrompt.Substring(0, 217) + '...' } else { $TaskPrompt }
    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add('<graphify_topology_prompt>')
    [void]$lines.Add('role: graph topology and relation aid')
    [void]$lines.Add("task_prompt: $shortPrompt")
    [void]$lines.Add("graph_artifact: $GraphPath")
    [void]$lines.Add("sample_root: $SamplePath")
    [void]$lines.Add('use_before_rescan:')
    [void]$lines.Add('- Use node and edge lines to understand related symbols and likely cross-file flow.')
    [void]$lines.Add('- Treat src= paths as high-value first reads only after checking changed-file overlay freshness.')
    [void]$lines.Add('- If the relationship matters, run graphify explain/path on the graph artifact before broad rg sweeps.')
    if ($pathHints.Count -gt 0) {
        [void]$lines.Add("path_hints: $($pathHints.Count) shown")
        foreach ($path in $pathHints) {
            [void]$lines.Add("- $path")
        }
    }
    else {
        [void]$lines.Add('path_hints: none parsed from graph query output')
    }
    [void]$lines.Add('visible_topology_digest:')
    foreach ($line in $queryLines) {
        [void]$lines.Add("> $line")
    }
    [void]$lines.Add('</graphify_topology_prompt>')
    $packet = $lines -join "`n"

    return [pscustomobject]@{
        name = 'graphify_topology_prompt'
        model_visible_chars = $packet.Length
        model_visible_tokens = ConvertTo-ApproxTokens $packet
        candidate_paths = $pathHints
        packet = $packet
        tool = $QueryResult
        artifact_path = $GraphPath
        usage_role = 'topology_prompt'
        note = 'Graphify is evaluated as a relation/topology prompt, not a raw repo selector.'
    }
}

function New-RepomixArtifactPromptRecord {
    param(
        [string]$TaskPrompt,
        [object]$HybridRecord,
        [object]$Result,
        [string]$ArtifactPath,
        [string]$Packet
    )

    $paths = @($HybridRecord.candidate_paths | Select-Object -First 24)
    $fileHeaders = @([regex]::Matches($Packet, '(?m)^## File: (?<path>.+)$') | ForEach-Object { $_.Groups['path'].Value.Trim() } | Select-Object -First 24)
    if ($fileHeaders.Count -gt 0) {
        $paths = @($fileHeaders)
    }
    $shortPrompt = if ($TaskPrompt.Length -gt 220) { $TaskPrompt.Substring(0, 217) + '...' } else { $TaskPrompt }
    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add('<repomix_artifact_context_prompt>')
    [void]$lines.Add('role: scoped packed artifact and token accounting aid')
    [void]$lines.Add("task_prompt: $shortPrompt")
    [void]$lines.Add("artifact_path: $ArtifactPath")
    [void]$lines.Add("artifact_tokens_estimated: $(ConvertTo-ApproxTokens $Packet)")
    [void]$lines.Add('use_before_rescan:')
    [void]$lines.Add('- Do not inject the whole artifact by default; it can be much larger than the scout packet.')
    [void]$lines.Add('- Use the artifact when a reviewer or parallel agent needs an auditable scoped snapshot.')
    [void]$lines.Add('- Read only specific file sections from the artifact, or open the source files directly when editing.')
    if ($paths.Count -gt 0) {
        [void]$lines.Add("included_or_selected_paths: $($paths.Count) shown")
        foreach ($path in $paths) {
            [void]$lines.Add("- $(ConvertTo-SlashPath $path)")
        }
    }
    else {
        [void]$lines.Add('included_or_selected_paths: none parsed')
    }
    [void]$lines.Add('</repomix_artifact_context_prompt>')
    $promptPacket = $lines -join "`n"

    return [pscustomobject]@{
        name = 'repomix_artifact_context_prompt'
        model_visible_chars = $promptPacket.Length
        model_visible_tokens = ConvertTo-ApproxTokens $promptPacket
        candidate_paths = @($paths | ForEach-Object { ConvertTo-SlashPath $_ })
        packet = $promptPacket
        tool = $Result
        artifact_path = $ArtifactPath
        source_artifact_tokens = ConvertTo-ApproxTokens $Packet
        usage_role = 'artifact_context_prompt'
        note = 'Repomix is evaluated as an artifact handle plus scoped path list, not as default prompt content.'
    }
}

function New-SerenaSemanticPromptRecord {
    param(
        [string]$TaskPrompt,
        [object]$HybridRecord,
        [object]$ToolCatalogResult
    )

    $terms = Get-PromptTerms $TaskPrompt
    $paths = @($HybridRecord.candidate_paths | Select-Object -First 12)
    $hasTools = $ToolCatalogResult.output -match 'find_symbol' -and $ToolCatalogResult.output -match 'get_symbols_overview'
    $shortPrompt = if ($TaskPrompt.Length -gt 220) { $TaskPrompt.Substring(0, 217) + '...' } else { $TaskPrompt }
    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add('<serena_semantic_lookup_prompt>')
    [void]$lines.Add('role: semantic symbol lookup route')
    [void]$lines.Add("task_prompt: $shortPrompt")
    [void]$lines.Add("tools_available: $hasTools")
    [void]$lines.Add('use_before_rescan:')
    [void]$lines.Add('- Activate the project, then use get_symbols_overview on the top scout paths before whole-file reads.')
    [void]$lines.Add('- Use find_symbol for prompt terms and find_referencing_symbols for cross-file impact questions.')
    [void]$lines.Add('- Fall back to raw file reads when the language server is cold, unavailable, or misses changed untracked files.')
    if ($terms.Count -gt 0) {
        [void]$lines.Add("symbol_query_terms: $($terms -join ', ')")
    }
    if ($paths.Count -gt 0) {
        [void]$lines.Add("overview_candidate_paths: $($paths.Count) shown")
        foreach ($path in $paths) {
            [void]$lines.Add("- $path")
        }
    }
    else {
        [void]$lines.Add('overview_candidate_paths: none')
    }
    [void]$lines.Add('</serena_semantic_lookup_prompt>')
    $packet = $lines -join "`n"

    return [pscustomobject]@{
        name = 'serena_semantic_lookup_prompt'
        model_visible_chars = $packet.Length
        model_visible_tokens = ConvertTo-ApproxTokens $packet
        candidate_paths = $paths
        packet = $packet
        tool = $ToolCatalogResult
        usage_role = 'semantic_lookup_prompt'
        note = 'Serena is evaluated as a semantic lookup route; MCP symbol calls need a separate quality benchmark.'
    }
}

function Invoke-ExternalBenchmarks {
    param(
        [string]$RepoRoot,
        [object]$Cache,
        [object]$Index,
        [object]$Changed,
        [object]$HybridRecord
    )

    $records = New-Object 'System.Collections.Generic.List[object]'
    if ($SkipExternalTools) {
        return $records.ToArray()
    }

    if (Get-Command gsd_exec -ErrorAction SilentlyContinue) {
        $script = @"
const cp = require('node:child_process');
function run(cmd) {
  try { return cp.execSync(cmd, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }); }
  catch (err) { return String(err.stdout || '') + String(err.stderr || err.message || ''); }
}
console.log('git status --short --branch');
console.log(run('git status --short --branch'));
console.log('rg --files sample');
console.log(run('rg --files').split(/\r?\n/).slice(0, 120).join('\n'));
"@
        $result = Invoke-CapturedCommand -Name 'gsd_exec_raw_exploration_digest' -FileName 'gsd_exec' -Arguments @('--project', $RepoRoot, '--purpose', 'context-scout-bench', 'node', $script) -WorkingDirectory $RepoRoot -ArtifactDirectory $Cache.artifacts -TimeoutMs 120000
        [void]$records.Add([pscustomobject]@{
            name = 'gsd_exec_raw_exploration_digest'
            model_visible_chars = $result.model_visible_chars
            model_visible_tokens = $result.model_visible_tokens
            candidate_paths = @()
            packet = $result.output
            tool = $result
        })
        [void]$records.Add((New-GsdExplorationPromptRecord -RepoRoot $RepoRoot -TaskPrompt $Prompt -Result $result))
    }

    if ((Get-Command graphify -ErrorAction SilentlyContinue) -and @($HybridRecord.candidate_paths).Count -gt 0) {
        $sample = Copy-GraphifySample -RepoRoot $RepoRoot -Candidates @($HybridRecord.candidate_paths | ForEach-Object { [pscustomobject]@{ path = $_ } }) -ArtifactRoot $Cache.artifacts
        $update = Invoke-CapturedCommand -Name 'graphify_update_sample' -FileName 'graphify' -Arguments @('update', $sample, '--force') -WorkingDirectory $sample -ArtifactDirectory $Cache.artifacts -TimeoutMs 180000
        $graphPath = Join-Path $sample 'graphify-out\graph.json'
        if (Test-Path -LiteralPath $graphPath) {
            $query = Invoke-CapturedCommand -Name 'graphify_query_sample' -FileName 'graphify' -Arguments @('query', $Prompt, '--budget', ([string]$MaxOutputTokens), '--graph', $graphPath) -WorkingDirectory $sample -ArtifactDirectory $Cache.artifacts -TimeoutMs 120000
            $graphPaths = Get-GraphifyPathHintsFromText -RepoRoot $RepoRoot -Text $query.output -Limit 40
            [void]$records.Add([pscustomobject]@{
                name = 'graphify_query_sample'
                model_visible_chars = $query.model_visible_chars
                model_visible_tokens = $query.model_visible_tokens
                candidate_paths = $graphPaths
                packet = $query.output
                tool = [pscustomobject]@{ update = $update; query = $query; sample = $sample }
                artifact_path = $graphPath
                usage_role = 'topology_query'
            })
            [void]$records.Add((New-GraphifyTopologyPromptRecord -RepoRoot $RepoRoot -TaskPrompt $Prompt -QueryResult $query -GraphPath $graphPath -SamplePath $sample))
        }
        else {
            [void]$records.Add([pscustomobject]@{
                name = 'graphify_query_sample'
                model_visible_chars = $update.model_visible_chars
                model_visible_tokens = $update.model_visible_tokens
                candidate_paths = @()
                packet = $update.output
                tool = $update
                error = 'graph.json not produced'
            })
        }
    }

    if ((Get-Command repomix -ErrorAction SilentlyContinue) -and @($HybridRecord.candidate_paths).Count -gt 0) {
        $out = Join-Path $Cache.artifacts 'repomix-scout.md'
        $include = (@($HybridRecord.candidate_paths | Select-Object -First 40) -join ',')
        $result = Invoke-CapturedCommand -Name 'repomix_compressed_selected' -FileName 'repomix' -Arguments @('--include', $include, '--compress', '--style', 'markdown', '--output', $out, '--quiet') -WorkingDirectory $RepoRoot -ArtifactDirectory $Cache.artifacts -TimeoutMs 180000
        $packet = if (Test-Path -LiteralPath $out) {
            Get-Content -LiteralPath $out -Raw
        }
        else {
            $result.output
        }
        [void]$records.Add([pscustomobject]@{
            name = 'repomix_compressed_selected'
            model_visible_chars = $packet.Length
            model_visible_tokens = ConvertTo-ApproxTokens $packet
            candidate_paths = @($HybridRecord.candidate_paths)
            packet = $packet
            tool = $result
            artifact_path = $out
        })
        [void]$records.Add((New-RepomixArtifactPromptRecord -TaskPrompt $Prompt -HybridRecord $HybridRecord -Result $result -ArtifactPath $out -Packet $packet))
    }

    if (Get-Command serena -ErrorAction SilentlyContinue) {
        $result = Invoke-CapturedCommand -Name 'serena_tool_catalog' -FileName 'serena' -Arguments @('tools', 'list', '--all') -WorkingDirectory $RepoRoot -ArtifactDirectory $Cache.artifacts -TimeoutMs 60000
        [void]$records.Add([pscustomobject]@{
            name = 'serena_tool_catalog'
            model_visible_chars = $result.model_visible_chars
            model_visible_tokens = $result.model_visible_tokens
            candidate_paths = @()
            packet = $result.output
            tool = $result
            note = 'CLI exposes symbolic tool catalog; prompt-specific retrieval requires MCP tool invocation.'
        })
        [void]$records.Add((New-SerenaSemanticPromptRecord -TaskPrompt $Prompt -HybridRecord $HybridRecord -ToolCatalogResult $result))
    }

    return $records.ToArray()
}

function Invoke-Bench {
    param(
        [string]$RepoRoot,
        [object]$Cache
    )

    $index = Read-Index -RepoRoot $RepoRoot -Cache $Cache
    $changed = Get-ChangedAreas -RepoRoot $RepoRoot
    Write-ChangedAreas -Changed $changed -Cache $Cache
    $raw = New-RawExplorationRecord -RepoRoot $RepoRoot -Changed $changed -Index $index
    $scouts = Get-ScoutRecords -Index $index -Changed $changed
    $hybrid = @($scouts | Where-Object { $_.name -eq 'hybrid_ranked_scout' } | Select-Object -First 1)[0]
    $external = Invoke-ExternalBenchmarks -RepoRoot $RepoRoot -Cache $Cache -Index $index -Changed $changed -HybridRecord $hybrid
    $allCandidates = @($scouts + $external)
    $comparisons = @($allCandidates | ForEach-Object { New-BenchComparison -Baseline $raw -Candidate $_ -Changed $changed })

    $bench = [pscustomobject]@{
        schema = 'repo_context_scout_bench.v1'
        repo_root = $RepoRoot
        repo_key = $Cache.repo_key
        prompt = $Prompt
        generated_at_utc = (Get-Date).ToUniversalTime().ToString('o')
        baseline = $raw
        candidates = $allCandidates
        comparisons = $comparisons
        selected = @($comparisons |
            Where-Object { $_.verdict -eq 'pass_shadow_candidate' } |
            Sort-Object savings_percent -Descending |
            Select-Object -First 1)
    }
    $bench | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $Cache.bench -Encoding UTF8
    return $bench
}

function Write-Result {
    param([object]$Value)

    if ($Json) {
        $Value | ConvertTo-Json -Depth 10
        return
    }

    switch ($Mode) {
        'Build' {
            "Built repo context index for $($Value.repo_key): $($Value.indexed_files) files -> $($Cache.index)"
        }
        'Status' {
            "Repo context index: $($Value.repo_key)"
            "Indexed files: $($Value.index.indexed_files)"
            "Changed paths: $(@($Value.changed.changed_paths).Count)"
            "Cache root: $($Cache.cache_root)"
        }
        'Scout' {
            foreach ($record in @($Value.scouts)) {
                $record.packet
                ''
            }
        }
        'Bench' {
            "Benchmark: $($Value.repo_key)"
            "Baseline tokens: $($Value.baseline.model_visible_tokens)"
            foreach ($comparison in @($Value.comparisons | Sort-Object savings_percent -Descending)) {
                "{0}: {1} tokens, saved {2}% verdict={3}" -f $comparison.name, $comparison.candidate_tokens, $comparison.savings_percent, $comparison.verdict
            }
            "Bench file: $($Cache.bench)"
        }
    }
}

$repoRoot = Resolve-RepoRoot $Project
$Cache = Get-CachePaths -RepoRoot $repoRoot -Root $CacheRoot

switch ($Mode) {
    'Build' {
        $result = Build-Index -RepoRoot $repoRoot -Cache $Cache -FileLimit $MaxFiles -BytesLimit $MaxFileBytes -AnchorLimit $MaxAnchorsPerFile
    }
    'Status' {
        $index = Read-Index -RepoRoot $repoRoot -Cache $Cache
        $changed = Get-ChangedAreas -RepoRoot $repoRoot
        Write-ChangedAreas -Changed $changed -Cache $Cache
        $result = [pscustomobject]@{
            repo_key = $Cache.repo_key
            index = $index
            changed = $changed
            cache_root = $Cache.cache_root
        }
    }
    'Scout' {
        $index = Read-Index -RepoRoot $repoRoot -Cache $Cache
        $changed = Get-ChangedAreas -RepoRoot $repoRoot
        Write-ChangedAreas -Changed $changed -Cache $Cache
        $result = [pscustomobject]@{
            repo_key = $Cache.repo_key
            scouts = Get-ScoutRecords -Index $index -Changed $changed
            cache_root = $Cache.cache_root
        }
    }
    'Bench' {
        $result = Invoke-Bench -RepoRoot $repoRoot -Cache $Cache
    }
}

Write-Result $result
