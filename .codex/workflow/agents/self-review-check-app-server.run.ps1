$repo = 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$runner = Join-Path $repo '.codex\workflow\agents\run-targeted-check-worker.ps1'
& $runner -Name 'self-review-check-app-server' -Repo $repo 'cargo test -p codex-app-server --test all review'
exit $LASTEXITCODE
