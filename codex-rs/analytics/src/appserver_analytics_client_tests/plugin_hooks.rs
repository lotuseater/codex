use super::common::*;
use super::*;
use pretty_assertions::assert_eq;

#[test]
fn plugin_used_event_serializes_expected_shape() {
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-3".to_string(),
        turn_id: "turn-3".to_string(),
    };
    let event = TrackEventRequest::PluginUsed(CodexPluginUsedEventRequest {
        event_type: "codex_plugin_used",
        event_params: codex_plugin_used_metadata(&tracking, sample_plugin_metadata()),
    });

    let payload = serde_json::to_value(&event).expect("serialize plugin used event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_plugin_used",
            "event_params": {
                "plugin_id": "sample@test",
                "plugin_name": "sample",
                "marketplace_name": "test",
                "has_skills": true,
                "mcp_server_count": 2,
                "connector_ids": ["calendar", "drive"],
                "product_client_id": originator().value,
                "thread_id": "thread-3",
                "turn_id": "turn-3",
                "model_slug": "gpt-5"
            }
        })
    );
}

#[test]
fn plugin_management_event_serializes_expected_shape() {
    let event = TrackEventRequest::PluginInstalled(CodexPluginEventRequest {
        event_type: "codex_plugin_installed",
        event_params: codex_plugin_metadata(sample_plugin_metadata()),
    });

    let payload = serde_json::to_value(&event).expect("serialize plugin installed event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_plugin_installed",
            "event_params": {
                "plugin_id": "sample@test",
                "plugin_name": "sample",
                "marketplace_name": "test",
                "has_skills": true,
                "mcp_server_count": 2,
                "connector_ids": ["calendar", "drive"],
                "product_client_id": originator().value
            }
        })
    );
}

#[test]
fn plugin_management_event_can_use_remote_plugin_id_override() {
    let mut plugin = sample_plugin_metadata();
    plugin.remote_plugin_id = Some("plugins~Plugin_remote".to_string());
    let event = TrackEventRequest::PluginInstalled(CodexPluginEventRequest {
        event_type: "codex_plugin_installed",
        event_params: codex_plugin_metadata(plugin),
    });

    let payload = serde_json::to_value(&event).expect("serialize plugin installed event");

    assert_eq!(
        payload["event_params"]["plugin_id"],
        "plugins~Plugin_remote"
    );
    assert_eq!(payload["event_params"]["plugin_name"], "sample");
    assert_eq!(payload["event_params"]["marketplace_name"], "test");
}

#[test]
fn hook_run_event_serializes_expected_shape() {
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-3".to_string(),
        turn_id: "turn-3".to_string(),
    };
    let event = TrackEventRequest::HookRun(CodexHookRunEventRequest {
        event_type: "codex_hook_run",
        event_params: codex_hook_run_metadata(
            &tracking,
            HookRunFact {
                event_name: HookEventName::PreToolUse,
                hook_source: HookSource::User,
                status: HookRunStatus::Completed,
            },
        ),
    });

    let payload = serde_json::to_value(&event).expect("serialize hook run event");

    assert_eq!(
        payload,
        json!({
            "event_type": "codex_hook_run",
            "event_params": {
                "thread_id": "thread-3",
                "turn_id": "turn-3",
                "model_slug": "gpt-5",
                "hook_name": "PreToolUse",
                "hook_source": "user",
                "status": "completed"
            }
        })
    );
}

#[test]
fn hook_run_metadata_maps_sources_and_statuses() {
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
    };

    let system = serde_json::to_value(codex_hook_run_metadata(
        &tracking,
        HookRunFact {
            event_name: HookEventName::SessionStart,
            hook_source: HookSource::System,
            status: HookRunStatus::Completed,
        },
    ))
    .expect("serialize system hook");
    let project = serde_json::to_value(codex_hook_run_metadata(
        &tracking,
        HookRunFact {
            event_name: HookEventName::Stop,
            hook_source: HookSource::Project,
            status: HookRunStatus::Blocked,
        },
    ))
    .expect("serialize project hook");
    let cloud_requirements = serde_json::to_value(codex_hook_run_metadata(
        &tracking,
        HookRunFact {
            event_name: HookEventName::Stop,
            hook_source: HookSource::CloudRequirements,
            status: HookRunStatus::Blocked,
        },
    ))
    .expect("serialize cloud requirements hook");
    let unknown = serde_json::to_value(codex_hook_run_metadata(
        &tracking,
        HookRunFact {
            event_name: HookEventName::UserPromptSubmit,
            hook_source: HookSource::Unknown,
            status: HookRunStatus::Failed,
        },
    ))
    .expect("serialize unknown hook");

    assert_eq!(system["hook_source"], "system");
    assert_eq!(system["status"], "completed");
    assert_eq!(project["hook_source"], "project");
    assert_eq!(project["status"], "blocked");
    assert_eq!(cloud_requirements["hook_source"], "cloud_requirements");
    assert_eq!(cloud_requirements["status"], "blocked");
    assert_eq!(unknown["hook_source"], "unknown");
    assert_eq!(unknown["status"], "failed");
}

#[test]
fn hook_run_metadata_maps_stopped_status() {
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
    };

    let stopped = serde_json::to_value(codex_hook_run_metadata(
        &tracking,
        HookRunFact {
            event_name: HookEventName::Stop,
            hook_source: HookSource::User,
            status: HookRunStatus::Stopped,
        },
    ))
    .expect("serialize stopped hook");

    assert_eq!(stopped["hook_source"], "user");
    assert_eq!(stopped["status"], "stopped");
}

#[tokio::test]
async fn reducer_ingests_skill_invoked_fact() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
    };
    let skill_path = PathBuf::from("/Users/abc/.codex/skills/doc/SKILL.md");
    let expected_skill_id = skill_id_for_local_skill(
        /*repo_url*/ None,
        /*repo_root*/ None,
        skill_path.as_path(),
        "doc",
    );

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::SkillInvoked(SkillInvokedInput {
                tracking,
                invocations: vec![SkillInvocation {
                    skill_name: "doc".to_string(),
                    skill_scope: codex_protocol::protocol::SkillScope::User,
                    skill_path,
                    plugin_id: None,
                    invocation_type: InvocationType::Explicit,
                }],
            })),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(
        payload,
        json!([{
            "event_type": "skill_invocation",
            "skill_id": expected_skill_id,
            "skill_name": "doc",
            "event_params": {
                "product_client_id": originator().value,
                "skill_scope": "user",
                "plugin_id": null,
                "repo_url": null,
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "invoke_type": "explicit",
                "model_slug": "gpt-5"
            }
        }])
    );
}

#[tokio::test]
async fn reducer_includes_plugin_id_for_plugin_skill_invocations() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
    };
    let skill_path =
        PathBuf::from("/Users/abc/.codex/plugins/cache/test/sample/skills/doc/SKILL.md");

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::SkillInvoked(SkillInvokedInput {
                tracking,
                invocations: vec![SkillInvocation {
                    skill_name: "sample:doc".to_string(),
                    skill_scope: codex_protocol::protocol::SkillScope::User,
                    skill_path,
                    plugin_id: Some("sample@test".to_string()),
                    invocation_type: InvocationType::Explicit,
                }],
            })),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(
        payload[0]["event_params"]["plugin_id"],
        json!("sample@test")
    );
}

#[tokio::test]
async fn reducer_ingests_hook_run_fact() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::HookRun(HookRunInput {
                tracking: TrackEventsContext {
                    model_slug: "gpt-5".to_string(),
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                },
                hook: HookRunFact {
                    event_name: HookEventName::PostToolUse,
                    hook_source: HookSource::Unknown,
                    status: HookRunStatus::Failed,
                },
            })),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(payload.as_array().expect("events array").len(), 1);
    assert_eq!(payload[0]["event_type"], "codex_hook_run");
    assert_eq!(payload[0]["event_params"]["hook_name"], "PostToolUse");
    assert_eq!(payload[0]["event_params"]["hook_source"], "unknown");
    assert_eq!(payload[0]["event_params"]["status"], "failed");
}

#[tokio::test]
async fn reducer_ingests_app_and_plugin_facts() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();
    let tracking = TrackEventsContext {
        model_slug: "gpt-5".to_string(),
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
    };

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::AppMentioned(AppMentionedInput {
                tracking: tracking.clone(),
                mentions: vec![AppInvocation {
                    connector_id: Some("calendar".to_string()),
                    app_name: Some("Calendar".to_string()),
                    invocation_type: Some(InvocationType::Explicit),
                }],
            })),
            &mut events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::AppUsed(AppUsedInput {
                tracking: tracking.clone(),
                app: AppInvocation {
                    connector_id: Some("drive".to_string()),
                    app_name: Some("Drive".to_string()),
                    invocation_type: Some(InvocationType::Implicit),
                },
            })),
            &mut events,
        )
        .await;
    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::PluginUsed(PluginUsedInput {
                tracking,
                plugin: sample_plugin_metadata(),
            })),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(payload.as_array().expect("events array").len(), 3);
    assert_eq!(payload[0]["event_type"], "codex_app_mentioned");
    assert_eq!(payload[1]["event_type"], "codex_app_used");
    assert_eq!(payload[2]["event_type"], "codex_plugin_used");
}

#[tokio::test]
async fn reducer_ingests_plugin_state_changed_fact() {
    let mut reducer = AnalyticsReducer::default();
    let mut events = Vec::new();

    reducer
        .ingest(
            AnalyticsFact::Custom(CustomAnalyticsFact::PluginStateChanged(
                PluginStateChangedInput {
                    plugin: sample_plugin_metadata(),
                    state: PluginState::Disabled,
                },
            )),
            &mut events,
        )
        .await;

    let payload = serde_json::to_value(&events).expect("serialize events");
    assert_eq!(
        payload,
        json!([{
            "event_type": "codex_plugin_disabled",
            "event_params": {
                "plugin_id": "sample@test",
                "plugin_name": "sample",
                "marketplace_name": "test",
                "has_skills": true,
                "mcp_server_count": 2,
                "connector_ids": ["calendar", "drive"],
                "product_client_id": originator().value
            }
        }])
    );
}
