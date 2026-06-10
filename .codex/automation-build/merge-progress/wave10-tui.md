# Wave 10 build-fix progress: codex-tui (6 errors)

State: STARTED. B1 self-assert OK (repo root = C:/Users/Oleh/Documents/GitHub/open_ai/codex).

Owned sites:
1+2. tui/src/app/loaded_threads.rs:100,109 E0308 (SubAgentSource wrong crate)
3. tui/src/debug_config.rs:71 E0599 (can_set_permission_profile gone)
4. tui/src/app/agent_status_feed.rs:162 E0004 (CollabAgentTool CompactAgent/RestartAgent)
5. tui/src/app/thread_routing.rs:1623 E0004 (ServerNotificationThreadTarget::AppScoped)
6. tui/src/multi_agents/activity.rs:73 E0004 (ThreadItem::SubAgentActivity)

Progress log:
- [init] Read log errors via grep; about to read enum defs + call sites.
- [triage] Upstream tui has NONE of agent_status_feed.rs / thread_routing.rs / multi_agents/activity.rs (fork-only files; `git show upstream/main:...` empty). Permissions successor found in-fork: permissions_menu.rs:131 comment — check moved to `permissions.permission_profile.can_set(&profile)` (Constrained<PermissionProfile>::can_set). collab_tool_name in activity.rs ALREADY covers CompactAgent/RestartAgent; only the ThreadItem match needed an arm.
- [edits applied, all 5 files]
  1+2. loaded_threads.rs: import swapped to codex_app_server_protocol::SubAgentSource (v2). v2 ThreadSpawn fields: parent_thread_id is String (was ThreadId), agent_path is Option<String> (was Option<AgentPath>). Adapted: agent_path.clone() (drop .map(String::from)); parent id via ThreadId::from_string(parent_thread_id).ok(). Tests construct sources via serde_json so unaffected.
  3. debug_config.rs: permissions.permission_profile.can_set(&permission_profile).is_ok() + comment citing permissions_menu.rs precedent.
  4. agent_status_feed.rs: CompactAgent => "Compacted an agent", RestartAgent => "Restarted an agent" (mirrors neighbor arm style).
  5. thread_routing.rs notification_thread_id: AppScoped added to the None group (matches app_server_events.rs:143 precedent — app-scoped = no thread target).
  6. activity.rs: added SubAgentActivity arm mirroring fork's own agent_status_feed labels (Started/Contacted/Interrupted a sub-agent + agent_path preview); imported SubAgentActivityKind. Chose rich arm over None because this pane summarizes activity and the feed already surfaces this item type.
- [verify] cargo check -p codex-tui --release (from codex-rs/, foreground): EXITCODE=0, "Finished release profile in 9m 21s", 35 warnings (all pre-existing dead-code/style, none introduced by these edits). git diff --stat confirms exactly the 5 owned files changed (+22/-4). All edits UNSTAGED. DONE.
