pub(crate) use codex_tui_render::markdown::*;

#[cfg(test)]
pub(crate) fn append_markdown_agent(
    markdown_source: &str,
    width: Option<usize>,
    lines: &mut Vec<ratatui::text::Line<'static>>,
) {
    append_markdown_agent_with_cwd(markdown_source, width, /*cwd*/ None, lines);
}
