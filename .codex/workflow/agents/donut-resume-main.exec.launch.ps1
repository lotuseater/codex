$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: donut-resume-main'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\DonutGame'
$codexArgs = @('resume', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\DonutGame', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', '--loop', '019e2258-f074-733b-a781-c2cb1ada1c47', 'go on, take a look on handoffs')
$redirectToLog = $false
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\donut-resume-main.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
