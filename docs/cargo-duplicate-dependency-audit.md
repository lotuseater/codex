# Cargo Duplicate Dependency Audit

Generated from `scripts/build-local-codex.ps1 -Mode Diagnose -DuplicateAuditLimit 0` on 2026-05-14 after the quick `quick-xml` dedupe pass.

## Summary

- `duplicate_package_names`: 84
- `action_required_count`: 54
- `known_unavoidable_count`: 30
- Source JSON: `logs/cargo-duplicate-dependency-audit-latest.json`
- Release artifact cleanup is intentionally separate from dependency dedupe: normal cleanup now removes only `target/release/deps/lib*.rlib` and `lib*.rmeta` files that Cargo dep-info no longer references. Same crate-name artifacts can be active feature/target variants and must not be deleted by filename age alone.

## Fast Dedupe Done

- `quick-xml` was the only obvious direct workspace duplicate that was safe to remove in this pass. `codex-rs/Cargo.toml` now uses `quick-xml = "0.39"`, matching the existing `plist -> syntect` transitive `quick-xml 0.39.4`, and `Cargo.lock` no longer contains `quick-xml 0.38.4`.
- A tiny external serde/XML canary passed against `quick-xml 0.39.4` for the exact `from_xml_str` / `to_xml_string` pattern used by `codex-protocol::items`.
- The repo-level `codex-protocol` release test was stopped because it pulled unrelated `reqwest`/`rustls` compilation and was not a fast dedupe verification lane.

## Fast Dedupe Rejected

- `which 6.0.3` is held by the `v8` build dependency, while the workspace direct version is `which 8.0.2`; deduping it needs an upstream `v8` update, not a local manifest tweak.
- `unicode-width 0.1.14` is held by `starlark_syntax` and `unicode-truncate`/`ratatui`, while the workspace direct version is `0.2.2`; deduping it requires upstream/fork movement.
- `base64 0.21.7` is held by `age` / `age-core`, while workspace direct usage is `0.22.1`; not a local-only safe bump.
- `zip 0.6.6` is not visible in the normal/build reverse tree on this Windows target; keep it as action-required until a `--target all` pass identifies the owner.

## Action-Required Duplicates

| Package | Versions |
| --- | --- |
| `base64` | 0.21.7 [registry+https://github.com/rust-lang/crates.io-index]<br>0.22.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `bit-vec` | 0.6.3 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `bitflags` | 1.3.2 [registry+https://github.com/rust-lang/crates.io-index]<br>2.11.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `bzip2` | 0.4.4 [registry+https://github.com/rust-lang/crates.io-index]<br>0.5.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `cfg_aliases` | 0.1.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.2.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `const-oid` | 0.10.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.6 [registry+https://github.com/rust-lang/crates.io-index] |
| `core-foundation` | 0.10.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.4 [registry+https://github.com/rust-lang/crates.io-index] |
| `cpufeatures` | 0.2.17 [registry+https://github.com/rust-lang/crates.io-index]<br>0.3.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `darling` | 0.20.11 [registry+https://github.com/rust-lang/crates.io-index]<br>0.23.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `darling_core` | 0.20.11 [registry+https://github.com/rust-lang/crates.io-index]<br>0.23.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `darling_macro` | 0.20.11 [registry+https://github.com/rust-lang/crates.io-index]<br>0.23.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `fixedbitset` | 0.4.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.5.7 [registry+https://github.com/rust-lang/crates.io-index] |
| `flume` | 0.11.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.12.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `foldhash` | 0.1.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.2.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `hashbrown` | 0.12.3 [registry+https://github.com/rust-lang/crates.io-index]<br>0.14.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.15.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.16.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.17.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `heck` | 0.4.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.5.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `http` | 0.2.12 [registry+https://github.com/rust-lang/crates.io-index]<br>1.4.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `http-body` | 0.4.6 [registry+https://github.com/rust-lang/crates.io-index]<br>1.0.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `indexmap` | 1.9.3 [registry+https://github.com/rust-lang/crates.io-index]<br>2.14.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `itertools` | 0.11.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.12.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.13.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.14.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `jni` | 0.21.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.22.4 [registry+https://github.com/rust-lang/crates.io-index] |
| `jni-sys` | 0.3.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.4.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `linux-raw-sys` | 0.12.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.4.15 [registry+https://github.com/rust-lang/crates.io-index] |
| `matchit` | 0.8.4 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `memoffset` | 0.6.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `nix` | 0.28.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.29.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.30.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.31.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `nom` | 7.1.3 [registry+https://github.com/rust-lang/crates.io-index]<br>8.0.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `petgraph` | 0.6.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.8.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `prost` | 0.12.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.14.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `prost-build` | 0.12.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.14.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `prost-derive` | 0.12.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.14.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `prost-types` | 0.12.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.14.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `r-efi` | 5.3.0 [registry+https://github.com/rust-lang/crates.io-index]<br>6.0.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `redox_syscall` | 0.5.18 [registry+https://github.com/rust-lang/crates.io-index]<br>0.7.5 [registry+https://github.com/rust-lang/crates.io-index] |
| `redox_users` | 0.4.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.5.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `regex-syntax` | 0.6.29 [registry+https://github.com/rust-lang/crates.io-index]<br>0.8.10 [registry+https://github.com/rust-lang/crates.io-index] |
| `rustc-hash` | 1.1.0 [registry+https://github.com/rust-lang/crates.io-index]<br>2.1.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `rustix` | 0.38.44 [registry+https://github.com/rust-lang/crates.io-index]<br>1.1.4 [registry+https://github.com/rust-lang/crates.io-index] |
| `security-framework` | 2.11.1 [registry+https://github.com/rust-lang/crates.io-index]<br>3.7.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `self_cell` | 0.10.3 [registry+https://github.com/rust-lang/crates.io-index]<br>1.2.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `similar` | 2.7.0 [registry+https://github.com/rust-lang/crates.io-index]<br>3.1.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `socket2` | 0.5.10 [registry+https://github.com/rust-lang/crates.io-index]<br>0.6.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `supports-color` | 2.1.0 [registry+https://github.com/rust-lang/crates.io-index]<br>3.0.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `toml` | 0.5.11 [registry+https://github.com/rust-lang/crates.io-index]<br>1.1.2+spec-1.1.0 [registry+https://github.com/rust-lang/crates.io-index] |
| `unicode-width` | 0.1.14 [registry+https://github.com/rust-lang/crates.io-index]<br>0.2.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `wasi` | 0.11.1+wasi-snapshot-preview1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.14.7+wasi-0.2.4 [registry+https://github.com/rust-lang/crates.io-index] |
| `wasite` | 0.1.0 [registry+https://github.com/rust-lang/crates.io-index]<br>1.0.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `webpki-roots` | 0.26.11 [registry+https://github.com/rust-lang/crates.io-index]<br>1.0.7 [registry+https://github.com/rust-lang/crates.io-index] |
| `which` | 6.0.3 [registry+https://github.com/rust-lang/crates.io-index]<br>8.0.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `whoami` | 1.6.1 [registry+https://github.com/rust-lang/crates.io-index]<br>2.1.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `wit-bindgen` | 0.51.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.57.1 [registry+https://github.com/rust-lang/crates.io-index] |
| `zip` | 0.6.6 [registry+https://github.com/rust-lang/crates.io-index]<br>2.4.2 [registry+https://github.com/rust-lang/crates.io-index] |
| `zstd` | 0.11.2+zstd.1.5.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.13.3 [registry+https://github.com/rust-lang/crates.io-index] |
| `zstd-safe` | 5.0.2+zstd.1.5.2 [registry+https://github.com/rust-lang/crates.io-index]<br>7.2.4 [registry+https://github.com/rust-lang/crates.io-index] |

## Known Unavoidable Or Transitional

| Package | Versions | Reason |
| --- | --- | --- |
| `block-buffer` | 0.10.4 [registry+https://github.com/rust-lang/crates.io-index]<br>0.12.0 [registry+https://github.com/rust-lang/crates.io-index] | crypto ecosystem major-version transition in transitive dependencies |
| `constant_time_eq` | 0.1.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.3.1 [registry+https://github.com/rust-lang/crates.io-index] | crypto/compression transitive users require incompatible versions |
| `crypto-common` | 0.1.7 [registry+https://github.com/rust-lang/crates.io-index]<br>0.2.1 [registry+https://github.com/rust-lang/crates.io-index] | crypto ecosystem major-version transition in transitive dependencies |
| `digest` | 0.10.7 [registry+https://github.com/rust-lang/crates.io-index]<br>0.11.3 [registry+https://github.com/rust-lang/crates.io-index] | crypto ecosystem major-version transition in transitive dependencies |
| `getrandom` | 0.2.17 [registry+https://github.com/rust-lang/crates.io-index]<br>0.3.4 [registry+https://github.com/rust-lang/crates.io-index]<br>0.4.2 [registry+https://github.com/rust-lang/crates.io-index] | randomness ecosystem major-version transition in transitive dependencies |
| `hmac` | 0.12.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.13.0 [registry+https://github.com/rust-lang/crates.io-index] | crypto ecosystem major-version transition in transitive dependencies |
| `pbkdf2` | 0.11.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.12.2 [registry+https://github.com/rust-lang/crates.io-index] | crypto ecosystem major-version transition in transitive dependencies |
| `rand` | 0.8.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.4 [registry+https://github.com/rust-lang/crates.io-index] | randomness ecosystem major-version transition in transitive dependencies |
| `rand_chacha` | 0.3.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.0 [registry+https://github.com/rust-lang/crates.io-index] | randomness ecosystem major-version transition in transitive dependencies |
| `rand_core` | 0.6.4 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.5 [registry+https://github.com/rust-lang/crates.io-index] | randomness ecosystem major-version transition in transitive dependencies |
| `schemars` | 0.8.22 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.0 [registry+https://github.com/rust-lang/crates.io-index]<br>1.2.1 [registry+https://github.com/rust-lang/crates.io-index] | schema ecosystem has incompatible major versions in active transitive users |
| `schemars_derive` | 0.8.22 [registry+https://github.com/rust-lang/crates.io-index]<br>1.2.1 [registry+https://github.com/rust-lang/crates.io-index] | schema ecosystem has incompatible major versions in active transitive users |
| `sha1` | 0.10.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.11.0 [registry+https://github.com/rust-lang/crates.io-index] | crypto ecosystem major-version transition in transitive dependencies |
| `sha2` | 0.10.9 [registry+https://github.com/rust-lang/crates.io-index]<br>0.11.0 [registry+https://github.com/rust-lang/crates.io-index] | crypto ecosystem major-version transition in transitive dependencies |
| `syn` | 1.0.109 [registry+https://github.com/rust-lang/crates.io-index]<br>2.0.117 [registry+https://github.com/rust-lang/crates.io-index] | proc-macro ecosystem still has v1/v2 transitive users |
| `thiserror` | 1.0.69 [registry+https://github.com/rust-lang/crates.io-index]<br>2.0.18 [registry+https://github.com/rust-lang/crates.io-index] | transitive ecosystem still has v1/v2 users |
| `thiserror-impl` | 1.0.69 [registry+https://github.com/rust-lang/crates.io-index]<br>2.0.18 [registry+https://github.com/rust-lang/crates.io-index] | transitive ecosystem still has v1/v2 users |
| `tokio-tungstenite` | 0.28.0 [git+https://github.com/openai-oss-forks/tokio-tungstenite?rev=132f5b39c862e3a970f731d709608b3e6276d5f6#132f5b39c862e3a970f731d709608b3e6276d5f6]<br>0.29.0 [registry+https://github.com/rust-lang/crates.io-index] | temporary fork/upstream websocket split during remote-control merge |
| `tungstenite` | 0.27.0 [git+https://github.com/openai-oss-forks/tungstenite-rs?rev=9200079d3b54a1ff51072e24d81fd354f085156f#9200079d3b54a1ff51072e24d81fd354f085156f]<br>0.29.0 [registry+https://github.com/rust-lang/crates.io-index] | temporary fork/upstream websocket split during remote-control merge |
| `untrusted` | 0.7.1 [registry+https://github.com/rust-lang/crates.io-index]<br>0.9.0 [registry+https://github.com/rust-lang/crates.io-index] | TLS stack has incompatible transitive versions |
| `windows_aarch64_gnullvm` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows_aarch64_msvc` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows_i686_gnu` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows_i686_gnullvm` | 0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows_i686_msvc` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows_x86_64_gnu` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows_x86_64_gnullvm` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows_x86_64_msvc` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.1 [registry+https://github.com/rust-lang/crates.io-index] | windows target crate pulled transitively by multiple windows crate generations |
| `windows-sys` | 0.45.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.59.0 [registry+https://github.com/rust-lang/crates.io-index]<br>0.60.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.61.2 [registry+https://github.com/rust-lang/crates.io-index] | windows crate ecosystem transition; active target variants can coexist |
| `windows-targets` | 0.42.2 [registry+https://github.com/rust-lang/crates.io-index]<br>0.48.5 [registry+https://github.com/rust-lang/crates.io-index]<br>0.52.6 [registry+https://github.com/rust-lang/crates.io-index]<br>0.53.5 [registry+https://github.com/rust-lang/crates.io-index] | windows crate ecosystem transition; platform target crates follow upstream transitive versions |

## Follow-Up Order

1. Prefer direct workspace dependency dedupes where the API surface is tiny and already covered by focused tests or canaries.
2. For transitive duplicates, identify the owning upstream crate with `cargo tree -i <crate>@<version> --manifest-path codex-rs\Cargo.toml --workspace --edges normal,build` before changing manifests.
3. Treat `known_unavoidable` as an allowlist, not a permanent exemption. Recheck it after remote-control, websocket, schema, crypto, and Windows crate transitions settle.
4. Add a CI or scheduled local check that fails or warns when `action_required_count` increases, while allowing known transitional names explicitly.
5. Keep cleanup conservative: delete dep-info orphans and test executables, not active same-name hash variants.
