mod support;

#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals.rs"]
mod approvals;
#[cfg(not(target_os = "windows"))]
#[path = "suite/hooks.rs"]
mod hooks;
#[cfg(not(target_os = "windows"))]
#[path = "suite/request_permissions_tool.rs"]
mod request_permissions_tool;
#[path = "suite/review.rs"]
mod review;
#[path = "suite/tool_harness.rs"]
mod tool_harness;
