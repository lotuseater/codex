# PostToolUse hook (matcher: Edit|Write|MultiEdit)
# If the edited file is .rs, format ONLY that one file with the repo's rustfmt
# settings (edition 2024, imports_granularity=Item). Never fail the tool: any
# missing rustfmt / error is swallowed. Always exit 0.

$ErrorActionPreference = 'SilentlyContinue'

try {
    $raw = [Console]::In.ReadToEnd()
} catch {
    exit 0
}
if ([string]::IsNullOrWhiteSpace($raw)) { exit 0 }

try {
    $j = $raw | ConvertFrom-Json
} catch {
    exit 0
}

$fp = $null
try { $fp = $j.tool_input.file_path } catch { $fp = $null }
if ([string]::IsNullOrWhiteSpace($fp)) { exit 0 }

if ($fp -notmatch '\.rs$') { exit 0 }
if (-not (Test-Path -LiteralPath $fp)) { exit 0 }

# Resolve rustfmt via the active toolchain (PATH). Fall back to `rustup run`.
$rustfmt = $null
try { $rustfmt = (Get-Command rustfmt -ErrorAction SilentlyContinue).Source } catch {}

try {
    if ($rustfmt) {
        & $rustfmt --edition 2024 --config imports_granularity=Item -- "$fp" 2>$null | Out-Null
    } else {
        $rustup = (Get-Command rustup -ErrorAction SilentlyContinue).Source
        if ($rustup) {
            & $rustup run stable rustfmt --edition 2024 --config imports_granularity=Item -- "$fp" 2>$null | Out-Null
        }
    }
} catch {
    # swallow — never fail the tool
}

exit 0
