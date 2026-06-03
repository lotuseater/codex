#![deny(clippy::print_stdout, clippy::print_stderr)]
#![deny(clippy::disallowed_methods)]

pub mod color;
pub mod diff_model;
pub mod diff_render;
pub mod exec_cell;
pub mod exec_command;
pub mod history_cell;
pub mod line_truncation;
pub mod live_wrap;
pub mod markdown;
pub mod markdown_render;
pub mod markdown_stream;
pub mod motion;
pub mod render;
pub mod session_state;
pub mod shimmer;
pub mod status_indicator_widget;
pub mod style;
pub mod table_detect;
pub mod terminal_hyperlinks;
pub mod terminal_palette;
#[cfg(unix)]
pub mod terminal_probe;
pub mod text_formatting;
pub mod ui_consts;
pub mod update_action;
pub mod version;
pub mod width;
pub mod wrapping;

#[cfg(test)]
pub(crate) mod test_support {
    pub(crate) use codex_utils_absolute_path::test_support::PathBufExt;
    pub(crate) use codex_utils_absolute_path::test_support::test_path_buf;
}
