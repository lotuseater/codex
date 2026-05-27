$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: final_build_deploy_wave2'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$promptPath = 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\final_build_deploy_wave2.prompt.md'
$logPath = 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\final_build_deploy_wave2.log.txt'
$exitPath = 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\final_build_deploy_wave2.exit.txt'
$prompt = Get-Content -Raw -LiteralPath $promptPath
$codexArgs = @(
    '-c', 'model=gpt-5.5',
    '-c', 'model_reasoning_effort=xhigh',
    '--cd', 'C:\Users\Oleh\Documents\GitHub\open_ai\codex',
    '--ask-for-approval', 'never',
    '--sandbox', 'danger-full-access',
    'exec',
    $prompt
)
& 'codex' @codexArgs *>&1 | Tee-Object -FilePath $logPath
$exitCode = if ($null -ne $LASTEXITCODE) { $LASTEXITCODE } else { 0 }
@("exit_code=$exitCode", "finished=$(Get-Date -Format o)") | Set-Content -LiteralPath $exitPath
exit $exitCode
