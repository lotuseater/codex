# Stop hook (read-only).
# Anti-loop: if stop_hook_active is true, exit 0 immediately.
# Otherwise inspect changed files (working + staged) and PRINT reminders to
# regenerate schemas / locks. Never edits any file. Always exit 0.

$ErrorActionPreference = 'SilentlyContinue'

$repo = 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'

try {
    $raw = [Console]::In.ReadToEnd()
} catch {
    exit 0
}

if (-not [string]::IsNullOrWhiteSpace($raw)) {
    try {
        $j = $raw | ConvertFrom-Json
        if ($j.stop_hook_active -eq $true) { exit 0 }
    } catch {
        # malformed stdin: continue as a normal (non-recursive) stop
    }
}

# Collect changed paths (working tree + staged). Tolerate git failure.
$paths = @()
try { $paths += git -C $repo diff --name-only 2>$null } catch {}
try { $paths += git -C $repo diff --cached --name-only 2>$null } catch {}
$paths = $paths | Where-Object { $_ -and $_.Trim() -ne '' } | Sort-Object -Unique
if (-not $paths -or $paths.Count -eq 0) { exit 0 }

$reminders = New-Object System.Collections.Generic.List[string]

# Config types changed -> regenerate config schema.
if ($paths | Where-Object { $_ -match 'config/src/config_toml\.rs$' -or $_ -match 'config/src/' }) {
    $reminders.Add('Config types changed: run `just write-config-schema`.')
}

# App-server protocol changed -> regenerate app-server schema + test.
if ($paths | Where-Object { $_ -match 'app-server-protocol/src/protocol/' }) {
    $reminders.Add('App-server protocol changed: run `just write-app-server-schema` + `just test -p codex-app-server-protocol`.')
}

# Cargo manifest/lock changed -> update + check bazel lock.
if ($paths | Where-Object { $_ -match '(^|/)Cargo\.toml$' -or $_ -match '(^|/)Cargo\.lock$' }) {
    $reminders.Add('Cargo.toml/Cargo.lock changed: run `just bazel-lock-update` + `just bazel-lock-check`.')
}

if ($reminders.Count -gt 0) {
    # Stop hooks: additionalContext JSON is NOT valid here — print plain text.
    Write-Output 'Reminders before finishing:'
    foreach ($r in $reminders) { Write-Output ("  - " + $r) }
}

exit 0
