//! Status-line, layout, rate-limit, pet, and hook rendering tests for `ChatWidget`.
//!
//! Split into cohesive submodules; shared fixtures live in [`common`].

mod common;

mod context_and_token;
mod goals;
mod hook_lifecycle;
mod hook_output;
mod layout_snapshots;
mod message_rendering;
mod pets;
mod rate_limit_snapshots;
mod rate_limit_switch_prompt;
mod rate_limit_warnings;
mod status_line_items;
mod status_line_items_config;
mod status_line_model_footer;
mod stream_error_and_warnings;
mod streaming;
mod task_lifecycle;
mod vt100_layout;
mod workspace_credit_nudge;
