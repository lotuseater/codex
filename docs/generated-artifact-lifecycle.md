# Generated Artifact Lifecycle

This note collects the repo surfaces where source changes intentionally produce
generated files. Use it as a local checklist before opening or reviewing changes
that touch schemas, protocol contracts, snapshots, SDK outputs, or lockfiles.

## Source-To-Artifact Map

| Surface | Source of truth | Generated artifacts | Regeneration command |
| --- | --- | --- | --- |
| Config schema | `codex-rs/core/src/config/` | `codex-rs/core/config.schema.json` | `just write-config-schema` |
| App-server v2 API schema | `codex-rs/app-server-protocol/src/protocol/` | app-server schema fixtures and exported TS/JSON schema | `just write-app-server-schema` |
| Experimental app-server API schema | experimental v2 protocol fields and methods | experimental schema fixtures | `just write-app-server-schema --experimental` |
| Hooks schema | `codex-rs/hooks/` | hook schema fixtures | `just write-hooks-schema` |
| TUI snapshots | TUI rendering code and tests | `*.snap` files under TUI snapshot dirs | `cargo test -p codex-tui`, then `cargo insta pending-snapshots -p codex-tui` and `cargo insta accept -p codex-tui` when intentional |
| Rust dependencies | `codex-rs/Cargo.toml` files and `Cargo.lock` | `MODULE.bazel.lock` | `just bazel-lock-update`, then `just bazel-lock-check` |

## Review Checklist

- Regenerate only the artifact that matches the source change.
- Include source and generated artifact changes in the same commit.
- For app-server v2 API changes, update `codex-rs/app-server/README.md` when
  client-visible behavior changes.
- For experimental app-server changes, mark the method or field with
  `#[experimental("...")]` and regenerate both stable and experimental schema
  fixtures. Stable fixtures should omit experimental surface area.
- For TUI output changes, inspect pending snapshots before accepting them.
- For dependency changes, include every required lockfile in the same commit and
  avoid broad dependency updates unrelated to the task.
- Do not update generated artifacts as cleanup if the source of truth did not
  change.

## Local Verification

Prefer the smallest verification lane that covers the changed surface:

- Config schema: `cargo test -p codex-core config` plus
  `just write-config-schema`.
- App-server protocol: `cargo test -p codex-app-server-protocol` plus
  `just write-app-server-schema` and, when needed,
  `just write-app-server-schema --experimental`.
- TUI snapshots: `cargo test -p codex-tui` or a narrower TUI test filter first,
  then snapshot inspection.
- Dependencies: `cargo check -p <changed-crate>` and Bazel lock checks after
  the lock update.

When local commands are unavailable on the current platform, record the exact
command that could not run and the platform/tooling blocker.
