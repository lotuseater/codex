[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$RepoRoot,

    [ValidateSet("FastRelease", "FullRelease")]
    [string]$BuildMode = "FastRelease",

    [switch]$SkipClean
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

function Invoke-ReleaseBuild {
    param(
        [string]$Root,
        [string]$Mode
    )

    $cargoBin = Join-Path $HOME ".cargo\bin"
    if ((Test-Path -LiteralPath $cargoBin) -and -not $env:Path.Contains($cargoBin)) {
        $env:Path = "$cargoBin;$env:Path"
    }

    $previousLto = $env:CARGO_PROFILE_RELEASE_LTO
    $previousCodegenUnits = $env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS
    try {
        if ($Mode -eq "FastRelease") {
            $env:CARGO_PROFILE_RELEASE_LTO = "off"
            $env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16"
        }

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
    finally {
        $env:CARGO_PROFILE_RELEASE_LTO = $previousLto
        $env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS = $previousCodegenUnits
    }
}

$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$targetRoot = Assert-UnderRoot -Path (Join-Path $RepoRoot "codex-rs\target") -Root $RepoRoot -Label "target root"
$releaseBinary = Assert-UnderRoot -Path (Join-Path $targetRoot "release\codex.exe") -Root $targetRoot -Label "release binary"

$cleanupPaths = @(
    (Join-Path $targetRoot "debug"),
    (Join-Path $targetRoot "release"),
    (Join-Path $targetRoot "tmp")
) | ForEach-Object {
    Assert-UnderRoot -Path $_ -Root $targetRoot -Label "cleanup path"
}

if (-not $SkipClean) {
    foreach ($path in $cleanupPaths) {
        if (Test-Path -LiteralPath $path) {
            if ($PSCmdlet.ShouldProcess($path, "delete local build folder")) {
                Remove-Item -LiteralPath $path -Recurse -Force
            }
        }
    }
}

if ($PSCmdlet.ShouldProcess($RepoRoot, "run $BuildMode cargo release build")) {
    Invoke-ReleaseBuild -Root $RepoRoot -Mode $BuildMode
}

if (-not (Test-Path -LiteralPath $releaseBinary)) {
    throw "Release build did not produce $releaseBinary"
}

if ($PSCmdlet.ShouldProcess($releaseBinary, "verify direct local codex binary")) {
    & $releaseBinary --version | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "Direct local codex --version failed with exit code $LASTEXITCODE"
    }
}

[ordered]@{
    status = "ok"
    mode = $BuildMode
    repo_root = $RepoRoot
    rebuilt_binary = $releaseBinary
    system_launcher_unchanged = $true
} | ConvertTo-Json -Depth 4
