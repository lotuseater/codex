param()

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$reviewTargetTs = Join-Path $repoRoot "codex-rs\app-server-protocol\schema\typescript\v2\ReviewTarget.ts"
if (-not (Test-Path -LiteralPath $reviewTargetTs)) {
    throw "ReviewTarget.ts not found: $reviewTargetTs"
}

$contents = Get-Content -LiteralPath $reviewTargetTs -Raw
if (-not $contents.Contains("title?: string | null")) {
    throw "ReviewTarget.ts must keep commit title optional and nullable: expected 'title?: string | null'."
}
if ($contents.Contains("title: string | null")) {
    throw "ReviewTarget.ts regressed: commit title is required in generated TypeScript."
}

[ordered]@{
    status = "ok"
    path = $reviewTargetTs
    assertion = "ReviewTarget commit title is optional and nullable"
} | ConvertTo-Json -Depth 3
