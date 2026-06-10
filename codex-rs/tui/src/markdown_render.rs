// fork-local: markdown rendering was extracted into the `codex_tui_render` crate; this
// module is now a thin re-export shim. Upstream's inline edits to the old in-tree module
// (e.g. the `DecodedTextMerge` wrapper from `markdown_text_merge`) must be ported into
// `codex-rs/tui-render/src/markdown_render.rs` rather than re-added here.
pub use codex_tui_render::markdown_render::*;
