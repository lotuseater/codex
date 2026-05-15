# Suspicious Code Audit - 2026-05-15

## Executive Summary

Scope: committed `HEAD` only. The audited revision is
`3677c3ddc5e048634bbe9bb4000704114ee6464b`. The audit used a clean `git archive`
export rather than the current dirty working tree or untracked files.

Coverage: 4,747 committed files were included, including 4,333 under `codex-rs`.
The scan covered source, scripts, CI, config, prompts, rules, skills, snapshots,
fixtures, generated schemas, and tracked hidden directories.

Overall result: I did not find a confirmed covert backdoor, hardcoded live
credential, or hidden product cheat. I did find four actionable security/design
risks and one strange tracked artifact:

| Severity | Count | Summary |
| --- | ---: | --- |
| High | 1 | Standalone update paths fetch remote installer scripts and execute them without local integrity verification; the Unix app-server daemon does this unattended. |
| Medium | 3 | Remote plugin bundle integrity is not bound client-side; plugin metadata can enter developer-role prompt text; issue-dedup workflow lacks explicit prompt-injection boundaries. |
| Low | 1 | A tracked `.gsd/exec` transcript leaks local path and dirty-tree state. |

## Findings

### High: Standalone update paths execute fetched installer scripts without local integrity verification

Evidence:

- `codex-rs/app-server-daemon/src/update_loop.rs:45` and
  `codex-rs/app-server-daemon/src/update_loop.rs:50` define a 5 minute initial
  delay and 1 hour update interval.
- `codex-rs/app-server-daemon/src/update_loop.rs:53` starts the Unix update
  loop, and `codex-rs/app-server-daemon/src/update_loop.rs:95` calls
  `install_latest_standalone().await?`.
- `codex-rs/app-server-daemon/src/update_loop.rs:157` implements the installer
  fetch; `codex-rs/app-server-daemon/src/update_loop.rs:158` fetches
  `https://chatgpt.com/codex/install.sh`.
- `codex-rs/app-server-daemon/src/update_loop.rs:167` launches `/bin/sh`;
  `codex-rs/app-server-daemon/src/update_loop.rs:170` and
  `codex-rs/app-server-daemon/src/update_loop.rs:171` suppress stdout/stderr;
  `codex-rs/app-server-daemon/src/update_loop.rs:179` writes the fetched bytes
  to the shell process.
- `codex-rs/app-server-daemon/src/lib.rs:187` exposes the PID update loop, and
  `codex-rs/app-server-daemon/src/backend/pid.rs:356` maps it to the hidden
  `app-server daemon pid-update-loop` subcommand.
- The app-server daemon README documents that `bootstrap` launches a detached
  updater loop that runs `install.sh` hourly:
  `codex-rs/app-server-daemon/README.md:55`.
- The TUI update action also maps standalone Unix and Windows installs to live
  installer commands: `codex-rs/tui/src/update_action.rs:15` documents
  `curl -fsSL https://chatgpt.com/codex/install.sh | sh`,
  `codex-rs/tui/src/update_action.rs:17` documents
  `irm https://chatgpt.com/codex/install.ps1|iex`,
  `codex-rs/tui/src/update_action.rs:44` returns the Unix command, and
  `codex-rs/tui/src/update_action.rs:48` returns the Windows command.
- The TUI does ask before running it: `codex-rs/tui/src/update_prompt.rs:74`
  returns `RunUpdate(update_action)` only after the update-now selection.
  The CLI then executes the chosen action through `run_update_action` at
  `codex-rs/cli/src/main.rs:662`, with `codex-rs/cli/src/main.rs:671` and
  `codex-rs/cli/src/main.rs:686` invoking `action.command_args()`.

Why it matters:

This is an installer pattern where TLS and the remote host are the only content
binding before execution. The app-server daemon case is the highest-risk form
because it is a detached hourly updater after bootstrap, making it an unattended
remote-code-execution path for Unix app-server deployments. The TUI and CLI
paths are user-triggered and visibly present the command, so their immediate
severity is lower, but they share the same missing local integrity check. If the
endpoint, CDN, DNS/TLS path, or served script is compromised or mis-served, the
client runs attacker-controlled shell or PowerShell code. Suppressing daemon
update stdout/stderr also makes failure or unexpected behavior harder to detect.

Recommended fix:

Do not pipe a live network response into a shell. Fetch signed release metadata
or a signed installer/package, verify a pinned digest/signature/attestation
locally before execution, and log unattended update output in a diagnosable
location. Prefer an updater path that downloads a specific artifact by version
and verifies it before install. Apply the same verified update primitive to the
daemon, TUI, and `codex update` paths.

### Medium: Remote plugin bundles are installed without client-side digest or signature binding

Evidence:

- `codex-rs/core-plugins/src/remote/remote_installed_plugin_sync.rs:205`
  validates backend-provided bundle metadata, including
  `codex-rs/core-plugins/src/remote/remote_installed_plugin_sync.rs:210` for
  `bundle_download_url`.
- `codex-rs/core-plugins/src/remote/remote_installed_plugin_sync.rs:226`
  downloads and installs the validated remote bundle.
- `codex-rs/core-plugins/src/remote_bundle.rs:134` validates a remote plugin
  bundle from `release_version` and `bundle_download_url`.
- `codex-rs/core-plugins/src/remote_bundle.rs:175` checks that the URL scheme is
  allowed, and `codex-rs/core-plugins/src/remote_bundle.rs:205` allows HTTPS
  generally, with loopback HTTP only for debug/test conditions.
- `codex-rs/core-plugins/src/remote_bundle.rs:222` starts the download/install
  path; `codex-rs/core-plugins/src/remote_bundle.rs:226` downloads bytes from
  the URL; `codex-rs/core-plugins/src/remote_bundle.rs:262` performs the GET.
- `codex-rs/core-plugins/src/remote_bundle.rs:280` checks the final URL scheme
  after redirects, and `codex-rs/core-plugins/src/remote_bundle.rs:302` reads
  the response with a size limit.
- Extraction has useful hardening: `codex-rs/core-plugins/src/remote_bundle.rs:528`
  rejects links, and `codex-rs/core-plugins/src/remote_bundle.rs:545` prevents
  archive path traversal.

Why it matters:

The path validates version strings, URL schemes, response status, size limits,
and tar extraction safety, but it does not bind the installed bundle bytes to a
digest or signature supplied by trusted metadata. A compromised backend response
or download endpoint could swap plugin bundle contents for the same release
version. Since plugins can add skills, tools, MCP/app surfaces, hooks, and prompt
context, plugin bundle integrity is part of the prompt and execution security
boundary.

Recommended fix:

Extend remote plugin metadata to include a cryptographic digest and, ideally, a
signature or transparency-backed attestation. Verify the digest before extraction
and bind cache/install identity to the verified digest, not only plugin ID and
release version. Keep the existing path-traversal, link, and size checks.

### Medium: Plugin display metadata is rendered into developer-role prompt text

Evidence:

- `codex-rs/core/src/session/mod.rs:2919` loads enabled plugin capability
  summaries for the current config.
- `codex-rs/core/src/session/mod.rs:2925` creates
  `AvailablePluginsInstructions` from those summaries, and
  `codex-rs/core/src/session/mod.rs:2927` pushes the rendered text into
  `developer_sections`.
- `codex-rs/core/src/context/available_plugins_instructions.rs:24` implements a
  contextual fragment for available plugins, and
  `codex-rs/core/src/context/available_plugins_instructions.rs:25` sets its role
  to `developer`.
- `codex-rs/core/src/context/available_plugins_instructions.rs:40` renders
  plugin display name and description directly into Markdown-like list text.
- `codex-rs/core-plugins/src/loader.rs:560` loads the manifest display name;
  `codex-rs/core-plugins/src/loader.rs:563` reads `interface.display_name`, and
  `codex-rs/core-plugins/src/loader.rs:565` only checks that the trimmed name is
  non-empty.
- `codex-rs/plugin/src/load_outcome.rs:66` builds the prompt-safe plugin
  description, but the hardening at `codex-rs/plugin/src/load_outcome.rs:75`
  collapses whitespace and truncates rather than escaping instruction syntax.

Why it matters:

Installed or remote plugin metadata can influence text that is delivered in a
developer-role prompt section. A malicious display name or description could
include backticks, Markdown structure breaks, or imperative text such as "ignore
previous instructions". Even if plugins are trusted enough to provide skills or
tools, capability-summary metadata should still be treated as untrusted data and
rendered so it cannot masquerade as policy.

Recommended fix:

Treat plugin names and descriptions as untrusted prompt data. Enforce a compact
single-line display-name character set or escape Markdown-sensitive characters,
truncate display names, and wrap metadata in explicit data framing that says it
is descriptive metadata only. Add tests with malicious display names and
descriptions that try to break out of the list and inject instructions.

### Medium: Issue deduplication workflow feeds untrusted issue text to Codex Action without prompt-injection boundaries

Evidence:

- `.github/workflows/issue-deduplicator.yml:45` and
  `.github/workflows/issue-deduplicator.yml:56` include issue bodies truncated to
  4,000 characters in JSON inputs.
- `.github/workflows/issue-deduplicator.yml:66` invokes
  `openai/codex-action@5c3f4ccdb2b8790f73d6b21751ac00e602aa0c02`.
- `.github/workflows/issue-deduplicator.yml:68` provides
  `CODEX_OPENAI_API_KEY`, and `.github/workflows/issue-deduplicator.yml:69`
  sets `allow-users: "*"`.
- `.github/workflows/issue-deduplicator.yml:71` starts the pass 1 prompt, but
  the prompt does not explicitly frame issue titles/bodies/comments as untrusted
  data whose instructions must be ignored.
- `.github/workflows/issue-deduplicator.yml:201`, `.github/workflows/issue-deduplicator.yml:204`,
  and `.github/workflows/issue-deduplicator.yml:205` repeat the same pattern for
  the open-issue fallback pass.
- `.github/workflows/issue-deduplicator.yml:346` gives the later comment job
  `issues: write`, and `.github/workflows/issue-deduplicator.yml:385` posts a
  duplicate-review comment derived from the model output after numeric filtering.

Why it matters:

GitHub issue bodies are untrusted user content. A malicious issue can embed
instructions targeting the model in the deduplication step. The workflow does
apply important output constraints and later filters outputs to issue numbers,
which limits blast radius, but the prompt itself does not state the core
instruction boundary: issue content is data, not instructions.

Recommended fix:

Add explicit prompt text that all JSON issue fields are untrusted data and any
instructions inside them must be ignored. Keep the schema and numeric filtering.
Consider removing `allow-users: "*"` if it is not needed, or tightening the
action invocation so untrusted issue authors cannot affect more than the
deduplication classification.

### Low: Tracked `.gsd/exec` transcript leaks local repo state

Evidence:

- `.gsd/exec/1e796248-5b37-405f-98a7-e264bc7737dc.meta.json` is tracked.
- The metadata records a local runtime and benchmark purpose, including a local
  Windows path under this checkout.
- `.gsd/exec/1e796248-5b37-405f-98a7-e264bc7737dc.stdout` starts with
  `git status --short --branch` output and includes local dirty-tree state.

Why it matters:

This does not look like executable malware or a backdoor. It does look like a
committed local execution transcript. It leaks local paths, branch names, and
working-tree state, and it creates a misleading hidden directory surface for
future audits.

Recommended fix:

Remove the tracked `.gsd/exec` artifacts unless they are intentional fixtures.
If `.gsd` is a local tool cache, add it to `.gitignore` or move durable fixtures
to a clearly named test fixture directory.

## Benign Strange Items

- The TUI pet feature is Easter-egg-like but appears intentional and visible:
  `codex-rs/tui/src/app.rs:198` includes the module, and
  `codex-rs/config/src/edit.rs:107` documents the `[tui].pet` config edit.
  The implementation includes picker, preview, terminal image protocol, cache,
  and tests under `codex-rs/tui/src/pets`.
- Hidden dynamic tool references in `codex-rs/core/tests/suite/code_mode.rs`
  are test coverage for deferred/hidden tool discovery behavior. For example,
  `codex-rs/core/tests/suite/code_mode.rs:2544` defines
  `code_mode_can_call_hidden_dynamic_tools`; this is suspicious by name but is a
  test fixture for expected tool-registry behavior.
- `codex-rs/protocol/src/approvals.rs:35` mentions letting an agent bypass
  approval for a matching prefix rule. This is an explicit approval-policy
  amendment mechanism, not a hidden bypass by itself.
- Prompt-input debug plumbing is exposed as a debug command rather than a hidden
  exfiltration path. It can reveal full prompt input if invoked locally, so it
  should remain debug-only and gated away from remote modes.
- Secret-like strings found by regex scans were dummy/test fixtures, public-key
  test material, snapshot placeholders, or redaction tests. Examples include
  dummy `sk-...` strings in CLI/login and memory redaction tests, snapshot
  `OPENAI_API_KEY` placeholders, and JWK modulus test data.
- Base64, hex, and long-token hits were mostly generated lockfile integrity
  values, schemas, snapshots, encrypted-content fixtures, image data, or test
  URLs. I did not find a decoded payload that looked like a hidden executable
  command path.

## Prompt And Rules Review

Confirmed prompt/rules surfaces include AGENTS instructions, contextual user
fragments, skills/apps instructions, plugin capability summaries, model-manager
prompts, guardian prompts, workflow prompts, and code-mode/deferred-tool
instructions.

Notable positives:

- Guardian prompts contain explicit untrusted-data framing for transcript/tool
  evidence.
- Tool discovery and hidden/deferred tool behavior is represented in tests and
  explicit specs rather than opaque runtime magic.
- Approval bypass terminology found in protocol files is tied to visible
  approval policy types and request fields.

Notable risks:

- Plugin capability summaries are currently rendered into a developer-role
  section without strong metadata escaping or data framing.
- The issue-dedup workflow should explicitly treat issue text as untrusted data.
- Remote plugin bundle integrity and auto-updater integrity are prompt-security
  issues as well as supply-chain issues because these paths can alter the tools,
  skills, or code that shape future model behavior.

## Coverage Notes

Baseline commands and artifacts:

- `git rev-parse HEAD` established
  `3677c3ddc5e048634bbe9bb4000704114ee6464b`.
- `git ls-tree -r --name-only HEAD` counted 4,747 committed files, including
  4,333 under `codex-rs`.
- A clean archive export was created from `HEAD` and scanned outside the dirty
  working tree.

Scan lanes run against the clean export:

| Lane | Lines |
| --- | ---: |
| `exec_policy.txt` | 49,019 |
| `network_urls.txt` | 5,229 |
| `executable_config_surface.txt` | 1,238 |
| `largest_files.json` | 1,002 |
| `hack_todo.txt` | 405 |
| `suspicious_terms.txt` | 344 |
| `dotfiles.txt` | 131 |
| `prompt_injection.txt` | 104 |
| `secrets_regex.txt` | 34 |
| `obfuscation_payloads.txt` | 13 |

Manual review focused on non-obvious hits from:

- Easter eggs, cheats, backdoors, hidden paths, bypass terms, and debug
  overrides.
- Prompt injection strings and all committed prompt/rules surfaces.
- Approval modes, sandbox/network environment variables, command execution,
  shell wrappers, hooks, CI, MCP/tool mutation, plugins, and skills.
- Hardcoded-token patterns, high-entropy strings, auth/keyring handling,
  telemetry, proxy, and network paths.
- Generated or minified blobs, base64/hex payloads, build scripts, vendored
  helpers, and tracked hidden directories.

Known limits:

- This is a static committed-code audit, not a dynamic runtime exploit test.
- The current dirty working tree and untracked files were intentionally excluded.
- External services and third-party action internals were not fetched or audited.
- Grep and manual review can find high-signal issues, but they cannot prove the
  absence of every malicious behavior.

## Recommended Follow-Ups

1. Replace the standalone updater fetch-and-pipe-to-shell / `irm|iex` flows with
   a verified artifact or signed metadata flow.
2. Add digest/signature verification to remote plugin bundle installation.
3. Harden plugin metadata rendering in developer-role prompts and add malicious
   metadata tests.
4. Add prompt-injection boundary language to the issue-dedup workflow prompts.
5. Remove or relocate tracked `.gsd/exec` artifacts.
