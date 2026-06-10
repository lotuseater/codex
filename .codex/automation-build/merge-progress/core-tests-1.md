# core-tests-1 progress — COMPLETE (markers=0)

Slice: core-tests-1 (test files). B1 guards PASS (toplevel ok, all 5 unmerged stages 1/2/3).

## Key fact
- protocol op.rs `Op::UserTurn` is already RESOLVED (0 markers) and uses the FLAT fork shape.
  It does NOT use upstream's ThreadSettingsOverrides/additional_context for UserTurn. =>
  test call-sites must use the flat fork shape => take FORK side for Op::UserTurn call-sites.

## Files done (all 0 markers)
- [x] codex-rs/core/tests/common/test_codex.rs — take-fork: Op::UserTurn call-site uses flat fork
      fields (incl context_budget_mode). Removed now-dead `turn_environment_selections` local.
      TurnEnvironmentSelections import still used (local_selections ~line 83). uncertainty=low
- [x] codex-rs/core/src/thread_manager_tests.rs — take-fork import union: body uses fork-structured
      symbols (codex_thread_store / codex_thread_store_api / lightweight+responses test support,
      UserInput). Both crate sets are core dev-deps. uncertainty=low
- [x] codex-rs/core/tests/responses_headers.rs — STRUCTURAL: fork split monolith into 3 top-level
      test bins + suite/ submodules. Wrote fork stage-2 (mod decls only). All 4 upstream tests already
      present in fork suite files (subagent/model_info/turn_metadata), functionally identical. uncertainty=low
- [x] codex-rs/core/src/config/config_loader_tests.rs — STRUCTURAL: fork split into config_loader_tests/*.rs
      submodules. Wrote fork stage-2 (imports + 11 mod decls). Fixed preamble import to fork's
      BUILT_IN_PERMISSION_PROFILE_READ_ONLY (was DANGER_FULL_ACCESS from merged preamble; submodules
      use READ_ONLY via super::*). uncertainty=MED (see dropped tests below)
- [x] codex-rs/core/src/config/config_tests.rs — STRUCTURAL: fork split into config_tests/*.rs submodules.
      Wrote fork stage-2 (imports + ~28 mod decls). Preamble identical fork-vs-merged. uncertainty=MED.

## FLAG for later wave (files_uncertain) — fork split DROPPED tests present in base+upstream
These are NOT conflict markers; they are coverage losses from the fork's monolith->submodule split.
The submodule files (config_loader_tests/*.rs, config_tests/*.rs) are NOT in my slice, so re-homing
the dropped test bodies into the correct submodules is a later test-repair wave.

config_loader_tests.rs — 16 dropped:
  upstream-NEW (7): system_allowed_permission_profiles_{fall_back_from_disallowed_danger_full_access,
    fall_back_from_disallowed_workspace, require_managed_default, select_managed_default_without_local_default,
    standard_pair_defaults_to_workspace}, system_managed_default_{must_be_allowed, requires_allowed_permission_profiles}
  base-present (9): cloud_config_bundle_are_not_overwritten_by_system_requirements,
    cloud_config_bundle_requirement_source, load_config_layers_fails_when_cloud_config_bundle_loader_fails,
    load_config_layers_includes_cloud_config_bundle, load_config_layers_inserts_cloud_config_between_system_and_user,
    load_config_layers_resolves_relative_bundle_requirements_paths_against_codex_home, load_single_requirements_toml,
    mdm_requirements_take_precedence_over_cloud_config_bundle, strict_config_rejects_unknown_cloud_config_key

config_tests.rs — 18 dropped (all present in base AND upstream, none upstream-only-new):
  catalog_v2_allows_agents_max_threads_when_feature_disabled,
  empty_config_defaults_to_builtin_profile_for_untrusted_project, load_config_resolves_code_mode_config,
  load_config_resolves_experimental_request_user_input_enabled,
  memory_tool_makes_memories_root_readable_without_creating_or_widening_writes,
  multi_agent_v2_empty_usage_hint_overrides_clear_default_hints, multi_agent_v2_feature_rejects_agents_max_threads,
  permission_profile_override_keeps_memories_root_out_of_legacy_projection,
  profile_approvals_reviewer_falls_back_when_disallowed_by_requirements,
  test_set_project_trusted_converts_inline_to_explicit,
  test_set_project_trusted_migrates_top_level_inline_projects_preserving_entries,
  test_set_project_trusted_writes_explicit_tables, to_mcp_config_applies_plugin_mcp_cloud_config_bundle,
  to_mcp_config_flows_mcp_tool_prefix_from_feature, tools_experimental_request_user_input_can_be_disabled,
  tools_experimental_request_user_input_defaults_to_enabled, windows_sandbox_mode_falls_back_when_disallowed_by_requirements,
  workspace_write_includes_configured_writable_root_once_without_memories_root

## Done criteria: all 5 files git diff --check clean, 0 markers. HANDOFF=success.
