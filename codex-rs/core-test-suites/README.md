# Core Test Suites

This directory is reserved for per-topic core test-suite crates. Topic workers
can split tests into these crates without also negotiating workspace membership.

- Keep one test topic per crate so each suite has a single responsibility.
- Do not add a broad `codex-core` dev-dependency unless the suite genuinely
  instantiates the runtime.
- Prefer `codex-core-test-runtime` only for runtime end-to-end tests.
- Use lighter test-support crates for fixtures, builders, protocol payloads, and
  domain data that do not need the full core runtime.
- Do not move tests into this directory from scaffold work; topic workers own
  the test moves and crate-specific dependency choices.
