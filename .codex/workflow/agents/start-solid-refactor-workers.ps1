$script = Join-Path $PSScriptRoot "start-codex-workers.ps1"
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script -Pattern "solid_refactor_wave3_*.prompt.md" @args
exit $LASTEXITCODE
