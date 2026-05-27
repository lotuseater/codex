$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$prompt = Get-Content -LiteralPath '.codex\workflow\agents\merge_wave5_app_protocol.prompt.md' -Raw
$codexArgs = @('-c','model=gpt-5.5','-c','model_reasoning_effort=xhigh','--cd','C:\Users\Oleh\Documents\GitHub\open_ai\codex','--ask-for-approval','never','--sandbox','danger-full-access','exec',$prompt)
& 'codex' @codexArgs *>&1 | Tee-Object -FilePath '.codex\workflow\agents\merge_wave5_app_protocol.exec.visible.log'
