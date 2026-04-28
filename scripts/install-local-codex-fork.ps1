[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [ValidateSet("Install", "Verify", "Rollback")]
    [string]$Action = "Install",

    [string]$RepoRoot,

    [string]$WrapperDir = (Join-Path $HOME ".codex\system-wrapper"),

    [string]$BackupRoot = (Join-Path $HOME ".codex\binary-backups"),

    [switch]$SkipBuild,

    [switch]$RunSmoke
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

function Resolve-LocalCodexBinary {
    param([string]$Root)

    $binary = Join-Path $Root "codex-rs\target\release\codex.exe"
    if (-not (Test-Path -LiteralPath $binary)) {
        throw "Local Codex binary not found at $binary. Run Install without -SkipBuild first."
    }
    return (Resolve-Path -LiteralPath $binary).Path
}

function Get-WrapperEnvPath {
    param([string]$Dir)

    $path = Join-Path $Dir "system.codex-wrapper.env.json"
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Wrapper env JSON not found at $path"
    }
    return $path
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

function New-Backup {
    param(
        [string]$Dir,
        [string]$EnvPath,
        [string]$CodexBinary,
        [hashtable]$PreviousEnv
    )

    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $backupDir = Join-Path $Dir "local-codex-fork-$stamp"
    New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

    $envBackup = Join-Path $backupDir "system.codex-wrapper.env.json.bak"
    Copy-Item -LiteralPath $EnvPath -Destination $envBackup -Force

    foreach ($name in @("codex.ps1", "codex.cmd")) {
        $wrapperPath = Join-Path $WrapperDir $name
        if (Test-Path -LiteralPath $wrapperPath) {
            Copy-Item -LiteralPath $wrapperPath -Destination (Join-Path $backupDir "$name.bak") -Force
        }
    }

    $manifest = [ordered]@{
        created_at = (Get-Date).ToString("o")
        action = "install-local-codex-fork"
        repo_root = $RepoRoot
        local_codex_binary = $CodexBinary
        wrapper_dir = $WrapperDir
        wrapper_env_path = $EnvPath
        wrapper_env_backup = $envBackup
        previous_real_exe = $PreviousEnv["WIZARD_CODEX_REAL_EXE"]
    }
    $manifestPath = Join-Path $backupDir "manifest.json"
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
    return $manifestPath
}

function Invoke-Build {
    param([string]$Root)

    Push-Location (Join-Path $Root "codex-rs")
    try {
        cargo build --release --bin codex
    }
    finally {
        Pop-Location
    }
}

function Invoke-Install {
    if (-not $SkipBuild) {
        Invoke-Build -Root $RepoRoot
    }

    $binary = Resolve-LocalCodexBinary -Root $RepoRoot
    $envPath = Get-WrapperEnvPath -Dir $WrapperDir
    $payload = Read-JsonObject -Path $envPath

    $manifestPath = New-Backup -Dir $BackupRoot -EnvPath $envPath -CodexBinary $binary -PreviousEnv $payload

    $payload["WIZARD_CODEX_REAL_EXE"] = $binary
    $payload["WIZARD_CODEX_LOCAL_FORK_REPO"] = $RepoRoot
    $payload["WIZARD_CODEX_LOCAL_FORK_MANIFEST"] = $manifestPath
    $payload["WIZARD_CODEX_LOCAL_FORK_INSTALLED_AT"] = (Get-Date).ToString("o")

    if ($PSCmdlet.ShouldProcess($envPath, "point WIZARD_CODEX_REAL_EXE at $binary")) {
        Write-JsonObject -Path $envPath -Payload $payload
    }

    Invoke-Verify
}

function Invoke-Verify {
    $envPath = Get-WrapperEnvPath -Dir $WrapperDir
    $payload = Read-JsonObject -Path $envPath
    $realExe = [string]$payload["WIZARD_CODEX_REAL_EXE"]

    if ([string]::IsNullOrWhiteSpace($realExe)) {
        throw "WIZARD_CODEX_REAL_EXE is empty in $envPath"
    }
    if (-not (Test-Path -LiteralPath $realExe)) {
        throw "WIZARD_CODEX_REAL_EXE target does not exist: $realExe"
    }

    $commands = Get-Command codex -All
    if (-not $commands) {
        throw "codex command is not discoverable on PATH"
    }

    $firstPath = [string]$commands[0].Path
    $expectedPrefix = (Resolve-Path -LiteralPath $WrapperDir).Path
    if (-not $firstPath.StartsWith($expectedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "First codex command is $firstPath, expected it under $expectedPrefix"
    }

    $version = & codex --version
    if ($LASTEXITCODE -ne 0) {
        throw "codex --version failed with exit code $LASTEXITCODE"
    }

    if ($RunSmoke) {
        & codex exec --skip-git-repo-check "Say ok."
        if ($LASTEXITCODE -ne 0) {
            throw "codex exec smoke failed with exit code $LASTEXITCODE"
        }
    }

    [ordered]@{
        status = "ok"
        wrapper_env_path = $envPath
        real_exe = $realExe
        first_codex_path = $firstPath
        version = ($version -join "`n")
    } | ConvertTo-Json -Depth 4
}

function Invoke-Rollback {
    $manifests = Get-ChildItem -LiteralPath $BackupRoot -Filter manifest.json -Recurse |
        Where-Object { $_.FullName -like "*local-codex-fork-*" } |
        Sort-Object LastWriteTime -Descending

    if (-not $manifests) {
        throw "No local-codex-fork rollback manifest found under $BackupRoot"
    }

    $manifest = Read-JsonObject -Path $manifests[0].FullName
    $envBackup = [string]$manifest["wrapper_env_backup"]
    $envPath = [string]$manifest["wrapper_env_path"]
    if (-not (Test-Path -LiteralPath $envBackup)) {
        throw "Rollback env backup does not exist: $envBackup"
    }

    if ($PSCmdlet.ShouldProcess($envPath, "restore wrapper env from $envBackup")) {
        Copy-Item -LiteralPath $envBackup -Destination $envPath -Force
    }

    Invoke-Verify
}

switch ($Action) {
    "Install" { Invoke-Install }
    "Verify" { Invoke-Verify }
    "Rollback" { Invoke-Rollback }
}
