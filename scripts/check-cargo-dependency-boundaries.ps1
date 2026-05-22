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
    [switch]$SolidRefactor,
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

function Get-RepoRelativePath {
    param([string]$Path)

    $resolvedPath = (Resolve-Path -LiteralPath $Path).Path
    if ($resolvedPath.StartsWith($RepoRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $resolvedPath.Substring($RepoRoot.Length).TrimStart([char[]]@('\', '/'))
    }
    return $resolvedPath
}

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

function Get-CargoManifestPaths {
    param([string]$Root)

    $skippedDirectories = @(".git", ".hg", ".svn", "node_modules", "target")
    $pending = New-Object "System.Collections.Generic.Queue[string]"
    $pending.Enqueue((Resolve-Path -LiteralPath $Root).Path)

    while ($pending.Count -gt 0) {
        $directory = $pending.Dequeue()
        $manifestPath = Join-Path $directory "Cargo.toml"
        if (Test-Path -LiteralPath $manifestPath -PathType Leaf) {
            $manifestPath
        }

        foreach ($child in Get-ChildItem -LiteralPath $directory -Directory -Force) {
            if ($skippedDirectories -contains $child.Name) {
                continue
            }
            $pending.Enqueue($child.FullName)
        }
    }
}

function Get-DependencyEdgeKind {
    param([string]$Section)

    if ($Section -eq "dependencies" -or $Section -like "target.*.dependencies") {
        return "normal"
    }
    if ($Section -eq "build-dependencies" -or $Section -like "target.*.build-dependencies") {
        return "build"
    }
    if ($Section -eq "dev-dependencies" -or $Section -like "target.*.dev-dependencies") {
        return "dev"
    }
    return $null
}

function Read-CargoManifest {
    param([string]$Path)

    $lines = @(Get-Content -LiteralPath $Path)
    $section = $null
    $packageName = $null
    $dependencies = @()
    $manifestSourcePaths = @()

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $rawLine = $lines[$i]
        $line = ($rawLine -replace "#.*$", "").Trim()
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }

        if ($line -match "^\[\[(?<section>[^\]]+)\]\]$") {
            $section = $Matches.section
            continue
        }

        if ($line -match "^\[(?<section>[^\]]+)\]$") {
            $section = $Matches.section
            continue
        }

        if ($section -eq "package" -and $line -match '^name\s*=\s*"(?<name>[^"]+)"') {
            $packageName = $Matches.name
            continue
        }

        if ($line -match '^path\s*=\s*"(?<source_path>[^"]+\.rs)"') {
            $manifestSourcePaths += $Matches.source_path
            continue
        }

        $edgeKind = Get-DependencyEdgeKind -Section $section
        if ($null -eq $edgeKind) {
            continue
        }

        if ($line -notmatch '^(?<key>"?[A-Za-z0-9_.-]+"?)\s*=') {
            continue
        }

        $dependencyKey = $Matches.key.Trim('"')
        $dependencyPackage = $dependencyKey
        if ($line -match 'package\s*=\s*"(?<package>[^"]+)"') {
            $dependencyPackage = $Matches.package
        }

        $dependencies += [pscustomobject]@{
            name = $dependencyPackage
            key = $dependencyKey
            edge_kind = $edgeKind
            section = $section
            line = $i + 1
            line_text = $rawLine.Trim()
        }
    }

    if ([string]::IsNullOrWhiteSpace($packageName)) {
        return $null
    }

    $packageDirectory = Split-Path -Parent (Resolve-Path -LiteralPath $Path).Path
    $sourceRoots = New-Object "System.Collections.Generic.List[string]"
    $defaultSourceRoot = Join-Path $packageDirectory "src"
    if (Test-Path -LiteralPath $defaultSourceRoot -PathType Container) {
        $sourceRoots.Add($defaultSourceRoot)
    }
    if (Test-Path -LiteralPath (Join-Path $packageDirectory "lib.rs") -PathType Leaf) {
        $sourceRoots.Add($packageDirectory)
    }
    foreach ($sourcePath in $manifestSourcePaths) {
        $sourceFile = Join-Path $packageDirectory $sourcePath
        if (-not (Test-Path -LiteralPath $sourceFile -PathType Leaf)) {
            continue
        }

        $sourceRoot = Split-Path -Parent (Resolve-Path -LiteralPath $sourceFile).Path
        if (-not $sourceRoots.Contains($sourceRoot)) {
            $sourceRoots.Add($sourceRoot)
        }
    }
    [pscustomobject]@{
        package = $packageName
        manifest_path = (Resolve-Path -LiteralPath $Path).Path
        package_directory = $packageDirectory
        source_root = Join-Path $packageDirectory "src"
        source_roots = @($sourceRoots)
        dependencies = @($dependencies)
    }
}

function Get-ManifestByPackage {
    param([string]$Root)

    $manifestByPackage = @{}
    foreach ($path in Get-CargoManifestPaths -Root $Root) {
        $manifest = Read-CargoManifest -Path $path
        if ($null -eq $manifest) {
            continue
        }
        if ($manifestByPackage.ContainsKey($manifest.package)) {
            $existingManifest = $manifestByPackage[$manifest.package]
            throw "Duplicate package '$($manifest.package)' in '$($manifest.manifest_path)' and '$($existingManifest.manifest_path)'."
        }
        $manifestByPackage[$manifest.package] = $manifest
    }
    return $manifestByPackage
}

function Get-ManifestOrThrow {
    param(
        [hashtable]$ManifestByPackage,
        [string]$Name
    )

    if (-not $ManifestByPackage.ContainsKey($Name)) {
        throw "Package '$Name' was not found in Cargo.toml manifests."
    }
    return $ManifestByPackage[$Name]
}

function Add-DependencyViolations {
    param(
        [System.Collections.Generic.List[object]]$Violations,
        [object]$Manifest,
        [string[]]$ForbiddenPackagePatterns,
        [string[]]$EdgeKinds,
        [string]$Policy
    )

    foreach ($dependency in $Manifest.dependencies) {
        if ($EdgeKinds -notcontains $dependency.edge_kind) {
            continue
        }
        if (-not (Package-Matches -Name $dependency.name -Patterns $ForbiddenPackagePatterns)) {
            continue
        }

        $Violations.Add([pscustomobject]@{
            type = "dependency"
            policy = $Policy
            package = $Manifest.package
            dependency = $dependency.name
            edge_kind = $dependency.edge_kind
            manifest = Get-RepoRelativePath -Path $Manifest.manifest_path
            line = $dependency.line
            path = "$($Manifest.package) -> $($dependency.name)"
        })
    }
}

function Add-SourcePatternViolations {
    param(
        [System.Collections.Generic.List[object]]$Violations,
        [object]$Manifest,
        [string[]]$Patterns,
        [string]$Policy
    )

    $sourceRoots = @($Manifest.source_roots)
    if ($sourceRoots.Count -eq 0) {
        $sourceRoots = @($Manifest.source_root)
    }

    $sourceFiles = @()
    foreach ($sourceRoot in $sourceRoots) {
        if (-not (Test-Path -LiteralPath $sourceRoot -PathType Container)) {
            continue
        }
        $sourceFiles += @(Get-ChildItem -LiteralPath $sourceRoot -Recurse -File -Filter "*.rs")
    }
    if ($sourceFiles.Count -eq 0) {
        return
    }
    $allowedDomainApiSourceReferences = @{
        "codex-app-server-protocol|codex_app_server_protocol::" = @(
            "codex-rs\app-server-protocol\src\bin\export.rs",
            "codex-rs\app-server-protocol\src\bin\write_schema_fixtures.rs"
        )
    }
    foreach ($match in ($sourceFiles | Select-String -SimpleMatch -Pattern $Patterns)) {
        foreach ($pattern in $Patterns) {
            if (-not $match.Line.Contains($pattern)) {
                continue
            }

            $relativePath = Get-RepoRelativePath -Path $match.Path
            $sourceReferenceKey = "$($Manifest.package)|$pattern"
            if (
                $Policy -eq "domain-api-no-concrete-source" -and
                $allowedDomainApiSourceReferences.ContainsKey($sourceReferenceKey) -and
                $allowedDomainApiSourceReferences[$sourceReferenceKey] -contains $relativePath
            ) {
                continue
            }

            $Violations.Add([pscustomobject]@{
                type = "source"
                policy = $Policy
                package = $Manifest.package
                pattern = $pattern
                manifest = Get-RepoRelativePath -Path $Manifest.manifest_path
                path = "$($relativePath):$($match.LineNumber):$($match.Line.Trim())"
            })
        }
    }
}

function Test-IsDomainApiPackage {
    param([string]$Name)

    $domainApiPackageNames = @(
        "codex-api",
        "codex-app-server-protocol",
        "codex-config",
        "codex-core-domain-types",
        "codex-execpolicy",
        "codex-execpolicy-legacy",
        "codex-protocol"
    )
    $domainApiPackagePatterns = @(
        "codex-*-api",
        "codex-*-types",
        "codex-*-ports",
        "codex-*-policy"
    )

    return ($domainApiPackageNames -contains $Name) -or (Package-Matches -Name $Name -Patterns $domainApiPackagePatterns)
}

function Add-TestSplitViolations {
    param(
        [System.Collections.Generic.List[object]]$Violations,
        [hashtable]$ManifestByPackage
    )

    $legacyCoreShimPackages = @("core_test_support")
    $legacyCoreShimSourcePatterns = @("core_test_support::")
    $splitTestSupportPackages = @(
        "app_test_support",
        "codex-test-support-context-fixtures",
        "codex-test-support-lightweight",
        "codex-test-support-responses"
    )

    foreach ($packageName in $splitTestSupportPackages) {
        if (-not $ManifestByPackage.ContainsKey($packageName)) {
            continue
        }

        $manifest = $ManifestByPackage[$packageName]
        Add-DependencyViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -ForbiddenPackagePatterns $legacyCoreShimPackages `
            -EdgeKinds @("normal", "build", "dev") `
            -Policy "split-test-support-no-legacy-core-shim-dependencies"
        Add-SourcePatternViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -Patterns $legacyCoreShimSourcePatterns `
            -Policy "split-test-support-no-legacy-core-shim-source"
    }

    if ($ManifestByPackage.ContainsKey("codex-core-test-runtime")) {
        $manifest = $ManifestByPackage["codex-core-test-runtime"]
        Add-DependencyViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -ForbiddenPackagePatterns $legacyCoreShimPackages `
            -EdgeKinds @("normal", "build", "dev") `
            -Policy "core-test-runtime-no-legacy-core-shim-dependencies"
        Add-SourcePatternViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -Patterns $legacyCoreShimSourcePatterns `
            -Policy "core-test-runtime-no-legacy-core-shim-source"
    }

    foreach ($manifest in @($ManifestByPackage.Values | Sort-Object package)) {
        $relativeManifestPath = Get-RepoRelativePath -Path $manifest.manifest_path
        if ($relativeManifestPath -notlike "codex-rs\core-test-suites\*\Cargo.toml") {
            continue
        }

        Add-DependencyViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -ForbiddenPackagePatterns $legacyCoreShimPackages `
            -EdgeKinds @("normal", "build", "dev") `
            -Policy "core-test-suite-no-legacy-core-shim-dependencies"
        Add-SourcePatternViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -Patterns $legacyCoreShimSourcePatterns `
            -Policy "core-test-suite-no-legacy-core-shim-source"
    }
}

function Add-SolidRefactorViolations {
    param(
        [System.Collections.Generic.List[object]]$Violations,
        [hashtable]$ManifestByPackage
    )

    $domainApiForbiddenPackages = @(
        "codex-core",
        "codex-core-api",
        "codex-app-server-protocol",
        "codex-app-server",
        "codex-app-server-client",
        "codex-app-server-daemon",
        "codex-app-server-test-client",
        "codex-app-server-transport",
        "codex-cli",
        "codex-exec",
        "codex-mcp-server",
        "codex-tui",
        "codex-agent-graph-store",
        "codex-blackboard",
        "codex-context-ops-impl",
        "codex-desktop-automation",
        "codex-exec-server",
        "codex-model-provider",
        "codex-operation-cache",
        "codex-state",
        "codex-thread-store",
        "codex-tools",
        "codex-workflow-batch"
    )
    $domainApiForbiddenSourcePatterns = @(
        "codex_core::",
        "codex_core_api::",
        "codex_app_server_protocol::",
        "codex_app_server::",
        "codex_mcp_server::",
        "codex_thread_store::",
        "codex_tui::"
    )
    $outerCoreForbiddenPackages = @(
        "codex-app-server-protocol",
        "codex-app-server",
        "codex-app-server-client",
        "codex-app-server-daemon",
        "codex-app-server-test-client",
        "codex-app-server-transport",
        "codex-cli",
        "codex-mcp-server",
        "codex-tui"
    )

    Add-TestSplitViolations -Violations $Violations -ManifestByPackage $ManifestByPackage

    $coreApi = Get-ManifestOrThrow -ManifestByPackage $ManifestByPackage -Name "codex-core-api"
    Add-DependencyViolations `
        -Violations $Violations `
        -Manifest $coreApi `
        -ForbiddenPackagePatterns $domainApiForbiddenPackages `
        -EdgeKinds @("normal", "build", "dev") `
        -Policy "core-api-no-concrete-app-or-core-deps"
    Add-SourcePatternViolations `
        -Violations $Violations `
        -Manifest $coreApi `
        -Patterns @("codex_app_server_protocol::", "codex_core::") `
        -Policy "core-api-no-app-server-or-core-source"

    foreach ($manifest in @($ManifestByPackage.Values | Sort-Object package)) {
        if ($manifest.package -eq "codex-core-api") {
            continue
        }
        if (-not (Test-IsDomainApiPackage -Name $manifest.package)) {
            continue
        }
        Add-DependencyViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -ForbiddenPackagePatterns $domainApiForbiddenPackages `
            -EdgeKinds @("normal", "build", "dev") `
            -Policy "domain-api-no-concrete-deps"
        Add-SourcePatternViolations `
            -Violations $Violations `
            -Manifest $manifest `
            -Patterns $domainApiForbiddenSourcePatterns `
            -Policy "domain-api-no-concrete-source"
    }

    $core = Get-ManifestOrThrow -ManifestByPackage $ManifestByPackage -Name "codex-core"
    Add-DependencyViolations `
        -Violations $Violations `
        -Manifest $core `
        -ForbiddenPackagePatterns $outerCoreForbiddenPackages `
        -EdgeKinds @("normal", "build", "dev") `
        -Policy "core-no-outer-app-or-ui-deps"

    $coreDependencyClassifications = [ordered]@{
        "domain-api-policy-and-wire" = @(
            "codex-agent-policy",
            "codex-api",
            "codex-app-catalog-types",
            "codex-auth-api",
            "codex-build-policy",
            "codex-compaction-policy",
            "codex-config",
            "codex-execpolicy",
            "codex-extension-api",
            "codex-features",
            "codex-mcp-elicitation-api",
            "codex-model-provider-info",
            "codex-prompt-context",
        "codex-protocol",
        "codex-session-api",
        "codex-shell-command",
            "codex-thread-manager-api",
            "codex-thread-store-api",
            "codex-tool-execution-api",
            "codex-tool-registry-api",
            "codex-tool-schema"
        )
        "runtime-orchestration-services" = @(
            "codex-analytics",
            "codex-blackboard",
            "codex-code-mode",
            "codex-connectors",
            "codex-core-plugins",
            "codex-core-skills",
            "codex-feedback",
            "codex-first-moves",
            "codex-hooks",
            "codex-login",
            "codex-mcp",
            "codex-memories-context",
            "codex-memories-read",
            "codex-model-provider",
            "codex-models-manager",
            "codex-plugin",
            "codex-prompt-reducer",
            "codex-repo-context-scout",
            "codex-response-debug-context",
            "codex-rollout",
            "codex-rollout-trace",
            "codex-self-review",
            "codex-task-memory",
            "codex-terminal-detection",
            "codex-turn-diff"
        )
        "state-store-and-context" = @(
            "codex-agent-graph-store",
            "codex-context-ops-impl",
            "codex-context-pack",
            "codex-context-reduction",
            "codex-operation-cache",
            "codex-state",
            "codex-thread-store"
        )
        "execution-tools-and-system" = @(
            "codex-apply-patch",
            "codex-async-utils",
            "codex-cognos-ops",
            "codex-desktop-automation",
            "codex-exec-server",
            "codex-git-utils",
            "codex-network-proxy",
            "codex-otel",
            "codex-rmcp-client",
            "codex-sandboxing",
            "codex-shell-escalation",
            "codex-tools",
            "codex-windows-sandbox",
            "codex-workflow-batch"
        )
        "utilities" = @(
            "codex-utils-absolute-path",
            "codex-utils-cache",
            "codex-utils-cargo-bin",
            "codex-utils-home-dir",
            "codex-utils-image",
            "codex-utils-output-truncation",
            "codex-utils-path",
            "codex-utils-plugins",
            "codex-utils-pty",
            "codex-utils-stream-parser",
            "codex-utils-string",
            "codex-utils-template"
        )
        "dev-test-support" = @(
            "codex-core-test-runtime",
            "codex-test-binary-support",
            "codex-test-support-context-fixtures",
            "codex-test-support-lightweight",
            "codex-test-support-responses"
        )
    }

    $classifiedCoreDependencies = @{}
    foreach ($classification in $coreDependencyClassifications.Keys) {
        foreach ($dependencyName in $coreDependencyClassifications[$classification]) {
            if ($classifiedCoreDependencies.ContainsKey($dependencyName)) {
                $Violations.Add([pscustomobject]@{
                    type = "duplicate_core_dependency_classification"
                    policy = "core-dependencies-explicitly-classified"
                    dependency = $dependencyName
                    first_classification = $classifiedCoreDependencies[$dependencyName]
                    second_classification = $classification
                })
                continue
            }
            $classifiedCoreDependencies[$dependencyName] = $classification
        }
    }

    foreach ($dependency in @($core.dependencies | Sort-Object name, edge_kind, line)) {
        if (-not $ManifestByPackage.ContainsKey($dependency.name)) {
            continue
        }
        if ($classifiedCoreDependencies.ContainsKey($dependency.name)) {
            continue
        }

        $Violations.Add([pscustomobject]@{
            type = "unclassified_core_dependency"
            policy = "core-dependencies-explicitly-classified"
            package = $core.package
            dependency = $dependency.name
            edge_kind = $dependency.edge_kind
            manifest = Get-RepoRelativePath -Path $core.manifest_path
            line = $dependency.line
            path = "$($core.package) -> $($dependency.name)"
        })
    }
}

$manifestByPackage = Get-ManifestByPackage -Root $CodexRs
$violations = New-Object "System.Collections.Generic.List[object]"
$packageChecks = @()

if (-not [string]::IsNullOrWhiteSpace($Package)) {
    $rootPackage = Get-ManifestOrThrow -ManifestByPackage $manifestByPackage -Name $Package
    $packageChecks += $Package
    Add-DependencyViolations `
        -Violations $violations `
        -Manifest $rootPackage `
        -ForbiddenPackagePatterns $ForbiddenPackages `
        -EdgeKinds @("normal", "build") `
        -Policy "package-forbidden-dependencies"
    Add-SourcePatternViolations `
        -Violations $violations `
        -Manifest $rootPackage `
        -Patterns $ForbiddenSourcePatterns `
        -Policy "package-forbidden-source-patterns"
}

if ($SolidRefactor) {
    Add-SolidRefactorViolations -Violations $violations -ManifestByPackage $manifestByPackage
}

$summary = [pscustomobject]@{
    package_checks = $packageChecks
    solid_refactor = $SolidRefactor.IsPresent
    forbidden_packages = $ForbiddenPackages
    forbidden_source_patterns = $ForbiddenSourcePatterns
    violation_count = $violations.Count
    violations = @($violations.ToArray())
}

if ($Json) {
    $summary | ConvertTo-Json -Depth 12
} else {
    if ($violations.Count -eq 0) {
        if ($SolidRefactor) {
            Write-Host "Dependency boundary check passed for $($packageChecks -join ', ') and SOLID refactor policies."
        } else {
            Write-Host "Dependency boundary check passed for $($packageChecks -join ', ')."
        }
    } else {
        Write-Host "Dependency boundary check failed with $($violations.Count) violation(s):"
        foreach ($violation in $violations) {
            if ($violation.type -eq "dependency") {
                Write-Host "  [$($violation.policy)] dependency: $($violation.path) [$($violation.edge_kind)] at $($violation.manifest):$($violation.line)"
            } elseif ($violation.type -eq "source") {
                Write-Host "  [$($violation.policy)] source: $($violation.path)"
            } elseif ($violation.type -eq "unclassified_core_dependency") {
                Write-Host "  [$($violation.policy)] unclassified core dependency: $($violation.path) [$($violation.edge_kind)] at $($violation.manifest):$($violation.line)"
            } elseif ($violation.type -eq "duplicate_core_dependency_classification") {
                Write-Host "  [$($violation.policy)] duplicate classification: $($violation.dependency) is in $($violation.first_classification) and $($violation.second_classification)"
            } else {
                Write-Host "  [$($violation.policy)] $($violation.type): $($violation | ConvertTo-Json -Compress)"
            }
        }
    }
}

if ($violations.Count -gt 0) {
    exit 1
}
