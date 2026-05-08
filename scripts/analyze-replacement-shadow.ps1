param(
    [string]$LogDir = (Join-Path $env:USERPROFILE '.codex\log\replacement-shadow'),

    [datetimeoffset]$Since,

    [string]$OutMarkdown,

    [switch]$Json,

    [int]$MinRecords = 5,

    [int]$MinRepos = 2,

    [int]$MinSavedTokens = 32,

    [double]$MinSavedPercent = 30.0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-Prop {
    param(
        [object]$Object,
        [string[]]$Names,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }

    foreach ($name in $Names) {
        $property = $Object.PSObject.Properties[$name]
        if ($null -ne $property) {
            return $property.Value
        }
    }
    return $Default
}

function ConvertTo-NullableInt {
    param([object]$Value)

    if ($null -eq $Value -or $Value -eq '') {
        return $null
    }
    try {
        return [int64]$Value
    }
    catch {
        return $null
    }
}

function ConvertTo-NullableDouble {
    param([object]$Value)

    if ($null -eq $Value -or $Value -eq '') {
        return $null
    }
    try {
        return [double]$Value
    }
    catch {
        return $null
    }
}

function ConvertTo-NullableBool {
    param([object]$Value)

    if ($null -eq $Value -or $Value -eq '') {
        return $null
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    $text = ([string]$Value).Trim()
    if ($text -match '^(true|1|yes)$') {
        return $true
    }
    if ($text -match '^(false|0|no)$') {
        return $false
    }
    return $null
}

function ConvertTo-RecordTimestamp {
    param([object]$Value)

    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [datetimeoffset]) {
        return $Value
    }
    if ($Value -is [datetime]) {
        return [datetimeoffset]::new($Value)
    }
    try {
        return [datetimeoffset]::Parse([string]$Value)
    }
    catch {
        return $null
    }
}

function Get-RepoName {
    param([string]$Cwd)

    if ([string]::IsNullOrWhiteSpace($Cwd)) {
        return '(unknown)'
    }
    try {
        return Split-Path -Leaf ([System.IO.Path]::GetFullPath($Cwd))
    }
    catch {
        return Split-Path -Leaf $Cwd
    }
}

function Get-CommandClass {
    param([string]$Command)

    if ([string]::IsNullOrWhiteSpace($Command)) {
        return '(unknown)'
    }

    $normalized = ($Command -replace '\s+', ' ').Trim().ToLowerInvariant()
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+diff\b' -and $normalized -match '(^|\s)--shortstat(\s|$)') {
        return 'git_diff_shortstat'
    }
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+diff\b' -and $normalized -match '(^|\s)--stat(\s|$)') {
        return 'git_diff_stat'
    }
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+diff\b' -and $normalized -match '(^|\s)--name-only(\s|$)') {
        return 'git_diff_name_only'
    }
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+diff\b' -and $normalized -match '(^|\s)--name-status(\s|$)') {
        return 'git_diff_name_status'
    }
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+diff\b' -and $normalized -match '(^|\s)--numstat(\s|$)') {
        return 'git_diff_numstat'
    }
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+diff\b') {
        return 'git_diff_full'
    }
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+status\b') {
        return 'git_status'
    }
    if ($normalized -match '^git(\s+-c\s+\S+)?\s+(log|show)\b') {
        return 'git_history'
    }
    if ($normalized -match '^rg\b' -and $normalized -match '(^|\s)--json(\s|$)') {
        return 'rg_json'
    }
    if ($normalized -match '^rg\b' -and $normalized -match '(^|\s)(-c|--count|--count-matches)(\s|$)') {
        return 'rg_count'
    }
    if ($normalized -match '^rg\b' -and $normalized -match '(^|\s)(--files|-l|--files-with-matches)(\s|$)') {
        return 'rg_file_set'
    }
    if ($normalized -match '^rg\b') {
        return 'rg_search'
    }
    if ($normalized -match '^(cat|type|gc|get-content|get-content\.exe)\b') {
        return 'file_read'
    }
    if ($normalized -match '^(head|tail)\b') {
        return 'file_excerpt'
    }
    if ($normalized -match '^(get-childitem|gci|dir|ls)\b') {
        return 'directory_listing'
    }
    if ($normalized -match '^(get-process|gps|ps)\b') {
        return 'process_table'
    }
    if ($normalized -match '^(cargo|just|npm|pnpm|yarn|pytest|python)\b') {
        return 'check_command'
    }
    if ($normalized -match 'build-local-codex\.ps1') {
        return 'check_command'
    }

    return ($normalized -split '\s+')[0]
}

function Get-Recommendation {
    param(
        [string]$Operation,
        [string]$Strategy,
        [int]$Count,
        [int]$RepoCount,
        [int]$ErrorCount,
        [int]$FallbackCount,
        [int]$GatePassCount
    )

    $removedRejected = @(
        'git_worktree_summary',
        'git_status_compact'
    )
    if ($removedRejected -contains $Operation) {
        return 'removed_rejected'
    }

    $shadowOnly = @(
        'file_outline',
        'search_text',
        'diff_hunk_summary',
        'run_check_digest',
        'rg_json_digest',
        'git_filtered_diff_digest',
        'git_history_digest',
        'directory_listing_compact',
        'process_table_compact'
    )
    if ($Strategy -eq 'legacy_unknown') {
        return 'legacy_review_only'
    }
    if ($shadowOnly -contains $Operation) {
        return 'keep_shadow_only'
    }
    if ($Count -ge $MinRecords -and $RepoCount -ge $MinRepos -and $ErrorCount -eq 0 -and $FallbackCount -eq 0 -and $GatePassCount -eq $Count) {
        return 'promotion_candidate'
    }
    if ($GatePassCount -ge $MinRecords -and $RepoCount -ge $MinRepos) {
        return 'needs_artifact_review'
    }
    return 'keep_collecting'
}

function Escape-MarkdownCell {
    param([object]$Value)

    if ($null -eq $Value) {
        return ''
    }
    return ([string]$Value).Replace('|', '\|').Replace("`r", ' ').Replace("`n", ' ')
}

if (-not (Test-Path -LiteralPath $LogDir -PathType Container)) {
    throw "Replacement shadow log directory does not exist: $LogDir"
}

$files = @(Get-ChildItem -LiteralPath $LogDir -Filter 'replacement-bench-*.jsonl' -File | Sort-Object Name)
$records = New-Object 'System.Collections.Generic.List[object]'
$parseErrors = New-Object 'System.Collections.Generic.List[object]'

foreach ($file in $files) {
    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $file.FullName) {
        $lineNumber++
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        try {
            $raw = $line | ConvertFrom-Json
        }
        catch {
            [void]$parseErrors.Add([pscustomobject]@{
                file = $file.FullName
                line = $lineNumber
                error = $_.Exception.Message
            })
            continue
        }

        $timestamp = ConvertTo-RecordTimestamp (Get-Prop $raw @('timestamp'))
        if ($PSBoundParameters.ContainsKey('Since') -and $null -ne $timestamp -and $timestamp -lt $Since) {
            continue
        }

        $operation = Get-Prop $raw @('replacement_operation', 'operation') '(unknown)'
        $strategy = Get-Prop $raw @('shadow_strategy') 'legacy_unknown'
        $cwd = Get-Prop $raw @('cwd') ''
        $command = Get-Prop $raw @('baseline_command') ''
        $savedTokens = ConvertTo-NullableInt (Get-Prop $raw @('saved_model_visible_tokens'))
        if ($null -eq $savedTokens) {
            $tokenSavings = Get-Prop $raw @('token_savings')
            $savedTokens = ConvertTo-NullableInt (Get-Prop $tokenSavings @('approx_tokens'))
        }
        $savedPercent = ConvertTo-NullableDouble (Get-Prop $raw @('saved_model_visible_percent'))
        if ($null -eq $savedPercent) {
            $tokenSavings = Get-Prop $raw @('token_savings')
            $savedPercent = ConvertTo-NullableDouble (Get-Prop $tokenSavings @('percent'))
        }
        $fallback = ConvertTo-NullableBool (Get-Prop $raw @('replacement_fallback_required'))
        $verdict = Get-Prop $raw @('verdict') ''
        if ($null -eq $fallback) {
            $fallback = ($verdict -eq 'fallback_required')
        }
        $errorText = Get-Prop $raw @('replacement_error') $null
        if ($null -eq $errorText) {
            $candidate = Get-Prop $raw @('candidate')
            $errorText = Get-Prop $candidate @('error') $null
        }
        $gateFromRecord = ConvertTo-NullableBool (Get-Prop $raw @('replacement_gate_passed'))
        $gatePassed = if ($null -ne $gateFromRecord) {
            $gateFromRecord
        }
        else {
            $null -eq $errorText -and
                -not [bool]$fallback -and
                $null -ne $savedTokens -and
                $null -ne $savedPercent -and
                $savedTokens -ge $MinSavedTokens -and
                $savedPercent -ge $MinSavedPercent
        }

        [void]$records.Add([pscustomobject]@{
            timestamp = $timestamp
            source_file = $file.FullName
            operation = [string]$operation
            strategy = [string]$strategy
            repo = Get-RepoName $cwd
            cwd = [string]$cwd
            command = [string]$command
            command_class = Get-CommandClass $command
            verdict = [string]$verdict
            fallback = [bool]$fallback
            error = $errorText
            saved_tokens = $savedTokens
            saved_percent = $savedPercent
            gate_passed = [bool]$gatePassed
            baseline_artifact_path = Get-Prop $raw @('baseline_artifact_path') ''
            replacement_artifact_path = Get-Prop $raw @('replacement_artifact_path') ''
        })
    }
}

$summaries = New-Object 'System.Collections.Generic.List[object]'
foreach ($group in ($records | Group-Object operation, strategy, command_class | Sort-Object Name)) {
    $items = @($group.Group)
    $first = $items[0]
    $repos = @($items | Select-Object -ExpandProperty repo -Unique)
    $errors = @($items | Where-Object { $null -ne $_.error -and $_.error -ne '' })
    $fallbacks = @($items | Where-Object { $_.fallback })
    $gatePassed = @($items | Where-Object { $_.gate_passed })
    $saved = @($items | Where-Object { $null -ne $_.saved_tokens } | Select-Object -ExpandProperty saved_tokens)
    $savedPercent = @($items | Where-Object { $null -ne $_.saved_percent } | Select-Object -ExpandProperty saved_percent)
    $avgSaved = if ($saved.Count -gt 0) { [Math]::Round((($saved | Measure-Object -Average).Average), 1) } else { $null }
    $avgPercent = if ($savedPercent.Count -gt 0) { [Math]::Round((($savedPercent | Measure-Object -Average).Average), 1) } else { $null }
    $sample = $items | Select-Object -First 1

    [void]$summaries.Add([pscustomobject]@{
        operation = $first.operation
        strategy = $first.strategy
        command_class = $first.command_class
        records = $items.Count
        repos = $repos.Count
        errors = $errors.Count
        fallbacks = $fallbacks.Count
        gate_passed = $gatePassed.Count
        avg_saved_tokens = $avgSaved
        avg_saved_percent = $avgPercent
        recommendation = Get-Recommendation `
            -Operation $first.operation `
            -Strategy $first.strategy `
            -Count $items.Count `
            -RepoCount $repos.Count `
            -ErrorCount $errors.Count `
            -FallbackCount $fallbacks.Count `
            -GatePassCount $gatePassed.Count
        sample_baseline_artifact = $sample.baseline_artifact_path
        sample_replacement_artifact = $sample.replacement_artifact_path
    })
}

$filePaths = New-Object 'System.Collections.Generic.List[string]'
foreach ($file in $files) {
    [void]$filePaths.Add($file.FullName)
}

$report = [pscustomobject]@{
    generated_at = [datetimeoffset]::Now.ToString('o')
    since = if ($PSBoundParameters.ContainsKey('Since')) { $Since.ToString('o') } else { $null }
    log_dir = (Resolve-Path -LiteralPath $LogDir).Path
    thresholds = [pscustomobject]@{
        min_records = $MinRecords
        min_repos = $MinRepos
        min_saved_tokens = $MinSavedTokens
        min_saved_percent = $MinSavedPercent
    }
    files = @($filePaths.ToArray())
    record_count = $records.Count
    parse_error_count = $parseErrors.Count
    parse_errors = @($parseErrors.ToArray())
    summaries = @($summaries.ToArray())
}

function ConvertTo-MarkdownReport {
    param([object]$Report)

    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add('# Replacement Shadow Benchmark Report')
    [void]$lines.Add('')
    [void]$lines.Add("Generated: $($Report.generated_at)")
    [void]$lines.Add('')
    [void]$lines.Add(('Log dir: `{0}`' -f $Report.log_dir))
    [void]$lines.Add('')
    if ($null -ne $Report.since) {
        [void]$lines.Add("Since: $($Report.since)")
        [void]$lines.Add('')
    }
    [void]$lines.Add("Records: $($Report.record_count)")
    [void]$lines.Add('')
    [void]$lines.Add("Parse errors: $($Report.parse_error_count)")
    [void]$lines.Add('')
    [void]$lines.Add("Thresholds: records >= $($Report.thresholds.min_records), repos >= $($Report.thresholds.min_repos), saved tokens >= $($Report.thresholds.min_saved_tokens), saved percent >= $($Report.thresholds.min_saved_percent)")
    [void]$lines.Add('')
    [void]$lines.Add('| Operation | Strategy | Command class | Records | Repos | Errors | Fallbacks | Gate passed | Avg saved tokens | Avg saved % | Recommendation |')
    [void]$lines.Add('|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|')
    foreach ($summary in ($Report.summaries | Sort-Object recommendation, operation, command_class)) {
        [void]$lines.Add((
            '| {0} | {1} | {2} | {3} | {4} | {5} | {6} | {7} | {8} | {9} | {10} |' -f
            (Escape-MarkdownCell $summary.operation),
            (Escape-MarkdownCell $summary.strategy),
            (Escape-MarkdownCell $summary.command_class),
            $summary.records,
            $summary.repos,
            $summary.errors,
            $summary.fallbacks,
            $summary.gate_passed,
            (Escape-MarkdownCell $summary.avg_saved_tokens),
            (Escape-MarkdownCell $summary.avg_saved_percent),
            (Escape-MarkdownCell $summary.recommendation)
        ))
    }
    [void]$lines.Add('')
    [void]$lines.Add('## Artifact Samples')
    [void]$lines.Add('')
    foreach ($summary in ($Report.summaries | Sort-Object recommendation, operation, command_class)) {
        [void]$lines.Add(('- `{0}` / `{1}` / `{2}`' -f $summary.operation, $summary.command_class, $summary.recommendation))
        if (-not [string]::IsNullOrWhiteSpace($summary.sample_baseline_artifact)) {
            [void]$lines.Add(('  - baseline: `{0}`' -f $summary.sample_baseline_artifact))
        }
        if (-not [string]::IsNullOrWhiteSpace($summary.sample_replacement_artifact)) {
            [void]$lines.Add(('  - replacement: `{0}`' -f $summary.sample_replacement_artifact))
        }
    }
    return ($lines -join "`n")
}

if ($Json) {
    $jsonText = $report | ConvertTo-Json -Depth 8
    if (-not [string]::IsNullOrWhiteSpace($OutMarkdown)) {
        Set-Content -LiteralPath $OutMarkdown -Value $jsonText -Encoding UTF8
    }
    $jsonText
}
else {
    $markdown = ConvertTo-MarkdownReport $report
    if (-not [string]::IsNullOrWhiteSpace($OutMarkdown)) {
        Set-Content -LiteralPath $OutMarkdown -Value $markdown -Encoding UTF8
    }
    $markdown
}
