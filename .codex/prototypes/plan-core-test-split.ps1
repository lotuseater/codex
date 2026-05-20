<#
.SYNOPSIS
Inventories codex-core integration test suite files for split-lane planning.

.DESCRIPTION
Reads codex-rs/core/tests/suite/*.rs and reports file size, approximate test
counts, super:: references, dependency hints, import roots, and a suggested
lane label. The script is read-only against source files.
#>

param(
    [string]$RepoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path,

    [ValidateSet('Name', 'Size', 'Tests', 'Lane')]
    [string]$SortBy = 'Name',

    [int]$Top = 0,

    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).Path
$suiteDir = Join-Path $RepoRoot 'codex-rs\core\tests\suite'
if (-not (Test-Path -LiteralPath $suiteDir -PathType Container)) {
    throw "Suite directory not found: $suiteDir"
}

function Get-RegexCount {
    param(
        [AllowEmptyString()]
        [string]$Text,

        [Parameter(Mandatory)]
        [string]$Pattern
    )

    return [regex]::Matches(
        $Text,
        $Pattern,
        [System.Text.RegularExpressions.RegexOptions]::Multiline
    ).Count
}

function Join-LabelList {
    param(
        [string[]]$Labels,
        [int]$Limit = 6
    )

    $values = @(
        $Labels |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            Sort-Object -Unique
    )

    if ($values.Count -eq 0) {
        return '-'
    }

    if ($values.Count -gt $Limit) {
        return (($values | Select-Object -First $Limit) -join ', ') + ', ...'
    }

    return $values -join ', '
}

function Get-ImportRoots {
    param(
        [AllowEmptyString()]
        [string]$Text
    )

    $roots = [System.Collections.Generic.List[string]]::new()
    foreach ($match in [regex]::Matches($Text, '(?m)^\s*use\s+([^;]+);')) {
        $import = $match.Groups[1].Value.Trim()
        $root = ($import -split '::|\s+as\s+|\{', 2)[0].Trim()
        if (-not [string]::IsNullOrWhiteSpace($root)) {
            [void]$roots.Add($root)
        }
    }

    return @($roots | Sort-Object -Unique)
}

$hintRules = @(
    [pscustomobject]@{ Label = 'apply-patch'; Pattern = '\bapply_patch\b|\bApplyPatch\b|CODEX_CORE_APPLY_PATCH_ARG1' }
    [pscustomobject]@{ Label = 'auth-login'; Pattern = '\bauth\b|\blogin\b|\bchatgpt\b|\btoken\b|\bAuthMode\b' }
    [pscustomobject]@{ Label = 'common-helper'; Pattern = '\bcore_test_support\b|\bcrate::common\b|\bcommon::|\bload_default_config_for_test\b|\btest_codex\b' }
    [pscustomobject]@{ Label = 'config'; Pattern = '\bConfig\b|\bconfig\b|\bConfigToml\b|\bConfigOverrides\b|\bconfig_types\b' }
    [pscustomobject]@{ Label = 'conversation-state'; Pattern = '\bconversation\b|\bhistory\b|\bsession\b|\bresume\b|\bcompact\b|\btruncate\b' }
    [pscustomobject]@{ Label = 'exec-sandbox'; Pattern = '\bsandbox\b|\bexec\b|\bseatbelt\b|\bCODEX_SANDBOX\b|\bwindows_sandbox\b|\bspawn\b' }
    [pscustomobject]@{ Label = 'git-vcs'; Pattern = '\bgit\b|\bGit\b|\bworktree\b|\bdiff\b|\bcommit\b' }
    [pscustomobject]@{ Label = 'mcp-tools'; Pattern = '\bmcp\b|\bMcp\b|\btool\b|\bTool\b|\bfunction_call\b|\bFunctionCall\b' }
    [pscustomobject]@{ Label = 'network-http'; Pattern = '\bhttp\b|\bserver\b|\bwiremock\b|\bmockito\b|\burl\b|\bSse\b|\bSSE\b' }
    [pscustomobject]@{ Label = 'protocol-events'; Pattern = '\bEvent\b|\bEventMsg\b|\bOp\b|\bResponseEvent\b|\bprotocol\b|\bturn\b' }
    [pscustomobject]@{ Label = 'responses-sse'; Pattern = '\bresponses::|\bmount_sse\b|\bsse\(|\bResponseMock\b|\bResponsesRequest\b|\bev_' }
    [pscustomobject]@{ Label = 'tokio-async'; Pattern = '#\s*\[\s*tokio::test|\btokio::|\basync\b|\bawait\b' }
)

function Get-DependencyHints {
    param(
        [AllowEmptyString()]
        [string]$Text
    )

    $labels = [System.Collections.Generic.List[string]]::new()
    foreach ($rule in $hintRules) {
        if ([regex]::IsMatch($Text, $rule.Pattern, [System.Text.RegularExpressions.RegexOptions]::Multiline)) {
            [void]$labels.Add($rule.Label)
        }
    }

    return @($labels | Sort-Object -Unique)
}

function Get-SuggestedLane {
    param(
        [Parameter(Mandatory)]
        [string]$FileBase,

        [string[]]$Hints,

        [int]$Tests,

        [int64]$SizeBytes
    )

    $name = $FileBase.ToLowerInvariant()
    $hintSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($hint in $Hints) {
        [void]$hintSet.Add($hint)
    }

    if ($Tests -eq 0) {
        return 'support/no-tests'
    }

    if ($SizeBytes -gt 35KB -or $Tests -gt 14) {
        return 'review-large'
    }

    switch -Regex ($name) {
        'mcp|tool|function_call' { return 'mcp-tools' }
        'sandbox|exec|seatbelt|windows' { return 'exec-sandbox' }
        'auth|login|chatgpt' { return 'auth-login' }
        'config|profile|approval' { return 'config' }
        'apply_patch|patch' { return 'apply-patch' }
        'git|diff|worktree' { return 'git-vcs' }
        'conversation|history|session|resume|compact|truncate' { return 'conversation-state' }
        'stream|sse|response|event|protocol' { return 'protocol-responses' }
    }

    if ($hintSet.Contains('mcp-tools')) { return 'mcp-tools' }
    if ($hintSet.Contains('exec-sandbox')) { return 'exec-sandbox' }
    if ($hintSet.Contains('auth-login')) { return 'auth-login' }
    if ($hintSet.Contains('config')) { return 'config' }
    if ($hintSet.Contains('apply-patch')) { return 'apply-patch' }
    if ($hintSet.Contains('git-vcs')) { return 'git-vcs' }
    if ($hintSet.Contains('conversation-state')) { return 'conversation-state' }
    if ($hintSet.Contains('responses-sse') -or $hintSet.Contains('protocol-events')) { return 'protocol-responses' }

    if ($hintSet.Count -gt 3 -or $SizeBytes -gt 24KB) {
        return 'review-mixed'
    }

    return 'core-flow'
}

$rows = foreach ($file in Get-ChildItem -LiteralPath $suiteDir -Filter '*.rs' -File) {
    $text = Get-Content -LiteralPath $file.FullName -Raw
    $plainTests = Get-RegexCount -Text $text -Pattern '^\s*#\s*\[\s*test(?:\s*\([^]]*\))?\s*\]'
    $tokioTests = Get-RegexCount -Text $text -Pattern '^\s*#\s*\[\s*tokio::test(?:\s*\([^]]*\))?\s*\]'
    $superTargets = @(
        [regex]::Matches($text, '\bsuper::[A-Za-z_][A-Za-z0-9_]*') |
            ForEach-Object { $_.Value } |
            Sort-Object -Unique
    )
    $imports = Get-ImportRoots -Text $text
    $hints = Get-DependencyHints -Text $text
    $lineCount = if ($text.Length -eq 0) { 0 } else { ($text -split "`r?`n").Count }
    $totalTests = $plainTests + $tokioTests

    [pscustomobject]@{
        File = $file.Name
        SizeBytes = $file.Length
        SizeKB = [math]::Round($file.Length / 1KB, 1)
        Lines = $lineCount
        Tests = $totalTests
        PlainTests = $plainTests
        TokioTests = $tokioTests
        SuperRefs = $superTargets.Count
        SuperTargets = $superTargets
        ImportRoots = $imports
        ImportRootsText = Join-LabelList -Labels $imports
        Hints = $hints
        HintsText = Join-LabelList -Labels $hints
        Lane = Get-SuggestedLane -FileBase $file.BaseName -Hints $hints -Tests $totalTests -SizeBytes $file.Length
    }
}

$sortSpec = switch ($SortBy) {
    'Size' { @(@{ Expression = 'SizeBytes'; Descending = $true }, @{ Expression = 'File'; Ascending = $true }) }
    'Tests' { @(@{ Expression = 'Tests'; Descending = $true }, @{ Expression = 'File'; Ascending = $true }) }
    'Lane' { @(@{ Expression = 'Lane'; Ascending = $true }, @{ Expression = 'File'; Ascending = $true }) }
    default { @(@{ Expression = 'File'; Ascending = $true }) }
}

$rows = @($rows | Sort-Object -Property $sortSpec)
if ($Top -gt 0) {
    $rows = @($rows | Select-Object -First $Top)
}

if ($Json) {
    $rows | ConvertTo-Json -Depth 5
    exit 0
}

Write-Output '# codex-core test split inventory'
Write-Output ''
Write-Output "Repo: $RepoRoot"
Write-Output "Suite: $suiteDir"
Write-Output "Files shown: $($rows.Count)"
Write-Output "Sort: $SortBy"
Write-Output ''
Write-Output '| File | KB | Lines | Tests | tokio | super:: | Lane | Hints | Imports |'
Write-Output '| --- | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |'
foreach ($row in $rows) {
    Write-Output "| $($row.File) | $($row.SizeKB) | $($row.Lines) | $($row.Tests) | $($row.TokioTests) | $($row.SuperRefs) | $($row.Lane) | $($row.HintsText) | $($row.ImportRootsText) |"
}

Write-Output ''
Write-Output '## Lane summary'
foreach ($group in $rows | Group-Object -Property Lane | Sort-Object -Property Name) {
    $testCount = ($group.Group | Measure-Object -Property Tests -Sum).Sum
    $sizeKb = [math]::Round((($group.Group | Measure-Object -Property SizeBytes -Sum).Sum) / 1KB, 1)
    Write-Output "- $($group.Name): $($group.Count) files, $testCount tests, $sizeKb KB"
}
