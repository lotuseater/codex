[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$WrapperDir = (Join-Path $HOME ".codex\system-wrapper")
)

$ErrorActionPreference = "Stop"

function Read-JsonObject {
    param([string]$Path)

    $json = Get-Content -LiteralPath $Path -Raw
    if ([string]::IsNullOrWhiteSpace($json)) {
        return [ordered]@{}
    }

    $parsed = $json | ConvertFrom-Json
    $result = [ordered]@{}
    foreach ($property in $parsed.PSObject.Properties) {
        $result[$property.Name] = $property.Value
    }
    return $result
}

function Write-JsonObject {
    param(
        [string]$Path,
        [System.Collections.IDictionary]$Payload
    )

    $Payload | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Path -Encoding UTF8
}

$envPath = Join-Path $WrapperDir "system.codex-wrapper.env.json"
if (-not (Test-Path -LiteralPath $envPath)) {
    throw "Wrapper env JSON not found: $envPath"
}

$payload = Read-JsonObject -Path $envPath
$standardExe = [string]$payload["WIZARD_CODEX_STANDARD_NPM_NATIVE_EXE"]
if ([string]::IsNullOrWhiteSpace($standardExe)) {
    throw "WIZARD_CODEX_STANDARD_NPM_NATIVE_EXE is missing from $envPath"
}
if (-not (Test-Path -LiteralPath $standardExe)) {
    throw "Standard Codex exe does not exist: $standardExe"
}

$payload["WIZARD_CODEX_REAL_EXE"] = $standardExe
$payload["WIZARD_CODEX_LOCAL_FORK_RESTORED_AT"] = (Get-Date).ToString("o")
$payload["WIZARD_CODEX_OPERATION_CACHE"] = "0"

if ($PSCmdlet.ShouldProcess($envPath, "restore WIZARD_CODEX_REAL_EXE to standard exe $standardExe")) {
    Write-JsonObject -Path $envPath -Payload $payload
}

& $standardExe --version | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "Standard Codex exe failed --version with exit code $LASTEXITCODE"
}

[ordered]@{
    status = "ok"
    wrapper_env_path = $envPath
    real_exe = $standardExe
} | ConvertTo-Json -Depth 4
