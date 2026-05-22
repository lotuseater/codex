#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals_support.rs"]
mod approvals_support;

#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals_patch.rs"]
mod approvals_patch;
