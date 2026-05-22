#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals_support.rs"]
mod approvals_support;

#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals_scenarios.rs"]
mod approvals_scenarios;

#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals_request_flow.rs"]
mod approvals_request_flow;
