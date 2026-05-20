mod support;

#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals.rs"]
mod approvals;
#[cfg(not(target_os = "windows"))]
#[path = "suite/hooks.rs"]
mod hooks;
#[cfg(not(target_os = "windows"))]
#[path = "suite/hooks_mcp.rs"]
mod hooks_mcp;
#[path = "suite/permissions_messages.rs"]
mod permissions_messages;
#[cfg(not(target_os = "windows"))]
#[path = "suite/request_permissions.rs"]
mod request_permissions;
#[cfg(not(target_os = "windows"))]
#[path = "suite/request_permissions_tool.rs"]
mod request_permissions_tool;
#[path = "suite/review.rs"]
mod review;
#[path = "suite/tool_harness.rs"]
mod tool_harness;
