//! Architecture-check crate.
//!
//! This crate intentionally contains no library code. It exists purely to host
//! integration tests under `tests/` that lock in cross-crate architecture
//! boundaries (e.g. that `codex-core` does not depend, directly or
//! transitively, on `codex-app-server-protocol`).
