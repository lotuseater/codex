$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: codex-resume-a'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$codexArgs = @('-c', 'model=gpt-5.3-codex', '-c', 'model_reasoning_effort=high', '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex', '--ask-for-approval', 'never', '--sandbox', 'danger-full-access', 'resume', '--loop', '019e2915-84fc-7002-b4f6-21d0a35efb0d', 'go on, take a look on handoffs')
& 'codex' @codexArgs
