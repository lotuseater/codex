pub use codex_tool_execution_api::can_request_original_image_detail;
pub use codex_tool_execution_api::normalize_output_image_detail;
pub use codex_tool_execution_api::sanitize_original_image_detail;

#[cfg(test)]
#[path = "image_detail_tests.rs"]
mod tests;
