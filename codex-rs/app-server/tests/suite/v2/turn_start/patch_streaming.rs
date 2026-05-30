use super::*;

#[tokio::test]
async fn turn_start_does_not_stream_apply_patch_change_updates_without_feature_v2() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let tmp = TempDir::new()?;
    let codex_home = tmp.path().join("codex_home");
    std::fs::create_dir(&codex_home)?;
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir(&workspace)?;

    let call_id = "patch-call";
    let item_id = "fc-patch-call";
    let patch = "*** Begin Patch\n*** Add File: live.txt\n+live line\n*** End Patch\n";
    let patch_delta_1 = "*** Begin Patch\n*** Add File: live.txt\n+live";
    let patch_delta_2 = " line\n*** End Patch\n";
    let responses = vec![
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            serde_json::json!({
                "type": "response.output_item.added",
                "item": {
                    "type": "custom_tool_call",
                    "id": item_id,
                    "call_id": call_id,
                    "name": "apply_patch",
                    "input": "",
                    "status": "in_progress"
                }
            }),
            serde_json::json!({
                "type": "response.custom_tool_call_input.delta",
                "item_id": item_id,
                "call_id": call_id,
                "delta": patch_delta_1,
            }),
            serde_json::json!({
                "type": "response.custom_tool_call_input.delta",
                "item_id": item_id,
                "call_id": call_id,
                "delta": patch_delta_2,
            }),
            responses::ev_apply_patch_custom_tool_call(call_id, patch),
            responses::ev_completed("resp-1"),
        ]),
        create_final_assistant_message_sse_response("patch applied")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    create_config_toml(&codex_home, &server.uri(), "never", &BTreeMap::default())?;

    let mut mcp = McpProcess::new(&codex_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let start_req = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            cwd: Some(workspace.to_string_lossy().into_owned()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_req)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_resp)?;

    let turn_req = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id,
            input: vec![V2UserInput::Text {
                text: "apply patch".into(),
                text_elements: Vec::new(),
            }],
            cwd: Some(workspace),
            ..Default::default()
        })
        .await?;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_req)),
    )
    .await??;
    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    assert!(
        !mcp.pending_notification_methods()
            .iter()
            .any(|method| method == "item/fileChange/patchUpdated")
    );

    Ok(())
}

#[tokio::test]
async fn turn_start_streams_apply_patch_change_updates_v2() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let tmp = TempDir::new()?;
    let codex_home = tmp.path().join("codex_home");
    std::fs::create_dir(&codex_home)?;
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir(&workspace)?;

    let call_id = "patch-call";
    let item_id = "fc-patch-call";
    let patch = "*** Begin Patch\n*** Add File: live.txt\n+live line\n*** End Patch\n";
    let patch_delta_1 = "*** Begin Patch\n*** Add File: live.txt\n+live";
    let patch_delta_2 = " line\n*** End Patch\n";
    let responses = vec![
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            serde_json::json!({
                "type": "response.output_item.added",
                "item": {
                    "type": "function_call",
                    "id": "fc-other-call",
                    "call_id": "other-call",
                    "name": "not_apply_patch",
                    "arguments": "",
                    "status": "in_progress"
                }
            }),
            serde_json::json!({
                "type": "response.function_call_arguments.delta",
                "item_id": "fc-other-call",
                "delta": r#"{"input":"*** Begin Patch\n*** Add File: ignored.txt\n+ignored"#,
            }),
            serde_json::json!({
                "type": "response.output_item.added",
                "item": {
                    "type": "custom_tool_call",
                    "id": item_id,
                    "call_id": call_id,
                    "name": "apply_patch",
                    "input": "",
                    "status": "in_progress"
                }
            }),
            serde_json::json!({
                "type": "response.custom_tool_call_input.delta",
                "item_id": item_id,
                "call_id": call_id,
                "delta": patch_delta_1,
            }),
            serde_json::json!({
                "type": "response.custom_tool_call_input.delta",
                "item_id": item_id,
                "call_id": call_id,
                "delta": patch_delta_2,
            }),
            responses::ev_apply_patch_custom_tool_call(call_id, patch),
            responses::ev_completed("resp-1"),
        ]),
        create_final_assistant_message_sse_response("patch applied")?,
    ];
    let server = create_mock_responses_server_sequence(responses).await;
    create_config_toml(
        &codex_home,
        &server.uri(),
        "never",
        &BTreeMap::from([
            (Feature::ApplyPatchStreamingEvents, true),
            (Feature::Plugins, false),
            (Feature::RemoteModels, false),
            (Feature::ShellSnapshot, false),
        ]),
    )?;
    write_models_cache(&codex_home)?;
    let cache_path = codex_home.join("models_cache.json");
    let mut cache: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&cache_path)?)?;
    let models = cache["models"]
        .as_array_mut()
        .expect("models_cache.json models should be an array");
    let model = models
        .first_mut()
        .expect("models_cache.json should contain at least one model");
    model["slug"] = serde_json::Value::from("mock-model");
    model["display_name"] = serde_json::Value::from("mock-model");
    model["apply_patch_tool_type"] = serde_json::Value::from("freeform");
    std::fs::write(&cache_path, serde_json::to_string_pretty(&cache)?)?;

    let mut mcp = McpProcess::new(&codex_home).await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize()).await??;

    let start_req = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            cwd: Some(workspace.to_string_lossy().into_owned()),
            ..Default::default()
        })
        .await?;
    let start_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(start_req)),
    )
    .await??;
    let ThreadStartResponse { thread, .. } = to_response::<ThreadStartResponse>(start_resp)?;

    let turn_req = mcp
        .send_turn_start_request(TurnStartParams {
            thread_id: thread.id.clone(),
            input: vec![V2UserInput::Text {
                text: "apply patch".into(),
                text_elements: Vec::new(),
            }],
            cwd: Some(workspace.clone()),
            ..Default::default()
        })
        .await?;
    let turn_resp: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(turn_req)),
    )
    .await??;
    let TurnStartResponse { turn } = to_response::<TurnStartResponse>(turn_resp)?;

    let mut streamed_content = String::new();
    while streamed_content != "live line\n" {
        let delta_notif = timeout(
            DEFAULT_READ_TIMEOUT,
            mcp.read_stream_until_notification_message("item/fileChange/patchUpdated"),
        )
        .await??;
        let delta: FileChangePatchUpdatedNotification = serde_json::from_value(
            delta_notif
                .params
                .clone()
                .expect("item/fileChange/patchUpdated params"),
        )?;
        assert_eq!(delta.thread_id, thread.id);
        assert_eq!(delta.turn_id, turn.id);
        assert_eq!(delta.item_id, call_id);
        let change = delta
            .changes
            .iter()
            .find(|change| change.path == "live.txt")
            .expect("live.txt change");
        assert!(matches!(change.kind, PatchChangeKind::Add));
        streamed_content = change.diff.clone();
    }

    timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("turn/completed"),
    )
    .await??;

    Ok(())
}
