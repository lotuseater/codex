param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
    [string]$CodexHome = (Join-Path $env:USERPROFILE ".codex"),
    [switch]$NoGlobalSkillCopy
)

$ErrorActionPreference = "Stop"

function Select-Python {
    $candidates = @(
        "C:\Program Files\Python314\python.exe",
        "python.exe",
        "python"
    )
    foreach ($candidate in $candidates) {
        try {
            $cmd = Get-Command $candidate -ErrorAction Stop
            return $cmd.Source
        } catch {
        }
    }
    throw "Python was not found for prototype-first hook installation."
}

function Json-String([string]$Value) {
    return ConvertTo-Json -Compress -InputObject $Value
}

function Get-HookHash {
    param(
        [string]$EventKey,
        [AllowNull()]$Matcher,
        [string]$Command,
        [int]$Timeout
    )
    $commandJson = Json-String $Command
    $canonical = "{`"event_name`":`"$EventKey`",`"hooks`":[{`"async`":false,`"command`":$commandJson,`"timeout`":$Timeout,`"type`":`"command`"}]"
    if ($null -ne $Matcher) {
        $matcherJson = Json-String $Matcher
        $canonical += ",`"matcher`":$matcherJson"
    }
    $canonical += "}"

    $bytes = [System.Text.Encoding]::UTF8.GetBytes($canonical)
    $hashBytes = [System.Security.Cryptography.SHA256]::HashData($bytes)
    $hex = -join ($hashBytes | ForEach-Object { $_.ToString("x2") })
    return "sha256:$hex"
}

function Merge-HookGroup {
    param(
        [pscustomobject]$HooksConfig,
        [string]$EventName,
        [AllowNull()]$Matcher,
        [object]$Hook
    )
    if (-not $HooksConfig.hooks.PSObject.Properties[$EventName]) {
        $HooksConfig.hooks | Add-Member -NotePropertyName $EventName -NotePropertyValue @()
    }
    $groups = @($HooksConfig.hooks.$EventName)
    for ($i = 0; $i -lt $groups.Count; $i++) {
        $group = $groups[$i]
        $groupMatcher = if ($group.PSObject.Properties["matcher"]) { $group.matcher } else { $null }
        if ($groupMatcher -eq $Matcher) {
            $existingHooks = @($group.hooks)
            $filtered = @($existingHooks | Where-Object { $_.command -ne $Hook.command })
            $group.hooks = @($filtered + $Hook)
            $HooksConfig.hooks.$EventName = @($groups)
            return
        }
    }

    if ($null -eq $Matcher) {
        $newGroup = [pscustomobject]@{ hooks = @($Hook) }
    } else {
        $newGroup = [pscustomobject]@{ matcher = $Matcher; hooks = @($Hook) }
    }
    $HooksConfig.hooks.$EventName = @($groups + $newGroup)
}

function Upsert-TrustState {
    param(
        [string]$ConfigPath,
        [string]$Key,
        [string]$Hash
    )
    $content = if (Test-Path $ConfigPath) { Get-Content -Path $ConfigPath -Raw } else { "" }
    if ($content -notmatch "(?m)^\[hooks\.state\]\s*$") {
        $content = $content.TrimEnd() + "`r`n`r`n[hooks.state]`r`n"
    }
    $header = "[hooks.state.'$Key']"
    $escapedHeader = [regex]::Escape($header)
    $block = "$header`r`ntrusted_hash = `"$Hash`"`r`n"
    if ($content -match "(?ms)^$escapedHeader\s*\r?\n.*?(?=^\[|\z)") {
        $content = [regex]::Replace($content, "(?ms)^$escapedHeader\s*\r?\n.*?(?=^\[|\z)", $block)
    } else {
        $content = $content.TrimEnd() + "`r`n`r`n$block"
    }
    Set-Content -Path $ConfigPath -Value $content -Encoding UTF8
}

function Find-HookIndex {
    param(
        [pscustomobject]$HooksConfig,
        [string]$EventName,
        [AllowNull()]$Matcher,
        [string]$Command
    )
    $groups = @($HooksConfig.hooks.$EventName)
    for ($groupIndex = 0; $groupIndex -lt $groups.Count; $groupIndex++) {
        $group = $groups[$groupIndex]
        $groupMatcher = if ($group.PSObject.Properties["matcher"]) { $group.matcher } else { $null }
        if ($groupMatcher -ne $Matcher) {
            continue
        }
        $hooks = @($group.hooks)
        for ($hookIndex = 0; $hookIndex -lt $hooks.Count; $hookIndex++) {
            if ($hooks[$hookIndex].command -eq $Command) {
                return @($groupIndex, $hookIndex)
            }
        }
    }
    throw "Installed hook was not found for $EventName."
}

function Remove-HookCommand {
    param(
        [pscustomobject]$HooksConfig,
        [string]$Command
    )
    foreach ($eventName in @("UserPromptSubmit", "PreToolUse", "PostToolUse")) {
        if (-not $HooksConfig.hooks.PSObject.Properties[$eventName]) {
            continue
        }
        $keptGroups = @()
        foreach ($group in @($HooksConfig.hooks.$eventName)) {
            $remainingHooks = @(@($group.hooks) | Where-Object { $_.command -ne $Command })
            if ($remainingHooks.Count -eq 0) {
                continue
            }
            $group.hooks = $remainingHooks
            $keptGroups += $group
        }
        $HooksConfig.hooks.$eventName = $keptGroups
    }
}

$python = Select-Python
$repoSkill = Join-Path $RepoRoot ".codex\skills\prototype-first-automation"
$repoHook = Join-Path $RepoRoot ".codex\hooks\prototype_first_hint.py"
if (-not (Test-Path $repoSkill)) {
    throw "Missing repo skill: $repoSkill"
}
if (-not (Test-Path $repoHook)) {
    throw "Missing repo hook: $repoHook"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backupRoot = Join-Path $CodexHome "backups\prototype-first-hook-$stamp"
New-Item -ItemType Directory -Force -Path $backupRoot | Out-Null

$hooksPath = Join-Path $CodexHome "hooks.json"
$configPath = Join-Path $CodexHome "config.toml"
if (Test-Path $hooksPath) {
    Copy-Item -LiteralPath $hooksPath -Destination (Join-Path $backupRoot "hooks.json")
}
if (Test-Path $configPath) {
    Copy-Item -LiteralPath $configPath -Destination (Join-Path $backupRoot "config.toml")
}

$globalHookDir = Join-Path $CodexHome "hooks"
New-Item -ItemType Directory -Force -Path $globalHookDir | Out-Null
$globalHook = Join-Path $globalHookDir "prototype_first_hint.py"
Copy-Item -LiteralPath $repoHook -Destination $globalHook -Force

if (-not $NoGlobalSkillCopy) {
    $globalSkill = Join-Path $CodexHome "skills\prototype-first-automation"
    New-Item -ItemType Directory -Force -Path $globalSkill | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoSkill "SKILL.md") -Destination (Join-Path $globalSkill "SKILL.md") -Force
}

$hooksConfig = if (Test-Path $hooksPath) {
    Get-Content -Path $hooksPath -Raw | ConvertFrom-Json
} else {
    [pscustomobject]@{ hooks = [pscustomobject]@{} }
}
if (-not $hooksConfig.PSObject.Properties["hooks"]) {
    $hooksConfig | Add-Member -NotePropertyName "hooks" -NotePropertyValue ([pscustomobject]@{})
}

$command = "`"$python`" `"$globalHook`""
$timeout = 5
$toolMatcher = "^(Bash|apply_patch|spawn_agent|followup_task|shell|local_shell|shell_command|exec_command|multi_tool_use\.parallel)$"
$hook = [pscustomobject]@{
    type = "command"
    command = $command
    timeout = $timeout
}

Remove-HookCommand -HooksConfig $hooksConfig -Command $command
Merge-HookGroup -HooksConfig $hooksConfig -EventName "UserPromptSubmit" -Matcher $null -Hook $hook
Merge-HookGroup -HooksConfig $hooksConfig -EventName "PreToolUse" -Matcher $toolMatcher -Hook $hook
Merge-HookGroup -HooksConfig $hooksConfig -EventName "PostToolUse" -Matcher $toolMatcher -Hook $hook

$hooksConfig | ConvertTo-Json -Depth 12 | Set-Content -Path $hooksPath -Encoding UTF8

$trustItems = @(
    @{ EventName = "UserPromptSubmit"; EventKey = "user_prompt_submit"; Matcher = $null },
    @{ EventName = "PreToolUse"; EventKey = "pre_tool_use"; Matcher = $toolMatcher },
    @{ EventName = "PostToolUse"; EventKey = "post_tool_use"; Matcher = $toolMatcher }
)
foreach ($item in $trustItems) {
    $indexes = Find-HookIndex -HooksConfig $hooksConfig -EventName $item.EventName -Matcher $item.Matcher -Command $command
    $key = "${hooksPath}:$($item.EventKey):$($indexes[0]):$($indexes[1])"
    $hash = Get-HookHash -EventKey $item.EventKey -Matcher $item.Matcher -Command $command -Timeout $timeout
    Upsert-TrustState -ConfigPath $configPath -Key $key -Hash $hash
}

Write-Host "Installed prototype-first hook."
Write-Host "Backup: $backupRoot"
Write-Host "Hook: $globalHook"
