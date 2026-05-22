use crate::realtime_conversation_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_uses_experimental_realtime_ws_startup_context_override() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let startup_server = start_websocket_server(vec![vec![]]).await;
    let realtime_server = start_websocket_server(vec![vec![vec![json!({
        "type": "session.updated",
        "session": { "id": "sess_custom_context", "instructions": "prompt from config" }
    })]]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
            config.experimental_realtime_ws_backend_prompt = Some("prompt from config".to_string());
            config.experimental_realtime_ws_startup_context =
                Some("custom startup context".to_string());
        }
    });
    let test = builder.build_with_websocket_server(&startup_server).await?;
    seed_recent_thread(
        &test,
        "Recent work: cleaned up startup flows and reviewed websocket routing.",
        "Investigate realtime startup context",
        "custom-context",
    )
    .await?;
    fs::create_dir_all(test.workspace_path("docs"))?;
    fs::write(test.workspace_path("README.md"), "workspace marker")?;
    assert!(
        startup_server
            .wait_for_handshakes(/*expected*/ 1, Duration::from_secs(2))
            .await
    );

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("prompt from op".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let startup_context_request = wait_for_matching_websocket_request(
        &realtime_server,
        "startup context request with instructions",
        |request| websocket_request_instructions(request).is_some(),
    )
    .await;
    let instructions = websocket_request_instructions(&startup_context_request)
        .expect("custom startup context request should contain instructions");

    assert_eq!(instructions, "prompt from config\n\ncustom startup context");
    assert!(!instructions.contains(STARTUP_CONTEXT_HEADER));
    assert!(!instructions.contains("## Machine / Workspace Map"));

    startup_server.shutdown().await;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_disables_realtime_startup_context_with_empty_override() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let startup_server = start_websocket_server(vec![vec![]]).await;
    let realtime_server = start_websocket_server(vec![vec![vec![json!({
        "type": "session.updated",
        "session": { "id": "sess_no_context", "instructions": "prompt from config" }
    })]]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
            config.experimental_realtime_ws_backend_prompt = Some("prompt from config".to_string());
            config.experimental_realtime_ws_startup_context = Some(String::new());
        }
    });
    let test = builder.build_with_websocket_server(&startup_server).await?;
    seed_recent_thread(
        &test,
        "Recent work: cleaned up startup flows and reviewed websocket routing.",
        "Investigate realtime startup context",
        "no-context",
    )
    .await?;
    fs::create_dir_all(test.workspace_path("docs"))?;
    fs::write(test.workspace_path("README.md"), "workspace marker")?;
    assert!(
        startup_server
            .wait_for_handshakes(/*expected*/ 1, Duration::from_secs(2))
            .await
    );

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("prompt from op".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let startup_context_request = wait_for_matching_websocket_request(
        &realtime_server,
        "startup context disable request with instructions",
        |request| websocket_request_instructions(request).is_some(),
    )
    .await;
    let instructions = websocket_request_instructions(&startup_context_request)
        .expect("startup context disable request should contain instructions");

    assert_eq!(instructions, "prompt from config");
    assert!(!instructions.contains(STARTUP_CONTEXT_HEADER));
    assert!(!instructions.contains("## Machine / Workspace Map"));

    startup_server.shutdown().await;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_start_injects_startup_context_from_thread_history() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let startup_server = start_websocket_server(vec![vec![]]).await;
    let realtime_server = start_websocket_server(vec![vec![vec![json!({
        "type": "session.updated",
        "session": { "id": "sess_context", "instructions": "backend prompt" }
    })]]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_websocket_server(&startup_server).await?;
    seed_recent_thread(
        &test,
        "Recent work: cleaned up startup flows and reviewed websocket routing.",
        "Investigate realtime startup context",
        "latest",
    )
    .await?;
    fs::create_dir_all(test.workspace_path("docs"))?;
    fs::write(test.workspace_path("README.md"), "workspace marker")?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let startup_context_request = wait_for_matching_websocket_request(
        &realtime_server,
        "startup context request with instructions",
        |request| websocket_request_instructions(request).is_some(),
    )
    .await;
    let startup_context = websocket_request_instructions(&startup_context_request)
        .expect("startup context request should contain instructions");

    assert!(startup_context.contains(STARTUP_CONTEXT_OPEN_TAG));
    assert!(startup_context.contains(STARTUP_CONTEXT_CLOSE_TAG));
    assert!(startup_context.contains(STARTUP_CONTEXT_HEADER));
    assert!(!startup_context.contains("## User"));
    assert!(startup_context.contains("### "));
    assert!(startup_context.contains("Recent sessions: 1"));
    assert!(startup_context.contains("Latest branch: branch-latest"));
    assert!(startup_context.contains("User asks:"));
    assert!(startup_context.contains("Investigate realtime startup context"));
    assert!(startup_context.contains("## Machine / Workspace Map"));
    assert!(startup_context.contains("README.md"));
    assert!(!startup_context.contains(MEMORY_PROMPT_PHRASE));

    startup_server.shutdown().await;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_startup_context_current_thread_selects_many_turns_by_budget() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let api_server = start_mock_server().await;
    let realtime_server = start_websocket_server(vec![vec![vec![json!({
        "type": "session.updated",
        "session": { "id": "sess_current_thread_budget", "instructions": "backend prompt" }
    })]]])
    .await;

    let latest_long_user_turn = format!(
        "latest-long-start {} latest-long-middle {} latest-long-end",
        "head detail ".repeat(120),
        "tail detail ".repeat(170),
    );
    let mut user_turns = (1..=7)
        .map(|index| {
            format!(
                "short-turn-{index}-start {} short-turn-{index}-end",
                "detail ".repeat(86)
            )
        })
        .collect::<Vec<_>>();
    user_turns.push(latest_long_user_turn.clone());

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build(&api_server).await?;

    // Seed completed turns through a resumed thread so this remains an
    // end-to-end startup-context test without paying for a model turn per
    // fixture entry in platform CI.
    let history = user_turns
        .into_iter()
        .enumerate()
        .flat_map(|(index, user_turn)| {
            let turn_number = index + 1;
            let assistant_turn = format!("assistant turn {turn_number}");
            [
                RolloutItem::ResponseItem(ResponseItem::Message {
                    id: None,
                    role: "user".to_string(),
                    content: vec![ContentItem::InputText { text: user_turn }],
                    phase: None,
                }),
                RolloutItem::ResponseItem(ResponseItem::Message {
                    id: None,
                    role: "assistant".to_string(),
                    content: vec![ContentItem::OutputText {
                        text: assistant_turn,
                    }],
                    phase: None,
                }),
            ]
        })
        .collect::<Vec<_>>();
    test.codex.shutdown_and_wait().await?;
    let resumed_thread = test
        .thread_manager
        .resume_thread_with_history(
            test.config.clone(),
            InitialHistory::Forked(history),
            auth_manager_from_auth(CodexAuth::from_api_key("dummy")),
            /*persist_extended_history*/ false,
            /*parent_trace*/ None,
        )
        .await?;
    let codex = resumed_thread.thread;

    codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let startup_context_request = wait_for_matching_websocket_request(
        &realtime_server,
        "current thread budget startup context request with instructions",
        |request| websocket_request_instructions(request).is_some(),
    )
    .await;
    let startup_context = websocket_request_instructions(&startup_context_request)
        .expect("startup context request should contain instructions");

    // Isolate only the Current Thread section; the startup prompt may also include
    // workspace and notes sections after it.
    let current_thread_start = startup_context
        .find("## Current Thread")
        .expect("startup context should include current thread section");
    let current_thread_and_rest = &startup_context[current_thread_start..];
    let current_thread_end = [
        "\n## Recent Work",
        "\n## Machine / Workspace Map",
        "\n## Notes",
    ]
    .iter()
    .filter_map(|marker| current_thread_and_rest.find(marker))
    .min()
    .unwrap_or(current_thread_and_rest.len());
    let current_thread = &current_thread_and_rest[..current_thread_end];

    let rendered_turns = current_thread
        .split("\n### ")
        .skip(1)
        .map(|turn| format!("### {turn}"))
        .collect::<Vec<_>>();
    let over_budget_turns = rendered_turns
        .iter()
        .filter_map(|turn| {
            let token_count = turn.len().div_ceil(4);
            (token_count > 300).then(|| {
                (
                    turn.lines().next().unwrap_or_default().to_string(),
                    token_count,
                )
            })
        })
        .collect::<Vec<_>>();
    let latest_rendered_source =
        format!("### Latest turn\nUser:\n{latest_long_user_turn}\n\nAssistant:\nassistant turn 8");

    // Snapshot the actual section so turn order, oldest-first omission, and
    // start/end truncation behavior are reviewed together.
    let snapshot = format!(
        "latest_source_tokens: {}\nrendered_turn_count: {}\nover_budget_turns: {over_budget_turns:?}\n\n{current_thread}",
        latest_rendered_source.len().div_ceil(4),
        rendered_turns.len(),
    );
    insta::assert_snapshot!(
        "conversation_startup_context_current_thread_selects_many_turns_by_budget",
        snapshot
    );

    // The input includes a turn over 300 approximate tokens, and every rendered
    // turn still fits the per-turn cap after labels and truncation markers.
    assert_eq!(
        (
            latest_rendered_source.len().div_ceil(4) > 300,
            over_budget_turns,
        ),
        (true, Vec::<(String, usize)>::new()),
    );

    codex.shutdown_and_wait().await?;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_startup_context_falls_back_to_workspace_map() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let startup_server = start_websocket_server(vec![vec![]]).await;
    let realtime_server = start_websocket_server(vec![vec![vec![json!({
        "type": "session.updated",
        "session": { "id": "sess_workspace", "instructions": "backend prompt" }
    })]]])
    .await;

    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_websocket_server(&startup_server).await?;
    fs::create_dir_all(test.workspace_path("codex-rs/core"))?;
    fs::write(test.workspace_path("notes.txt"), "workspace marker")?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let startup_context_request = wait_for_matching_websocket_request(
        &realtime_server,
        "workspace-map startup context request with instructions",
        |request| websocket_request_instructions(request).is_some(),
    )
    .await;
    let startup_context = websocket_request_instructions(&startup_context_request)
        .expect("startup context request should contain instructions");

    assert!(startup_context.contains(STARTUP_CONTEXT_OPEN_TAG));
    assert!(startup_context.contains(STARTUP_CONTEXT_CLOSE_TAG));
    assert!(startup_context.contains(STARTUP_CONTEXT_HEADER));
    assert!(startup_context.contains("## Machine / Workspace Map"));
    assert!(startup_context.contains("notes.txt"));
    assert!(startup_context.contains("codex-rs/"));

    startup_server.shutdown().await;
    realtime_server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conversation_startup_context_is_truncated_and_sent_once_per_start() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let startup_server = start_websocket_server(vec![vec![]]).await;
    let realtime_server = start_websocket_server(vec![vec![
        vec![json!({
            "type": "session.updated",
            "session": { "id": "sess_truncated", "instructions": "backend prompt" }
        })],
        vec![],
    ]])
    .await;

    let oversized_summary = "recent work ".repeat(3_500);
    let mut builder = test_codex().with_config({
        let realtime_base_url = realtime_server.uri().to_string();
        move |config| {
            config.experimental_realtime_ws_base_url = Some(realtime_base_url);
            config.realtime.version = RealtimeWsVersion::V1;
        }
    });
    let test = builder.build_with_websocket_server(&startup_server).await?;
    seed_recent_thread(&test, &oversized_summary, "summary", "oversized").await?;
    fs::write(test.workspace_path("marker.txt"), "marker")?;

    test.codex
        .submit(Op::RealtimeConversationStart(ConversationStartParams {
            output_modality: RealtimeOutputModality::Audio,
            prompt: Some(Some("backend prompt".to_string())),
            realtime_session_id: None,
            transport: None,
            voice: None,
        }))
        .await?;

    let startup_context_request = wait_for_matching_websocket_request(
        &realtime_server,
        "truncated startup context request with instructions",
        |request| websocket_request_instructions(request).is_some(),
    )
    .await;
    let startup_context = websocket_request_instructions(&startup_context_request)
        .expect("startup context request should contain instructions");
    assert!(startup_context.contains(STARTUP_CONTEXT_OPEN_TAG));
    assert!(startup_context.contains(STARTUP_CONTEXT_CLOSE_TAG));
    assert!(startup_context.contains(STARTUP_CONTEXT_HEADER));
    assert!(startup_context.len() <= 20_500);

    test.codex
        .submit(Op::RealtimeConversationText(ConversationTextParams {
            text: "hello".to_string(),
        }))
        .await?;

    let explicit_text_request = wait_for_matching_websocket_request(
        &realtime_server,
        "explicit realtime text request",
        |request| websocket_request_text(request).as_deref() == Some("hello"),
    )
    .await;
    assert_eq!(
        websocket_request_text(&explicit_text_request),
        Some("hello".to_string())
    );

    startup_server.shutdown().await;
    realtime_server.shutdown().await;
    Ok(())
}
