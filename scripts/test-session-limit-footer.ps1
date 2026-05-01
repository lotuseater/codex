param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
    [string]$CodexExe = "",
    [switch]$SkipWrapperSmoke
)

$ErrorActionPreference = "Stop"

function Assert-FileContains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$Pattern,
        [Parameter(Mandatory = $true)]
        [string]$Description
    )

    $fullPath = Join-Path $RepoRoot $Path
    if (-not (Test-Path -LiteralPath $fullPath)) {
        throw "Missing file for $Description`: $Path"
    }

    $content = Get-Content -LiteralPath $fullPath -Raw
    if ($content -notmatch $Pattern) {
        throw "Missing $Description in $Path"
    }
}

Assert-FileContains `
    -Path "codex-rs/tui/src/status/rate_limits.rs" `
    -Pattern "resets_at_datetime:\s*Option<DateTime<Local>>" `
    -Description "raw reset time preserved for footer elapsed percent"

Assert-FileContains `
    -Path "codex-rs/tui/src/bottom_pane/chat_composer.rs" `
    -Pattern "set_session_limit_status_line" `
    -Description "composer session limit footer setter"

Assert-FileContains `
    -Path "codex-rs/tui/src/bottom_pane/chat_composer.rs" `
    -Pattern "combine_right_context_lines" `
    -Description "right-aligned footer line combiner"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget.rs" `
    -Pattern "mod session_limit_footer;" `
    -Description "dedicated footer production module registration"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/session_limit_footer.rs" `
    -Pattern "fn token_used_percent" `
    -Description "token used percentage calculation"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/session_limit_footer.rs" `
    -Pattern "fn reset_elapsed_percent" `
    -Description "reset elapsed percentage calculation"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/tests.rs" `
    -Pattern "mod session_limit_footer;" `
    -Description "dedicated footer test module registration"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/session_limit_footer.rs" `
    -Pattern "combines_token_and_reset_percentages" `
    -Description "decoupled footer formatting test"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/session_limit_footer.rs" `
    -Pattern "renders_reset_percentage_without_token_usage" `
    -Description "reset-only footer formatting test"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/session_limit_footer.rs" `
    -Pattern "uses_secondary_reset_window_when_primary_has_no_reset_metadata" `
    -Description "secondary reset-window fallback test"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/tests/session_limit_footer.rs" `
    -Pattern "renders_in_bottom_right_context" `
    -Description "focused footer render test"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__session_limit_footer_right_status.snap" `
    -Pattern "70% tokens\s+.*50% reset" `
    -Description "accepted footer snapshot"

Assert-FileContains `
    -Path "codex-rs/tui/src/chatwidget/snapshots/codex_tui__chatwidget__tests__session_limit_footer_with_side_context.snap" `
    -Pattern "Side from main thread\s+.*70% tokens\s+.*50% reset" `
    -Description "accepted side-context footer snapshot"

if ([string]::IsNullOrWhiteSpace($CodexExe)) {
    $wrapperEnv = Join-Path $env:USERPROFILE ".codex/system-wrapper/system.codex-wrapper.env.json"
    if (Test-Path -LiteralPath $wrapperEnv) {
        $envJson = Get-Content -LiteralPath $wrapperEnv -Raw | ConvertFrom-Json
        $CodexExe = $envJson.WIZARD_CODEX_REAL_EXE
        Write-Output "Wrapper real exe: $CodexExe"
    }
}

if (-not [string]::IsNullOrWhiteSpace($CodexExe)) {
    if (-not (Test-Path -LiteralPath $CodexExe)) {
        throw "Configured Codex exe does not exist: $CodexExe"
    }

    $version = & $CodexExe --version
    if ($LASTEXITCODE -ne 0) {
        throw "Codex exe smoke check failed with exit code $LASTEXITCODE"
    }
    Write-Output "Codex exe smoke: $version"
}

if (-not $SkipWrapperSmoke) {
    $wrapperVersion = & codex --version
    if ($LASTEXITCODE -ne 0) {
        throw "Codex wrapper smoke check failed with exit code $LASTEXITCODE"
    }
    Write-Output "Codex wrapper smoke: $wrapperVersion"

    $wrapperHelp = & codex --help
    if ($LASTEXITCODE -ne 0) {
        throw "Codex wrapper help smoke check failed with exit code $LASTEXITCODE"
    }
    $wrapperHelpText = $wrapperHelp -join "`n"
    if ($wrapperHelpText -notmatch "Codex CLI" -or $wrapperHelpText -notmatch "--no-alt-screen") {
        throw "Codex wrapper help output did not look like the interactive CLI entrypoint"
    }
    Write-Output "Codex wrapper help smoke: ok"
}

Write-Output "Session limit footer checks passed."
