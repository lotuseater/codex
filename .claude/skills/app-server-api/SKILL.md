---
name: app-server-api
description: Use when adding or changing app-server v2 API surface — new RPC methods, *Params/*Response/*Notification types, discriminated unions, experimental gating, or TypeScript bindings regeneration.
---

# App-server API Checklist

## Core rules

- **v2 only** — do not add new API surface to v1.
- **Payload naming:** `*Params` (client→server request), `*Response` (server→client response), `*Notification` (server-pushed event).
- **Route naming:** `<resource>/<method>` with singular resource — e.g. `thread/read`, `app/list`.
- **Wire format:** camelCase via `#[serde(rename_all = "camelCase")]` on every v2 type.
  - Exception: config RPC payloads mirror `config.toml` keys → snake_case (see config read/write/list in `v2.rs`).
- **TS export namespace:** every v2 request/response/notification type needs `#[ts(export_to = "v2/")]`.

## Field-level rules

- **Never** use `#[serde(skip_serializing_if = "Option::is_none")]` on v2 payload fields.
  - Sole exception: intentionally-no-params requests may use:
    ```rust
    #[ts(type = "undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<()>,
    ```
- **Keep Rust/TS renames aligned:** if a field/variant has `#[serde(rename = "...")]`, also add `#[ts(rename = "...")]`.
- **Discriminated unions:** tag in both serializers — `#[serde(tag = "type", ...)]` and `#[ts(tag = "type", ...)]`.
- **IDs:** plain `String` at the boundary; do UUID parsing internally.
- **Timestamps:** `i64` Unix seconds, named `*_at` (e.g. `created_at`, `updated_at`).

## Client→server `*Params` specifics

- Every optional field: `#[ts(optional = nullable)]`. Use this annotation ONLY on `*Params` types.
- Optional collections (`Vec<…>`, `HashMap<…>`): use `Option<…>` + `#[ts(optional = nullable)]`; do NOT use `#[serde(default)]`.
- Omission-means-false booleans:
  ```rust
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub my_flag: bool,
  ```
  Prefer this over `Option<bool>` when omission should mean `false`.
- **Cursor pagination** (required for all new list methods):
  ```rust
  // Params
  pub cursor: Option<String>,
  pub limit:  Option<u32>,
  // Response
  pub data:        Vec<MyItem>,
  pub next_cursor: Option<String>,
  ```

## Experimental API surface

- Type-level gating: add `#[experimental("method/or/type")]`.
- Field-level gating: derive `ExperimentalApi`.
- Partial method gating: set `inspect_params: true` in `common.rs` for that method.

## Files to touch

| File | When |
|------|------|
| `app-server-protocol/src/protocol/common.rs` | Shared types, method registry, experimental flags |
| `app-server-protocol/src/protocol/v2.rs` | New v2 types and their serde/ts annotations |
| `app-server/README.md` | Always — document the new or changed API behaviour |

## Workflow (run after edits)

```sh
# Regenerate TS/JSON schema fixtures
just write-app-server-schema
# Also run with --experimental when experimental fixtures are affected
just write-app-server-schema --experimental

# Validate protocol tests
just test -p codex-app-server-protocol
```

> Do NOT write boilerplate tests that only assert experimental-field markers on individual
> `common.rs` request fields; rely on schema generation and behavioural coverage instead.
