# dep-snapshot.ps1
#
# Persistent dependency-graph snapshot for the codex-rs Cargo workspace.
# Writes a Markdown report into docs/dep-snapshot.md. Intended to be re-run
# whenever Cargo.toml or Cargo.lock changes; safe to commit so future sessions
# don't have to recompute the survey.
#
# Sections:
#   1. Workspace direct deps (from `[workspace.dependencies]`).
#   2. Per-crate non-workspace pins (consolidation candidates: each entry is a
#      `dep = "x.y.z"` or `dep = { version = "x.y.z", ... }` style pin in a
#      per-crate Cargo.toml that should usually be `dep = { workspace = true }`).
#   3. Duplicate-version families from `cargo tree --duplicates --workspace`,
#      with the immediate parents that pull each version.
#
# Usage: pwsh -NoProfile -File scripts/dep-snapshot.ps1
# Output: docs/dep-snapshot.md (overwrites previous snapshot).

param(
    [string]$RepoRoot = (Resolve-Path "$PSScriptRoot\..").Path,
    [string]$OutFile  = $null
)

$ErrorActionPreference = 'Stop'
$cargoRoot = Join-Path $RepoRoot 'codex-rs'
if (-not (Test-Path $cargoRoot)) { throw "codex-rs/ not found under $RepoRoot" }
if (-not $OutFile) { $OutFile = Join-Path $RepoRoot 'docs/dep-snapshot.md' }

Write-Host "Snapshot root: $cargoRoot"
Write-Host "Writing to:    $OutFile"

# ---- 1. Workspace direct deps ----
$wsCargo = Join-Path $cargoRoot 'Cargo.toml'
$wsLines = Get-Content $wsCargo
$inWs = $false
$wsDeps = @{}
foreach ($line in $wsLines) {
    if ($line -match '^\[workspace\.dependencies\]') { $inWs = $true; continue }
    if ($line -match '^\[' -and $inWs) { $inWs = $false }
    if (-not $inWs) { continue }
    if ($line -match '^([A-Za-z0-9_-]+)\s*=\s*(.+)$') {
        $wsDeps[$Matches[1]] = $Matches[2].Trim()
    }
}
Write-Host ("Workspace deps: {0}" -f $wsDeps.Count)

# ---- 2. Per-crate non-workspace pins ----
$pins = @()
$crateManifests = Get-ChildItem -Path $cargoRoot -Recurse -Filter 'Cargo.toml' -File `
    | Where-Object { $_.FullName -ne $wsCargo -and $_.FullName -notmatch '\\target\\' }

function Add-PinIfWorkspaceCandidate {
    param(
        [string]$Crate,
        [string]$Section,
        [string]$Line
    )

    if ($Line -match '^([A-Za-z0-9_-]+)\s*=\s*"([^"]+)"\s*$') {
        $name = $Matches[1]
        $ver  = $Matches[2]
        if ($wsDeps.ContainsKey($name)) {
            return [pscustomobject]@{
                Crate     = $Crate
                Section   = $Section
                Dep       = $name
                DirectPin = $ver
                Workspace = $wsDeps[$name]
            }
        }
    }

    if ($Line -match '^([A-Za-z0-9_-]+)\s*=\s*(\{\s*.*\s*\})\s*$') {
        $name = $Matches[1]
        $expr = $Matches[2].Trim()
        if ($wsDeps.ContainsKey($name) -and $expr -notmatch 'workspace\s*=\s*true' -and $expr -match 'version\s*=\s*"([^"]+)"') {
            return [pscustomobject]@{
                Crate     = $Crate
                Section   = $Section
                Dep       = $name
                DirectPin = ($expr -replace '\s+', ' ')
                Workspace = $wsDeps[$name]
            }
        }
    }

    return $null
}

foreach ($mf in $crateManifests) {
    $crate = $mf.Directory.FullName.Substring($cargoRoot.Length).TrimStart('\','/')
    $content = Get-Content $mf.FullName
    $section = $null
    $pending = $null
    $braceDepth = 0
    foreach ($line in $content) {
        if ($line -match '^\[(.+)\]\s*$') {
            $section = $Matches[1]
            $pending = $null
            $braceDepth = 0
            continue
        }

        if ($pending) {
            $pending += " $($line.Trim())"
            $braceDepth += ([regex]::Matches($line, '\{')).Count
            $braceDepth -= ([regex]::Matches($line, '\}')).Count
            if ($braceDepth -le 0) {
                $pin = Add-PinIfWorkspaceCandidate -Crate $crate -Section $section -Line $pending
                if ($pin) { $pins += $pin }
                $pending = $null
                $braceDepth = 0
            }
            continue
        }

        if (-not $section) { continue }
        # only [dependencies], [dev-dependencies], [build-dependencies], plus target.* variants
        if ($section -notmatch '^(target\..+\.)?(dev-|build-)?dependencies$') { continue }

        if ($line -match '^([A-Za-z0-9_-]+)\s*=\s*\{' -and $line -notmatch '\}\s*$') {
            $pending = $line.Trim()
            $braceDepth = ([regex]::Matches($line, '\{')).Count - ([regex]::Matches($line, '\}')).Count
            continue
        }

        $pin = Add-PinIfWorkspaceCandidate -Crate $crate -Section $section -Line $line.Trim()
        if ($pin) {
            $pins += $pin
        }
    }
}
Write-Host ("Non-workspace pins (consolidation candidates): {0}" -f $pins.Count)

# ---- 3. cargo tree --duplicates ----
Push-Location $cargoRoot
try {
    Write-Host 'Running cargo tree --duplicates...'
    $treeRaw = & cargo tree --duplicates --workspace --depth 1 2>&1
} finally {
    Pop-Location
}

# Group raw output into per-family blocks separated by blank line.
$blocks = @()
$cur = New-Object System.Collections.Generic.List[string]
foreach ($l in $treeRaw) {
    if ([string]::IsNullOrWhiteSpace($l)) {
        if ($cur.Count -gt 0) { $blocks += ,@($cur.ToArray()) ; $cur.Clear() }
    } else {
        [void]$cur.Add([string]$l)
    }
}
if ($cur.Count -gt 0) { $blocks += ,@($cur.ToArray()) }

# Each block: header line is "<name> <version> [...]"; rest are parents.
$dupFamilies = @{}
foreach ($block in $blocks) {
    $head = $block[0]
    if ($head -match '^([A-Za-z0-9_-]+)\s+v([0-9][^\s]*)') {
        $name = $Matches[1]
        $ver  = $Matches[2]
        $parents = @()
        if ($block.Count -gt 1) {
            foreach ($pl in $block[1..($block.Count-1)]) {
                if ($pl -match '^[^A-Za-z0-9_-]*([A-Za-z0-9_-]+)\s+v([0-9][^\s]*)') {
                    $parents += "$($Matches[1])@$($Matches[2])"
                }
            }
        }
        if (-not $dupFamilies.ContainsKey($name)) { $dupFamilies[$name] = @{} }
        if (-not $dupFamilies[$name].ContainsKey($ver)) { $dupFamilies[$name][$ver] = @() }
        $dupFamilies[$name][$ver] += $parents
    }
}

# Filter to families with > 1 distinct version (real cross-version dups).
$realDups = @{}
foreach ($k in $dupFamilies.Keys) {
    if ($dupFamilies[$k].Keys.Count -gt 1) {
        $realDups[$k] = $dupFamilies[$k]
    }
}
Write-Host ("Real cross-version dup families: {0}" -f $realDups.Count)

# ---- write Markdown ----
$now  = Get-Date -Format 'yyyy-MM-ddTHH:mm:ssK'
$lock = Join-Path $cargoRoot 'Cargo.lock'
$lockTotal = (Select-String -Path $lock -Pattern '^name = ').Count
$treeDupVersionRows = 0
foreach ($familyName in $realDups.Keys) {
    $treeDupVersionRows += $realDups[$familyName].Keys.Count
}

$out = New-Object System.Collections.Generic.List[string]
$out.Add("# codex-rs dep snapshot")
$out.Add('')
$out.Add("Generated: $now (regenerate via ``scripts/dep-snapshot.ps1``).")
$out.Add('')
$out.Add("## Lockfile census")
$out.Add('')
$out.Add("- Total locked entries: **$lockTotal**")
$out.Add("- Cross-version duplicate families: **$($realDups.Count)** (sum $treeDupVersionRows version entries)")
$out.Add("- Workspace-declared deps: **$($wsDeps.Count)**")
$out.Add("- Per-crate non-workspace pins (consolidation candidates): **$($pins.Count)**")
$out.Add('')

$out.Add("## Per-crate pins to consolidate")
$out.Add('')
if ($pins.Count -eq 0) {
    $out.Add("_None — every per-crate dep that has a workspace declaration uses ``{ workspace = true }``._")
} else {
    $out.Add("Each row is a per-crate Cargo.toml pin that bypasses the workspace declaration. Replacing with ``{ workspace = true }`` (preserving any features) eliminates a hidden source of version skew.")
    $out.Add('')
    $out.Add("| Crate | Section | Dep | Direct pin | Workspace pin |")
    $out.Add("|---|---|---|---|---|")
    foreach ($p in ($pins | Sort-Object Crate, Dep)) {
        $ws = $p.Workspace -replace '\|','\|'
        $out.Add("| $($p.Crate) | $($p.Section) | $($p.Dep) | ``$($p.DirectPin)`` | ``$ws`` |")
    }
}
$out.Add('')

$out.Add("## Cross-version duplicate families")
$out.Add('')
if ($realDups.Count -eq 0) {
    $out.Add("_None._")
} else {
    foreach ($name in ($realDups.Keys | Sort-Object)) {
        $vers = $realDups[$name]
        $out.Add("### $name")
        $out.Add('')
        foreach ($v in ($vers.Keys | Sort-Object)) {
            $parents = $vers[$v] | Sort-Object -Unique
            if ($parents.Count -eq 0) {
                $parents = @('_no immediate parent parsed_')
            }
            $out.Add("- **v$v** ← $($parents -join ', ')")
        }
        $out.Add('')
    }
}

[IO.File]::WriteAllLines($OutFile, $out)
Write-Host "Wrote $OutFile"
