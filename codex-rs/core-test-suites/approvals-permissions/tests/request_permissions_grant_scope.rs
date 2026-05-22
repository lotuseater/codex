#[cfg(not(target_os = "windows"))]
#[path = "suite/request_permissions_common.rs"]
mod request_permissions_common;

#[cfg(not(target_os = "windows"))]
#[path = "suite/request_permissions_grant_scope.rs"]
mod request_permissions_grant_scope;
