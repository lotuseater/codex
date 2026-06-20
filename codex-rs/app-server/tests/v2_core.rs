// Independent integration test binary for v2 domains except the larger
// turn_start and realtime_conversation groups, which have their own binaries.
#[path = "suite/v2/account.rs"]
mod account;
#[path = "suite/v2/analytics.rs"]
mod analytics;
#[path = "suite/v2/app_list.rs"]
mod app_list;
#[path = "suite/v2/attestation.rs"]
mod attestation;
#[path = "suite/v2/client_metadata.rs"]
mod client_metadata;
#[path = "suite/v2/collaboration_mode_list.rs"]
mod collaboration_mode_list;
#[cfg(unix)]
#[path = "suite/v2/command_exec.rs"]
mod command_exec;
#[path = "suite/v2/compaction.rs"]
mod compaction;
#[path = "suite/v2/config_rpc.rs"]
mod config_rpc;
#[path = "suite/v2/connection_handling_websocket.rs"]
mod connection_handling_websocket;
#[cfg(unix)]
#[path = "suite/v2/connection_handling_websocket_unix.rs"]
mod connection_handling_websocket_unix;
#[path = "suite/v2/dynamic_tools.rs"]
mod dynamic_tools;
#[path = "suite/v2/executor_skills.rs"]
mod executor_skills;
#[path = "suite/v2/experimental_api.rs"]
mod experimental_api;
#[path = "suite/v2/experimental_feature_list.rs"]
mod experimental_feature_list;
#[path = "suite/v2/external_agent_config.rs"]
mod external_agent_config;
#[path = "suite/v2/fs.rs"]
mod fs;
#[path = "suite/v2/hooks_list.rs"]
mod hooks_list;
#[path = "suite/v2/imagegen_extension.rs"]
mod imagegen_extension;
#[path = "suite/v2/initialize.rs"]
mod initialize;
#[path = "suite/v2/marketplace_add.rs"]
mod marketplace_add;
#[path = "suite/v2/marketplace_remove.rs"]
mod marketplace_remove;
#[path = "suite/v2/marketplace_upgrade.rs"]
mod marketplace_upgrade;
#[path = "suite/v2/mcp_cache_status.rs"]
mod mcp_cache_status;
#[path = "suite/v2/mcp_resource.rs"]
mod mcp_resource;
#[path = "suite/v2/mcp_server_elicitation.rs"]
mod mcp_server_elicitation;
#[path = "suite/v2/mcp_server_status.rs"]
mod mcp_server_status;
#[path = "suite/v2/mcp_tool.rs"]
mod mcp_tool;
#[path = "suite/v2/memory_reset.rs"]
mod memory_reset;
#[path = "suite/v2/model_list.rs"]
mod model_list;
#[path = "suite/v2/model_provider_capabilities_read.rs"]
mod model_provider_capabilities_read;
#[path = "suite/v2/output_schema.rs"]
mod output_schema;
#[path = "suite/v2/permission_profile_list.rs"]
mod permission_profile_list;
#[path = "suite/v2/plan_item.rs"]
mod plan_item;
#[path = "suite/v2/plugin_install.rs"]
mod plugin_install;
#[path = "suite/v2/plugin_list.rs"]
mod plugin_list;
#[path = "suite/v2/plugin_read.rs"]
mod plugin_read;
#[path = "suite/v2/plugin_share.rs"]
mod plugin_share;
#[path = "suite/v2/plugin_uninstall.rs"]
mod plugin_uninstall;
#[path = "suite/v2/process_exec.rs"]
mod process_exec;
#[path = "suite/v2/rate_limits.rs"]
mod rate_limits;
#[path = "suite/v2/remote_control.rs"]
mod remote_control;
#[cfg(debug_assertions)]
#[path = "suite/v2/remote_thread_store.rs"]
mod remote_thread_store;
#[path = "suite/v2/request_permissions.rs"]
mod request_permissions;
#[path = "suite/v2/request_user_input.rs"]
mod request_user_input;
#[path = "suite/v2/review.rs"]
mod review;
#[path = "suite/v2/safety_check_downgrade.rs"]
mod safety_check_downgrade;
#[path = "suite/v2/skills_list.rs"]
mod skills_list;
#[path = "suite/v2/thread_archive.rs"]
mod thread_archive;
#[path = "suite/v2/thread_fork.rs"]
mod thread_fork;
#[path = "suite/v2/thread_inject_items.rs"]
mod thread_inject_items;
#[path = "suite/v2/thread_list.rs"]
mod thread_list;
#[path = "suite/v2/thread_loaded_list.rs"]
mod thread_loaded_list;
#[path = "suite/v2/thread_memory_mode_set.rs"]
mod thread_memory_mode_set;
#[path = "suite/v2/thread_metadata_update.rs"]
mod thread_metadata_update;
#[path = "suite/v2/thread_name_websocket.rs"]
mod thread_name_websocket;
#[path = "suite/v2/thread_read.rs"]
mod thread_read;
#[path = "suite/v2/thread_resume.rs"]
mod thread_resume;
#[path = "suite/v2/thread_rollback.rs"]
mod thread_rollback;
#[path = "suite/v2/thread_settings_update.rs"]
mod thread_settings_update;
#[path = "suite/v2/thread_shell_command.rs"]
mod thread_shell_command;
#[path = "suite/v2/thread_start.rs"]
mod thread_start;
#[path = "suite/v2/thread_status.rs"]
mod thread_status;
#[path = "suite/v2/thread_unarchive.rs"]
mod thread_unarchive;
#[path = "suite/v2/thread_unsubscribe.rs"]
mod thread_unsubscribe;
#[path = "suite/v2/turn_interrupt.rs"]
mod turn_interrupt;
#[path = "suite/v2/turn_start_zsh_fork.rs"]
mod turn_start_zsh_fork;
#[path = "suite/v2/turn_steer.rs"]
mod turn_steer;
#[path = "suite/v2/web_search.rs"]
mod web_search;
#[path = "suite/v2/windows_sandbox_setup.rs"]
mod windows_sandbox_setup;
