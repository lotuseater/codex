[CmdletBinding()]
param(
    [string]$RepoRoot,
    [string]$Package = "codex-config",
    [string[]]$ForbiddenPackages = @(
        "codex-protocol",
        "codex-app-server-protocol",
        "codex-api",
        "codex-otel",
        "codex-network-proxy",
        "gix*",
        "hyper*",
        "prost*",
        "rama*",
        "starlark*",
        "tonic*"
    ),
    [string[]]$ForbiddenSourcePatterns = @(
        "codex_protocol::",
        "codex_app_server_protocol::"
    ),
    [switch]$Json
)

$ErrorActionPreference = "Stop"
if (Test-Path variable:PSNativeCommandUseErrorActionPreference) {
    $PSNativeCommandUseErrorActionPreference = $false
}

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$CodexRs = Join-Path $RepoRoot "codex-rs"

function Package-Matches {
    param(
        [string]$Name,
        [string[]]$Patterns
    )

    foreach ($pattern in $Patterns) {
        if ($Name -like $pattern) {
            return $true
        }
    }
    return $false
}

$metadataText = & cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $CodexRs "Cargo.toml")
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed with exit code $LASTEXITCODE."
}

$metadata = $metadataText | ConvertFrom-Json
$rootPackage = @($metadata.packages | Where-Object { $_.name -eq $Package })[0]
if ($null -eq $rootPackage) {
    throw "Package '$Package' was not found in cargo metadata."
}

$violations = @()

$treeOutput = & cargo tree `
    --manifest-path (Join-Path $CodexRs "Cargo.toml") `
    --package $Package `
    --edges normal,build `
    --prefix depth `
    --charset ascii `
    --format "{p}"
if ($LASTEXITCODE -ne 0) {
    throw "cargo tree failed with exit code $LASTEXITCODE."
}

$pathStack = New-Object System.Collections.Generic.List[string]
foreach ($line in $treeOutput) {
    if ($line -notmatch "^(?<depth>\d+)(?<package>.+)$") {
        continue
    }

    $depth = [int]$Matches.depth
    $packageDescription = $Matches.package.Trim()
    $packageName = ($packageDescription -replace " \(\*\)$", "").Split(" ")[0]

    while ($pathStack.Count -gt $depth) {
        $pathStack.RemoveAt($pathStack.Count - 1)
    }
    if ($pathStack.Count -eq $depth) {
        $pathStack.Add($packageName)
    } else {
        $pathStack[$depth] = $packageName
    }

    if ($depth -gt 0 -and (Package-Matches -Name $packageName -Patterns $ForbiddenPackages)) {
        $depPath = for ($i = 0; $i -le $depth; $i++) {
            $pathStack[$i]
        }
        $violations += [pscustomobject]@{
            type = "dependency"
            package = $packageName
            edge_kinds = "normal/build"
            path = ($depPath -join " -> ")
        }
    }
}

$sourceRoot = Join-Path (Split-Path -Parent $rootPackage.manifest_path) "src"
if (Test-Path -LiteralPath $sourceRoot) {
    foreach ($pattern in $ForbiddenSourcePatterns) {
        $rgOutput = & rg --line-number --fixed-strings --color never --glob "*.rs" -- $pattern $sourceRoot 2>$null
        if ($LASTEXITCODE -eq 0) {
            foreach ($line in $rgOutput) {
                $violations += [pscustomobject]@{
                    type = "source"
                    pattern = $pattern
                    path = $line
                }
            }
        } elseif ($LASTEXITCODE -ne 1) {
            throw "rg failed while checking '$pattern' under '$sourceRoot'."
        }
    }
}

$summary = [pscustomobject]@{
    package = $Package
    forbidden_packages = $ForbiddenPackages
    forbidden_source_patterns = $ForbiddenSourcePatterns
    violation_count = $violations.Count
    violations = $violations
}

if ($Json) {
    $summary | ConvertTo-Json -Depth 12
} else {
    if ($violations.Count -eq 0) {
        Write-Host "Dependency boundary check passed for $Package."
    } else {
        Write-Host "Dependency boundary check failed for $Package with $($violations.Count) violation(s):"
        foreach ($violation in $violations) {
            if ($violation.type -eq "dependency") {
                Write-Host "  dependency: $($violation.path) [$($violation.edge_kinds)]"
            } else {
                Write-Host "  source: $($violation.path)"
            }
        }
    }
}

if ($violations.Count -gt 0) {
    exit 1
}
