param(
    [string]$CodexCommand = 'codex',

    [string]$OutRoot = (Join-Path ([System.IO.Path]::GetTempPath()) 'codex-multiagent-v2-canaries'),

    [switch]$Execute,

    [switch]$KeepProject,

    [switch]$StoryDialog,

    [string]$TranscriptPath,

    [switch]$Json
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Quote-CommandPart {
    param([string]$Value)

    if ($Value -match '\s|["]') {
        return '"' + ($Value.Replace('"', '\"')) + '"'
    }
    return $Value
}

function New-CanaryProject {
    param(
        [string]$Root,
        [switch]$StoryDialog
    )

    $stamp = [DateTimeOffset]::Now.ToString('yyyyMMdd-HHmmss')
    $project = Join-Path $Root "multiagent-v2-canary-$stamp"
    $src = Join-Path $project 'src'
    New-Item -ItemType Directory -Force -Path $src | Out-Null

    Set-Content -LiteralPath (Join-Path $project 'README.md') -Encoding UTF8 -Value @'
# MultiAgentV2 Canary

The task is to split three independent evidence reads across three workers and
merge their findings into one final report.
'@
    Set-Content -LiteralPath (Join-Path $src 'a.txt') -Encoding UTF8 -Value 'alpha fact: compact/restart controls must reject root targets.'
    Set-Content -LiteralPath (Join-Path $src 'b.txt') -Encoding UTF8 -Value 'bravo fact: child activity should show model, effort, and token percent near the label.'
    Set-Content -LiteralPath (Join-Path $src 'c.txt') -Encoding UTF8 -Value 'charlie fact: workers should script repeated checks when that saves time and tokens.'

    if ($StoryDialog) {
        Set-Content -LiteralPath (Join-Path $project 'append_dialog_line.py') -Encoding UTF8 -Value @'
from __future__ import annotations

import argparse
import os
import time
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--file", required=True)
    parser.add_argument("--line-number", type=int, action="append", required=True)
    parser.add_argument("--line", action="append", required=True)
    parser.add_argument("--worker-id", default="")
    args = parser.parse_args()

    if len(args.line_number) != len(args.line):
        raise SystemExit("line-number mode requires one --line-number for each --line")

    target = Path(args.file)
    lock = target.with_suffix(target.suffix + ".lock")
    deadline = time.time() + 30.0
    fd = None
    while time.time() < deadline:
        try:
            fd = os.open(str(lock), os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            break
        except FileExistsError:
            time.sleep(0.05)
    if fd is None:
        raise SystemExit(f"could not acquire lock: {lock}")

    try:
        existing: list[str] = []
        if target.exists():
            existing = target.read_text(encoding="utf-8", errors="replace").splitlines()
        for line_number, line in zip(args.line_number, args.line):
            index = line_number - 1
            if index < 0:
                raise SystemExit("line-number must be positive")
            while len(existing) <= index:
                existing.append("")
            if existing[index].strip() and existing[index].strip() != line:
                raise SystemExit(f"line {line_number} already differs: {existing[index]!r}")
            existing[index] = line
        target.write_text("\n".join(existing) + "\n", encoding="utf-8")
    finally:
        os.close(fd)
        try:
            lock.unlink()
        except FileNotFoundError:
            pass

    print(f"FILE: {target.name}")
    print(f"COUNT: {len([line for line in existing if line.strip()])}")
    if args.worker_id:
        print(f"WORKER: {args.worker_id}")
    for index, line in enumerate(args.line):
        print(f"LINE-NUMBER: {args.line_number[index]}")
        print(f"LINE: {line}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
'@
    }

    return (Resolve-Path -LiteralPath $project).Path
}

function New-CanaryPrompt {
    param([string]$Project)

    return @"
Use MultiAgentV2 for this canary. Spawn exactly three subagents with stable task names: alpha_reader, bravo_reader, and charlie_reader.

Each worker must stay within its assignment and write exactly one output file:
- alpha_reader may inspect only src/a.txt and must write worker_1_output.md.
- bravo_reader may inspect only src/b.txt and must write worker_2_output.md.
- charlie_reader may inspect only src/c.txt and must write worker_3_output.md.

Each worker output must include:
- the assigned source path
- the fact found
- a verification line
- any automation used, or "automation: not needed"

After all workers finish, the root agent must read the three worker files and write final_report.md combining all three facts. Do not read other source files. Use scripts or shell automation for repeated artifact checks if useful. Current canary project: $Project
"@
}

function New-StoryDialogPrompt {
    param([string]$Project)

    $dialogFile = 'story_dialog.txt'
    $topic = 'silver observatory signal'
    $commands = @(
        '1: story_dialog.txt => `python append_dialog_line.py --file "story_dialog.txt" --worker-id 1 --line-number 1 --line "Explorer: <fresh Explorer line 1 about silver observatory signal>" --line-number 6 --line "Explorer: <fresh Explorer line 2 about silver observatory signal>" --line-number 11 --line "Explorer: <fresh Explorer line 3 about silver observatory signal>"`',
        '2: story_dialog.txt => `python append_dialog_line.py --file "story_dialog.txt" --worker-id 2 --line-number 2 --line "Skeptic: <fresh Skeptic line 1 about silver observatory signal>" --line-number 7 --line "Skeptic: <fresh Skeptic line 2 about silver observatory signal>" --line-number 12 --line "Skeptic: <fresh Skeptic line 3 about silver observatory signal>"`',
        '3: story_dialog.txt => `python append_dialog_line.py --file "story_dialog.txt" --worker-id 3 --line-number 3 --line "Builder: <fresh Builder line 1 about silver observatory signal>" --line-number 8 --line "Builder: <fresh Builder line 2 about silver observatory signal>" --line-number 13 --line "Builder: <fresh Builder line 3 about silver observatory signal>"`',
        '4: story_dialog.txt => `python append_dialog_line.py --file "story_dialog.txt" --worker-id 4 --line-number 4 --line "Witness: <fresh Witness line 1 about silver observatory signal>" --line-number 9 --line "Witness: <fresh Witness line 2 about silver observatory signal>" --line-number 14 --line "Witness: <fresh Witness line 3 about silver observatory signal>"`',
        '5: story_dialog.txt => `python append_dialog_line.py --file "story_dialog.txt" --worker-id 5 --line-number 5 --line "Mediator: <fresh Mediator line 1 about silver observatory signal>" --line-number 10 --line "Mediator: <fresh Mediator line 2 about silver observatory signal>" --line-number 15 --line "Mediator: <fresh Mediator line 3 about silver observatory signal>"`'
    )

    return @"
Use MultiAgentV2 for this canary. Work only in: $Project

Full mode, operational coordination only. No repo exploration, no coding, and no helper scripts beyond the existing append_dialog_line.py.

Plan first. The plan must include a Work Split section and start it with WORKER_COUNT: 5. Spawn exactly five subagents with stable task names: dialog_worker_1, dialog_worker_2, dialog_worker_3, dialog_worker_4, dialog_worker_5.

Shared file: $dialogFile
Topic: $topic

All five workers must be used. Each worker runs exactly its mapped command once. Replace every angle-bracket placeholder with that worker's own fresh original phrase before running it. Each written line must start with the assigned role prefix and a colon. Keep this exact mapping:
$($commands -join "`n")

Keep the file to exactly fifteen non-empty lines total. The final line order must be three rounds of Worker 1, Worker 2, Worker 3, Worker 4, Worker 5. After all workers finish, verify the file line count, uniqueness, role order, and absence of angle-bracket placeholders. Final result must quote the full fifteen-line dialog.
"@
}

function Test-CanaryArtifacts {
    param([string]$Project)

    $checks = [ordered]@{}
    $expected = @(
        @{ file = 'worker_1_output.md'; assigned = 'src/a.txt'; forbidden = @('src/b.txt', 'src/c.txt'); fact = 'alpha fact' },
        @{ file = 'worker_2_output.md'; assigned = 'src/b.txt'; forbidden = @('src/a.txt', 'src/c.txt'); fact = 'bravo fact' },
        @{ file = 'worker_3_output.md'; assigned = 'src/c.txt'; forbidden = @('src/a.txt', 'src/b.txt'); fact = 'charlie fact' }
    )

    foreach ($item in $expected) {
        $path = Join-Path $Project $item.file
        $text = if (Test-Path -LiteralPath $path) { Get-Content -LiteralPath $path -Raw } else { '' }
        $checks["$($item.file)_exists"] = [bool](Test-Path -LiteralPath $path)
        $checks["$($item.file)_mentions_assigned"] = [bool]($text -match [regex]::Escape($item.assigned))
        $checks["$($item.file)_mentions_fact"] = [bool]($text -match [regex]::Escape($item.fact))
        $checks["$($item.file)_mentions_verification"] = [bool]($text -match '(?i)verification')
        $checks["$($item.file)_avoids_other_sources"] = [bool](-not ($item.forbidden | Where-Object {
            $text -match [regex]::Escape($_)
        }))
    }

    $finalPath = Join-Path $Project 'final_report.md'
    $finalText = if (Test-Path -LiteralPath $finalPath) { Get-Content -LiteralPath $finalPath -Raw } else { '' }
    $checks['final_report_exists'] = [bool](Test-Path -LiteralPath $finalPath)
    $checks['final_report_has_alpha'] = [bool]($finalText -match 'alpha fact')
    $checks['final_report_has_bravo'] = [bool]($finalText -match 'bravo fact')
    $checks['final_report_has_charlie'] = [bool]($finalText -match 'charlie fact')

    return [pscustomobject]$checks
}

function Test-StoryDialogArtifacts {
    param([string]$Project)

    $dialogPath = Join-Path $Project 'story_dialog.txt'
    $text = if (Test-Path -LiteralPath $dialogPath) { Get-Content -LiteralPath $dialogPath -Raw } else { '' }
    $lines = @($text -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_.Trim() })
    $expectedRoles = @(
        'Explorer', 'Skeptic', 'Builder', 'Witness', 'Mediator',
        'Explorer', 'Skeptic', 'Builder', 'Witness', 'Mediator',
        'Explorer', 'Skeptic', 'Builder', 'Witness', 'Mediator'
    )
    $roleOrderOk = $lines.Count -eq $expectedRoles.Count
    if ($roleOrderOk) {
        for ($i = 0; $i -lt $expectedRoles.Count; $i++) {
            if (-not $lines[$i].StartsWith("$($expectedRoles[$i]): ")) {
                $roleOrderOk = $false
                break
            }
        }
    }
    $checks = [ordered]@{
        story_dialog_exists = [bool](Test-Path -LiteralPath $dialogPath)
        story_dialog_has_15_lines = [bool]($lines.Count -eq 15)
        story_dialog_lines_unique = [bool](($lines | Select-Object -Unique).Count -eq $lines.Count)
        story_dialog_role_order_ok = [bool]$roleOrderOk
        story_dialog_no_placeholders = [bool](-not ($text -match '<[^>]+>'))
        story_dialog_mentions_topic = [bool]($text -match 'silver|observatory|signal')
    }
    return [pscustomobject]$checks
}

function Test-Transcript {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{
            transcript_exists = $false
            has_agent_label = $false
            has_model_label = $false
            has_effort_label = $false
            has_token_percent = $false
        }
    }

    $text = Get-Content -LiteralPath $Path -Raw
    return [pscustomobject]@{
        transcript_exists = $true
        has_agent_label = [bool]($text -match 'alpha_reader|bravo_reader|charlie_reader|dialog_worker_[1-5]')
        has_model_label = [bool]($text -match '(?i)\bmodel\b')
        has_effort_label = [bool]($text -match '(?i)\beffort\b')
        has_token_percent = [bool]($text -match '\d{1,3}% used|--% used')
    }
}

New-Item -ItemType Directory -Force -Path $OutRoot | Out-Null
$project = New-CanaryProject -Root $OutRoot -StoryDialog:$StoryDialog
$prompt = if ($StoryDialog) { New-StoryDialogPrompt -Project $project } else { New-CanaryPrompt -Project $project }
if ([string]::IsNullOrWhiteSpace($TranscriptPath)) {
    $TranscriptPath = Join-Path $project 'codex-transcript.txt'
}

$arguments = @(
    'exec',
    '-C', $project,
    '--skip-git-repo-check',
    '--dangerously-bypass-approvals-and-sandbox',
    '-c', 'features.multi_agent_v2=true',
    '-c', 'features.multi_agent_v2.usage_hint_enabled=true',
    $prompt
)
$commandLine = "$CodexCommand " + (($arguments | ForEach-Object { Quote-CommandPart $_ }) -join ' ')

$exitCode = $null
if ($Execute) {
    $output = & $CodexCommand @arguments 2>&1
    $exitCode = $LASTEXITCODE
    $output | Set-Content -LiteralPath $TranscriptPath -Encoding UTF8
}

$artifactChecks = if ($Execute -and $StoryDialog) { Test-StoryDialogArtifacts -Project $project } elseif ($Execute) { Test-CanaryArtifacts -Project $project } else { $null }
$transcriptChecks = if ($Execute) { Test-Transcript -Path $TranscriptPath } else { $null }
$passed = if ($Execute) {
    $allChecks = @()
    $artifactChecks.PSObject.Properties | ForEach-Object { $allChecks += [bool]$_.Value }
    $transcriptChecks.PSObject.Properties | ForEach-Object { $allChecks += [bool]$_.Value }
    ($exitCode -eq 0) -and (-not ($allChecks -contains $false))
}
else {
    $null
}

$result = [pscustomobject]@{
    generated_at = [datetimeoffset]::Now.ToString('o')
    execute = [bool]$Execute
    passed = $passed
    exit_code = $exitCode
    project = $project
    transcript_path = $TranscriptPath
    codex_command_line = $commandLine
    prompt = $prompt
    artifact_checks = $artifactChecks
    transcript_checks = $transcriptChecks
}

if ($Execute -and -not $KeepProject -and $passed) {
    Remove-Item -LiteralPath $project -Recurse -Force
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
}
else {
    $lines = New-Object 'System.Collections.Generic.List[string]'
    [void]$lines.Add('# MultiAgentV2 Canary')
    [void]$lines.Add('')
    [void]$lines.Add("Generated: $($result.generated_at)")
    [void]$lines.Add("Execute: $($result.execute)")
    if ($Execute) {
        [void]$lines.Add("Passed: $($result.passed)")
        [void]$lines.Add("Exit code: $($result.exit_code)")
    }
    [void]$lines.Add("Project: $($result.project)")
    [void]$lines.Add("Transcript: $($result.transcript_path)")
    [void]$lines.Add('')
    [void]$lines.Add('Command:')
    [void]$lines.Add('')
    [void]$lines.Add('```powershell')
    [void]$lines.Add($result.codex_command_line)
    [void]$lines.Add('```')
    [void]$lines.Add('')
    [void]$lines.Add('Prompt:')
    [void]$lines.Add('')
    [void]$lines.Add('```text')
    [void]$lines.Add($result.prompt)
    [void]$lines.Add('```')
    if ($Execute) {
        [void]$lines.Add('')
        [void]$lines.Add('Artifact checks:')
        $artifactChecks.PSObject.Properties | ForEach-Object {
            [void]$lines.Add("- $($_.Name): $($_.Value)")
        }
        [void]$lines.Add('')
        [void]$lines.Add('Transcript checks:')
        $transcriptChecks.PSObject.Properties | ForEach-Object {
            [void]$lines.Add("- $($_.Name): $($_.Value)")
        }
    }
    $lines -join "`n"
}
