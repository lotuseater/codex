# core-tests-2 merge progress

Guards: toplevel_ok=true, unmerged_seen=true (all 12 files stages 1/2/3).

## Files (12)
- [x] apply_patch_cli.rs DONE structural take-fork. Fork reduced this to a HELPERS-ONLY module (143 lines: submit_without_wait[_with_turn_permissions], restrictive_workspace_write_profile, workspace_write_with_read_only_root, workspace_write_with_unreadable_path, create_file_symlink cfg-variants). Consuming files (apply_patch_output_aggregation/formatting/safety_paths/success, exec_apply_patch) include via #[path] mod and import only those 5 symbols. Upstream's 217-1887 block (all #[tokio::test] fns + mount_apply_patch helpers) DROPPED - belongs in standalone test files (test-repair wave). FLAG uncertain: upstream NEW apply-patch test cases may need porting.
- [x] image_rollout.rs DONE take-fork all 3 (Op::UserTurn x2 + empty import block; fork already imports codex_core_test_runtime::responses module so responses::mount_sse_once OK)
- [x] pending_input.rs DONE take-fork (Op::UserTurn), removed dead local_selections import
- [x] personality.rs DONE take-fork both (Op::UserTurn with personality field preserved; empty import block)
- [x] mcp_turn_metadata.rs DONE take-upstream imports
- [x] model_switching.rs DONE take-fork (Op::UserTurn flat) both conflicts
- [x] models_cache_ttl.rs DONE take-fork (Op::UserTurn), removed dead local_selections import
- [x] models_etag_responses.rs DONE take-fork (Op::UserTurn), removed dead local_selections import
- [x] override_updates.rs DONE take-fork (Op::OverrideTurnContext + codex_core_test_runtime imports)
- [x] prompt_caching.rs DONE take-fork structural (8-line module-decls; fork split into prompt_caching_{tools,prefixes,turn_overrides,environment_context}.rs - all 4 exist)
- [x] remote_env.rs DONE take-fork stage2 ENTIRELY (652 lines). Fork extracted imports/helpers to remote_env/support.rs (exists, clean) + uses support::*. Auto-merge tail was CORRUPTED (had upstream local() calls not provided by support.rs). UNCERTAIN: upstream added NEW tests (remote_test_env_can_connect_and_use_filesystem, remote_test_env_sandboxed_read_*, remote_test_env_remove/copy_*) DROPPED - port in test-repair wave.
- [x] remote_models.rs DONE take-fork stage2 ENTIRELY (1260 lines). Fork extracted to remote_models/support.rs + support::*. Same 17 test fns as upstream (no upstream-new lost). Uses thread_settings{cwd:Some} (fork form). UNCERTAIN: depends on protocol.rs ThreadSettingsOverrides resolution (cwd vs environments - still conflicted in protocol.rs, another resolver).

## CROSS-FILE UNRESOLVED DEPENDENCY (flag uncertain)
- protocol.rs ThreadSettingsOverrides is STILL conflicted (not my slice): fork keeps `cwd: Option<AbsolutePathBuf>`, upstream replaces with `environments: Option<TurnEnvironmentSelections>`. Files using thread_settings depend on its resolution:
  * mcp_turn_metadata.rs -> auto-merged to UPSTREAM body (environments+local_selections), I took upstream imports -> internally consistent, needs ThreadSettingsOverrides.environments.
  * remote_models.rs -> fork body (cwd) -> needs ThreadSettingsOverrides.cwd.
  These two are mutually inconsistent until protocol.rs resolves; test-repair wave must align.
- Files using Op::UserTurn (flat) are SAFE regardless: op.rs confirmed Op::UserTurn retains all flat fields incl context_budget_mode/personality (models_cache_ttl, models_etag_responses, model_switching, pending_input, personality, image_rollout, apply_patch_cli helper).

ALL 12 FILES: 0 conflict markers, git diff --check clean (remote_env/remote_models CRLF-normalized).

## Gotchas
- KEY PATTERN: upstream refactored Op::UserTurn (flat fields) -> Op::UserInput (nested thread_settings:ThreadSettingsOverrides + responsesapi_client_metadata + additional_context). Fork RETAINS Op::UserTurn (still valid in merged op.rs with context_budget_mode/personality/collaboration_mode flat). Merge auto-merged the unconflicted prefix to fork's `Op::UserTurn{environments:None,...}` -> so MUST take FORK body for these (the upstream nested form is incompatible with the prefix). take-fork preserves fork features and loses no upstream test logic (same assertions, just call-shape).
- CRATE RENAME: fork renamed core_test_support -> codex_core_test_runtime (crate at test-support/core-runtime). BOTH crates are deps of codex-core (core_test_support still valid too). Fork already migrated most imports. Upstream import blocks add core_test_support::local_selections etc - when taking fork body, drop the now-unused local_selections.
- Strategy: union committable, keep upstream NEW cases + fork structural split.
- personality.rs exercises fork feature; preserve assertions.
