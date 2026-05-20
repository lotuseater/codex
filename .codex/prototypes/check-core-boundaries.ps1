param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
)

$ErrorActionPreference = 'Stop'

$codexRs = Join-Path $RepoRoot 'codex-rs'
$workspaceManifest = Join-Path $codexRs 'Cargo.toml'
$violations = [System.Collections.Generic.List[string]]::new()

function Add-Violation([string]$Message) {
    $violations.Add($Message) | Out-Null
}

function Normalize-RelPath([string]$Path) {
    return ($Path -replace '\\', '/').Trim('/')
}

function Read-TextIfExists([string]$Path) {
    if (Test-Path $Path) {
        return Get-Content -Path $Path -Raw
    }

    return ''
}

function Get-WorkspaceCrates {
    $text = Read-TextIfExists $workspaceManifest
    $crates = @{}
    $pattern = '(?m)^\s*(codex-[A-Za-z0-9-]+)\s*=\s*\{\s*path\s*=\s*"([^"]+)"'
    foreach ($match in [regex]::Matches($text, $pattern)) {
        $name = $match.Groups[1].Value
        $relPath = Normalize-RelPath $match.Groups[2].Value
        $crates[$name] = $relPath
    }

    return $crates
}

function Get-LocalDeps([string]$ManifestPath, [hashtable]$WorkspaceCrates) {
    $text = Read-TextIfExists $ManifestPath
    $deps = [System.Collections.Generic.HashSet[string]]::new()
    $depPattern = '(?m)^\s*(codex-[A-Za-z0-9-]+)\s*='
    foreach ($match in [regex]::Matches($text, $depPattern)) {
        $dep = $match.Groups[1].Value
        if ($WorkspaceCrates.ContainsKey($dep)) {
            $deps.Add($dep) | Out-Null
        }
    }

    return @($deps)
}

function Get-TransitiveDeps([string]$Crate, [hashtable]$Graph) {
    $seen = [System.Collections.Generic.HashSet[string]]::new()
    $queue = [System.Collections.Generic.Queue[string]]::new()

    if (-not $Graph.ContainsKey($Crate)) {
        return @()
    }

    foreach ($dep in $Graph[$Crate]) {
        $queue.Enqueue($dep)
    }

    while ($queue.Count -gt 0) {
        $dep = $queue.Dequeue()
        if (-not $seen.Add($dep)) {
            continue
        }

        if ($Graph.ContainsKey($dep)) {
            foreach ($next in $Graph[$dep]) {
                $queue.Enqueue($next)
            }
        }
    }

    return @($seen)
}

function Test-ProtectedCrate([string]$Name, [string]$RelPath) {
    $rel = Normalize-RelPath $RelPath

    if ($Name -in @('codex-core', 'codex-thread-store-api')) {
        return $true
    }

    if ($rel -match '^(session|turn|context-domain|tools-domain|runtime-domain)/') {
        return $true
    }

    if ($rel -match '^thread/(thread-api|thread-handle-api|thread-store-api|thread-manager-api|thread-manager)(/|$)') {
        return $true
    }

    return $false
}

function Assert-NoTransitiveDeps(
    [string]$Crate,
    [string[]]$Forbidden,
    [hashtable]$Graph,
    [string]$Reason
) {
    if (-not $Graph.ContainsKey($Crate)) {
        return
    }

    $direct = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($dep in [string[]]$Graph[$Crate]) {
        [void]$direct.Add($dep)
    }

    $transitive = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($dep in [string[]](Get-TransitiveDeps $Crate $Graph)) {
        [void]$transitive.Add($dep)
    }

    foreach ($forbiddenCrate in $Forbidden) {
        if ($direct.Contains($forbiddenCrate)) {
            Add-Violation "$Crate directly depends on forbidden crate $forbiddenCrate ($Reason)"
        } elseif ($transitive.Contains($forbiddenCrate)) {
            Add-Violation "$Crate transitively depends on forbidden crate $forbiddenCrate ($Reason)"
        }
    }
}

function Assert-NoSourcePattern(
    [string]$Path,
    [string[]]$Patterns,
    [string]$Reason
) {
    if (-not (Test-Path $Path)) {
        return
    }

    $files = Get-ChildItem -Path $Path -Recurse -File -Include '*.rs', '*.toml'
    foreach ($file in $files) {
        $text = Get-Content -Path $file.FullName -Raw
        foreach ($pattern in $Patterns) {
            if ($text -match $pattern) {
                $rel = Resolve-Path -Path $file.FullName -Relative
                Add-Violation "$rel matches forbidden pattern '$pattern' ($Reason)"
            }
        }
    }
}

$workspaceCrates = Get-WorkspaceCrates
$graph = @{}
foreach ($crate in $workspaceCrates.Keys) {
    $manifest = Join-Path $codexRs (Join-Path $workspaceCrates[$crate] 'Cargo.toml')
    $graph[$crate] = @(Get-LocalDeps $manifest $workspaceCrates)
}

$coreForbiddenDeps = @(
    'codex-app-server',
    'codex-app-server-protocol',
    'codex-core-api',
    'codex-mcp-server',
    'codex-thread-store',
    'codex-tui'
)

$domainForbiddenDeps = @(
    'codex-app-server',
    'codex-app-server-protocol',
    'codex-core',
    'codex-core-api',
    'codex-mcp-server',
    'codex-thread-store',
    'codex-tui'
)

foreach ($crate in $workspaceCrates.Keys) {
    $relPath = $workspaceCrates[$crate]
    if (-not (Test-ProtectedCrate $crate $relPath)) {
        continue
    }

    if ($crate -eq 'codex-core') {
        Assert-NoTransitiveDeps $crate $coreForbiddenDeps $graph 'core must be driven by outer adapters and abstract ports'
    } else {
        Assert-NoTransitiveDeps $crate $domainForbiddenDeps $graph 'domain/API crates must depend only on abstractions'
    }
}

Assert-NoSourcePattern (Join-Path $codexRs 'core/src') @(
    'codex_thread_store::',
    'LocalThreadStore',
    'LocalThreadStoreConfig',
    'InMemoryThreadStore',
    'StoreLiveThreadFactory',
    'thread_store_from_config',
    'codex_app_server_protocol::'
) 'codex-core source must not import concrete stores or app-server protocol'

foreach ($crate in $workspaceCrates.Keys) {
    $relPath = Normalize-RelPath $workspaceCrates[$crate]
    if (-not (Test-ProtectedCrate $crate $relPath) -or $crate -eq 'codex-core') {
        continue
    }

    Assert-NoSourcePattern (Join-Path $codexRs (Join-Path $relPath 'src')) @(
        'codex_core::',
        'codex_core_api::',
        'codex_app_server_protocol::',
        'codex_thread_store::'
    ) 'protected domain/API crates must not import outer facades or concrete stores'
}

if ($violations.Count -gt 0) {
    $violations | Sort-Object | ForEach-Object { Write-Output $_ }
    exit 1
}

Write-Output 'architecture boundary canary passed'
