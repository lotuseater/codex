# Manifests slice — resolved by ORCHESTRATOR (not a resolver agent)

Merge: upstream/main 51b3cd51f6 -> fork claude-automation-toolkit. Resolved 2026-06-10.

## codex-rs/Cargo.toml
- workspace members: UNION — kept fork `codex-mcp-elicitation-api = { path = "mcp/elicitation-api" }`
  AND upstream `codex-mcp-extension = { path = "ext/mcp" }` (both dirs verified present).
- version skews (take-higher / upstream): `starlark = { version = "0.14.2", default-features = false }`
  (upstream, higher); `strum = "0.28"` (HEAD higher than upstream 0.27.2 — superset); `strum_macros = "0.28.0"`;
  `supports-color = "3.0.2"`; `v8 = "=149.2.0"` (upstream higher exact pin); `vt100 = "0.16.2"`;
  `walkdir = "2.5.0"`; `webbrowser = "1.0"`.
  WATCH in Phase 3: starlark `default-features = false` (upstream) may drop a feature the fork's
  starlark usage needs -> if a build error mentions a missing starlark feature, add it back.

## codex-rs/core/Cargo.toml (dev-dependencies)
- UNION — kept fork `codex-core-test-runtime` AND upstream `codex-image-generation-extension`,
  `codex-web-search-extension`, `core_test_support`.

## codex-rs/app-server-protocol/schema/json/v2/ConfigRequirementsReadResponse.json
- UNION of all three properties (fork `allowedApprovalsReviewers` + `allowedPermissions`,
  upstream `allowedPermissionProfiles`), reconstructed as valid JSON. FLAGGED FOR REGEN:
  regen-all.ps1 / `just write-app-server-schema` produces the authoritative version from the
  merged `config/src/config_requirements.rs` (resolved in the proto-config slice).

## codex-rs/Cargo.toml — icu family (added during Cargo.lock reconciliation)
- FORK pinned `icu_decimal`/`icu_locale_core`/`icu_provider = "~2.1"` (tilde, caps <2.2);
  UPSTREAM relaxed to `"2.1"` (caret, allows 2.2.x). Took UPSTREAM (caret) on all three.
  Reason: we took upstream `v8 = "=149.2.0"`, whose chain (temporal_capi 0.2.3 ->
  icu_calendar 2.2.1 -> icu_locale_core ^2.2.0) REQUIRES icu_locale_core 2.2.0. The fork's
  tilde `~2.1` forbade 2.2.0 -> graph was UNSATISFIABLE (no lock could resolve). Caret 2.1
  unifies cleanly (2.2.0 satisfies both protocol's ^2.1 and icu_calendar's ^2.2.0).

## codex-rs/Cargo.lock — RECONCILED (2026-06-10)
- Base = upstream stage 3 (0 markers). Reconciled against merged toml from codex-rs/:
  `cargo update -p icu_locale_core --precise 2.2.0` (online) -> `cargo metadata` exit 0.
- Result: 1379 packages = upstream 1333 + 46 (fork path crates + transitive). Pins verified:
  icu_locale_core 2.2.0, v8 149.2.0, rmcp 1.7.0, reqwest 0.12.28 (+0.13.4 = upstream baseline,
  NOT new trap), whoami 2.1.2. 0 markers. Root reqwest line keeps default-features=false +
  rustls-tls. COMMIT-READY.

STATUS: manifests marker-zero, Cargo.lock reconciled & consistent. Ready for merge commit.
