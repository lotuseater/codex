use super::*;
use codex_protocol::approvals::ElicitationRequest as CoreElicitationRequest;
use codex_protocol::config_types::ContextBudgetMode;
use codex_protocol::items::AgentMessageContent;
use codex_protocol::items::AgentMessageItem;
use codex_protocol::items::FileChangeItem;
use codex_protocol::items::ImageViewItem;
use codex_protocol::items::McpToolCallItem;
use codex_protocol::items::McpToolCallStatus as CoreMcpToolCallStatus;
use codex_protocol::items::ReasoningItem;
use codex_protocol::items::TurnItem;
use codex_protocol::items::UserMessageItem;
use codex_protocol::items::WebSearchItem;
use codex_protocol::mcp::CallToolResult;
use codex_protocol::memory_citation::MemoryCitation as CoreMemoryCitation;
use codex_protocol::memory_citation::MemoryCitationEntry as CoreMemoryCitationEntry;
use codex_protocol::models::AdditionalPermissionProfile as CoreAdditionalPermissionProfile;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
use codex_protocol::models::FileSystemPermissions as CoreFileSystemPermissions;
use codex_protocol::models::ImageDetail;
use codex_protocol::models::MessagePhase;
use codex_protocol::models::NetworkPermissions as CoreNetworkPermissions;
use codex_protocol::models::WebSearchAction as CoreWebSearchAction;
use codex_protocol::permissions::FileSystemAccessMode as CoreFileSystemAccessMode;
use codex_protocol::permissions::FileSystemPath as CoreFileSystemPath;
use codex_protocol::permissions::FileSystemSandboxEntry as CoreFileSystemSandboxEntry;
use codex_protocol::permissions::FileSystemSpecialPath as CoreFileSystemSpecialPath;
use codex_protocol::protocol::AgentStatus as CoreAgentStatus;
use codex_protocol::protocol::AskForApproval as CoreAskForApproval;
use codex_protocol::protocol::GranularApprovalConfig as CoreGranularApprovalConfig;
use codex_protocol::protocol::NetworkAccess as CoreNetworkAccess;
use codex_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;
use codex_protocol::user_input::UserInput as CoreUserInput;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_absolute_path::test_support::PathBufExt;
use codex_utils_absolute_path::test_support::test_path_buf;
use pretty_assertions::assert_eq;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

fn absolute_path_string(path: &str) -> String {
    let path = format!("/{}", path.trim_start_matches('/'));
    test_path_buf(&path).display().to_string()
}

fn absolute_path(path: &str) -> AbsolutePathBuf {
    let path = format!("/{}", path.trim_start_matches('/'));
    test_path_buf(&path).abs()
}

fn test_absolute_path() -> AbsolutePathBuf {
    absolute_path("readable")
}

mod approval_config;
mod command_exec;
mod error_dynamic_tool;
mod fs;
mod mcp_elicitation;
mod network_requirements;
mod permissions;
mod plugin;
mod plugin_share;
mod sandbox_policy;
mod thread_item;
mod thread_list;
mod thread_turn_params;

// fork-local: upstream test not yet relocated into a submodule by the fork's test split.
// (The sibling upstream tests `approvals_reviewer_serializes_auto_review_and_accepts_legacy_guardian_subagent`,
// `turn_defaults_legacy_missing_items_view_to_full`, and `thread_turns_list_params_accepts_items_view`
// already live in the v2/tests submodules, so they are intentionally NOT duplicated here.)
#[test]
fn thread_sources_round_trip_as_scalar_labels() {
    for (source, label) in [
        (ThreadSource::User, "user"),
        (ThreadSource::Subagent, "subagent"),
        (
            ThreadSource::Feature("automation".to_string()),
            "automation",
        ),
        (ThreadSource::MemoryConsolidation, "memory_consolidation"),
    ] {
        let value = serde_json::to_value(&source).expect("serialize thread source");

        assert_eq!(value, json!(label));
        assert_eq!(
            serde_json::from_value::<ThreadSource>(value).expect("deserialize thread source"),
            source
        );

        let core_source: codex_protocol::protocol::ThreadSource = source.clone().into();
        assert_eq!(ThreadSource::from(core_source), source);
    }
}

// fork-local: upstream tests that the fork's pre-merge test split did not relocate
// into a submodule. Kept inline here so upstream coverage is not dropped during the
// merge; the test-repair wave may later move these into the appropriate submodule.
#[test]
fn thread_resume_params_accept_turns_page_bootstrap() {
    let params = serde_json::from_value::<ThreadResumeParams>(json!({
        "threadId": "thr_123",
        "initialTurnsPage": {
            "limit": 25,
            "sortDirection": "asc",
            "itemsView": "full",
        },
    }))
    .expect("thread resume params should deserialize");

    assert_eq!(params.thread_id, "thr_123");
    assert_eq!(
        params.initial_turns_page,
        Some(ThreadResumeInitialTurnsPageParams {
            limit: Some(25),
            sort_direction: Some(SortDirection::Asc),
            items_view: Some(TurnItemsView::Full),
        })
    );
}

#[test]
fn thread_resume_response_round_trips_initial_turns_page() {
    let response = ThreadResumeResponse {
        thread: Thread {
            id: "thr_123".to_string(),
            session_id: "thr_123".to_string(),
            forked_from_id: None,
            parent_thread_id: None,
            preview: String::new(),
            ephemeral: false,
            model_provider: "openai".to_string(),
            created_at: 1,
            updated_at: 1,
            status: ThreadStatus::Idle,
            path: None,
            cwd: absolute_path("tmp"),
            cli_version: "0.0.0".to_string(),
            source: SessionSource::Exec,
            thread_source: None,
            agent_nickname: None,
            agent_role: None,
            git_info: None,
            name: None,
            turns: Vec::new(),
        },
        model: "gpt-5".to_string(),
        model_provider: "openai".to_string(),
        service_tier: None,
        cwd: absolute_path("tmp"),
        runtime_workspace_roots: Vec::new(),
        instruction_sources: Vec::new(),
        approval_policy: AskForApproval::OnFailure,
        approvals_reviewer: ApprovalsReviewer::User,
        sandbox: SandboxPolicy::DangerFullAccess,
        active_permission_profile: None,
        reasoning_effort: None,
        initial_turns_page: Some(TurnsPage {
            data: Vec::new(),
            next_cursor: Some("cursor_next".to_string()),
            backwards_cursor: Some("cursor_back".to_string()),
        }),
    };

    let value = serde_json::to_value(&response).expect("serialize thread resume response");
    assert_eq!(
        value.get("initialTurnsPage"),
        Some(&json!({
            "data": [],
            "nextCursor": "cursor_next",
            "backwardsCursor": "cursor_back",
        }))
    );
    let decoded = serde_json::from_value::<ThreadResumeResponse>(value)
        .expect("deserialize thread resume response");
    assert_eq!(decoded, response);
}

#[test]
fn mcp_server_status_serializes_absent_server_info_as_null() {
    let response = ListMcpServerStatusResponse {
        data: vec![McpServerStatus {
            name: "not-ready".to_string(),
            server_info: None,
            tools: HashMap::new(),
            resources: Vec::new(),
            resource_templates: Vec::new(),
            auth_status: McpAuthStatus::Unsupported,
        }],
        next_cursor: None,
    };

    assert_eq!(
        serde_json::to_value(response).expect("response should serialize"),
        json!({
            "data": [{
                "name": "not-ready",
                "serverInfo": null,
                "tools": {},
                "resources": [],
                "resourceTemplates": [],
                "authStatus": "unsupported",
            }],
            "nextCursor": null,
        })
    );
}

#[test]
fn mcp_server_status_updated_accepts_missing_thread_id() {
    let notification: McpServerStatusUpdatedNotification = serde_json::from_value(json!({
        "name": "optional_broken",
        "status": "failed",
        "error": "handshake failed",
    }))
    .expect("notification without threadId should deserialize");

    let expected = McpServerStatusUpdatedNotification {
        thread_id: None,
        name: "optional_broken".to_string(),
        status: McpServerStartupState::Failed,
        error: Some("handshake failed".to_string()),
    };
    assert_eq!(notification, expected);
    assert_eq!(
        serde_json::to_value(notification).expect("notification should serialize"),
        json!({
            "threadId": null,
            "name": "optional_broken",
            "status": "failed",
            "error": "handshake failed",
        })
    );
}

#[test]
fn mcp_server_status_serializes_absent_server_info_metadata_as_null() {
    let response = ListMcpServerStatusResponse {
        data: vec![McpServerStatus {
            name: "initialized".to_string(),
            server_info: Some(McpServerInfo {
                name: "lookup-server".to_string(),
                title: None,
                version: "1.0.0".to_string(),
                description: None,
                icons: None,
                website_url: None,
            }),
            tools: HashMap::new(),
            resources: Vec::new(),
            resource_templates: Vec::new(),
            auth_status: McpAuthStatus::Unsupported,
        }],
        next_cursor: None,
    };

    assert_eq!(
        serde_json::to_value(response).expect("response should serialize"),
        json!({
            "data": [{
                "name": "initialized",
                "serverInfo": {
                    "name": "lookup-server",
                    "title": null,
                    "version": "1.0.0",
                    "description": null,
                    "icons": null,
                    "websiteUrl": null,
                },
                "tools": {},
                "resources": [],
                "resourceTemplates": [],
                "authStatus": "unsupported",
            }],
            "nextCursor": null,
        })
    );
}

#[test]
fn skills_extra_roots_set_params_serialization_uses_extra_roots() {
    assert_eq!(
        serde_json::to_value(SkillsExtraRootsSetParams {
            extra_roots: vec![absolute_path("tmp/skills")],
        })
        .unwrap(),
        json!({
            "extraRoots": [absolute_path_string("tmp/skills")],
        }),
    );
}

#[test]
fn skills_extra_roots_set_params_rejects_relative_roots() {
    let result = serde_json::from_value::<SkillsExtraRootsSetParams>(json!({
        "extraRoots": ["relative/path"],
    }));
    assert!(result.is_err());
}
