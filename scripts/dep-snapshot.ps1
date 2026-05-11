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

function Get-CargoTreeDupFamilies {
    # Runs `cargo tree <TreeArgs>` from $CargoRoot, parses the
    # blank-line-separated duplicate blocks, and returns
    # @{ Label; RealDups; Error }. RealDups is a hashtable keyed by
    # crate name, each value a hashtable keyed by version with the
    # immediate-parent list.
    param(
        [string[]]$TreeArgs,
        [string]$CargoRoot,
        [string]$Label
    )

    Push-Location $CargoRoot
    try {
        Write-Host ("Running cargo tree {0} ..." -f ($TreeArgs -join ' '))
        $raw  = & cargo tree @TreeArgs 2>&1
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }

    if ($code -ne 0) {
        return [pscustomobject]@{
            Label    = $Label
            RealDups = @{}
            Error    = ($raw | Out-String).Trim()
        }
    }

    $blocks = @()
    $cur = New-Object System.Collections.Generic.List[string]
    foreach ($l in $raw) {
        if ([string]::IsNullOrWhiteSpace($l)) {
            if ($cur.Count -gt 0) { $blocks += ,@($cur.ToArray()) ; $cur.Clear() }
        } else {
            [void]$cur.Add([string]$l)
        }
    }
    if ($cur.Count -gt 0) { $blocks += ,@($cur.ToArray()) }

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

    $realDups = @{}
    foreach ($k in $dupFamilies.Keys) {
        if ($dupFamilies[$k].Keys.Count -gt 1) {
            $realDups[$k] = $dupFamilies[$k]
        }
    }

    return [pscustomobject]@{
        Label    = $Label
        RealDups = $realDups
        Error    = $null
    }
}

function Get-ReleaseArtifactFamilies {
    # Enumerates target/release/deps/*.rlib + *.rmeta, groups by crate
    # root (the file basename minus the trailing -<hash>), returns the
    # top-N rows sorted by total bytes descending. Returns $null if
    # target/release/deps does not exist (skip the section gracefully).
    param([string]$CargoRoot, [int]$TopN = 25)

    $depsDir = Join-Path $CargoRoot 'target/release/deps'
    if (-not (Test-Path -LiteralPath $depsDir)) { return $null }

    $files = @()
    $files += Get-ChildItem -LiteralPath $depsDir -Filter '*.rlib'  -File -ErrorAction SilentlyContinue
    $files += Get-ChildItem -LiteralPath $depsDir -Filter '*.rmeta' -File -ErrorAction SilentlyContinue
    if (-not $files -or $files.Count -eq 0) { return @() }

    $byCrate = @{}
    foreach ($f in $files) {
        $base = $f.BaseName
        if ($base.StartsWith('lib')) { $base = $base.Substring(3) }
        if ($base -match '^(.+)-[0-9a-f]{8,}$') {
            $crate = $Matches[1]
        } else {
            $crate = $base
        }
        if (-not $byCrate.ContainsKey($crate)) {
            $byCrate[$crate] = [pscustomobject]@{
                Crate = $crate
                Count = 0
                Bytes = [int64]0
            }
        }
        $byCrate[$crate].Count += 1
        $byCrate[$crate].Bytes += [int64]$f.Length
    }

    return @($byCrate.Values | Sort-Object -Property Bytes -Descending | Select-Object -First $TopN)
}

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

# ---- 3. cargo tree --duplicates (workspace-wide + codex-cli deploy graph) ----
$wsDup = Get-CargoTreeDupFamilies `
    -TreeArgs @('--duplicates','--workspace','--depth','1') `
    -CargoRoot $cargoRoot `
    -Label 'workspace-wide'
$realDups = $wsDup.RealDups
Write-Host ("Real cross-version dup families (workspace): {0}" -f $realDups.Count)

# Strict subgraph of what compiles into codex.exe: no dev edges, no isolated
# test crates. Families here are the ones that would actually shrink the
# deployed binary if collapsed.
$deployDup = Get-CargoTreeDupFamilies `
    -TreeArgs @('-p','codex-cli','--duplicates','--edges','normal,build','--depth','1') `
    -CargoRoot $cargoRoot `
    -Label 'codex-cli (normal+build)'
if ($deployDup.Error) {
    Write-Host ("Deploy-graph cargo tree failed: {0}" -f $deployDup.Error.Split([Environment]::NewLine)[0])
} else {
    Write-Host ("Real cross-version dup families (codex-cli deploy): {0}" -f $deployDup.RealDups.Count)
}

# ---- 4. Release artifact families (target/release/deps) ----
$releaseArtifacts = Get-ReleaseArtifactFamilies -CargoRoot $cargoRoot -TopN 25
if ($null -eq $releaseArtifacts) {
    Write-Host 'target/release/deps absent — release-artifact section will be skipped.'
} else {
    Write-Host ("Release artifact crates inventoried (top {0}): {1}" -f 25, $releaseArtifacts.Count)
}

# ---- write Markdown ----
$now  = Get-Date -Format 'yyyy-MM-ddTHH:mm:ssK'
$lock = Join-Path $cargoRoot 'Cargo.lock'
$lockTotal = (Select-String -Path $lock -Pattern '^name = ').Count
$treeDupVersionRows = 0
foreach ($familyName in $realDups.Keys) {
    $treeDupVersionRows += $realDups[$familyName].Keys.Count
}
$deployDupCount = 0
if (-not $deployDup.Error) { $deployDupCount = $deployDup.RealDups.Count }
$releaseArtifactCount = 0
if ($null -ne $releaseArtifacts) { $releaseArtifactCount = $releaseArtifacts.Count }

$out = New-Object System.Collections.Generic.List[string]
$out.Add("# codex-rs dep snapshot")
$out.Add('')
$out.Add("Generated: $now (regenerate via ``scripts/dep-snapshot.ps1``).")
$out.Add('')
$out.Add("## Lockfile census")
$out.Add('')
$out.Add("- Total locked entries: **$lockTotal**")
$out.Add("- Cross-version duplicate families (workspace-wide): **$($realDups.Count)** (sum $treeDupVersionRows version entries)")
if ($deployDup.Error) {
    $out.Add("- Cross-version duplicate families (codex-cli deploy graph): **unknown** (cargo tree errored — see section below)")
} else {
    $out.Add("- Cross-version duplicate families (codex-cli deploy graph): **$deployDupCount**")
}
$out.Add("- Workspace-declared deps: **$($wsDeps.Count)**")
$out.Add("- Per-crate non-workspace pins (consolidation candidates): **$($pins.Count)**")
if ($releaseArtifactCount -gt 0) {
    $out.Add("- Top release artifact crates inventoried: **$releaseArtifactCount** (see section below)")
}
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

$out.Add("## Cross-version duplicate families (workspace-wide)")
$out.Add('')
$out.Add('Includes dev-deps and test-only crates — wider than what ships in `codex.exe`. See the deploy-graph section below for the strict shipping subset.')
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

$out.Add("## Deploy-graph duplicates (codex-cli, normal+build edges)")
$out.Add('')
$out.Add('Strict subgraph of what actually compiles into `codex.exe` — dev-deps and isolated test crates excluded. Collapsing a family here would shrink the deployed binary.')
$out.Add('')
if ($deployDup.Error) {
    $out.Add('_`cargo tree -p codex-cli --edges normal,build --duplicates --depth 1` failed:_')
    $out.Add('')
    $out.Add('```')
    foreach ($line in ($deployDup.Error -split "(`r`n|`n)")) {
        if ($line -notmatch '^(`r`n|`n)?$') { $out.Add($line.TrimEnd()) }
    }
    $out.Add('```')
} elseif ($deployDup.RealDups.Count -eq 0) {
    $out.Add('_None._')
} else {
    foreach ($name in ($deployDup.RealDups.Keys | Sort-Object)) {
        $vers = $deployDup.RealDups[$name]
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
$out.Add('')

$out.Add("## Top release artifact families (target/release/deps)")
$out.Add('')
if ($null -eq $releaseArtifacts) {
    $out.Add('_`target/release/deps` not present — run a release build before regenerating to populate this section._')
} elseif ($releaseArtifacts.Count -eq 0) {
    $out.Add('_No `.rlib` / `.rmeta` files found under `target/release/deps`._')
} else {
    $out.Add('Top 25 crate roots by total artifact bytes in `target/release/deps` (`.rlib` + `.rmeta`). Multiple artifacts per crate usually indicate cross-version splits being compiled.')
    $out.Add('')
    $out.Add('| Crate | Artifacts | Total MB |')
    $out.Add('|---|---:|---:|')
    foreach ($row in $releaseArtifacts) {
        $mb = '{0:N1}' -f ($row.Bytes / 1MB)
        $out.Add("| $($row.Crate) | $($row.Count) | $mb |")
    }
}

while ($out.Count -gt 0 -and [string]::IsNullOrWhiteSpace($out[$out.Count - 1])) {
    $out.RemoveAt($out.Count - 1)
}

[IO.File]::WriteAllLines($OutFile, $out)
Write-Host "Wrote $OutFile"
