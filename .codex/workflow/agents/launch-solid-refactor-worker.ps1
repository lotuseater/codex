param(
    [Parameter(Mandatory = $true)]
    [string]$Repo,

    [Parameter(Mandatory = $true)]
    [string]$PromptPath,

    [Parameter(Mandatory = $true)]
    [string]$CodexCommand,

    [string]$LogPath
)

$ErrorActionPreference = "Stop"
$prompt = Get-Content -LiteralPath $PromptPath -Raw

if ($LogPath) {
    $logDir = Split-Path -Parent $LogPath
    if ($logDir) {
        New-Item -ItemType Directory -Force -Path $logDir | Out-Null
    }

    & $CodexCommand --cd $Repo --ask-for-approval never --sandbox danger-full-access exec $prompt 2>&1 |
        Tee-Object -FilePath $LogPath
} else {
    & $CodexCommand --cd $Repo --ask-for-approval never --sandbox danger-full-access exec $prompt
}

exit $LASTEXITCODE
