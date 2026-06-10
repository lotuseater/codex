# core-runtime-B merge progress  — COMPLETE (markers_remaining=0)

## DONE (all zero markers, git diff --check clean, brace+paren balanced)
- state/turn.rs: union imports (kept TurnInput fork + AgentExecutionGuard upstream). strategy=union.
- tasks/mod.rs: kept fork `tasks.is_empty()` (multi-task IndexMap), added upstream agent_execution_guard
  computation + RunningTask `_agent_execution_guard` field assignment + upstream `turn_extension_data`
  local. strategy=union. task-memory logic untouched (not in conflict regions).
- guardian/review_session.rs: FORK FEATURE. Kept fork Op::UserTurn submission (take-fork) to preserve
  per-turn context_budget_mode — upstream's Op::UserInput/ThreadSettingsOverrides path has NO
  context_budget_mode field (verified protocol.rs both stages). Dropped upstream's guardian_permission_profile
  + parent_turn_* locals (only consumed by the dropped UserInput call). Added // fork-local: note.
- config/mod.rs: STRUCTURAL. The fork extracted Config/MultiAgentV2Config/ThreadStoreConfig/ConfigBuilder/
  impl Config methods to submodules (config_struct.rs, config_types.rs, builder.rs, config_loaders.rs,
  config_accessors.rs, config_transforms.rs). Two large upstream inline blocks (the monolithic defs +
  impl Config) were taken as EMPTY fork side (they duplicate fork submodules — verified every method
  exists in a fork submodule). Removed a stray `}` (upstream impl Config close) and restored the fork
  fn's own `}` for uses_deprecated_instructions_file. Small conflicts:
    * import line: union (RepoContextScout* fork + ExtraConfig upstream re-export).
    * usage-hint consts: dropped upstream inline consts (collide with fork re-export at lines 175-176,
      and consumed only by the dropped inline MultiAgentV2Config::default()).
    * 3 requirements-profile fns (resolve_default_permissions, validate_required_permission_profile_catalog,
      requirements_force_profile_selection field): took FORK side — the fork deliberately DISABLED these
      (underscored params `_requirements_toml`/`_startup_warnings`/`_available_permissions`, stub bodies,
      hardcoded `false`). Upstream's bodies would not compile against the fork's underscored signatures.
    * Removed upstream-orphan helpers implicit_default_permissions + is_permission_allowed (no callers
      after taking fork side; absent from fork stage-2) and their now-unused imports
      BUILT_IN_READ_ONLY_PROFILE + BUILT_IN_WORKSPACE_PROFILE.

## UNCERTAIN / for later waves
- config_tests.rs (UU, NOT my file) — upstream side calls `resolve_profile_v2_config_path` and uses
  `CodeModeConfig`, both of which I removed from mod.rs per the fork's structural extraction (the fork
  never had them). The config_tests resolver / test-repair wave must drop those upstream test cases or
  the test compile will fail. This is expected per the brief's test-repair-wave note.
- ConfigToml/Config shape unchanged by me, but config schema regen is handled separately by orchestrator.

## GOTCHAS
- protocol.rs / protocol/op.rs NOT mine. Op::UserTurn AND Op::UserInput both exist in merged protocol;
  UserTurn keeps context_budget_mode, ThreadSettingsOverrides does NOT.
