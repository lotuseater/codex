$repo = 'C:\Users\Oleh\Documents\GitHub\open_ai\codex'
$runner = Join-Path $repo '.codex\workflow\agents\run-targeted-check-worker.ps1'
& $runner -Name 'self-review-check-core-protocol' -Repo $repo 'cargo test -p codex-core tasks::review' 'cargo test -p codex-app-server-protocol --test schema_fixtures'
exit $LASTEXITCODE
