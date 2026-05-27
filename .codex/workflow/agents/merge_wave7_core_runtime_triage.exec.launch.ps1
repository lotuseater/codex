$ErrorActionPreference = 'Stop'
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
Set-Location -LiteralPath $Repo
$prompt = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'merge_wave7_core_runtime_triage.prompt.md') -Raw
$codexArgs = @('-c','model=gpt-5.5','-c','model_reasoning_effort=xhigh','--cd',$Repo,'--ask-for-approval','never','--sandbox','danger-full-access','exec',$prompt)
& 'codex' @codexArgs *>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot 'merge_wave7_core_runtime_triage.exec.log')
