[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$RepoRoot,

    [string]$WrapperDir = (Join-Path $HOME ".codex\system-wrapper"),

    [string]$BackupRoot = (Join-Path $HOME ".codex\binary-backups"),

    [switch]$KeepFallbackOnSuccess,

    [switch]$RunInstallVerify
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

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
        [hashtable]$Payload
    )

    $Payload | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Find-VsDevCmd {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path -LiteralPath $vswhere) {
        $installPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if (-not [string]::IsNullOrWhiteSpace($installPath)) {
            $candidate = Join-Path $installPath "Common7\Tools\VsDevCmd.bat"
            if (Test-Path -LiteralPath $candidate) {
                return $candidate
            }
        }
    }

    $roots = @(
        (Join-Path $env:ProgramFiles "Microsoft Visual Studio"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio")
    )
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) {
            continue
        }
        $candidate = Get-ChildItem -LiteralPath $root -Recurse -Filter VsDevCmd.bat -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($candidate) {
            return $candidate.FullName
        }
    }

    return $null
}

function Get-WrapperEnvPath {
    param([string]$Dir)

    $path = Join-Path $Dir "system.codex-wrapper.env.json"
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Wrapper env JSON not found at $path"
    }
    return (Resolve-Path -LiteralPath $path).Path
}

function Assert-UnderRoot {
    param(
        [string]$Path,
        [string]$Root,
        [string]$Label
    )

    $resolvedRoot = (Resolve-Path -LiteralPath $Root).Path
    $resolvedPath = if (Test-Path -LiteralPath $Path) {
        (Resolve-Path -LiteralPath $Path).Path
    } else {
        $parent = Split-Path -Parent $Path
        $leaf = Split-Path -Leaf $Path
        (Join-Path (Resolve-Path -LiteralPath $parent).Path $leaf)
    }

    if (-not $resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label resolves outside expected root: $resolvedPath (root: $resolvedRoot)"
    }
    return $resolvedPath
}

function Set-WrapperRealExe {
    param(
        [string]$EnvPath,
        [string]$RealExe
    )

    $payload = Read-JsonObject -Path $EnvPath
    $payload["WIZARD_CODEX_REAL_EXE"] = $RealExe
    $payload["WIZARD_CODEX_LOCAL_FORK_INSTALLED_AT"] = (Get-Date).ToString("o")
    Write-JsonObject -Path $EnvPath -Payload $payload
}

function Invoke-FastReleaseBuild {
    param([string]$Root)

    $cargoBin = Join-Path $HOME ".cargo\bin"
    if ((Test-Path -LiteralPath $cargoBin) -and -not $env:Path.Contains($cargoBin)) {
        $env:Path = "$cargoBin;$env:Path"
    }

    $env:CARGO_PROFILE_RELEASE_LTO = "off"
    $env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16"

    Push-Location (Join-Path $Root "codex-rs")
    try {
        $link = Get-Command link.exe -ErrorAction SilentlyContinue
        if ($link) {
            cargo build --release --bin codex
            return
        }

        $vsDevCmd = Find-VsDevCmd
        if ([string]::IsNullOrWhiteSpace($vsDevCmd)) {
            throw "MSVC linker link.exe is not on PATH and VsDevCmd.bat was not found. Install Visual Studio Build Tools with the C++ workload."
        }

        & cmd.exe /d /s /c "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && cargo build --release --bin codex"
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$targetRoot = Assert-UnderRoot -Path (Join-Path $RepoRoot "codex-rs\target") -Root $RepoRoot -Label "target root"
$releaseBinary = Join-Path $targetRoot "release\codex.exe"
$envPath = Get-WrapperEnvPath -Dir $WrapperDir
$payload = Read-JsonObject -Path $envPath
$previousRealExe = [string]$payload["WIZARD_CODEX_REAL_EXE"]
if ([string]::IsNullOrWhiteSpace($previousRealExe)) {
    throw "WIZARD_CODEX_REAL_EXE is empty in $envPath"
}
if (-not (Test-Path -LiteralPath $previousRealExe)) {
    throw "Current WIZARD_CODEX_REAL_EXE target does not exist: $previousRealExe"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$fallbackDir = Join-Path $BackupRoot "clean-fast-build-$stamp"
$fallbackExe = Join-Path $fallbackDir "codex-fallback.exe"
$manifestPath = Join-Path $fallbackDir "manifest.json"

$cleanupPaths = @(
    (Join-Path $targetRoot "debug"),
    (Join-Path $targetRoot "release"),
    (Join-Path $targetRoot "tmp")
) | ForEach-Object {
    Assert-UnderRoot -Path $_ -Root $targetRoot -Label "cleanup path"
}

$manifest = [ordered]@{
    created_at = (Get-Date).ToString("o")
    action = "clean-fast-release-build"
    repo_root = $RepoRoot
    wrapper_env_path = $envPath
    previous_real_exe = $previousRealExe
    fallback_exe = $fallbackExe
    cleanup_paths = $cleanupPaths
    target_release_binary = $releaseBinary
}

if ($PSCmdlet.ShouldProcess($fallbackDir, "create fallback directory and manifest")) {
    New-Item -ItemType Directory -Force -Path $fallbackDir | Out-Null
    Copy-Item -LiteralPath $previousRealExe -Destination $fallbackExe -Force
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
}

try {
    if ($PSCmdlet.ShouldProcess($envPath, "temporarily point WIZARD_CODEX_REAL_EXE at fallback $fallbackExe")) {
        Set-WrapperRealExe -EnvPath $envPath -RealExe $fallbackExe
    }

    foreach ($path in $cleanupPaths) {
        if (Test-Path -LiteralPath $path) {
            if ($PSCmdlet.ShouldProcess($path, "delete build folder")) {
                Remove-Item -LiteralPath $path -Recurse -Force
            }
        }
    }

    if ($PSCmdlet.ShouldProcess($RepoRoot, "run fast release cargo build")) {
        Invoke-FastReleaseBuild -Root $RepoRoot
    }

    if (-not (Test-Path -LiteralPath $releaseBinary)) {
        throw "Fast release build did not produce $releaseBinary"
    }

    if ($PSCmdlet.ShouldProcess($releaseBinary, "verify direct codex binary")) {
        & $releaseBinary --version | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Direct codex --version failed with exit code $LASTEXITCODE"
        }
    }

    if ($PSCmdlet.ShouldProcess($envPath, "point WIZARD_CODEX_REAL_EXE at rebuilt binary $releaseBinary")) {
        Set-WrapperRealExe -EnvPath $envPath -RealExe $releaseBinary
    }

    if ($RunInstallVerify) {
        $installScript = Join-Path $RepoRoot "scripts\install-local-codex-fork.ps1"
        if ($PSCmdlet.ShouldProcess($installScript, "run install verification")) {
            & pwsh -NoProfile -ExecutionPolicy Bypass -File $installScript -Action Verify
            if ($LASTEXITCODE -ne 0) {
                throw "install-local-codex-fork verify failed with exit code $LASTEXITCODE"
            }
        }
    }

    if ((-not $KeepFallbackOnSuccess) -and (Test-Path -LiteralPath $fallbackDir)) {
        if ($PSCmdlet.ShouldProcess($fallbackDir, "delete fallback directory after successful rebuild")) {
            Remove-Item -LiteralPath $fallbackDir -Recurse -Force
        }
    }

    [ordered]@{
        status = "ok"
        rebuilt_binary = $releaseBinary
        wrapper_env_path = $envPath
        fallback_manifest = $manifestPath
        fallback_kept = [bool]$KeepFallbackOnSuccess
    } | ConvertTo-Json -Depth 4
}
catch {
    Write-Warning "Clean fast build failed. The wrapper should remain pointed at the fallback binary: $fallbackExe"
    Write-Warning "Manifest: $manifestPath"
    throw
}
