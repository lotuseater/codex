# core-tests-3 merge progress — COMPLETE

Guards: toplevel_ok=true, unmerged_seen=true.

## Key finding
Merged protocol `Op::UserTurn` (codex-rs/protocol/src/protocol/op.rs, already resolved by
another slice, no markers) keeps the FORK FLAT shape. Upstream's
`thread_settings: ThreadSettingsOverrides` + `responsesapi_client_metadata`/`additional_context`
shape was NOT adopted in the protocol. => For every UserTurn conflict, TAKE HEAD (fork flat shape).
Upstream's duplicate `core_test_support::*` import blocks (incl. `local_selections`) are the OLD crate
name; the fork renamed it to `codex_core_test_runtime` and already imports everything => drop the
upstream block. `local_selections`/`core_test_support` end up with 0 refs (verified).

## DONE (all 11, markers=0)
- request_user_input.rs       — union->take HEAD x3 (UserTurn flat) + removed unused upstream
                                `use core_test_support::test_codex::local_selections;` (auto-merged outside markers).
- responses_api_proxy_headers.rs — take HEAD x2 (import block empty + UserTurn flat).
- review.rs                   — structural: fork split into review_{git_context,history,model_selection,outputs}.
                                Submodules exist & cover ALL upstream test fns => took HEAD module shell.
- rmcp_client.rs              — structural: fork compatibility shim (content in rmcp_client_{connection,
                                responses,streamable_http,tool_calls,support}.rs which exist) => took HEAD shim.
- safety_check_downgrade.rs   — take HEAD x2 (import block + UserTurn flat).
- search_tool.rs              — structural: fork include!(core-test-suites/tools-router/.../search_tool.rs).
                                Target exists. NOTE: relocated suite has a DIFFERENT/older test-fn set than
                                upstream's monolith (missing ~9 upstream tool_search_* cases). Relocated file
                                is OUTSIDE my slice. FLAGGED uncertain — test-repair wave must reconcile.
- skill_approval.rs           — take HEAD x2; fork skill assertions preserved.
- skills.rs                   — take HEAD x2; fork skill assertions preserved.
- tool_harness.rs             — structural: fork include!(...tools-router/.../tool_harness.rs). Target exists,
                                fully covers upstream test fns => took HEAD include.
- tools.rs                    — structural: fork include!(...tools-router/.../tools.rs). Target exists,
                                fully covers upstream test fns => took HEAD include.
- view_image.rs               — structural: fork split into view_image_{user_turn,tool_local,tool_remote,
                                tool_errors}.rs (all exist; cover all upstream test cases) => took HEAD shell.

MARKERS_REMAINING = 0. Verified via git diff --check (only LF/CRLF warnings) + grep scan.
