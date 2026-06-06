[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Custom', 'Standard')]
    [string]$Mode,

    [string]$CustomCliPath,
    [string]$StandardCliPath,
    [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$stateRoot = Join-Path $env:USERPROFILE '.codex\ui-codex-switch'
$statePath = Join-Path $stateRoot 'state.json'
$configTomlPath = Join-Path $env:USERPROFILE '.codex\config.toml'
$wrapperEnvPath = Join-Path $env:USERPROFILE '.codex\system-wrapper\system.codex-wrapper.env.json'
$localManifestPath = Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\chrome-native-hosts-v2.json'
$codexManifestPath = Join-Path $env:USERPROFILE '.codex\chrome-native-hosts-v2.json'
$manifestPaths = @($codexManifestPath, $localManifestPath)

function Get-ObjectProperty {
    param(
        [object]$Object,
        [string]$Name
    )
    if ($null -eq $Object) {
        return $null
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Normalize-PathText {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }
    try {
        return [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
    } catch {
        return $Path.TrimEnd('\')
    }
}

function Test-SamePath {
    param(
        [string]$Left,
        [string]$Right
    )
    $leftNorm = Normalize-PathText $Left
    $rightNorm = Normalize-PathText $Right
    if ($null -eq $leftNorm -or $null -eq $rightNorm) {
        return $false
    }
    return $leftNorm -ieq $rightNorm
}

function Get-FileHashShort {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.Substring(0, 16)
}

function Read-JsonFile {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Write-JsonFile {
    param(
        [string]$Path,
        [object]$Value
    )
    $json = $Value | ConvertTo-Json -Depth 32
    Set-Content -LiteralPath $Path -Value $json -Encoding UTF8
}

function Get-State {
    if (-not (Test-Path -LiteralPath $statePath -PathType Leaf)) {
        return [pscustomobject]@{}
    }
    return Read-JsonFile $statePath
}

function Get-ConfigCliPath {
    if (-not (Test-Path -LiteralPath $configTomlPath -PathType Leaf)) {
        return $null
    }
    $raw = Get-Content -LiteralPath $configTomlPath -Raw
    $match = [regex]::Match($raw, '(?m)^CODEX_CLI_PATH\s*=\s*[''"](?<path>[^''"]+)[''"]')
    if (-not $match.Success) {
        return $null
    }
    return $match.Groups['path'].Value
}

function Get-ManifestCliPath {
    foreach ($path in $manifestPaths) {
        $json = Read-JsonFile $path
        if ($null -eq $json) {
            continue
        }
        foreach ($entry in @($json.entries)) {
            $paths = Get-ObjectProperty $entry 'paths'
            $cliPath = Get-ObjectProperty $paths 'codexCliPath'
            if ($cliPath) {
                return $cliPath
            }
        }
    }
    return $null
}

function Get-CachedUiCliPath {
    $binRoot = Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\bin'
    if (-not (Test-Path -LiteralPath $binRoot -PathType Container)) {
        return $null
    }
    $candidate = Get-ChildItem -LiteralPath $binRoot -Filter codex.exe -Recurse -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($null -eq $candidate) {
        return $null
    }
    return $candidate.FullName
}

function Resolve-CustomCliPath {
    if ($CustomCliPath) {
        return $CustomCliPath
    }
    $wrapperEnv = Read-JsonFile $wrapperEnvPath
    $wrapperCli = Get-ObjectProperty $wrapperEnv 'WIZARD_CODEX_REAL_EXE'
    if ($wrapperCli) {
        return $wrapperCli
    }
    $repoCandidate = Join-Path $PSScriptRoot '..\codex-rs\target\release\codex.exe'
    return [System.IO.Path]::GetFullPath($repoCandidate)
}

function Resolve-StandardCliPath {
    param([string]$ResolvedCustomCliPath)

    if ($StandardCliPath) {
        return $StandardCliPath
    }

    $state = Get-State
    $stateStandard = Get-ObjectProperty $state 'standardCliPath'
    if ($stateStandard -and (Test-Path -LiteralPath $stateStandard -PathType Leaf)) {
        return $stateStandard
    }

    foreach ($candidate in @((Get-ConfigCliPath), (Get-ManifestCliPath), (Get-CachedUiCliPath))) {
        if ($candidate -and
            -not (Test-SamePath $candidate $ResolvedCustomCliPath) -and
            (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return $candidate
        }
    }

    throw 'Could not resolve the standard Codex UI CLI path. Pass -StandardCliPath explicitly.'
}

function Backup-LiveFiles {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $backupDir = Join-Path $stateRoot "backups\$stamp"
    if ($DryRun) {
        return $backupDir
    }

    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
    if (Test-Path -LiteralPath $configTomlPath -PathType Leaf) {
        Copy-Item -LiteralPath $configTomlPath -Destination (Join-Path $backupDir 'config.toml.bak') -Force
    }
    foreach ($manifestPath in $manifestPaths) {
        if (Test-Path -LiteralPath $manifestPath -PathType Leaf) {
            $name = if ($manifestPath -like "$env:USERPROFILE\.codex\*") {
                'dotcodex.chrome-native-hosts-v2.json.bak'
            } else {
                'localappdata.chrome-native-hosts-v2.json.bak'
            }
            Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $backupDir $name) -Force
        }
    }
    return $backupDir
}

function Set-ConfigCliPath {
    param([string]$TargetCliPath)

    if (-not (Test-Path -LiteralPath $configTomlPath -PathType Leaf)) {
        throw "Missing Codex config: $configTomlPath"
    }

    $raw = Get-Content -LiteralPath $configTomlPath -Raw
    $regex = [regex]'(?m)^CODEX_CLI_PATH\s*=.*$'
    if (-not $regex.IsMatch($raw)) {
        throw "Could not find CODEX_CLI_PATH in $configTomlPath"
    }
    $replacement = "CODEX_CLI_PATH = '$TargetCliPath'"
    $updated = $regex.Replace($raw, $replacement, 1)

    if (-not $DryRun) {
        Set-Content -LiteralPath $configTomlPath -Value $updated -Encoding UTF8
    }
}

function Set-ManifestCliPath {
    param([string]$TargetCliPath)

    foreach ($manifestPath in $manifestPaths) {
        $json = Read-JsonFile $manifestPath
        if ($null -eq $json) {
            continue
        }

        $changed = $false
        foreach ($entry in @($json.entries)) {
            $paths = Get-ObjectProperty $entry 'paths'
            if ($null -eq $paths) {
                continue
            }
            $cliPath = Get-ObjectProperty $paths 'codexCliPath'
            if ($cliPath) {
                $paths.codexCliPath = $TargetCliPath
                $changed = $true
            }
            $updatedAt = Get-ObjectProperty $entry 'updatedAt'
            if ($updatedAt) {
                $entry.updatedAt = (Get-Date).ToUniversalTime().ToString('o')
            }
        }

        if ($changed -and -not $DryRun) {
            Write-JsonFile -Path $manifestPath -Value $json
        }
    }
}

function Write-State {
    param(
        [string]$TargetCliPath,
        [string]$ResolvedCustomCliPath,
        [string]$ResolvedStandardCliPath,
        [string]$BackupDir
    )
    if ($DryRun) {
        return
    }

    New-Item -ItemType Directory -Path $stateRoot -Force | Out-Null
    $state = [pscustomobject]@{
        mode = $Mode
        targetCliPath = $TargetCliPath
        standardCliPath = $ResolvedStandardCliPath
        customCliPath = $ResolvedCustomCliPath
        configTomlPath = $configTomlPath
        manifestPaths = @($manifestPaths | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
        backupDir = $BackupDir
        updatedAt = (Get-Date).ToUniversalTime().ToString('o')
        targetSha256Prefix = Get-FileHashShort $TargetCliPath
        standardSha256Prefix = Get-FileHashShort $ResolvedStandardCliPath
        customSha256Prefix = Get-FileHashShort $ResolvedCustomCliPath
    }
    Write-JsonFile -Path $statePath -Value $state
}

function Get-ActiveUiCodexProcesses {
    $paths = @()
    foreach ($path in @($manifestPaths + @($configTomlPath))) {
        if (Test-Path -LiteralPath $path) {
            $paths += $path
        }
    }

    $binRoot = Normalize-PathText (Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\bin')
    Get-CimInstance Win32_Process |
        Where-Object {
            $_.Name -ieq 'codex.exe' -and
            $_.ExecutablePath -and
            (Normalize-PathText $_.ExecutablePath).StartsWith($binRoot, [System.StringComparison]::OrdinalIgnoreCase)
        } |
        Select-Object ProcessId, ExecutablePath, CommandLine
}

$resolvedCustomCliPath = Resolve-CustomCliPath
if (-not (Test-Path -LiteralPath $resolvedCustomCliPath -PathType Leaf)) {
    throw "Custom Codex CLI does not exist: $resolvedCustomCliPath"
}

$resolvedStandardCliPath = Resolve-StandardCliPath -ResolvedCustomCliPath $resolvedCustomCliPath
if (-not (Test-Path -LiteralPath $resolvedStandardCliPath -PathType Leaf)) {
    throw "Standard Codex UI CLI does not exist: $resolvedStandardCliPath"
}

$targetCliPath = if ($Mode -eq 'Custom') { $resolvedCustomCliPath } else { $resolvedStandardCliPath }
$backupDir = Backup-LiveFiles

Set-ConfigCliPath -TargetCliPath $targetCliPath
Set-ManifestCliPath -TargetCliPath $targetCliPath
Write-State -TargetCliPath $targetCliPath -ResolvedCustomCliPath $resolvedCustomCliPath -ResolvedStandardCliPath $resolvedStandardCliPath -BackupDir $backupDir

$activeProcesses = @(Get-ActiveUiCodexProcesses)

[pscustomobject]@{
    mode = $Mode
    dryRun = [bool]$DryRun
    targetCliPath = $targetCliPath
    standardCliPath = $resolvedStandardCliPath
    customCliPath = $resolvedCustomCliPath
    statePath = $statePath
    backupDir = $backupDir
    activeUiCodexProcessCount = $activeProcesses.Count
    note = if ($activeProcesses.Count -gt 0) {
        'Existing UI app-server processes keep their current executable until that thread/app is restarted.'
    } else {
        'No cached UI codex.exe processes are currently running.'
    }
} | ConvertTo-Json -Depth 8
