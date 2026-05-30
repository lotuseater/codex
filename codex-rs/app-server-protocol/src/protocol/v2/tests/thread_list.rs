use super::*;

#[test]
fn turn_defaults_legacy_missing_items_view_to_full() {
    let turn: Turn = serde_json::from_value(json!({
        "id": "turn_123",
        "items": [],
        "status": "completed",
        "error": null,
        "startedAt": null,
        "completedAt": null,
        "durationMs": null,
    }))
    .expect("legacy turn should deserialize");

    assert_eq!(turn.items_view, TurnItemsView::Full);
}

#[test]
fn thread_turns_list_params_accepts_items_view() {
    let params = serde_json::from_value::<ThreadTurnsListParams>(json!({
        "threadId": "thr_123",
        "cursor": null,
        "limit": 25,
        "sortDirection": "desc",
        "itemsView": "notLoaded",
    }))
    .expect("thread turns list params should deserialize");

    assert_eq!(params.thread_id, "thr_123");
    assert_eq!(params.items_view, Some(TurnItemsView::NotLoaded));
}

#[test]
fn thread_turns_items_list_round_trips() {
    let params = ThreadTurnsItemsListParams {
        thread_id: "thr_123".to_string(),
        turn_id: "turn_456".to_string(),
        cursor: Some("cursor_1".to_string()),
        limit: Some(50),
        sort_direction: Some(SortDirection::Asc),
    };

    assert_eq!(
        serde_json::to_value(&params).expect("serialize params"),
        json!({
            "threadId": "thr_123",
            "turnId": "turn_456",
            "cursor": "cursor_1",
            "limit": 50,
            "sortDirection": "asc",
        })
    );
    let response = ThreadTurnsItemsListResponse {
        data: vec![ThreadItem::ContextCompaction {
            id: "item_1".to_string(),
        }],
        next_cursor: None,
        backwards_cursor: Some("cursor_0".to_string()),
    };

    assert_eq!(
        serde_json::to_value(&response).expect("serialize response"),
        json!({
            "data": [{"type": "contextCompaction", "id": "item_1"}],
            "nextCursor": null,
            "backwardsCursor": "cursor_0",
        })
    );
}

#[test]
fn thread_list_params_accepts_single_cwd() {
    let params = serde_json::from_value::<ThreadListParams>(json!({
        "cwd": "/workspace",
    }))
    .expect("single cwd should deserialize");

    assert_eq!(
        params.cwd,
        Some(ThreadListCwdFilter::One("/workspace".to_string()))
    );
    assert!(!params.use_state_db_only);
}

#[test]
fn thread_list_params_accepts_multiple_cwds() {
    let params = serde_json::from_value::<ThreadListParams>(json!({
        "cwd": ["/workspace", "/other-workspace"],
    }))
    .expect("cwd array should deserialize");

    assert_eq!(
        params.cwd,
        Some(ThreadListCwdFilter::Many(vec![
            "/workspace".to_string(),
            "/other-workspace".to_string(),
        ]))
    );
}

#[test]
fn thread_list_params_accepts_state_db_only_flag() {
    let params = serde_json::from_value::<ThreadListParams>(json!({
        "useStateDbOnly": true,
    }))
    .expect("state db only flag should deserialize");

    assert!(params.use_state_db_only);
}

#[test]
fn collab_agent_state_maps_interrupted_status() {
    assert_eq!(
        CollabAgentState::from(CoreAgentStatus::Interrupted),
        CollabAgentState {
            status: CollabAgentStatus::Interrupted,
            message: None,
        }
    );
}

#[test]
fn external_agent_config_plugins_details_round_trip() {
    let item: ExternalAgentConfigMigrationItem = serde_json::from_value(json!({
        "itemType": "PLUGINS",
        "description": "Install supported plugins from Claude settings",
        "cwd": absolute_path_string("repo"),
        "details": {
            "plugins": [
                {
                    "marketplaceName": "team-marketplace",
                    "pluginNames": ["asana"]
                }
            ]
        }
    }))
    .expect("plugins migration item should deserialize");

    assert_eq!(
        item,
        ExternalAgentConfigMigrationItem {
            item_type: ExternalAgentConfigMigrationItemType::Plugins,
            description: "Install supported plugins from Claude settings".to_string(),
            cwd: Some(PathBuf::from(absolute_path_string("repo"))),
            details: Some(MigrationDetails {
                plugins: vec![PluginsMigration {
                    marketplace_name: "team-marketplace".to_string(),
                    plugin_names: vec!["asana".to_string()],
                }],
                ..Default::default()
            }),
        }
    );
}

#[test]
fn external_agent_config_import_params_accept_legacy_plugin_details() {
    let params: ExternalAgentConfigImportParams = serde_json::from_value(json!({
        "migrationItems": [{
            "itemType": "PLUGINS",
            "description": "Install supported plugins from Claude settings",
            "cwd": absolute_path_string("repo"),
            "details": {
                "plugins": [
                    {
                        "marketplaceName": "team-marketplace",
                        "pluginNames": ["asana"]
                    }
                ]
            }
        }]
    }))
    .expect("legacy plugin import params should deserialize");

    assert_eq!(
        params,
        ExternalAgentConfigImportParams {
            migration_items: vec![ExternalAgentConfigMigrationItem {
                item_type: ExternalAgentConfigMigrationItemType::Plugins,
                description: "Install supported plugins from Claude settings".to_string(),
                cwd: Some(PathBuf::from(absolute_path_string("repo"))),
                details: Some(MigrationDetails {
                    plugins: vec![PluginsMigration {
                        marketplace_name: "team-marketplace".to_string(),
                        plugin_names: vec!["asana".to_string()],
                    }],
                    ..Default::default()
                }),
            }],
        }
    );
}
