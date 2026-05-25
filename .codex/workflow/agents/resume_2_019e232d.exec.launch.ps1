$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: resume_2_019e232d'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('resume', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--approval-policy', 'never', '--sandbox', 'danger-full-access', '--loop', '019e232d-007e-7530-bf2b-d545a37d83d5', 'go on, take a look on handoffs')
$redirectToLog = $false
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\resume_2_019e232d.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
