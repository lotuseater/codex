$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: resume_1_019e22c8'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\Serial_to_Google_Doc_topdown'
$codexArgs = @('resume', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\Serial_to_Google_Doc_topdown', '--approval-policy', 'never', '--sandbox', 'danger-full-access', '--loop', '019e22c8-60de-7843-ad6e-813ef4f6521e', 'go on, take a look on handoffs')
$redirectToLog = $false
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\resume_1_019e22c8.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
