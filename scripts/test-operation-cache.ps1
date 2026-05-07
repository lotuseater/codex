param(
    [string]$WizardRoot = "C:\Users\Oleh\Documents\GitHub\Wizard_Erasmus",

    [string]$LogDir = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")).Path "logs"),

    [int]$Jobs = 2,

    [switch]$SkipPython,

    [switch]$SkipRust
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$codexRs = Join-Path $repoRoot "codex-rs"
$wizardRootFull = (Resolve-Path -LiteralPath $WizardRoot).Path

if (-not $SkipPython) {
    Push-Location $wizardRootFull
    try {
        python -m pytest -q `
            src/mcp/test_codex_cache_bridge_cli.py `
            src/mcp/test_codex_cache_bridge.py `
            -k "cli or codex_hits_claude_read_entry or claude_hits_codex_stored_entry or codex_shell_grep_canonicalization_is_conservative"
        if ($LASTEXITCODE -ne 0) {
            throw "Wizard operation-cache bridge tests failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

if (-not $SkipRust) {
    New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $log = Join-Path $LogDir "codex-core-operation-cache-lib-test-$stamp.log"

    Push-Location $codexRs
    try {
        $oldRustMinStack = $env:RUST_MIN_STACK
        $oldNativeCommandPreference = $PSNativeCommandUseErrorActionPreference

        $env:RUST_MIN_STACK = "33554432"
        $PSNativeCommandUseErrorActionPreference = $false

        cargo test -p codex-core --lib operation_cache --release -j $Jobs *> $log
        $exit = $LASTEXITCODE
    }
    finally {
        $env:RUST_MIN_STACK = $oldRustMinStack
        $PSNativeCommandUseErrorActionPreference = $oldNativeCommandPreference
        Pop-Location
    }

    Get-Content -LiteralPath $log -Tail 120
    if ($exit -ne 0) {
        throw "Codex operation-cache lib tests failed with exit code $exit. Log: $log"
    }
}
