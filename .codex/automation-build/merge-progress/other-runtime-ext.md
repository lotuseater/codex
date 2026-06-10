# Slice: other-runtime-ext — progress

Guards: toplevel_ok=true, unmerged_seen=true (all 7 files stages 1/2/3).

## DONE
- contributors.rs — UNION: kept fork `mod approval_review`+`pub use ApprovalReview*` AND upstream `mod mcp`+`pub use McpServerContribution`. No markers.
- extension-api/lib.rs — UNION: kept fork ExtensionToolExecutor/Future/Output re-exports AND upstream McpServerContribution/McpServerContributor. No markers.
- image-generation/extension.rs — structural/take-fork: PRESERVED fork Feature::ImageGenExt gating (enabled field, gated From). Return type kept fork `ExtensionToolExecutor` (surviving fork adapter design in tools.rs stage-0). Removed unused ToolCall/ToolExecutor imports. Doc restored "feature-gated".
- image-generation/tool.rs — take-fork: surviving fork ToolExecutor trait (tool-execution-api/src/lib.rs:486) uses spec()->Option<ToolSpec> + RPITIT handle. Took fork spec()/handle() blocks; renamed call request_for_action -> request_for_args to match surviving (upstream-renamed) helper def. type Output=Box<dyn ToolOutput> matches ExtensionToolOutput.

## KEY ARCHITECTURE FACT
Fork's `ExtensionToolExecutor` object-safe adapter (contributors/tools.rs, stage-0/not conflicted) SURVIVED. ToolContributor::tools() returns Vec<Arc<dyn ExtensionToolExecutor>>. Fork ToolExecutor trait: spec()->Option<ToolSpec>, handle RPITIT. Upstream's async-fn/ToolSpec-direct shape did NOT survive -> take fork side on ext tool impls.

- web-search/output.rs — take-upstream: fork `:2`==base `:1` (EncryptedSearchOutput); upstream REFACTORED standalone search to plaintext SearchOutput + added contains_external_context. NO fork feature here. Took fork-side of the to_response_item conflict (matches surviving codex_extension_api::ToolOutput trait = &dyn ToolOutputPayload, has post_tool_use_* hooks, NO contains_external_context) and dropped upstream contains_external_context (targets non-surviving codex_tools::ToolOutput). Struct/test auto-merged to upstream SearchOutput. No markers.
- web-search/tool.rs — take-upstream return + keep fork RPITIT handle: surviving import is SearchOutput; SearchResponse has both encrypted_output:Option<String> AND output:String. Used `Ok(Box::new(SearchOutput::new(response.output)) as Box<dyn ToolOutput>)` inside fork's RPITIT async-move block. No markers.

- features/src/lib.rs — UNION: inline enum-variant conflict (NOT the trailing fork-local block). Kept BOTH upstream TerminalVisualizationInstructions (first) + fork ApplyPatchFreeform. Both already had non-conflicted metadata-table entries (lines ~880/1128) + no dups. Fork-local block at 266-281 untouched. No markers.

## ALL 7 FILES DONE — 0 markers (git diff --check clean; only a harmless LF->CRLF warning on output.rs).
HANDOFF: success.
