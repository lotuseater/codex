# check_severance_net.ps1 — verify every fork anchor still present in the merged tree.
# Replaces the buggy bash `grep -cE ... || echo 0` net (which printed "0\n0" on a miss,
# breaking arithmetic). PowerShell Select-String -SimpleMatch = literal, no shell trap.
#
# Usage:
#   pwsh -File .codex/tmp/merge_2026-06-20/check_severance_net.ps1            # full 47-anchor net
#   pwsh -File ... -Net .codex/tmp/merge_2026-06-20/anchors/C1_session.txt    # one slice's anchors
#
# TSV format: <repo-relative file><TAB><literal anchor string>
# Exit 0 always (this is a REPORT, not a gate); MISSING list = investigate union-preserve.
param(
  [string]$Net  = "$PSScriptRoot/severance_net.tsv",
  [string]$Repo = "C:/Users/Oleh/Documents/GitHub/open_ai/codex"
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Net)) { Write-Host "NET NOT FOUND: $Net"; exit 0 }

$missing = New-Object System.Collections.Generic.List[string]
$nofile  = New-Object System.Collections.Generic.List[string]
$present = 0
$total   = 0

foreach ($line in Get-Content -LiteralPath $Net) {
  if ([string]::IsNullOrWhiteSpace($line)) { continue }
  $parts = $line -split "`t", 2
  if ($parts.Count -lt 2) { continue }   # skip malformed rows
  $total++
  $file   = $parts[0].Trim()
  $anchor = $parts[1].Trim()
  $full   = Join-Path $Repo $file
  if (-not (Test-Path -LiteralPath $full)) { $nofile.Add("$file`t$anchor"); continue }
  $hit = Select-String -LiteralPath $full -SimpleMatch -Pattern $anchor -List -ErrorAction SilentlyContinue
  if ($hit) { $present++ } else { $missing.Add("$file`t$anchor") }
}

Write-Host "ANCHORS=$total  PRESENT=$present  MISSING=$($missing.Count)  NOFILE=$($nofile.Count)"
if ($missing.Count) {
  Write-Host "`n--- MISSING (fork anchor not found — investigate; severance suspected) ---"
  $missing | ForEach-Object { Write-Host "  $_" }
}
if ($nofile.Count) {
  Write-Host "`n--- FILE NOT FOUND (rename/delete — confirm intended) ---"
  $nofile | ForEach-Object { Write-Host "  $_" }
}
if ($missing.Count -eq 0 -and $nofile.Count -eq 0) { Write-Host "ALL FORK ANCHORS PRESENT." }
exit 0
