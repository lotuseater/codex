param(
    [string]$LogPath,
    [string]$ExitMarkerPath,
    [string]$CargoManifestPath,
    [string]$Package = "codex-cli",
    [string]$FilterPlatform = "x86_64-pc-windows-msvc",
    [int]$IntervalSeconds = 600,
    [int]$InitialDelaySeconds = 0,
    [int]$MaxSamples = 1,
    [switch]$Watch,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function Get-RepoRoot {
    $scriptDir = Split-Path -Parent $PSCommandPath
    Resolve-Path (Join-Path $scriptDir "..")
}

function Resolve-BuildLogPath {
    param([string]$RequestedPath, [string]$RepoRoot)

    if ($RequestedPath) {
        return (Resolve-Path $RequestedPath).Path
    }

    $latest = Get-ChildItem -Path (Join-Path $RepoRoot "logs") -Filter "local-codex-build-*.log" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latest) {
        throw "No local Codex build log found under logs\\local-codex-build-*.log."
    }
    $latest.FullName
}

function Get-DependencyNames {
    param(
        [string]$ManifestPath,
        [string]$PackageName,
        [string]$Platform
    )

    $metadataJson = & cargo metadata --format-version 1 --manifest-path $ManifestPath --filter-platform $Platform 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed for $ManifestPath."
    }
    $metadata = $metadataJson | ConvertFrom-Json

    $packageById = @{}
    foreach ($package in $metadata.packages) {
        $packageById[$package.id] = $package.name
    }

    $rootPackage = $metadata.packages | Where-Object { $_.name -eq $PackageName } | Select-Object -First 1
    if (-not $rootPackage) {
        throw "Package '$PackageName' was not found in cargo metadata."
    }

    $nodeById = @{}
    foreach ($node in $metadata.resolve.nodes) {
        $nodeById[$node.id] = $node
    }

    $seenIds = [System.Collections.Generic.HashSet[string]]::new()
    function Add-Node {
        param([string]$Id)

        if (-not $seenIds.Add($Id)) {
            return
        }
        if ($nodeById.ContainsKey($Id)) {
            foreach ($dep in $nodeById[$Id].deps) {
                Add-Node $dep.pkg
            }
        }
    }

    Add-Node $rootPackage.id

    $names = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($id in $seenIds) {
        if ($packageById.ContainsKey($id)) {
            [void]$names.Add($packageById[$id])
        }
    }
    $names
}

function Read-BuildProgress {
    param(
        [string]$ResolvedLogPath,
        [string]$ResolvedExitMarkerPath,
        [System.Collections.Generic.HashSet[string]]$DependencyNames,
        [datetime]$StartedAt,
        [string]$RepoRoot
    )

    $compiledNames = [System.Collections.Generic.HashSet[string]]::new()
    $finished = $false
    $failed = $false
    $latestCompilingLine = $null
    $latestLine = $null

    if (Test-Path $ResolvedLogPath) {
        foreach ($line in Get-Content -Path $ResolvedLogPath) {
            $latestLine = $line
            if ($line -match "^\s+Compiling\s+([^\s]+)\s+") {
                $latestCompilingLine = $line.Trim()
                [void]$compiledNames.Add($Matches[1])
            } elseif ($line -match "^\s+Finished\s+") {
                $finished = $true
            } elseif ($line -match "error:|failed to run|could not compile|Build failed") {
                $failed = $true
            }
        }
    }

    $matched = 0
    foreach ($name in $compiledNames) {
        if ($DependencyNames.Contains($name)) {
            $matched++
        }
    }

    $total = [Math]::Max(1, $DependencyNames.Count)
    $markerExists = $ResolvedExitMarkerPath -and (Test-Path $ResolvedExitMarkerPath)
    $markerText = if ($markerExists) { (Get-Content -Path $ResolvedExitMarkerPath -Raw).Trim() } else { "" }
    $markerExitCode = $null
    if ($markerText -match "-?\d+") {
        $markerExitCode = [int]$Matches[0]
    }

    if ($finished -or ($markerExitCode -eq 0)) {
        $percent = 100.0
    } else {
        $percent = [Math]::Min(99.0, [Math]::Round(($matched / $total) * 100.0, 1))
    }

    $elapsed = (Get-Date) - $StartedAt
    $remainingText = "ETA not stable"
    $etaText = ""
    if ($percent -gt 1 -and $percent -lt 100) {
        $estimatedTotalSeconds = $elapsed.TotalSeconds / ($percent / 100.0)
        $remaining = [TimeSpan]::FromSeconds([Math]::Max(0, $estimatedTotalSeconds - $elapsed.TotalSeconds))
        $eta = (Get-Date).Add($remaining)
        $remainingText = "$([Math]::Round($remaining.TotalMinutes, 1)) min left"
        $etaText = $eta.ToString("HH:mm")
    } elseif ($percent -ge 100) {
        $remainingText = "done"
    }

    $repoRegex = [regex]::Escape($RepoRoot)
    $active = Get-CimInstance Win32_Process |
        Where-Object {
            $_.Name -in @("cargo.exe", "rustc.exe", "link.exe", "cmd.exe", "pwsh.exe", "powershell.exe") -and
            ($_.CommandLine -match "cargo build -p $Package" -or $_.CommandLine -match $repoRegex)
        }

    $status = "running"
    if ($markerExitCode -eq 0 -or $finished) {
        $status = "finished"
    } elseif (($markerExitCode -ne $null -and $markerExitCode -ne 0) -or $failed) {
        $status = "failed"
    } elseif (-not $active) {
        $status = "not-active"
    }

    [pscustomobject]@{
        time = (Get-Date).ToString("HH:mm:ss")
        status = $status
        percent = $percent
        compiled = $matched
        total = $total
        elapsedMinutes = [Math]::Round($elapsed.TotalMinutes, 1)
        remaining = $remainingText
        eta = $etaText
        latest = if ($latestCompilingLine) { $latestCompilingLine } else { $latestLine }
        logPath = $ResolvedLogPath
        exitMarkerPath = $ResolvedExitMarkerPath
        exitMarker = $markerText
        activeProcessCount = $active.Count
    }
}

function Write-ProgressSample {
    param([pscustomobject]$Sample)

    if ($Json) {
        $Sample | ConvertTo-Json -Depth 4 -Compress
        return
    }

    $etaSuffix = if ($Sample.eta) { ", ETA $($Sample.eta)" } else { "" }
    "[{0}] {1}: {2}% ({3}/{4}), elapsed {5} min, {6}{7}; latest: {8}" -f `
        $Sample.time,
        $Sample.status,
        $Sample.percent,
        $Sample.compiled,
        $Sample.total,
        $Sample.elapsedMinutes,
        $Sample.remaining,
        $etaSuffix,
        $Sample.latest
}

$repoRoot = (Get-RepoRoot).Path
if (-not $CargoManifestPath) {
    $CargoManifestPath = Join-Path $repoRoot "codex-rs\Cargo.toml"
}
$resolvedLogPath = Resolve-BuildLogPath -RequestedPath $LogPath -RepoRoot $repoRoot
$resolvedExitMarkerPath = if ($ExitMarkerPath) { $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($ExitMarkerPath) } else { "" }
$logItem = Get-Item -Path $resolvedLogPath
$startedAt = $logItem.CreationTime
$dependencyNames = Get-DependencyNames -ManifestPath $CargoManifestPath -PackageName $Package -Platform $FilterPlatform

if ($InitialDelaySeconds -gt 0) {
    Start-Sleep -Seconds $InitialDelaySeconds
}

$samplesWritten = 0
do {
    $sample = Read-BuildProgress `
        -ResolvedLogPath $resolvedLogPath `
        -ResolvedExitMarkerPath $resolvedExitMarkerPath `
        -DependencyNames $dependencyNames `
        -StartedAt $startedAt `
        -RepoRoot $repoRoot
    Write-ProgressSample $sample
    $samplesWritten++

    if ($sample.status -in @("finished", "failed", "not-active")) {
        break
    }
    if (-not $Watch) {
        break
    }
    if ($MaxSamples -gt 0 -and $samplesWritten -ge $MaxSamples) {
        break
    }
    Start-Sleep -Seconds $IntervalSeconds
} while ($true)
