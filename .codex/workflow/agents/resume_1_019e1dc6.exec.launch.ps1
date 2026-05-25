$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: resume_1_019e1dc6'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\DonutGame'
$codexArgs = @('resume', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\DonutGame', '--approval-policy', 'never', '--sandbox', 'danger-full-access', '--loop', '019e1dc6-1ed3-7463-b56e-59986d56b7fd', 'go on, take a look on handoffs')
$redirectToLog = $false
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\resume_1_019e1dc6.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
