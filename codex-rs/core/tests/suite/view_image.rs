#![cfg(not(target_os = "windows"))]

#[path = "view_image_user_turn.rs"]
mod user_turn;

#[path = "view_image_tool_local.rs"]
mod tool_local;

#[path = "view_image_tool_remote.rs"]
mod tool_remote;

#[path = "view_image_tool_errors.rs"]
mod tool_errors;
