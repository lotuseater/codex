# codex-rs dep snapshot

Generated: 2026-05-11T15:51:33+03:00 (regenerate via `scripts/dep-snapshot.ps1`).

## Lockfile census

- Total locked entries: **1286**
- Cross-version duplicate families (workspace-wide): **48** (sum 100 version entries)
- Cross-version duplicate families (codex-cli deploy graph): **45**
- Workspace-declared deps: **293**
- Per-crate non-workspace pins (consolidation candidates): **0**

## Per-crate pins to consolidate

_None — every per-crate dep that has a workspace declaration uses `{ workspace = true }`._

## Cross-version duplicate families (workspace-wide)

Includes dev-deps and test-only crates — wider than what ships in `codex.exe`. See the deploy-graph section below for the strict shipping subset.

### base64

- **v0.21.7** ← age-core@0.11.0, age@0.11.3
- **v0.22.1** ← app_test_support@0.0.0, axum@0.8.9, codex-agent-identity@0.0.0, codex-api@0.0.0, codex-app-server-transport@0.0.0, codex-app-server@0.0.0, codex-cloud-requirements@0.0.0, codex-config@0.0.0, codex-core@0.0.0, codex-desktop-automation@0.0.0, codex-device-key@0.0.0, codex-exec-server@0.0.0, codex-login@0.0.0, codex-response-debug-context@0.0.0, codex-secrets@0.0.0, codex-shell-command@0.0.0, codex-tui@0.0.0, codex-utils-image@0.0.0, codex-windows-sandbox@0.0.0, core_test_support@0.0.0, headers@0.4.1, hyper-util@0.1.20, jsonwebtoken@9.3.1, oauth2@5.0.0, opentelemetry-proto@0.31.0, pem@3.0.6, plist@1.9.0, rama-http-headers@0.3.0-alpha.4, rama-http@0.3.0-alpha.4, reqwest@0.12.28, rmcp@0.15.0, serde_with@3.19.0, sqlx-core@0.8.6, tonic@0.14.6, wiremock@0.6.5

### bitflags

- **v1.3.2** ← lsp-types@0.94.1, portable-pty@0.9.0
- **v2.11.1** ← bindgen@0.72.1, crossterm@0.28.1, gix-config-value@0.18.0, gix-glob@0.26.0, gix-sec@0.14.0, gix-traverse@0.57.0, nix@0.28.0, notify-types@2.1.0, onig@6.5.3, png@0.18.1, pulldown-cmark@0.10.3, rama-http@0.3.0-alpha.4, ratatui@0.29.0, rustix@1.1.4, rustyline@18.0.0, tower-http@0.6.10, v8@146.4.0

### block-buffer

- **v0.10.4** ← digest@0.10.7
- **v0.12.0** ← digest@0.11.3

### cfg_aliases

- **v0.1.1** ← nix@0.28.0
- **v0.2.1** ← sentry@0.46.2

### const-oid

- **v0.10.2** ← digest@0.11.3
- **v0.9.6** ← der@0.7.10, digest@0.10.7

### cpufeatures

- **v0.2.17** ← aes@0.8.4, chacha20@0.9.1, const-hex@1.19.0, curve25519-dalek@4.1.3, poly1305@0.8.0, sha1@0.10.6, sha2@0.10.9
- **v0.3.0** ← sha1@0.11.0, sha2@0.11.0

### crypto-common

- **v0.1.7** ← aead@0.5.2, cipher@0.4.4, digest@0.10.7, universal-hash@0.5.1
- **v0.2.1** ← digest@0.11.3

### darling

- **v0.20.11** ← cached_proc_macro@0.25.0
- **v0.23.0** ← instability@0.3.12, rmcp-macros@0.15.0, serde_with_macros@3.19.0

### darling_core

- **v0.20.11** ← darling_macro@0.20.11, darling@0.20.11
- **v0.23.0** ← darling_macro@0.23.0, darling@0.23.0

### darling_macro

- **v0.20.11** ← _no immediate parent parsed_
- **v0.23.0** ← _no immediate parent parsed_

### digest

- **v0.10.7** ← _no immediate parent parsed_
- **v0.11.3** ← _no immediate parent parsed_

### fixedbitset

- **v0.4.2** ← petgraph@0.6.5
- **v0.5.7** ← petgraph@0.8.3

### flume

- **v0.11.1** ← sqlx-sqlite@0.8.6
- **v0.12.0** ← rama-net@0.3.0-alpha.4

### foldhash

- **v0.1.5** ← hashbrown@0.15.5
- **v0.2.0** ← hashbrown@0.16.1, hashbrown@0.17.1

### getrandom

- **v0.2.17** ← rand_core@0.6.4, ring@0.17.14
- **v0.3.4** ← ahash@0.8.12, jobserver@0.1.34, rand_core@0.9.5, zip@2.4.2
- **v0.4.2** ← tempfile@3.27.0, uuid@1.23.1

### hashbrown

- **v0.14.5** ← allocative@0.3.4, starlark_map@0.13.0, starlark@0.13.0
- **v0.15.5** ← _no immediate parent parsed_
- **v0.16.1** ← _no immediate parent parsed_
- **v0.17.1** ← _no immediate parent parsed_

### hmac

- **v0.12.1** ← age@0.11.3, hkdf@0.12.4, pbkdf2@0.12.2, rfc6979@0.4.0, zip@2.4.2
- **v0.13.0** ← aws-sigv4@1.4.3, codex-app-server-transport@0.0.0, codex-app-server@0.0.0, codex-cloud-requirements@0.0.0

### http

- **v0.2.12** ← aws-sdk-sso@1.98.0, aws-sdk-ssooidc@1.100.0, aws-sdk-sts@1.103.0, aws-sigv4@1.4.3, aws-smithy-runtime-api@1.12.0, aws-smithy-runtime@1.11.1, aws-smithy-types@1.4.7, http-body@0.4.6
- **v1.4.0** ← aws-config@1.8.16, aws-runtime@1.7.3, aws-sdk-sso@1.98.0, aws-sdk-ssooidc@1.100.0, aws-sdk-sts@1.103.0, aws-sigv4@1.4.3, aws-smithy-http-client@1.1.12, aws-smithy-http@0.63.6, aws-smithy-runtime-api@1.12.0, aws-smithy-runtime@1.11.1, aws-smithy-types@1.4.7, axum-core@0.5.6, axum@0.8.9, codex-api@0.0.0, codex-aws-auth@0.0.0, codex-client@0.0.0, codex-core@0.0.0, codex-model-provider-info@0.0.0, codex-model-provider@0.0.0, codex-otel@0.0.0, codex-protocol@0.0.0, codex-response-debug-context@0.0.0, h2@0.4.14, headers-core@0.3.0, headers@0.4.1, http-body-util@0.1.3, http-body@1.0.1, hyper-rustls@0.27.9, hyper-util@0.1.20, hyper@1.9.0, oauth2@5.0.0, opentelemetry-http@0.31.0, opentelemetry-otlp@0.31.1, rama-http-types@0.3.0-alpha.4, rama-http@0.3.0-alpha.4, reqwest@0.12.28, rmcp@0.15.0, tonic@0.14.6, tower-http@0.6.10, tungstenite@0.27.0, tungstenite@0.29.0, wiremock@0.6.5

### http-body

- **v0.4.6** ← _no immediate parent parsed_
- **v1.0.1** ← _no immediate parent parsed_

### itertools

- **v0.11.0** ← lalrpop@0.20.2
- **v0.13.0** ← bindgen@0.72.1, ratatui@0.29.0, unicode-truncate@1.1.0
- **v0.14.0** ← codex-tui@0.0.0, prost-build@0.14.3, prost-derive@0.14.3, rama-net@0.3.0-alpha.4, starlark@0.13.0

### matchit

- **v0.8.4** ← axum@0.8.9
- **v0.9.2** ← rama-http@0.3.0-alpha.4

### memoffset

- **v0.6.5** ← starlark@0.13.0
- **v0.9.1** ← uds_windows@1.2.1

### nom

- **v7.1.3** ← age-core@0.11.0, age@0.11.3, ansi-to-tui@7.0.0, asn1-rs@0.7.1, cexpr@0.6.0, der-parser@10.0.0, eventsource-stream@0.2.3, rusticata-macros@4.1.0, x509-parser@0.18.1
- **v8.0.0** ← rama-http-types@0.3.0-alpha.4, rama-net@0.3.0-alpha.4

### petgraph

- **v0.6.5** ← _no immediate parent parsed_
- **v0.8.3** ← _no immediate parent parsed_

### quick-xml

- **v0.38.4** ← codex-protocol@0.0.0
- **v0.39.4** ← plist@1.9.0

### rand

- **v0.8.6** ← age-core@0.11.0, age@0.11.3, oauth2@5.0.0
- **v0.9.4** ← codex-agent-identity@0.0.0, codex-client@0.0.0, codex-core@0.0.0, codex-device-key@0.0.0, codex-login@0.0.0, codex-secrets@0.0.0, codex-tui@0.0.0, codex-windows-sandbox@0.0.0, hickory-proto@0.25.2, hickory-resolver@0.25.2, opentelemetry_sdk@0.31.0, rama-http-headers@0.3.0-alpha.4, rama-http-types@0.3.0-alpha.4, rama-http@0.3.0-alpha.4, rama-tcp@0.3.0-alpha.4, rmcp@0.15.0, sentry-core@0.46.2, sentry-types@0.46.2, tungstenite@0.27.0, tungstenite@0.29.0

### rand_chacha

- **v0.3.1** ← rand@0.8.6
- **v0.9.0** ← rand@0.9.4

### rand_core

- **v0.6.4** ← _no immediate parent parsed_
- **v0.9.5** ← _no immediate parent parsed_

### regex-syntax

- **v0.6.29** ← logos-derive@0.12.1
- **v0.8.10** ← globset@0.4.18, lalrpop@0.20.2, regex-automata@0.4.14, regex@1.12.3, syntect@5.3.0, tree-sitter@0.25.10

### rustc-hash

- **v1.1.0** ← fluent-bundle@0.15.3
- **v2.1.2** ← bindgen@0.72.1, type-map@0.5.1

### schemars

- **v0.8.22** ← codex-app-server-protocol@0.0.0, codex-config@0.0.0, codex-features@0.0.0, codex-git-utils@0.0.0, codex-hooks@0.0.0, codex-mcp-server@0.0.0, codex-memories-mcp@0.0.0, codex-model-provider-info@0.0.0, codex-protocol@0.0.0, codex-secrets@0.0.0, codex-utils-absolute-path@0.0.0
- **v1.2.1** ← _no immediate parent parsed_

### schemars_derive

- **v0.8.22** ← schemars@0.8.22
- **v1.2.1** ← schemars@1.2.1

### self_cell

- **v0.10.3** ← fluent-bundle@0.15.3
- **v1.2.2** ← self_cell@0.10.3

### sha1

- **v0.10.6** ← _no immediate parent parsed_
- **v0.11.0** ← _no immediate parent parsed_

### sha2

- **v0.10.9** ← _no immediate parent parsed_
- **v0.11.0** ← _no immediate parent parsed_

### similar

- **v2.7.0** ← insta@1.47.2
- **v3.1.0** ← codex-app-server-protocol@0.0.0, codex-apply-patch@0.0.0, codex-config@0.0.0, codex-core@0.0.0, codex-git-utils@0.0.0, core_test_support@0.0.0

### supports-color

- **v2.1.0** ← owo-colors@4.3.0
- **v3.0.2** ← codex-cli@0.0.0, codex-cloud-tasks@0.0.0, codex-exec@0.0.0, codex-tui@0.0.0, owo-colors@4.3.0

### syn

- **v1.0.109** ← logos-derive@0.12.1, schemafy_lib@0.5.2, schemafy@0.5.2
- **v2.0.117** ← allocative_derive@0.3.4, asn1-rs-derive@0.6.0, asn1-rs-impl@0.2.0, async-stream-impl@0.3.6, async-trait@0.1.89, aws-smithy-runtime-api-macros@1.0.0, bindgen@0.72.1, cached_proc_macro@0.25.0, clap_derive@4.6.1, codex-experimental-api-macros@0.0.0, curve25519-dalek-derive@0.1.1, darling_core@0.20.11, darling_core@0.23.0, darling_macro@0.20.11, darling_macro@0.23.0, derive_more-impl@2.1.1, diplomat_core@0.14.0, diplomat@0.14.0, displaydoc@0.2.5, dupe_derive@0.9.0, enum-as-inner@0.6.1, futures-macro@0.3.32, i18n-embed-fl@0.9.4, i18n-embed-impl@0.8.4, instability@0.3.12, maybe-async@0.2.10, pin-project-internal@1.1.12, prettyplease@0.2.37, proc-macro-error2@2.0.1, prost-build@0.14.3, prost-derive@0.14.3, rama-macros@0.3.0-alpha.4, ref-cast-impl@1.0.25, rmcp-macros@0.15.0, rust-embed-impl@8.11.0, schemars_derive@0.8.22, schemars_derive@1.2.1, serde_derive_internals@0.29.1, serde_derive@1.0.228, serde_repr@0.1.20, serde_with_macros@3.19.0, serial_test_derive@3.4.0, sqlx-macros-core@0.8.6, sqlx-macros@0.8.6, starlark_derive@0.13.0, strum_macros@0.28.0, synstructure@0.13.2, test-case-core@3.3.1, test-case-macros@3.3.1, test-log-core@0.2.20, test-log-macros@0.2.20, thiserror-impl@1.0.69, thiserror-impl@2.0.18, tokio-macros@2.7.0, tonic-build@0.14.6, tonic-prost-build@0.14.3, tracing-attributes@0.1.31, tracing-test-macro@0.2.6, ts-rs-macros@11.1.0, windows-implement@0.60.2, windows-interface@0.59.3, yoke-derive@0.8.2, zerofrom-derive@0.1.7, zeroize_derive@1.4.3, zerovec-derive@0.11.3

### thiserror

- **v1.0.69** ← ansi-to-tui@7.0.0, filedescriptor@0.8.3, fluent-syntax@0.11.1, i18n-config@0.4.8, i18n-embed@0.15.4, oauth2@5.0.0
- **v2.0.18** ← asn1-rs@0.7.1, cached@0.56.0, codex-agent-graph-store@0.0.0, codex-api@0.0.0, codex-app-server-protocol@0.0.0, codex-app-server@0.0.0, codex-apply-patch@0.0.0, codex-aws-auth@0.0.0, codex-client@0.0.0, codex-cloud-requirements@0.0.0, codex-cloud-tasks-client@0.0.0, codex-config@0.0.0, codex-core-plugins@0.0.0, codex-core@0.0.0, codex-desktop-automation@0.0.0, codex-device-key@0.0.0, codex-exec-server@0.0.0, codex-execpolicy@0.0.0, codex-first-moves@0.0.0, codex-git-utils@0.0.0, codex-login@0.0.0, codex-mcp@0.0.0, codex-memories-mcp@0.0.0, codex-network-proxy@0.0.0, codex-otel@0.0.0, codex-plugin@0.0.0, codex-protocol@0.0.0, codex-realtime-webrtc@0.0.0, codex-reasoning-logic@0.0.0, codex-repo-context-scout@0.0.0, codex-rmcp-client@0.0.0, codex-skills@0.0.0, codex-thread-store@0.0.0, codex-tui@0.0.0, codex-utils-cargo-bin@0.0.0, codex-utils-image@0.0.0, codex-utils-readiness@0.0.0, gix-config-value@0.18.0, gix-config@0.56.0, gix-diff@0.63.0, gix-discover@0.51.0, gix-features@0.48.0, gix-fs@0.21.1, gix-hash@0.25.0, gix-lock@23.0.0, gix-object@0.60.0, gix-odb@0.80.0, gix-pack@0.70.0, gix-packetline@0.21.3, gix-path@0.12.0, gix-protocol@0.61.0, gix-ref@0.63.0, gix-refspec@0.41.0, gix-revwalk@0.31.0, gix-shallow@0.12.0, gix-transport@0.57.0, gix-traverse@0.57.0, gix-url@0.36.0, gix@0.83.0, hickory-proto@0.25.2, hickory-resolver@0.25.2, opentelemetry_sdk@0.31.0, opentelemetry-otlp@0.31.1, opentelemetry@0.31.0, rmcp@0.15.0, sentry-types@0.46.2, simple_asn1@0.6.4, sqlx-core@0.8.6, sqlx-sqlite@0.8.6, starlark_syntax@0.13.0, starlark@0.13.0, syntect@5.3.0, tracing-appender@0.2.5, ts-rs@11.1.0, tungstenite@0.27.0, tungstenite@0.29.0, wildcard@0.3.0, x509-parser@0.18.1, zip@2.4.2

### thiserror-impl

- **v1.0.69** ← _no immediate parent parsed_
- **v2.0.18** ← _no immediate parent parsed_

### tokio-tungstenite

- **v0.28.0** ← _no immediate parent parsed_
- **v0.29.0** ← _no immediate parent parsed_

### toml

- **v0.5.11** ← find-crate@0.6.3, winres@0.1.12
- **v1.1.2+spec-1.1.0** ← _no immediate parent parsed_

### tungstenite

- **v0.27.0** ← _no immediate parent parsed_
- **v0.29.0** ← _no immediate parent parsed_

### unicode-width

- **v0.1.14** ← annotate-snippets@0.9.2, unicode-truncate@1.1.0
- **v0.2.2** ← codex-cloud-tasks@0.0.0, codex-tui@0.0.0, getopts@0.2.24, ratatui@0.29.0, rustyline@18.0.0, textwrap@0.16.2, vt100@0.16.2

### untrusted

- **v0.7.1** ← aws-lc-rs@1.16.3
- **v0.9.0** ← ring@0.17.14, rustls-webpki@0.103.13

### webpki-roots

- **v0.26.11** ← sqlx-core@0.8.6
- **v1.0.7** ← hyper-rustls@0.27.9, rama-tls-rustls@0.3.0-alpha.4, reqwest@0.12.28, webpki-roots@0.26.11

### which

- **v6.0.3** ← v8@146.4.0
- **v8.0.2** ← codex-core@0.0.0, codex-lmstudio@0.0.0, codex-rmcp-client@0.0.0, codex-sandboxing@0.0.0, codex-shell-command@0.0.0, codex-tui@0.0.0

### windows-sys

- **v0.60.2** ← arboard@3.6.1, keyring@3.6.3, notify@8.2.0
- **v0.61.2** ← anstyle-query@1.1.5, anstyle-wincon@3.0.11, async-io@2.6.0, codex-config@0.0.0, codex-tui@0.0.0, codex-utils-sleep-inhibitor@0.0.0, codex-windows-sandbox@0.0.0, console@0.16.3, dirs-sys@0.5.0, errno@0.3.14, gix-sec@0.14.0, home@0.5.12, ipconfig@0.3.4, is-terminal@0.4.17, jiff@0.2.24, mio@1.2.0, nu-ansi-term@0.50.3, os_info@3.14.0, polling@3.11.0, rustix@1.1.4, rustyline@18.0.0, schannel@0.1.29, socket2@0.6.3, tempfile@3.27.0, terminal_size@0.4.4, tokio@1.52.3, uds_windows@1.2.1, winapi-util@0.1.11

## Deploy-graph duplicates (codex-cli, normal+build edges)

Strict subgraph of what actually compiles into `codex.exe` — dev-deps and isolated test crates excluded. Collapsing a family here would shrink the deployed binary.

### base64

- **v0.21.7** ← age-core@0.11.0, age@0.11.3
- **v0.22.1** ← axum@0.8.9, codex-agent-identity@0.0.0, codex-api@0.0.0, codex-app-server-transport@0.0.0, codex-app-server@0.0.0, codex-cloud-requirements@0.0.0, codex-config@0.0.0, codex-core@0.0.0, codex-desktop-automation@0.0.0, codex-device-key@0.0.0, codex-exec-server@0.0.0, codex-login@0.0.0, codex-response-debug-context@0.0.0, codex-secrets@0.0.0, codex-shell-command@0.0.0, codex-tui@0.0.0, codex-utils-image@0.0.0, codex-windows-sandbox@0.0.0, headers@0.4.1, hyper-util@0.1.20, jsonwebtoken@9.3.1, oauth2@5.0.0, opentelemetry-proto@0.31.0, pem@3.0.6, plist@1.9.0, rama-http-headers@0.3.0-alpha.4, rama-http@0.3.0-alpha.4, reqwest@0.12.28, rmcp@0.15.0, serde_with@3.19.0, sqlx-core@0.8.6, tonic@0.14.6, wiremock@0.6.5

### bitflags

- **v1.3.2** ← lsp-types@0.94.1, portable-pty@0.9.0
- **v2.11.1** ← bindgen@0.72.1, crossterm@0.28.1, gix-config-value@0.18.0, gix-glob@0.26.0, gix-sec@0.14.0, gix-traverse@0.57.0, nix@0.28.0, notify-types@2.1.0, onig@6.5.3, png@0.18.1, pulldown-cmark@0.10.3, rama-http@0.3.0-alpha.4, ratatui@0.29.0, rustix@1.1.4, rustyline@18.0.0, tower-http@0.6.10, v8@146.4.0

### block-buffer

- **v0.10.4** ← digest@0.10.7
- **v0.12.0** ← digest@0.11.3

### cfg_aliases

- **v0.1.1** ← nix@0.28.0
- **v0.2.1** ← sentry@0.46.2

### const-oid

- **v0.10.2** ← digest@0.11.3
- **v0.9.6** ← der@0.7.10, digest@0.10.7

### cpufeatures

- **v0.2.17** ← aes@0.8.4, chacha20@0.9.1, const-hex@1.19.0, curve25519-dalek@4.1.3, poly1305@0.8.0, sha1@0.10.6, sha2@0.10.9
- **v0.3.0** ← sha1@0.11.0, sha2@0.11.0

### crypto-common

- **v0.1.7** ← aead@0.5.2, cipher@0.4.4, digest@0.10.7, universal-hash@0.5.1
- **v0.2.1** ← digest@0.11.3

### darling

- **v0.20.11** ← cached_proc_macro@0.25.0
- **v0.23.0** ← instability@0.3.12, rmcp-macros@0.15.0, serde_with_macros@3.19.0

### darling_core

- **v0.20.11** ← darling_macro@0.20.11, darling@0.20.11
- **v0.23.0** ← darling_macro@0.23.0, darling@0.23.0

### darling_macro

- **v0.20.11** ← _no immediate parent parsed_
- **v0.23.0** ← _no immediate parent parsed_

### digest

- **v0.10.7** ← _no immediate parent parsed_
- **v0.11.3** ← _no immediate parent parsed_

### flume

- **v0.11.1** ← sqlx-sqlite@0.8.6
- **v0.12.0** ← rama-net@0.3.0-alpha.4

### foldhash

- **v0.1.5** ← hashbrown@0.15.5
- **v0.2.0** ← hashbrown@0.16.1, hashbrown@0.17.1

### getrandom

- **v0.2.17** ← rand_core@0.6.4, ring@0.17.14
- **v0.3.4** ← ahash@0.8.12, jobserver@0.1.34, rand_core@0.9.5, zip@2.4.2
- **v0.4.2** ← tempfile@3.27.0, uuid@1.23.1

### hashbrown

- **v0.14.5** ← allocative@0.3.4, starlark_map@0.13.0, starlark@0.13.0
- **v0.15.5** ← _no immediate parent parsed_
- **v0.16.1** ← _no immediate parent parsed_
- **v0.17.1** ← _no immediate parent parsed_

### hmac

- **v0.12.1** ← age@0.11.3, hkdf@0.12.4, pbkdf2@0.12.2, rfc6979@0.4.0, zip@2.4.2
- **v0.13.0** ← aws-sigv4@1.4.3, codex-app-server-transport@0.0.0, codex-cloud-requirements@0.0.0

### http

- **v0.2.12** ← aws-sdk-sso@1.98.0, aws-sdk-ssooidc@1.100.0, aws-sdk-sts@1.103.0, aws-sigv4@1.4.3, aws-smithy-runtime-api@1.12.0, aws-smithy-runtime@1.11.1, aws-smithy-types@1.4.7, http-body@0.4.6
- **v1.4.0** ← aws-config@1.8.16, aws-runtime@1.7.3, aws-sdk-sso@1.98.0, aws-sdk-ssooidc@1.100.0, aws-sdk-sts@1.103.0, aws-sigv4@1.4.3, aws-smithy-http-client@1.1.12, aws-smithy-http@0.63.6, aws-smithy-runtime-api@1.12.0, aws-smithy-runtime@1.11.1, aws-smithy-types@1.4.7, axum-core@0.5.6, axum@0.8.9, codex-api@0.0.0, codex-aws-auth@0.0.0, codex-client@0.0.0, codex-core@0.0.0, codex-model-provider-info@0.0.0, codex-model-provider@0.0.0, codex-otel@0.0.0, codex-response-debug-context@0.0.0, h2@0.4.14, headers-core@0.3.0, headers@0.4.1, http-body-util@0.1.3, http-body@1.0.1, hyper-rustls@0.27.9, hyper-util@0.1.20, hyper@1.9.0, oauth2@5.0.0, opentelemetry-http@0.31.0, opentelemetry-otlp@0.31.1, rama-http-types@0.3.0-alpha.4, rama-http@0.3.0-alpha.4, reqwest@0.12.28, rmcp@0.15.0, tonic@0.14.6, tower-http@0.6.10, tungstenite@0.27.0, tungstenite@0.29.0, wiremock@0.6.5

### http-body

- **v0.4.6** ← _no immediate parent parsed_
- **v1.0.1** ← _no immediate parent parsed_

### itertools

- **v0.11.0** ← lalrpop@0.20.2
- **v0.13.0** ← bindgen@0.72.1, ratatui@0.29.0, unicode-truncate@1.1.0
- **v0.14.0** ← codex-tui@0.0.0, prost-derive@0.14.3, rama-net@0.3.0-alpha.4, starlark@0.13.0

### matchit

- **v0.8.4** ← axum@0.8.9
- **v0.9.2** ← rama-http@0.3.0-alpha.4

### memoffset

- **v0.6.5** ← starlark@0.13.0
- **v0.9.1** ← uds_windows@1.2.1

### nom

- **v7.1.3** ← age-core@0.11.0, age@0.11.3, ansi-to-tui@7.0.0, asn1-rs@0.7.1, cexpr@0.6.0, der-parser@10.0.0, eventsource-stream@0.2.3, rusticata-macros@4.1.0, x509-parser@0.18.1
- **v8.0.0** ← rama-http-types@0.3.0-alpha.4, rama-net@0.3.0-alpha.4

### quick-xml

- **v0.38.4** ← codex-protocol@0.0.0
- **v0.39.4** ← plist@1.9.0

### rand

- **v0.8.6** ← age-core@0.11.0, age@0.11.3, oauth2@5.0.0
- **v0.9.4** ← codex-agent-identity@0.0.0, codex-client@0.0.0, codex-core@0.0.0, codex-device-key@0.0.0, codex-login@0.0.0, codex-secrets@0.0.0, codex-tui@0.0.0, codex-windows-sandbox@0.0.0, hickory-proto@0.25.2, hickory-resolver@0.25.2, opentelemetry_sdk@0.31.0, rama-http-headers@0.3.0-alpha.4, rama-http-types@0.3.0-alpha.4, rama-http@0.3.0-alpha.4, rama-tcp@0.3.0-alpha.4, rmcp@0.15.0, sentry-core@0.46.2, sentry-types@0.46.2, tungstenite@0.27.0, tungstenite@0.29.0

### rand_chacha

- **v0.3.1** ← rand@0.8.6
- **v0.9.0** ← rand@0.9.4

### rand_core

- **v0.6.4** ← _no immediate parent parsed_
- **v0.9.5** ← _no immediate parent parsed_

### regex-syntax

- **v0.6.29** ← logos-derive@0.12.1
- **v0.8.10** ← globset@0.4.18, lalrpop@0.20.2, regex-automata@0.4.14, regex@1.12.3, syntect@5.3.0, tree-sitter@0.25.10

### rustc-hash

- **v1.1.0** ← fluent-bundle@0.15.3
- **v2.1.2** ← bindgen@0.72.1, type-map@0.5.1

### schemars

- **v0.8.22** ← codex-app-server-protocol@0.0.0, codex-config@0.0.0, codex-features@0.0.0, codex-git-utils@0.0.0, codex-hooks@0.0.0, codex-mcp-server@0.0.0, codex-memories-mcp@0.0.0, codex-model-provider-info@0.0.0, codex-protocol@0.0.0, codex-secrets@0.0.0, codex-utils-absolute-path@0.0.0
- **v1.2.1** ← _no immediate parent parsed_

### schemars_derive

- **v0.8.22** ← schemars@0.8.22
- **v1.2.1** ← schemars@1.2.1

### self_cell

- **v0.10.3** ← fluent-bundle@0.15.3
- **v1.2.2** ← self_cell@0.10.3

### sha1

- **v0.10.6** ← _no immediate parent parsed_
- **v0.11.0** ← _no immediate parent parsed_

### sha2

- **v0.10.9** ← _no immediate parent parsed_
- **v0.11.0** ← _no immediate parent parsed_

### supports-color

- **v2.1.0** ← owo-colors@4.3.0
- **v3.0.2** ← codex-cli@0.0.0, codex-cloud-tasks@0.0.0, codex-exec@0.0.0, codex-tui@0.0.0, owo-colors@4.3.0

### syn

- **v1.0.109** ← logos-derive@0.12.1, schemafy_lib@0.5.2, schemafy@0.5.2
- **v2.0.117** ← allocative_derive@0.3.4, asn1-rs-derive@0.6.0, asn1-rs-impl@0.2.0, async-stream-impl@0.3.6, async-trait@0.1.89, aws-smithy-runtime-api-macros@1.0.0, bindgen@0.72.1, cached_proc_macro@0.25.0, clap_derive@4.6.1, codex-experimental-api-macros@0.0.0, curve25519-dalek-derive@0.1.1, darling_core@0.20.11, darling_core@0.23.0, darling_macro@0.20.11, darling_macro@0.23.0, derive_more-impl@2.1.1, diplomat_core@0.14.0, diplomat@0.14.0, displaydoc@0.2.5, dupe_derive@0.9.0, enum-as-inner@0.6.1, futures-macro@0.3.32, i18n-embed-fl@0.9.4, i18n-embed-impl@0.8.4, instability@0.3.12, maybe-async@0.2.10, pin-project-internal@1.1.12, prettyplease@0.2.37, proc-macro-error2@2.0.1, prost-derive@0.14.3, rama-macros@0.3.0-alpha.4, ref-cast-impl@1.0.25, rmcp-macros@0.15.0, rust-embed-impl@8.11.0, schemars_derive@0.8.22, schemars_derive@1.2.1, serde_derive_internals@0.29.1, serde_derive@1.0.228, serde_repr@0.1.20, serde_with_macros@3.19.0, sqlx-macros-core@0.8.6, sqlx-macros@0.8.6, starlark_derive@0.13.0, strum_macros@0.28.0, synstructure@0.13.2, thiserror-impl@1.0.69, thiserror-impl@2.0.18, tokio-macros@2.7.0, tracing-attributes@0.1.31, ts-rs-macros@11.1.0, windows-implement@0.60.2, windows-interface@0.59.3, yoke-derive@0.8.2, zerofrom-derive@0.1.7, zeroize_derive@1.4.3, zerovec-derive@0.11.3

### thiserror

- **v1.0.69** ← ansi-to-tui@7.0.0, filedescriptor@0.8.3, fluent-syntax@0.11.1, i18n-config@0.4.8, i18n-embed@0.15.4, oauth2@5.0.0
- **v2.0.18** ← asn1-rs@0.7.1, cached@0.56.0, codex-agent-graph-store@0.0.0, codex-api@0.0.0, codex-app-server-protocol@0.0.0, codex-app-server@0.0.0, codex-apply-patch@0.0.0, codex-aws-auth@0.0.0, codex-client@0.0.0, codex-cloud-requirements@0.0.0, codex-cloud-tasks-client@0.0.0, codex-config@0.0.0, codex-core-plugins@0.0.0, codex-core@0.0.0, codex-desktop-automation@0.0.0, codex-device-key@0.0.0, codex-exec-server@0.0.0, codex-execpolicy@0.0.0, codex-first-moves@0.0.0, codex-git-utils@0.0.0, codex-login@0.0.0, codex-mcp@0.0.0, codex-memories-mcp@0.0.0, codex-network-proxy@0.0.0, codex-otel@0.0.0, codex-plugin@0.0.0, codex-protocol@0.0.0, codex-realtime-webrtc@0.0.0, codex-repo-context-scout@0.0.0, codex-rmcp-client@0.0.0, codex-skills@0.0.0, codex-thread-store@0.0.0, codex-tui@0.0.0, codex-utils-image@0.0.0, codex-utils-readiness@0.0.0, gix-config-value@0.18.0, gix-config@0.56.0, gix-diff@0.63.0, gix-discover@0.51.0, gix-features@0.48.0, gix-fs@0.21.1, gix-hash@0.25.0, gix-lock@23.0.0, gix-object@0.60.0, gix-odb@0.80.0, gix-pack@0.70.0, gix-packetline@0.21.3, gix-path@0.12.0, gix-protocol@0.61.0, gix-ref@0.63.0, gix-refspec@0.41.0, gix-revwalk@0.31.0, gix-shallow@0.12.0, gix-transport@0.57.0, gix-traverse@0.57.0, gix-url@0.36.0, gix@0.83.0, hickory-proto@0.25.2, hickory-resolver@0.25.2, opentelemetry_sdk@0.31.0, opentelemetry-otlp@0.31.1, opentelemetry@0.31.0, rmcp@0.15.0, sentry-types@0.46.2, simple_asn1@0.6.4, sqlx-core@0.8.6, sqlx-sqlite@0.8.6, starlark_syntax@0.13.0, starlark@0.13.0, syntect@5.3.0, tracing-appender@0.2.5, ts-rs@11.1.0, tungstenite@0.27.0, tungstenite@0.29.0, wildcard@0.3.0, x509-parser@0.18.1, zip@2.4.2

### thiserror-impl

- **v1.0.69** ← _no immediate parent parsed_
- **v2.0.18** ← _no immediate parent parsed_

### tokio-tungstenite

- **v0.28.0** ← _no immediate parent parsed_
- **v0.29.0** ← _no immediate parent parsed_

### toml

- **v0.5.11** ← find-crate@0.6.3, winres@0.1.12
- **v1.1.2+spec-1.1.0** ← _no immediate parent parsed_

### tungstenite

- **v0.27.0** ← _no immediate parent parsed_
- **v0.29.0** ← _no immediate parent parsed_

### unicode-width

- **v0.1.14** ← annotate-snippets@0.9.2, unicode-truncate@1.1.0
- **v0.2.2** ← codex-cloud-tasks@0.0.0, codex-tui@0.0.0, getopts@0.2.24, ratatui@0.29.0, rustyline@18.0.0, textwrap@0.16.2

### untrusted

- **v0.7.1** ← aws-lc-rs@1.16.3
- **v0.9.0** ← ring@0.17.14, rustls-webpki@0.103.13

### webpki-roots

- **v0.26.11** ← sqlx-core@0.8.6
- **v1.0.7** ← hyper-rustls@0.27.9, rama-tls-rustls@0.3.0-alpha.4, reqwest@0.12.28, webpki-roots@0.26.11

### which

- **v6.0.3** ← v8@146.4.0
- **v8.0.2** ← codex-core@0.0.0, codex-lmstudio@0.0.0, codex-rmcp-client@0.0.0, codex-sandboxing@0.0.0, codex-shell-command@0.0.0, codex-tui@0.0.0

### windows-sys

- **v0.60.2** ← arboard@3.6.1, keyring@3.6.3, notify@8.2.0
- **v0.61.2** ← anstyle-query@1.1.5, anstyle-wincon@3.0.11, async-io@2.6.0, codex-config@0.0.0, codex-tui@0.0.0, codex-utils-sleep-inhibitor@0.0.0, codex-windows-sandbox@0.0.0, dirs-sys@0.5.0, errno@0.3.14, gix-sec@0.14.0, home@0.5.12, ipconfig@0.3.4, is-terminal@0.4.17, jiff@0.2.24, mio@1.2.0, nu-ansi-term@0.50.3, os_info@3.14.0, polling@3.11.0, rustix@1.1.4, rustyline@18.0.0, schannel@0.1.29, socket2@0.6.3, tempfile@3.27.0, terminal_size@0.4.4, tokio@1.52.3, uds_windows@1.2.1, winapi-util@0.1.11


## Top release artifact families (target/release/deps)

_`target/release/deps` not present — run a release build before regenerating to populate this section._
