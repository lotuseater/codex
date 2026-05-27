param(
    [Parameter(Mandatory = $true)]
    [string]$Name,

    [Parameter(Mandatory = $true)]
    [string]$Repo,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Commands
)

$ErrorActionPreference = 'Continue'
$logDir = Join-Path $Repo '.codex\workflow\agents\logs'
$handoffDir = Join-Path $Repo '.codex\workflow\agents\handoffs'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
New-Item -ItemType Directory -Force -Path $handoffDir | Out-Null

$log = Join-Path $logDir "$Name.log"
$handoff = Join-Path $handoffDir "$Name.md"
Set-Location (Join-Path $Repo 'codex-rs')

@"
# $Name
Started: $(Get-Date -Format o)
Commands:
$($Commands -join "`n")
"@ | Set-Content -Path $log -Encoding UTF8

$overall = 0
foreach ($cmd in $Commands) {
    "`n---- RUN: $cmd ----" | Add-Content -Path $log -Encoding UTF8
    cmd.exe /d /c $cmd *>> $log
    $code = $LASTEXITCODE
    "---- EXIT: $code ----" | Add-Content -Path $log -Encoding UTF8
    if ($code -ne 0) {
        $overall = $code
        break
    }
}

$status = if ($overall -eq 0) { 'pass' } else { 'fail' }
$tail = Get-Content -Path $log -Tail 100 | Out-String
$commandLines = ($Commands | ForEach-Object { "- ``$_``" }) -join "`n"
@"
# $Name

Status: $status
ExitCode: $overall
Completed: $(Get-Date -Format o)

Commands:
$commandLines

Log: $log

Tail:
``````text
$tail
``````
"@ | Set-Content -Path $handoff -Encoding UTF8

exit $overall
