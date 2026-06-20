use super::*;
use pretty_assertions::assert_eq;

#[test]
fn core_turn_item_into_thread_item_converts_supported_variants() {
    let user_item = TurnItem::UserMessage(UserMessageItem {
        id: "user-1".to_string(),
        content: vec![
            CoreUserInput::Text {
                text: "hello".to_string(),
                text_elements: Vec::new(),
            },
            CoreUserInput::Image {
                image_url: "https://example.com/image.png".to_string(),
                detail: Some(ImageDetail::Original),
            },
            CoreUserInput::LocalImage {
                path: PathBuf::from("local/image.png"),
                detail: Some(ImageDetail::Original),
            },
            CoreUserInput::Skill {
                name: "skill-creator".to_string(),
                path: PathBuf::from("/repo/.codex/skills/skill-creator/SKILL.md"),
            },
            CoreUserInput::Mention {
                name: "Demo App".to_string(),
                path: "app://demo-app".to_string(),
            },
        ],
    });

    assert_eq!(
        ThreadItem::from(user_item),
        ThreadItem::UserMessage {
            id: "user-1".to_string(),
            content: vec![
                UserInput::Text {
                    text: "hello".to_string(),
                    text_elements: Vec::new(),
                },
                UserInput::Image {
                    url: "https://example.com/image.png".to_string(),
                    detail: Some(ImageDetail::Original),
                },
                UserInput::LocalImage {
                    path: PathBuf::from("local/image.png"),
                    detail: Some(ImageDetail::Original),
                },
                UserInput::Skill {
                    name: "skill-creator".to_string(),
                    path: PathBuf::from("/repo/.codex/skills/skill-creator/SKILL.md"),
                },
                UserInput::Mention {
                    name: "Demo App".to_string(),
                    path: "app://demo-app".to_string(),
                },
            ],
        }
    );

    let agent_item = TurnItem::AgentMessage(AgentMessageItem {
        id: "agent-1".to_string(),
        content: vec![
            AgentMessageContent::Text {
                text: "Hello ".to_string(),
            },
            AgentMessageContent::Text {
                text: "world".to_string(),
            },
        ],
        phase: None,
        memory_citation: None,
    });

    assert_eq!(
        ThreadItem::from(agent_item),
        ThreadItem::AgentMessage {
            id: "agent-1".to_string(),
            text: "Hello world".to_string(),
            phase: None,
            memory_citation: None,
        }
    );

    let agent_item_with_phase = TurnItem::AgentMessage(AgentMessageItem {
        id: "agent-2".to_string(),
        content: vec![AgentMessageContent::Text {
            text: "final".to_string(),
        }],
        phase: Some(MessagePhase::FinalAnswer),
        memory_citation: Some(CoreMemoryCitation {
            entries: vec![CoreMemoryCitationEntry {
                path: "MEMORY.md".to_string(),
                line_start: 1,
                line_end: 2,
                note: "summary".to_string(),
            }],
            rollout_ids: vec!["rollout-1".to_string()],
        }),
    });

    assert_eq!(
        ThreadItem::from(agent_item_with_phase),
        ThreadItem::AgentMessage {
            id: "agent-2".to_string(),
            text: "final".to_string(),
            phase: Some(MessagePhase::FinalAnswer),
            memory_citation: Some(MemoryCitation {
                entries: vec![MemoryCitationEntry {
                    path: "MEMORY.md".to_string(),
                    line_start: 1,
                    line_end: 2,
                    note: "summary".to_string(),
                }],
                thread_ids: vec!["rollout-1".to_string()],
            }),
        }
    );

    let reasoning_item = TurnItem::Reasoning(ReasoningItem {
        id: "reasoning-1".to_string(),
        summary_text: vec!["line one".to_string(), "line two".to_string()],
        raw_content: vec![],
    });

    assert_eq!(
        ThreadItem::from(reasoning_item),
        ThreadItem::Reasoning {
            id: "reasoning-1".to_string(),
            summary: vec!["line one".to_string(), "line two".to_string()],
            content: vec![],
        }
    );

    let search_item = TurnItem::WebSearch(WebSearchItem {
        id: "search-1".to_string(),
        query: "docs".to_string(),
        action: CoreWebSearchAction::Search {
            query: Some("docs".to_string()),
            queries: None,
        },
    });

    assert_eq!(
        ThreadItem::from(search_item),
        ThreadItem::WebSearch {
            id: "search-1".to_string(),
            query: "docs".to_string(),
            action: Some(WebSearchAction::Search {
                query: Some("docs".to_string()),
                queries: None,
            }),
        }
    );

    let image_view_item = TurnItem::ImageView(ImageViewItem {
        id: "view-image-1".to_string(),
        path: test_path_buf("/tmp/view-image.png").abs(),
    });

    assert_eq!(
        ThreadItem::from(image_view_item),
        ThreadItem::ImageView {
            id: "view-image-1".to_string(),
            path: test_path_buf("/tmp/view-image.png").abs(),
        }
    );

    let file_change_item = TurnItem::FileChange(FileChangeItem {
        id: "patch-1".to_string(),
        changes: [(
            PathBuf::from("README.md"),
            codex_protocol::protocol::FileChange::Add {
                content: "hello\n".to_string(),
            },
        )]
        .into_iter()
        .collect(),
        status: Some(codex_protocol::protocol::PatchApplyStatus::Completed),
        auto_approved: None,
        stdout: Some("Done!".to_string()),
        stderr: Some(String::new()),
    });

    assert_eq!(
        ThreadItem::from(file_change_item),
        ThreadItem::FileChange {
            id: "patch-1".to_string(),
            changes: vec![FileUpdateChange {
                path: "README.md".to_string(),
                kind: PatchChangeKind::Add,
                diff: "hello\n".to_string(),
            }],
            status: PatchApplyStatus::Completed,
        }
    );

    let mcp_tool_call_item = TurnItem::McpToolCall(McpToolCallItem {
        id: "mcp-1".to_string(),
        server: "server".to_string(),
        tool: "tool".to_string(),
        arguments: json!({"arg": "value"}),
        mcp_app_resource_uri: Some("app://connector".to_string()),
        plugin_id: Some("sample@test".to_string()),
        status: CoreMcpToolCallStatus::InProgress,
        result: None,
        error: None,
        duration: None,
    });

    assert_eq!(
        ThreadItem::from(mcp_tool_call_item),
        ThreadItem::McpToolCall {
            id: "mcp-1".to_string(),
            server: "server".to_string(),
            tool: "tool".to_string(),
            status: McpToolCallStatus::InProgress,
            arguments: json!({"arg": "value"}),
            mcp_app_resource_uri: Some("app://connector".to_string()),
            plugin_id: Some("sample@test".to_string()),
            result: None,
            error: None,
            duration_ms: None,
        }
    );

    let completed_mcp_tool_call_item = TurnItem::McpToolCall(McpToolCallItem {
        id: "mcp-2".to_string(),
        server: "server".to_string(),
        tool: "tool".to_string(),
        arguments: JsonValue::Null,
        mcp_app_resource_uri: None,
        plugin_id: None,
        status: CoreMcpToolCallStatus::Completed,
        result: Some(CallToolResult {
            content: vec![json!({"type": "text", "text": "ok"})],
            structured_content: Some(json!({"ok": true})),
            is_error: Some(false),
            meta: Some(json!({"trace": "1"})),
        }),
        error: None,
        duration: Some(Duration::from_millis(42)),
    });

    assert_eq!(
        ThreadItem::from(completed_mcp_tool_call_item),
        ThreadItem::McpToolCall {
            id: "mcp-2".to_string(),
            server: "server".to_string(),
            tool: "tool".to_string(),
            status: McpToolCallStatus::Completed,
            arguments: JsonValue::Null,
            mcp_app_resource_uri: None,
            plugin_id: None,
            result: Some(Box::new(McpToolCallResult {
                content: vec![json!({"type": "text", "text": "ok"})],
                structured_content: Some(json!({"ok": true})),
                meta: Some(json!({"trace": "1"})),
            })),
            error: None,
            duration_ms: Some(42),
        }
    );
}

#[test]
fn user_input_into_core_preserves_image_detail() {
    assert_eq!(
        UserInput::Image {
            url: "https://example.com/image.png".to_string(),
            detail: Some(ImageDetail::Original),
        }
        .into_core(),
        CoreUserInput::Image {
            image_url: "https://example.com/image.png".to_string(),
            detail: Some(ImageDetail::Original),
        }
    );

    assert_eq!(
        UserInput::LocalImage {
            path: PathBuf::from("local/image.png"),
            detail: Some(ImageDetail::Original),
        }
        .into_core(),
        CoreUserInput::LocalImage {
            path: PathBuf::from("local/image.png"),
            detail: Some(ImageDetail::Original),
        }
    );
}
