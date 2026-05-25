$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: serial-resume-main'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\Serial_to_Google_Doc_topdown'
$codexArgs = @('resume', '-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\Serial_to_Google_Doc_topdown', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', '--loop', '019e2251-bf22-71e2-9fe1-e4dd35de6304', 'go on, take a look on handoffs')
$redirectToLog = $false
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\serial-resume-main.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
