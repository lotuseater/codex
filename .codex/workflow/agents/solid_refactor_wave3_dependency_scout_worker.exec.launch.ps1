$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = 'Codex worker: solid_refactor_wave3_dependency_scout_worker'
Set-Location -LiteralPath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
Remove-Item Env:\CODEX_THREAD_ID, Env:\CODEX_SHELL, Env:\CODEX_INTERNAL_ORIGINATOR_OVERRIDE -ErrorAction SilentlyContinue
$codexArgs = @([System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('LWM=')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('bW9kZWw9Z3B0LTUuNQ==')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('LWM=')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('bW9kZWxfcmVhc29uaW5nX2VmZm9ydD14aGlnaA==')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('LS1jZA==')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('QzpcVXNlcnNcT2xlaFxEb2N1bWVudHNcR2l0SHViXG9wZW5fYWlcY29kZXg=')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('LS1hc2stZm9yLWFwcHJvdmFs')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('bmV2ZXI=')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('LS1zYW5kYm94')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('ZGFuZ2VyLWZ1bGwtYWNjZXNz')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('ZXhlYw==')), [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('LmNvZGV4XHdvcmtmbG93XGFnZW50c1xwdGVfdjRfMDJfbGFiX2RlZmF1bHRzLnByb21wdC5tZA==')))
$redirectToLog = $true
if ($redirectToLog) {
    & 'codex' @codexArgs *>&1 | Tee-Object -FilePath 'C:\Users\Oleh\Documents\GitHub\open_ai\codex\.codex\workflow\agents\solid_refactor_wave3_dependency_scout_worker.exec.visible.log'
} else {
    & 'codex' @codexArgs
}
