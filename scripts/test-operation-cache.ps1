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

$toolsMod = Join-Path $repoRoot "codex-rs\core\src\tools\mod.rs"
$toolsRegistry = Join-Path $repoRoot "codex-rs\core\src\tools\registry.rs"
$operationCache = Join-Path $repoRoot "codex-rs\core\src\tools\operation_cache.rs"

function Assert-FileContainsText {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Needle
    )

    $content = Get-Content -Raw -LiteralPath $Path
    if (-not $content.Contains($Needle)) {
        throw "$Path is missing required operation-cache wiring: $Needle"
    }
}

Assert-FileContainsText -Path $toolsMod -Needle "pub(crate) mod operation_cache;"
Assert-FileContainsText -Path $toolsRegistry -Needle "operation_cache::lookup"
Assert-FileContainsText -Path $toolsRegistry -Needle "operation_cache::store"
Assert-FileContainsText -Path $operationCache -Needle "mod tests"
Assert-FileContainsText -Path $operationCache -Needle "tool_is_cacheable"

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

    $logText = Get-Content -Raw -LiteralPath $log
    if ($logText -match "running\s+0\s+tests") {
        throw "Codex operation-cache lib test filter matched zero tests. Log: $log"
    }
    if ($logText -notmatch "operation_cache::tests::") {
        throw "Codex operation-cache lib test log did not show operation_cache tests. Log: $log"
    }
}
