param(
    [string[]]$Projects = @(
        (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,
        (Join-Path (Split-Path -Parent (Split-Path -Parent (Resolve-Path (Join-Path $PSScriptRoot '..')).Path)) 'Serial_to_Google_Doc_topdown')
    ),

    [string]$CodexCommand = 'codex',

    [switch]$Execute,

    [string]$OutFile,

    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-CodexText {
    param([string[]]$Arguments)

    $output = & $CodexCommand @Arguments 2>&1
    return ($output | Out-String).Trim()
}

function Test-DeployedFeatures {
    $version = Invoke-CodexText @('--version')
    $features = Invoke-CodexText @('features', 'list')
    $shadow = $features -match '(?m)^context_ops_shadow\s+.*\btrue\s*$'
    $replace = $features -match '(?m)^context_ops_replace\s+.*\btrue\s*$'
    return [pscustomobject]@{
        version = $version
        context_ops_shadow = [bool]$shadow
        context_ops_replace = [bool]$replace
        feature_text = $features
    }
}

function Get-RepoCanaryFile {
    param([string]$Project)

    Push-Location $Project
    try {
        $paths = @(& rg --files 2>$null | Where-Object {
            $_ -match '\.(rs|ps1|py|ts|tsx|js|md)$' -and $_ -notmatch '(^|/)(target|node_modules|dist|build|\.git)/'
        } | Select-Object -First 1)
        if ($paths.Count -gt 0) {
            return $paths[0]
        }
        $gitPaths = @(& git ls-files 2>$null | Select-Object -First 1)
        if ($gitPaths.Count -gt 0) {
            return $gitPaths[0]
        }
    }
    finally {
        Pop-Location
    }
    return $null
}

function New-Canary {
    param(
        [string]$Project,
        [string]$Name,
        [string]$ShellCommand
    )

    $prompt = "Run exactly this read-only shell command once from the current repo, then answer with one concise sentence. Do not run extra exploration.`n`n$ShellCommand"
    $arguments = @(
        '-C', $Project,
        '-c', 'features.context_ops_shadow=true',
        '-c', 'features.context_ops_replace=true',
        $prompt
    )
    return [pscustomobject]@{
        project = $Project
        repo = Split-Path -Leaf $Project
        name = $Name
        shell_command = $ShellCommand
        codex_arguments = $arguments
        codex_command_line = "$CodexCommand " + (($arguments | ForEach-Object {
            if ($_ -match '\s') { '"' + ($_.Replace('"', '\"')) + '"' } else { $_ }
        }) -join ' ')
    }
}

function Get-CanariesForProject {
    param([string]$Project)

    $canaries = New-Object 'System.Collections.Generic.List[object]'
    $sampleFile = Get-RepoCanaryFile $Project
    [void]$canaries.Add((New-Canary $Project 'git_diff_stat' 'git diff --stat'))
    [void]$canaries.Add((New-Canary $Project 'git_diff_shortstat' 'git diff --shortstat'))
    [void]$canaries.Add((New-Canary $Project 'git_status_short' 'git status --short'))
    [void]$canaries.Add((New-Canary $Project 'git_changed_files' 'git diff --name-only'))
    [void]$canaries.Add((New-Canary $Project 'git_name_status' 'git diff --name-status'))
    [void]$canaries.Add((New-Canary $Project 'git_numstat' 'git diff --numstat'))
    [void]$canaries.Add((New-Canary $Project 'git_history_digest' 'git log --oneline -n 30'))
    [void]$canaries.Add((New-Canary $Project 'rg_count' 'rg --count TODO .'))
    [void]$canaries.Add((New-Canary $Project 'rg_files' 'rg --files'))
    [void]$canaries.Add((New-Canary $Project 'rg_json' 'rg --json TODO .'))
    [void]$canaries.Add((New-Canary $Project 'directory_listing' 'Get-ChildItem -Recurse -File | Select-Object -First 200 FullName,Length'))
    [void]$canaries.Add((New-Canary $Project 'process_table' 'Get-Process | Select-Object -First 80 ProcessName,Id,CPU'))
    if ($sampleFile) {
        [void]$canaries.Add((New-Canary $Project 'file_excerpt' "Get-Content -TotalCount 80 -Path $sampleFile"))
        [void]$canaries.Add((New-Canary $Project 'file_outline_shadow' "Get-Content -Path $sampleFile"))
        [void]$canaries.Add((New-Canary $Project 'select_string' "Select-String -Path $sampleFile -Pattern TODO"))
    }
    if (Test-Path -LiteralPath (Join-Path $Project 'scripts\build-local-codex.ps1')) {
        [void]$canaries.Add((New-Canary $Project 'run_check_digest' 'powershell -ExecutionPolicy Bypass -File scripts\build-local-codex.ps1 -Mode Status'))
    }
    return $canaries
}

$existingProjects = @($Projects | Where-Object { Test-Path -LiteralPath $_ -PathType Container } | ForEach-Object {
    (Resolve-Path -LiteralPath $_).Path
})
if ($existingProjects.Count -eq 0) {
    throw 'No canary projects exist.'
}

$featureState = Test-DeployedFeatures
if ($Execute -and (-not $featureState.context_ops_shadow -or -not $featureState.context_ops_replace)) {
    throw "Refusing to execute canaries because deployed features are not ready. context_ops_shadow=$($featureState.context_ops_shadow), context_ops_replace=$($featureState.context_ops_replace)"
}

$canaries = New-Object 'System.Collections.Generic.List[object]'
foreach ($project in $existingProjects) {
    foreach ($canary in Get-CanariesForProject $project) {
        [void]$canaries.Add($canary)
    }
}

if ($Execute) {
    foreach ($canary in $canaries) {
        Start-Process -FilePath $CodexCommand -ArgumentList $canary.codex_arguments -WorkingDirectory $canary.project
    }
}

$result = [pscustomobject]@{
    generated_at = [datetimeoffset]::Now.ToString('o')
    execute = [bool]$Execute
    codex_version = $featureState.version
    context_ops_shadow = $featureState.context_ops_shadow
    context_ops_replace = $featureState.context_ops_replace
    projects = @($existingProjects)
    canaries = @($canaries.ToArray())
}

if ($Json) {
    $text = $result | ConvertTo-Json -Depth 8
}
else {
    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add("# Replacement Shadow Canaries")
    [void]$lines.Add("")
    [void]$lines.Add("Generated: $($result.generated_at)")
    [void]$lines.Add("")
    [void]$lines.Add("Execute: $($result.execute)")
    [void]$lines.Add("")
    [void]$lines.Add("Codex version: $($result.codex_version)")
    [void]$lines.Add("")
    [void]$lines.Add("Features: context_ops_shadow=$($result.context_ops_shadow), context_ops_replace=$($result.context_ops_replace)")
    [void]$lines.Add("")
    foreach ($canary in $canaries) {
        [void]$lines.Add("## $($canary.repo) / $($canary.name)")
        [void]$lines.Add("")
        [void]$lines.Add("Shell command:")
        [void]$lines.Add("")
        [void]$lines.Add('```powershell')
        [void]$lines.Add($canary.shell_command)
        [void]$lines.Add('```')
        [void]$lines.Add("")
        [void]$lines.Add("Codex command:")
        [void]$lines.Add("")
        [void]$lines.Add('```powershell')
        [void]$lines.Add($canary.codex_command_line)
        [void]$lines.Add('```')
        [void]$lines.Add("")
    }
    $text = $lines -join "`n"
}

if (-not [string]::IsNullOrWhiteSpace($OutFile)) {
    Set-Content -LiteralPath $OutFile -Value $text -Encoding UTF8
}

$text
