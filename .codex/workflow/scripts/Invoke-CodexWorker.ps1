[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Repo,

    [Parameter(Mandatory = $true)]
    [string]$PromptFile,

    [ValidateSet("Interactive", "Exec", "Version")]
    [string]$Mode = "Interactive",

    [string]$MarkerFile
)

$ErrorActionPreference = "Stop"

function Write-Marker {
    param([string]$Message)

    if (-not $MarkerFile) {
        return
    }

    $markerParent = Split-Path -Parent $MarkerFile
    if ($markerParent) {
        New-Item -ItemType Directory -Force -Path $markerParent | Out-Null
    }
    Add-Content -LiteralPath $MarkerFile -Value "$(Get-Date -Format o) $Message"
}

$repoPath = (Resolve-Path -LiteralPath $Repo).Path
$promptPath = (Resolve-Path -LiteralPath $PromptFile).Path

Set-Location -LiteralPath $repoPath
$codexCommand = Get-Command codex -ErrorAction Stop

Write-Marker "starting mode=$Mode repo=$repoPath prompt=$promptPath codex=$($codexCommand.Source)"

try {
    switch ($Mode) {
        "Version" {
            & codex --version
            $exitCode = $LASTEXITCODE
        }
        "Exec" {
            $prompt = Get-Content -Raw -LiteralPath $promptPath
            & codex exec $prompt
            $exitCode = $LASTEXITCODE
        }
        "Interactive" {
            $prompt = Get-Content -Raw -LiteralPath $promptPath
            & codex $prompt
            $exitCode = $LASTEXITCODE
        }
    }
}
catch {
    Write-Marker "failed error=$($_.Exception.Message)"
    throw
}

Write-Marker "completed mode=$Mode exit=$exitCode"
exit $exitCode
