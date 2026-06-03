<#
.SYNOPSIS
    Regenerate all merge-sensitive generated artifacts (config schema, app-server
    protocol schema, bazel lockfile) in one shot, so they are never left stale after
    an upstream merge. Convenience wrapper around the root `justfile` recipes.

.DESCRIPTION
    Prevents the documented past-merge mistake: forgetting to regenerate
    schemas/locks after merging `upstream/main`, leaving generated files out of sync
    with the source types they are derived from.

    Runs (from repo root, via `just`):
        just write-config-schema     -> codex-rs/core/config.schema.json
        just write-app-server-schema -> codex-rs/app-server-protocol/schema/{json,typescript}
        just bazel-lock-update       -> MODULE.bazel.lock        (skipped with -SkipBazel)

    `just` itself sets `working-directory := codex-rs`, so the recipes run in the
    workspace dir regardless of where this wrapper is invoked from.

    Exit codes:
        0  success (all requested recipes ran clean; with -Check nothing was stale)
        1  a recipe failed, OR -Check found a regenerated file that differs from HEAD
        (a missing `just` is a non-fatal warning -> exit 0, prints the raw commands)

.PARAMETER Check
    After regenerating, run `git status --porcelain` over the schema/lock output
    paths and FAIL (exit 1) if anything changed -- i.e. the committed generated files
    were stale. Use this as a CI / preflight gate.

.PARAMETER SkipBazel
    Skip `just bazel-lock-update` (e.g. when bazel is not installed locally).

.PARAMETER DryRun
    Do NOT run any recipe. Only verify the root justfile exists and contains the
    three expected recipe names, and PRINT the commands that would run. Intended for
    offline/no-compile validation of this wrapper.

.PARAMETER RepoRoot
    Repo root. Defaults to this script's parent dir ($PSScriptRoot/..), so the script
    works from anywhere.

.EXAMPLE
    pwsh -File scripts/regen-all.ps1
    Regenerate everything.

.EXAMPLE
    pwsh -File scripts/regen-all.ps1 -Check -SkipBazel
    Regenerate config + app-server schema, then fail if either is now dirty.

.EXAMPLE
    pwsh -File scripts/regen-all.ps1 -DryRun
    Verify the justfile + recipes exist and print what would run, compiling nothing.
#>
[CmdletBinding()]
param(
    [switch]$Check,
    [switch]$SkipBazel,
    [switch]$DryRun,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

# --- Recipe + output-path plan ---------------------------------------------
# Each entry: the just recipe name and the paths -Check should watch for staleness.
$plan = [ordered]@{
    "write-config-schema"     = @("codex-rs/core/config.schema.json")
    "write-app-server-schema" = @("codex-rs/app-server-protocol/schema/")
    "bazel-lock-update"       = @("MODULE.bazel.lock", "codex-rs/Cargo.lock")
}

# Recipes to actually run this invocation (order matters; bazel last).
$recipes = @("write-config-schema", "write-app-server-schema")
if (-not $SkipBazel) { $recipes += "bazel-lock-update" }

$justfile = Join-Path $RepoRoot "justfile"
if (-not (Test-Path -LiteralPath $justfile -PathType Leaf)) {
    Write-Error "No justfile found at '$justfile'. Is -RepoRoot correct?"
    exit 1
}

# Underlying raw commands (printed for -DryRun and for the no-`just` fallback).
$rawCommands = [ordered]@{
    "write-config-schema"     = "cargo run --release -p codex-config --bin codex-write-config-schema  (run inside codex-rs/)"
    "write-app-server-schema" = "cargo run --release -p codex-app-server-protocol --bin write_schema_fixtures  (run inside codex-rs/)"
    "bazel-lock-update"       = "bazel mod deps --lockfile_mode=update"
}

# --- Verify the recipe names exist in the root justfile ---------------------
$justText = Get-Content -LiteralPath $justfile -Raw
$missingRecipes = @()
foreach ($recipe in @("write-config-schema", "write-app-server-schema", "bazel-lock-update")) {
    # A just recipe is declared as `name:` or `name *args:` at the start of a line.
    $pattern = "(?m)^$([regex]::Escape($recipe))(\s+\*?\w+)?\s*:"
    if ($justText -notmatch $pattern) { $missingRecipes += $recipe }
}
if ($missingRecipes.Count -gt 0) {
    Write-Error ("Root justfile is missing expected recipe(s): " + ($missingRecipes -join ", ") +
        ". Has the justfile changed? Update regen-all.ps1's plan.")
    exit 1
}

# --- DryRun: verify + print, run nothing ------------------------------------
if ($DryRun) {
    Write-Host "[regen-all] DryRun -- justfile: $justfile" -ForegroundColor Cyan
    Write-Host "[regen-all] All 3 expected recipes present in root justfile: write-config-schema, write-app-server-schema, bazel-lock-update" -ForegroundColor Green
    Write-Host ""
    Write-Host "Would run (from repo root '$RepoRoot'):"
    foreach ($recipe in $recipes) {
        Write-Host "    just $recipe"
        Write-Host "        # underlying: $($rawCommands[$recipe])"
    }
    if ($SkipBazel) { Write-Host "    (bazel-lock-update SKIPPED via -SkipBazel)" }
    if ($Check) {
        Write-Host ""
        Write-Host "Then with -Check, would verify these paths are clean via 'git status --porcelain':"
        foreach ($recipe in $recipes) {
            foreach ($p in $plan[$recipe]) { Write-Host "    $p" }
        }
    }
    exit 0
}

# --- Is `just` available? Non-fatal if not. ---------------------------------
$haveJust = $null -ne (Get-Command just -ErrorAction SilentlyContinue)
if (-not $haveJust) {
    Write-Warning "`just` is not on PATH; cannot run the recipes automatically."
    Write-Host "Install just (cargo install just) or run the underlying commands manually:" -ForegroundColor Yellow
    foreach ($recipe in $recipes) {
        Write-Host "    # $recipe"
        Write-Host "    $($rawCommands[$recipe])"
    }
    Write-Host ""
    Write-Host "This is a convenience wrapper -- not hard-failing." -ForegroundColor Yellow
    exit 0
}

# --- Run the recipes from repo root -----------------------------------------
Push-Location $RepoRoot
try {
    $failed = @()
    foreach ($recipe in $recipes) {
        Write-Host "[regen-all] just $recipe" -ForegroundColor Cyan
        & just $recipe
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[regen-all] FAILED: just $recipe (exit $LASTEXITCODE)" -ForegroundColor Red
            $failed += $recipe
        }
    }

    if ($failed.Count -gt 0) {
        Write-Error ("Recipe(s) failed: " + ($failed -join ", "))
        exit 1
    }

    # --- -Check: fail if any watched output path is now dirty ----------------
    if ($Check) {
        $watchPaths = New-Object System.Collections.Generic.List[string]
        foreach ($recipe in $recipes) {
            foreach ($p in $plan[$recipe]) { $watchPaths.Add($p) }
        }
        Write-Host ""
        Write-Host "[regen-all] -Check: inspecting generated paths for staleness..." -ForegroundColor Cyan
        $status = & git -C $RepoRoot status --porcelain -- @($watchPaths)
        $dirty = @($status | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        if ($dirty.Count -gt 0) {
            Write-Host "[regen-all] STALE generated files detected (regenerate and commit them):" -ForegroundColor Red
            foreach ($line in $dirty) { Write-Host "    $line" }
            Write-Error "Generated schemas/locks were stale -> exit 1"
            exit 1
        }
        Write-Host "[regen-all] -Check OK: all generated schemas/locks already up to date." -ForegroundColor Green
    }
}
finally {
    Pop-Location
}

Write-Host "[regen-all] done." -ForegroundColor Green
exit 0
