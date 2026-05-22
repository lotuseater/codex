#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals_support.rs"]
mod approvals_support;

#[cfg(not(target_os = "windows"))]
#[path = "suite/approvals_network.rs"]
mod approvals_network;
