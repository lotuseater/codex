# Slice other-tests-cts1 progress

Guards: toplevel_ok=true, unmerged_seen=true. All 9 files unmerged (stages 1/2/3).

## Files
- [x] codex-rs/core-plugins/src/manager_tests.rs DONE-but-FLAG (2 huge conflict regions both pure upstream ADD with empty HEAD side -> kept all upstream-new tests). FLAG files_uncertain: upstream-new tests use codex_app_server_protocol::{PluginInstallPolicy,PluginAuthPolicy,PluginInterface} (5 refs) by fully-qualified path; codex-app-server-protocol NOT a dep of core-plugins/Cargo.toml -> won't compile until dep added (Cargo.toml owner/later wave).

## ALL 9 FILES: 0 conflict markers. markers_remaining=0. status=success.
- [x] agents/agents_md.rs DONE (union; kept fork core_test_runtime imports + fork-local create_directory_symlink; added upstream constants/ForkSnapshot/anyhow; translated upstream core_test_support imports to runtime crate)
- [x] agents/collaboration_instructions.rs DONE (take-fork for Op::UserTurn body: op.rs authoritative shape is fork flat-fields, NOT upstream thread_settings refactor; dropped upstream thread_settings/responsesapi_client_metadata/additional_context; kept fork imports; removed unused local_selections import)

## KEY FINDING: Op::UserTurn post-merge shape = FORK flat fields (op.rs L181-251). Has environments/context_budget_mode/collaboration_mode/personality; NO thread_settings/responsesapi_client_metadata/additional_context. Non-conflicted sibling items_message_events.rs confirms. => take-fork on all UserTurn body conflicts.
- [x] agents/subagent_notifications.rs DONE (union imports: kept fork runtime imports + added local_selections; only 1 conflict region = imports; body Op::UserInput uses thread_settings which IS authoritative post-merge shape per op.rs L62)

## KEY FINDING 2: Op::UserInput post-merge shape = UPSTREAM thread_settings refactor (op.rs L62-79). HAS thread_settings/responsesapi_client_metadata/additional_context/environments/final_output_json_schema. => take-upstream on UserInput body conflicts (opposite of UserTurn!).
- [x] agents/tool_parallelism.rs DONE (take-fork UserTurn x2; removed stray core_test_support local_selections import line 4)
- [x] suite/json_result.rs DONE (take-fork UserTurn; dropped upstream import dups + local_selections)
- [x] suite/sqlite_state.rs DONE-but-FLAG (take-fork UserTurn; kept upstream-new imports Config/ExtensionRegistryBuilder/install_web_search_extension used in real test body L434-435; removed unused local_selections). FLAG files_uncertain: needs codex-extension-api + codex-web-search-extension added to core-test-suites/agents-thread-store/Cargo.toml (NOT in manifest -> won't compile until dep added by Cargo.toml owner/later wave).
- [x] suite/truncation.rs DONE (take-fork UserTurn; dropped upstream import dups + unused TempDirExt/local_selections)
- [x] approvals-permissions/request_permissions_tool.rs DONE (take-fork UserTurn; dropped upstream import dups + unused local_selections)

## Notes
Tests -> committable union. Flag structurally broken under files_uncertain.
