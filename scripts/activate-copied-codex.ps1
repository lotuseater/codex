[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$SourceExe,

    [string]$InstallRoot = (Join-Path $HOME ".codex\local-builds\active-custom-codex"),

    [string]$WrapperDir = (Join-Path $HOME ".codex\system-wrapper"),

    [string]$BackupRoot = (Join-Path $HOME ".codex\binary-backups"),

    [string]$CacheBridgePy = "C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus\src\mcp\hooks\codex_cache_bridge_cli.py",

    [string]$ToolCacheDir = (Join-Path $HOME ".claude\cache")
)

$ErrorActionPreference = "Stop"

function Read-JsonObject {
    param([string]$Path)

    $json = Get-Content -LiteralPath $Path -Raw
    if ([string]::IsNullOrWhiteSpace($json)) {
        return [ordered]@{}
    }

    $parsed = $json | ConvertFrom-Json
    $result = [ordered]@{}
    foreach ($property in $parsed.PSObject.Properties) {
        $result[$property.Name] = $property.Value
    }
    return $result
}

function Write-JsonObject {
    param(
        [string]$Path,
        [System.Collections.IDictionary]$Payload
    )

    $Payload | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Resolve-LatestCopiedExe {
    $repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
    $repoReleaseExe = Join-Path $repoRoot "codex-rs\target\release\codex.exe"
    $buildRoot = Join-Path $HOME ".codex\local-builds"
    $candidates = @()

    if (Test-Path -LiteralPath $repoReleaseExe) {
        return (Get-Item -LiteralPath $repoReleaseExe).FullName
    }

    if (Test-Path -LiteralPath $buildRoot) {
        $candidates += Get-ChildItem -LiteralPath $buildRoot -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -ne "active-custom-codex" } |
            ForEach-Object { Join-Path $_.FullName "codex.exe" } |
            Where-Object { Test-Path -LiteralPath $_ } |
            ForEach-Object { Get-Item -LiteralPath $_ }
    }

    $candidate = $candidates | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $candidate) {
        throw "No Codex exe found under $buildRoot. Pass -SourceExe explicitly."
    }
    return $candidate.FullName
}

if ([string]::IsNullOrWhiteSpace($SourceExe)) {
    $SourceExe = Resolve-LatestCopiedExe
}
else {
    $SourceExe = (Resolve-Path -LiteralPath $SourceExe).Path
}

$envPath = Join-Path $WrapperDir "system.codex-wrapper.env.json"
if (-not (Test-Path -LiteralPath $envPath)) {
    throw "Wrapper env JSON not found: $envPath"
}
if (-not (Test-Path -LiteralPath $SourceExe)) {
    throw "Source Codex exe not found: $SourceExe"
}
if (-not (Test-Path -LiteralPath $CacheBridgePy)) {
    throw "Codex cache bridge CLI not found: $CacheBridgePy"
}

New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
$activeExe = Join-Path $InstallRoot "codex.exe"

$payload = Read-JsonObject -Path $envPath
$previousRealExe = [string]$payload["WIZARD_CODEX_REAL_EXE"]

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backupDir = Join-Path $BackupRoot "activate-copied-codex-$stamp"
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null
$envBackup = Join-Path $backupDir "system.codex-wrapper.env.json.bak"
Copy-Item -LiteralPath $envPath -Destination $envBackup -Force

if ($PSCmdlet.ShouldProcess($activeExe, "copy $SourceExe")) {
    try {
        Copy-Item -LiteralPath $SourceExe -Destination $activeExe -Force
    }
    catch {
        if ($PSBoundParameters.ContainsKey("InstallRoot")) {
            throw
        }

        $versionedInstallRoot = Join-Path $HOME ".codex\local-builds\codex-custom-$stamp"
        New-Item -ItemType Directory -Force -Path $versionedInstallRoot | Out-Null
        $activeExe = Join-Path $versionedInstallRoot "codex.exe"
        Write-Warning "Could not overwrite default active Codex exe, likely because an existing session still has it open. Falling back to $activeExe"
        Copy-Item -LiteralPath $SourceExe -Destination $activeExe -Force
    }
}

& $activeExe --version | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "Copied Codex exe failed --version with exit code $LASTEXITCODE"
}

$manifestPath = Join-Path $backupDir "manifest.json"
[ordered]@{
    created_at = (Get-Date).ToString("o")
    action = "activate-copied-codex"
    source_exe = $SourceExe
    active_exe = $activeExe
    wrapper_env_path = $envPath
    wrapper_env_backup = $envBackup
    previous_real_exe = $previousRealExe
    standard_real_exe = [string]$payload["WIZARD_CODEX_STANDARD_NPM_NATIVE_EXE"]
} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

$payload["WIZARD_CODEX_REAL_EXE"] = $activeExe
$payload["WIZARD_CODEX_LOCAL_FORK_MANIFEST"] = $manifestPath
$payload["WIZARD_CODEX_LOCAL_FORK_INSTALLED_AT"] = (Get-Date).ToString("o")
$payload["WIZARD_CODEX_OPERATION_CACHE"] = "1"
$payload["WIZARD_CODEX_CACHE_BRIDGE_PY"] = $CacheBridgePy
$payload["WIZARD_TOOL_CACHE_DIR"] = $ToolCacheDir
$payload["WIZARD_AGENT"] = "codex"

if ($PSCmdlet.ShouldProcess($envPath, "point WIZARD_CODEX_REAL_EXE at $activeExe")) {
    Write-JsonObject -Path $envPath -Payload $payload
}

[ordered]@{
    status = "ok"
    active_exe = $activeExe
    wrapper_env_path = $envPath
    backup_manifest = $manifestPath
    previous_real_exe = $previousRealExe
} | ConvertTo-Json -Depth 4
