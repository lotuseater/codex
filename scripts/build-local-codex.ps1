[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [ValidateSet("Status", "FastRelease", "LowMemRelease", "DevRelease", "FullRelease", "DeployOnly", "Rollback")]
    [string]$Mode = "Status",

    [string]$RepoRoot,

    [string]$SourceExe,

    [string]$WrapperDir = (Join-Path $HOME ".codex\system-wrapper"),

    [string]$BackupRoot = (Join-Path $HOME ".codex\binary-backups"),

    [string]$LocalBuildRoot = (Join-Path $HOME ".codex\local-builds"),

    [switch]$SkipDeploy,

    [switch]$SkipVerify,

    [switch]$Timings,

    [int]$Jobs = 0
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path

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
    $rootPrefix = $resolvedRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) +
        [System.IO.Path]::DirectorySeparatorChar

    if (($resolvedPath -ine $resolvedRoot) -and
        (-not $resolvedPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase))) {
        throw "$Label resolves outside expected root: $resolvedPath (root: $resolvedRoot)"
    }

    return $resolvedPath
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
        [System.Collections.IDictionary]$Payload
    )

    $Payload | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Get-WrapperEnvPath {
    param([string]$Dir)

    $path = Join-Path $Dir "system.codex-wrapper.env.json"
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Wrapper env JSON not found: $path"
    }
    return (Resolve-Path -LiteralPath $path).Path
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

function Test-ContainsText {
    param(
        [string]$Haystack,
        [string]$Needle
    )

    return (-not [string]::IsNullOrWhiteSpace($Haystack)) -and
        ($Haystack.IndexOf($Needle, [System.StringComparison]::OrdinalIgnoreCase) -ge 0)
}

function Get-RepoBuildProcesses {
    param([string]$Root)

    $codexRs = Join-Path $Root "codex-rs"
    $previousWhatIfPreference = $WhatIfPreference
    $WhatIfPreference = $false
    try {
        $processes = Get-CimInstance Win32_Process -Filter "Name = 'cargo.exe' OR Name = 'rustc.exe' OR Name = 'link.exe' OR Name = 'cmd.exe'" -ErrorAction SilentlyContinue
    }
    finally {
        $WhatIfPreference = $previousWhatIfPreference
    }
    $matching = @{}

    foreach ($process in $processes) {
        $commandLine = [string]$process.CommandLine
        if ((Test-ContainsText -Haystack $commandLine -Needle $Root) -or
            (Test-ContainsText -Haystack $commandLine -Needle $codexRs)) {
            $matching[[int]$process.ProcessId] = $true
            if ($process.ParentProcessId) {
                $matching[[int]$process.ParentProcessId] = $true
            }
        }
    }

    return $processes |
        Where-Object {
            $matching.ContainsKey([int]$_.ProcessId) -or
                $matching.ContainsKey([int]$_.ParentProcessId)
        } |
        Sort-Object ProcessId |
        ForEach-Object {
            [ordered]@{
                process_name = $_.Name
                process_id = [int]$_.ProcessId
                parent_process_id = [int]$_.ParentProcessId
                command_line = [string]$_.CommandLine
            }
        }
}

function Get-RecommendedJobs {
    param(
        [int]$PerJobMemoryMB = 1800,
        [int]$Floor = 1,
        [int]$Ceiling = 0
    )

    $os = Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue
    $cpuCount = 0
    try {
        $cpuCount = [int]((Get-CimInstance Win32_Processor -ErrorAction SilentlyContinue |
            Measure-Object -Property NumberOfLogicalProcessors -Sum).Sum)
    } catch {}
    if ($cpuCount -le 0) { $cpuCount = [Environment]::ProcessorCount }
    if ($Ceiling -le 0) { $Ceiling = $cpuCount }

    if (-not $os) { return [math]::Max($Floor, [math]::Min($Ceiling, 2)) }
    $freeMB = [int]($os.FreePhysicalMemory / 1KB)
    $headroomMB = 1500
    $usable = [math]::Max(0, $freeMB - $headroomMB)
    $byMem = [math]::Floor($usable / $PerJobMemoryMB)
    if ($byMem -lt $Floor) { $byMem = $Floor }
    return [int][math]::Max($Floor, [math]::Min($Ceiling, $byMem))
}

function Get-BuildPlan {
    param(
        [string]$BuildMode,
        [string]$TargetRoot,
        [int]$JobsOverride = 0
    )

    $envOverrides = [ordered]@{}
    $cargoArgs = @("build", "-p", "codex-cli", "--release", "--bin", "codex")
    $description = "full release build using Cargo.toml release profile"
    $binary = Join-Path $TargetRoot "release\codex.exe"

    switch ($BuildMode) {
        "FastRelease" {
            $description = "fast release build (LTO off, cu=16, opt=2, incremental on)"
            $envOverrides["CARGO_PROFILE_RELEASE_LTO"] = "off"
            $envOverrides["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "16"
            $envOverrides["CARGO_PROFILE_RELEASE_OPT_LEVEL"] = "2"
            $envOverrides["CARGO_INCREMENTAL"] = "1"
            $envOverrides["RUST_MIN_STACK"] = "33554432"
            if ($JobsOverride -le 0) {
                $JobsOverride = Get-RecommendedJobs -PerJobMemoryMB 1800
            }
        }
        "LowMemRelease" {
            $description = "low-memory release build (LTO off, cu=256, opt=1, incremental on, RAM-aware jobs)"
            $envOverrides["CARGO_PROFILE_RELEASE_LTO"] = "off"
            $envOverrides["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "256"
            $envOverrides["CARGO_PROFILE_RELEASE_OPT_LEVEL"] = "1"
            $envOverrides["CARGO_PROFILE_RELEASE_DEBUG"] = "0"
            $envOverrides["CARGO_PROFILE_RELEASE_STRIP"] = "symbols"
            $envOverrides["CARGO_INCREMENTAL"] = "1"
            $envOverrides["RUST_MIN_STACK"] = "67108864"
            if ($JobsOverride -le 0) {
                $JobsOverride = Get-RecommendedJobs -PerJobMemoryMB 1100 -Ceiling 4
            }
        }
        "DevRelease" {
            $description = "dev-small build (no opt, fastest iteration, smallest memory peak)"
            $cargoArgs = @("build", "-p", "codex-cli", "--profile", "dev-small", "--bin", "codex")
            $binary = Join-Path $TargetRoot "dev-small\codex.exe"
            $envOverrides["CARGO_INCREMENTAL"] = "1"
            $envOverrides["RUST_MIN_STACK"] = "33554432"
            if ($JobsOverride -le 0) {
                $JobsOverride = Get-RecommendedJobs -PerJobMemoryMB 800
            }
        }
        "FullRelease" {
            if ($JobsOverride -le 0) {
                $JobsOverride = Get-RecommendedJobs -PerJobMemoryMB 2400 -Ceiling 4
            }
        }
    }

    if ($JobsOverride -gt 0) {
        $cargoArgs += @("--jobs", "$JobsOverride")
    }

    if ($Timings) {
        $cargoArgs += "--timings"
    }

    return [ordered]@{
        cargo_args = $cargoArgs
        env_overrides = $envOverrides
        binary = $binary
        description = $description
    }
}

# Disk-space defenses: full release builds of this workspace need ~5 GB of
# headroom (target/release/deps libraries + intermediate .rmeta + final link
# scratch). Without a pre-check, the build silently fails mid-link with
# "There is not enough space on the disk. (os error 112)" — observed
# 2026-05-04 when stacked builds + a packed `incremental/` dir squeezed C:
# below 1 GB. This guard reclaims known-regeneratable dirs before the build
# starts, errors out if still below the safety floor, and (after success)
# evicts the incremental cache that release builds don't need anyway.
function Test-AndFreeDiskSpace {
    param(
        [string]$RepoRoot,
        [int]$RequiredGB = 5,
        [int]$WarnGB = 8
    )

    $codexRs = Join-Path $RepoRoot "codex-rs"
    $tgtRelease = Join-Path $codexRs "target\release"
    $reclaimable = @(
        # 'incremental/' is created on every build but useless for `--release`
        # builds; cargo treats incremental compilation as a debug-mode feature.
        @{ Path = (Join-Path $tgtRelease "incremental"); Reason = "release/incremental (release builds don't use incremental)" },
        # build-script outputs are regenerated on demand from the build.rs
        # invocations stored in target/release/.fingerprint.
        @{ Path = (Join-Path $tgtRelease "build"); Reason = "release/build (build script outputs, regenerated)" },
        # gn_out is GN's intermediate dir for v8/skia; rebuilt from build.rs.
        @{ Path = (Join-Path $tgtRelease "gn_out"); Reason = "release/gn_out (GN intermediate, regenerated)" },
        # .fingerprint is small but tied to the artifacts above; cleaning it
        # together avoids stale-fingerprint mismatches on the next build.
        @{ Path = (Join-Path $tgtRelease ".fingerprint"); Reason = "release/.fingerprint (regenerated together with build/)" },
        # Cargo's extracted source dir; cargo re-extracts from registry/cache
        # on demand when a crate is needed.
        @{ Path = (Join-Path $HOME ".cargo\registry\src"); Reason = "~/.cargo/registry/src (re-extracted from registry/cache)" }
    )

    $freeGB = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    if ($freeGB -ge $WarnGB) {
        Write-Host "Disk OK ($freeGB GB free, threshold $WarnGB GB)."
        return
    }

    Write-Host "Disk pre-check: $freeGB GB free (below warn threshold $WarnGB GB). Reclaiming..."
    foreach ($entry in $reclaimable) {
        if (-not (Test-Path -LiteralPath $entry.Path)) { continue }
        $sizeMB = 0
        try {
            $sizeMB = [math]::Round((Get-ChildItem -LiteralPath $entry.Path -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1MB, 1)
        } catch {}
        try {
            Remove-Item -LiteralPath $entry.Path -Recurse -Force -ErrorAction Stop
            Write-Host ("  - reclaimed {0,7:N1} MB from {1}" -f $sizeMB, $entry.Reason)
        } catch {
            Write-Host ("  - skip (in use): {0}" -f $entry.Path)
        }
    }

    $freeAfterGB = [math]::Round((Get-PSDrive C).Free / 1GB, 2)
    Write-Host "Disk after reclaim: $freeAfterGB GB free."
    if ($freeAfterGB -lt $RequiredGB) {
        throw "Disk space too low for safe build: $freeAfterGB GB free (need >= $RequiredGB GB after auto-clean). Free more space manually before retrying — candidate dirs to inspect: ~/.codex/sessions, ~/.codex/logs_2.sqlite, AppData/Local/Temp."
    }
}

function Invoke-PostBuildDiskCleanup {
    param([string]$RepoRoot)

    # Release builds don't use incremental; cargo creates the dir anyway when
    # CARGO_INCREMENTAL is not '0'. Sweep it after success so the next build
    # starts with maximum headroom. Safe — the dir is rebuilt on demand.
    $inc = Join-Path $RepoRoot "codex-rs\target\release\incremental"
    if (Test-Path -LiteralPath $inc) {
        try {
            $sizeMB = [math]::Round((Get-ChildItem -LiteralPath $inc -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1MB, 1)
            Remove-Item -LiteralPath $inc -Recurse -Force -ErrorAction Stop
            Write-Host ("Post-build cleanup: reclaimed {0:N1} MB from release/incremental." -f $sizeMB)
        } catch {
            # Non-fatal: build already succeeded.
        }
    }
}

function Show-FailureLines {
    param([string]$Path)

    $matches = Select-String -Path $Path -Pattern "error:|fatal error|failed|No space|no space|insufficient disk|LINK : fatal" -ErrorAction SilentlyContinue |
        Select-Object -Last 40
    if ($matches) {
        $matches | ForEach-Object { $_.Line } | Out-Host
        return
    }

    Get-Content -LiteralPath $Path -Tail 80 | Out-Host
}

function Invoke-CodexBuild {
    param(
        [string]$Root,
        [string[]]$CargoArgs,
        [System.Collections.IDictionary]$EnvOverrides,
        [string]$LogPath
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

    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogPath) | Out-Null
    Push-Location (Join-Path $Root "codex-rs")
    try {
        $link = Get-Command link.exe -ErrorAction SilentlyContinue
        if ($link) {
            & cargo @CargoArgs > $LogPath 2>&1
            return $LASTEXITCODE
        }

        $vsDevCmd = Find-VsDevCmd
        if ([string]::IsNullOrWhiteSpace($vsDevCmd)) {
            throw "MSVC linker link.exe is not on PATH and VsDevCmd.bat was not found. Install Visual Studio Build Tools with the C++ workload."
        }

        $cargoLine = "cargo " + (Join-CommandLine -Args $CargoArgs)
        $cmdLine = "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && $cargoLine > `"$LogPath`" 2>&1"
        & cmd.exe /d /s /c $cmdLine
        return $LASTEXITCODE
    }
    finally {
        Pop-Location
        foreach ($key in $EnvOverrides.Keys) {
            [Environment]::SetEnvironmentVariable($key, $previousEnv[$key], "Process")
        }
    }
}

function Invoke-Deploy {
    param(
        [string]$ExePath,
        [string]$ModeName
    )

    $source = (Resolve-Path -LiteralPath $ExePath).Path
    $envPath = Get-WrapperEnvPath -Dir $WrapperDir
    $payload = Read-JsonObject -Path $envPath
    $previousRealExe = [string]$payload["WIZARD_CODEX_REAL_EXE"]

    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $installDir = Join-Path $LocalBuildRoot "codex-custom-$stamp"
    $activeExe = Join-Path $installDir "codex.exe"
    $backupDir = Join-Path $BackupRoot "build-local-codex-$stamp"
    $envBackup = Join-Path $backupDir "system.codex-wrapper.env.json.bak"
    $manifestPath = Join-Path $backupDir "manifest.json"

    if ($PSCmdlet.ShouldProcess($activeExe, "copy verified Codex binary from $source")) {
        New-Item -ItemType Directory -Force -Path $installDir | Out-Null
        New-Item -ItemType Directory -Force -Path $backupDir | Out-Null
        Copy-Item -LiteralPath $source -Destination $activeExe -Force
        Copy-Item -LiteralPath $envPath -Destination $envBackup -Force

        [ordered]@{
            created_at = (Get-Date).ToString("o")
            action = "build-local-codex"
            mode = $ModeName
            source_exe = $source
            active_exe = $activeExe
            wrapper_env_path = $envPath
            wrapper_env_backup = $envBackup
            previous_real_exe = $previousRealExe
        } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

        $payload["WIZARD_CODEX_REAL_EXE"] = $activeExe
        $payload["WIZARD_CODEX_LOCAL_FORK_MANIFEST"] = $manifestPath
        $payload["WIZARD_CODEX_LOCAL_FORK_INSTALLED_AT"] = (Get-Date).ToString("o")
        Write-JsonObject -Path $envPath -Payload $payload
    }

    if (-not $SkipVerify -and (Test-Path -LiteralPath $activeExe)) {
        & $activeExe --version | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Copied Codex exe failed --version with exit code $LASTEXITCODE"
        }
    }

    return [ordered]@{
        active_exe = $activeExe
        wrapper_env_path = $envPath
        backup_manifest = $manifestPath
        previous_real_exe = $previousRealExe
    }
}

function Invoke-Rollback {
    $envPath = Get-WrapperEnvPath -Dir $WrapperDir
    $manifests = @()
    if (Test-Path -LiteralPath $BackupRoot) {
        $manifests = Get-ChildItem -LiteralPath $BackupRoot -Filter manifest.json -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -like "*build-local-codex-*" } |
            Sort-Object LastWriteTime -Descending
    }

    if ($manifests) {
        $manifest = Read-JsonObject -Path $manifests[0].FullName
        $envBackup = [string]$manifest["wrapper_env_backup"]
        if (-not (Test-Path -LiteralPath $envBackup)) {
            throw "Rollback env backup does not exist: $envBackup"
        }

        if ($PSCmdlet.ShouldProcess($envPath, "restore wrapper env from $envBackup")) {
            Copy-Item -LiteralPath $envBackup -Destination $envPath -Force
        }
    }
    else {
        $payload = Read-JsonObject -Path $envPath
        $standardExe = [string]$payload["WIZARD_CODEX_STANDARD_NPM_NATIVE_EXE"]
        if ([string]::IsNullOrWhiteSpace($standardExe)) {
            throw "No build-local-codex rollback manifest found and WIZARD_CODEX_STANDARD_NPM_NATIVE_EXE is missing."
        }
        if (-not (Test-Path -LiteralPath $standardExe)) {
            throw "Standard Codex exe does not exist: $standardExe"
        }

        $payload["WIZARD_CODEX_REAL_EXE"] = $standardExe
        $payload["WIZARD_CODEX_LOCAL_FORK_RESTORED_AT"] = (Get-Date).ToString("o")
        if ($PSCmdlet.ShouldProcess($envPath, "restore WIZARD_CODEX_REAL_EXE to standard exe $standardExe")) {
            Write-JsonObject -Path $envPath -Payload $payload
        }
    }

    $payload = Read-JsonObject -Path $envPath
    $realExe = [string]$payload["WIZARD_CODEX_REAL_EXE"]
    if (-not $SkipVerify) {
        & $realExe --version | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Rollback Codex exe failed --version with exit code $LASTEXITCODE"
        }
    }

    return [ordered]@{
        status = "ok"
        wrapper_env_path = $envPath
        real_exe = $realExe
    }
}

$targetRoot = Assert-UnderRoot -Path (Join-Path $RepoRoot "codex-rs\target") -Root $RepoRoot -Label "target root"
$releaseBinary = Assert-UnderRoot -Path (Join-Path $targetRoot "release\codex.exe") -Root $targetRoot -Label "release binary"
$activeBuilds = @(Get-RepoBuildProcesses -Root $RepoRoot)
$envPath = if (Test-Path -LiteralPath (Join-Path $WrapperDir "system.codex-wrapper.env.json")) {
    Get-WrapperEnvPath -Dir $WrapperDir
}
else {
    $null
}
$wrapperPayload = if ($envPath) { Read-JsonObject -Path $envPath } else { [ordered]@{} }

if ($Mode -eq "Status") {
    [ordered]@{
        status = "ok"
        repo_root = $RepoRoot
        active_build_processes = $activeBuilds
        release_binary = if (Test-Path -LiteralPath $releaseBinary) {
            $item = Get-Item -LiteralPath $releaseBinary
            [ordered]@{
                path = $item.FullName
                length = $item.Length
                last_write_time = $item.LastWriteTime.ToString("o")
            }
        } else { $null }
        wrapper_env_path = $envPath
        wrapper_real_exe = [string]$wrapperPayload["WIZARD_CODEX_REAL_EXE"]
        free_c_drive_bytes = (Get-PSDrive C).Free
    } | ConvertTo-Json -Depth 8
    return
}

if ($Mode -eq "Rollback") {
    Invoke-Rollback | ConvertTo-Json -Depth 6
    return
}

if ($Mode -eq "DeployOnly") {
    if ([string]::IsNullOrWhiteSpace($SourceExe)) {
        $SourceExe = $releaseBinary
    }
    Invoke-Deploy -ExePath $SourceExe -ModeName $Mode | ConvertTo-Json -Depth 6
    return
}

if ($activeBuilds.Count -gt 0) {
    $ids = ($activeBuilds | ForEach-Object { $_["process_id"] }) -join ", "
    throw "Repo-local cargo/rustc build process already active ($ids). Run Status to inspect it; this script will not start a competing build."
}

$plan = Get-BuildPlan -BuildMode $Mode -TargetRoot $targetRoot -JobsOverride $Jobs
$releaseBinary = $plan.binary
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$logPath = Assert-UnderRoot -Path (Join-Path $RepoRoot "logs\local-codex-build-$($Mode.ToLowerInvariant())-$stamp.log") -Root $RepoRoot -Label "build log"

# Disk-space defense added 2026-05-04 after a build silently failed mid-link
# with "There is not enough space on the disk. (os error 112)". Reclaims
# regeneratable artifacts when below WarnGB; aborts before starting if still
# below RequiredGB so we don't waste 30+ minutes on a doomed build.
Test-AndFreeDiskSpace -RepoRoot $RepoRoot -RequiredGB 5 -WarnGB 8

$buildRan = $false
if ($PSCmdlet.ShouldProcess($RepoRoot, "run $($plan.description)")) {
    $exitCode = Invoke-CodexBuild -Root $RepoRoot -CargoArgs $plan.cargo_args -EnvOverrides $plan.env_overrides -LogPath $logPath
    $buildRan = $true
    if ($exitCode -ne 0) {
        Show-FailureLines -Path $logPath
        throw "cargo build failed with exit code $exitCode. Log: $logPath"
    }
    # Post-build housekeeping: free release/incremental (unused for release
    # builds) so the next build starts with maximum headroom.
    Invoke-PostBuildDiskCleanup -RepoRoot $RepoRoot
}
else {
    [ordered]@{
        status = "planned"
        mode = $Mode
        cargo_args = $plan.cargo_args
        env_overrides = $plan.env_overrides
        log_path = $logPath
        release_binary = $releaseBinary
        deploy_skipped = [bool]$SkipDeploy
    } | ConvertTo-Json -Depth 8
    return
}

if ($buildRan -and -not (Test-Path -LiteralPath $releaseBinary)) {
    throw "Build did not produce $releaseBinary"
}

if ($buildRan -and -not $SkipVerify) {
    & $releaseBinary --version | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "Built Codex exe failed --version with exit code $LASTEXITCODE"
    }
}

$deployResult = $null
if (-not $SkipDeploy) {
    $deployResult = Invoke-Deploy -ExePath $releaseBinary -ModeName $Mode
}

[ordered]@{
    status = "ok"
    mode = $Mode
    cargo_args = $plan.cargo_args
    env_overrides = $plan.env_overrides
    log_path = $logPath
    release_binary = $releaseBinary
    deploy = $deployResult
} | ConvertTo-Json -Depth 8
