[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$RepoRoot,

    [ValidateSet("DevSmall", "FastRelease", "FullRelease")]
    [string]$BuildMode = "DevSmall",

    [switch]$Clean,

    [switch]$CleanDebug,

    [switch]$SkipClean,

    [switch]$SkipVerify,

    [int]$Jobs = 0
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

function Resolve-FullPath {
    param([string]$Path)

    return [System.IO.Path]::GetFullPath($Path)
}

function Assert-UnderRoot {
    param(
        [string]$Path,
        [string]$Root,
        [string]$Label
    )

    $resolvedRoot = Resolve-FullPath -Path $Root
    $resolvedPath = Resolve-FullPath -Path $Path
    $rootPrefix = $resolvedRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar

    if (($resolvedPath -ine $resolvedRoot) -and (-not $resolvedPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase))) {
        throw "$Label resolves outside expected root: $resolvedPath (root: $resolvedRoot)"
    }

    return $resolvedPath
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

function Get-BuildPlan {
    param(
        [string]$Mode,
        [string]$TargetRoot
    )

    $envOverrides = [ordered]@{}
    switch ($Mode) {
        "DevSmall" {
            $cargoArgs = @("build", "-p", "codex-cli", "--profile", "dev-small", "--bin", "codex")
            $binary = Join-Path $TargetRoot "dev-small\codex.exe"
            $description = "minimal local dev-small build"
        }
        "FastRelease" {
            $cargoArgs = @("build", "-p", "codex-cli", "--release", "--bin", "codex")
            $binary = Join-Path $TargetRoot "release\codex.exe"
            $description = "fast release build with LTO disabled"
            $envOverrides["CARGO_PROFILE_RELEASE_LTO"] = "off"
            $envOverrides["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "16"
        }
        "FullRelease" {
            $cargoArgs = @("build", "-p", "codex-cli", "--release", "--bin", "codex")
            $binary = Join-Path $TargetRoot "release\codex.exe"
            $description = "full release build using Cargo.toml release profile"
        }
    }

    if ($Jobs -gt 0) {
        $cargoArgs += @("--jobs", [string]$Jobs)
    }

    return [ordered]@{
        cargo_args = $cargoArgs
        binary = $binary
        description = $description
        env_overrides = $envOverrides
    }
}

function Join-CommandLine {
    param([string[]]$Args)

    return ($Args | ForEach-Object {
        if ($_ -match "\s") {
            '"' + ($_ -replace '"', '\"') + '"'
        }
        else {
            $_
        }
    }) -join " "
}

function Invoke-CargoBuild {
    param(
        [string]$Root,
        [string[]]$CargoArgs,
        [System.Collections.IDictionary]$EnvOverrides
    )

    $cargoBin = Join-Path $HOME ".cargo\bin"
    if ((Test-Path -LiteralPath $cargoBin) -and -not $env:Path.Contains($cargoBin)) {
        $env:Path = "$cargoBin;$env:Path"
    }

    $previousEnv = @{}
    foreach ($key in $EnvOverrides.Keys) {
        $previousEnv[$key] = [Environment]::GetEnvironmentVariable($key, "Process")
        [Environment]::SetEnvironmentVariable($key, [string]$EnvOverrides[$key], "Process")
    }

    Push-Location (Join-Path $Root "codex-rs")
    try {
        $link = Get-Command link.exe -ErrorAction SilentlyContinue
        if ($link) {
            & cargo @CargoArgs
            if ($LASTEXITCODE -ne 0) {
                throw "cargo build failed with exit code $LASTEXITCODE"
            }
            return
        }

        $vsDevCmd = Find-VsDevCmd
        if ([string]::IsNullOrWhiteSpace($vsDevCmd)) {
            throw "MSVC linker link.exe is not on PATH and VsDevCmd.bat was not found. Install Visual Studio Build Tools with the C++ workload."
        }

        $cargoLine = "cargo " + (Join-CommandLine -Args $CargoArgs)
        & cmd.exe /d /s /c "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && $cargoLine"
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
        foreach ($key in $EnvOverrides.Keys) {
            [Environment]::SetEnvironmentVariable($key, $previousEnv[$key], "Process")
        }
    }
}

$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$targetRoot = Assert-UnderRoot -Path (Join-Path $RepoRoot "codex-rs\target") -Root $RepoRoot -Label "target root"
$plan = Get-BuildPlan -Mode $BuildMode -TargetRoot $targetRoot
$binary = Assert-UnderRoot -Path $plan.binary -Root $targetRoot -Label "local codex binary"

$cleanupPaths = @()
if ($Clean -and -not $SkipClean) {
    $profileDir = if ($BuildMode -eq "DevSmall") { "dev-small" } else { "release" }
    $cleanupPaths += (Join-Path $targetRoot $profileDir)
    $cleanupPaths += (Join-Path $targetRoot "tmp")
}
if ($CleanDebug) {
    $cleanupPaths += (Join-Path $targetRoot "debug")
}
$cleanupPaths = $cleanupPaths | Select-Object -Unique | ForEach-Object {
    Assert-UnderRoot -Path $_ -Root $targetRoot -Label "cleanup path"
}

foreach ($path in $cleanupPaths) {
    if (Test-Path -LiteralPath $path) {
        if ($PSCmdlet.ShouldProcess($path, "delete local build folder")) {
            Remove-Item -LiteralPath $path -Recurse -Force
        }
    }
}

$buildRan = $false
if ($PSCmdlet.ShouldProcess($RepoRoot, "run $($plan.description)")) {
    Invoke-CargoBuild -Root $RepoRoot -CargoArgs $plan.cargo_args -EnvOverrides $plan.env_overrides
    $buildRan = $true
}

if ($buildRan -and -not (Test-Path -LiteralPath $binary)) {
    throw "Build did not produce $binary"
}

if ($buildRan -and -not $SkipVerify) {
    if ($PSCmdlet.ShouldProcess($binary, "verify direct local codex binary")) {
        & $binary --version | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Direct local codex --version failed with exit code $LASTEXITCODE"
        }
    }
}

[ordered]@{
    status = if ($buildRan) { "ok" } else { "planned" }
    mode = $BuildMode
    repo_root = $RepoRoot
    cargo_args = $plan.cargo_args
    rebuilt_binary = $binary
    clean_requested = [bool]($Clean -or $CleanDebug)
    cleanup_paths = $cleanupPaths
    system_launcher_unchanged = $true
} | ConvertTo-Json -Depth 4
