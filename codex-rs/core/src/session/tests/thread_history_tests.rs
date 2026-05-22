use super::*;

#[tokio::test]
async fn reconstruct_history_matches_live_compactions() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, expected) = sample_rollout(&session, &turn_context).await;

    let reconstruction_turn = session.new_default_turn().await;
    let reconstructed = session
        .reconstruct_history_from_rollout(reconstruction_turn.as_ref(), &rollout_items)
        .await;

    assert_eq!(expected, reconstructed.history);
}

#[tokio::test]
async fn reconstruct_history_uses_replacement_history_verbatim() {
    let (session, turn_context) = make_session_and_context().await;
    let summary_item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "summary".to_string(),
        }],
        phase: None,
    };
    let replacement_history = vec![
        summary_item.clone(),
        ResponseItem::Message {
            id: None,
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: "stale developer instructions".to_string(),
            }],
            phase: None,
        },
    ];
    let rollout_items = vec![RolloutItem::Compacted(CompactedItem {
        message: String::new(),
        replacement_history: Some(replacement_history.clone()),
    })];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.history, replacement_history);
}

#[tokio::test]
async fn record_initial_history_reconstructs_resumed_transcript() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, expected) = sample_rollout(&session, &turn_context).await;

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await;

    let history = session.state.lock().await.clone_history();
    assert_eq!(expected, history.raw_items());
}

#[tokio::test]
async fn record_initial_history_new_defers_initial_context_until_first_turn() {
    let (session, _turn_context) = make_session_and_context().await;

    session.record_initial_history(InitialHistory::New).await;

    let history = session.clone_history().await;
    assert_eq!(history.raw_items().to_vec(), Vec::<ResponseItem>::new());
    assert!(session.reference_context_item().await.is_none());
    assert_eq!(session.previous_turn_settings().await, None);
}

#[tokio::test]
async fn resumed_history_injects_initial_context_on_first_context_update_only() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, mut expected) = sample_rollout(&session, &turn_context).await;

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await;

    let history_before_seed = session.state.lock().await.clone_history();
    assert_eq!(expected, history_before_seed.raw_items());

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    expected.extend(session.build_initial_context(&turn_context).await);
    let history_after_seed = session.clone_history().await;
    assert_eq!(expected, history_after_seed.raw_items());

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    let history_after_second_seed = session.clone_history().await;
    assert_eq!(
        history_after_seed.raw_items(),
        history_after_second_seed.raw_items()
    );
}

#[tokio::test]
async fn record_initial_history_seeds_token_info_from_rollout() {
    let (session, turn_context) = make_session_and_context().await;
    let (mut rollout_items, _expected) = sample_rollout(&session, &turn_context).await;

    let info1 = TokenUsageInfo {
        total_token_usage: TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 0,
            output_tokens: 20,
            reasoning_output_tokens: 0,
            total_tokens: 30,
        },
        last_token_usage: TokenUsage {
            input_tokens: 3,
            cached_input_tokens: 0,
            output_tokens: 4,
            reasoning_output_tokens: 0,
            total_tokens: 7,
        },
        model_context_window: Some(1_000),
    };
    let info2 = TokenUsageInfo {
        total_token_usage: TokenUsage {
            input_tokens: 100,
            cached_input_tokens: 50,
            output_tokens: 200,
            reasoning_output_tokens: 25,
            total_tokens: 375,
        },
        last_token_usage: TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 0,
            output_tokens: 20,
            reasoning_output_tokens: 5,
            total_tokens: 35,
        },
        model_context_window: Some(2_000),
    };

    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: Some(info1),
            rate_limits: None,
        },
    )));
    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: None,
            rate_limits: None,
        },
    )));
    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: Some(info2.clone()),
            rate_limits: None,
        },
    )));
    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: None,
            rate_limits: None,
        },
    )));

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await;

    let actual = session.state.lock().await.token_info();
    assert_eq!(actual, Some(info2));
}

#[tokio::test]
async fn record_initial_history_reconstructs_forked_transcript() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, expected) = sample_rollout(&session, &turn_context).await;

    session
        .record_initial_history(InitialHistory::Forked(rollout_items))
        .await;

    let history = session.state.lock().await.clone_history();
    assert_eq!(expected, history.raw_items());
}

#[tokio::test]
async fn record_initial_history_forked_hydrates_previous_turn_settings() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "forked-rollout-model";
    let previous_context_item = TurnContextItem {
        turn_id: Some(turn_context.sub_id.clone()),
        trace_id: turn_context.trace_id.clone(),
        cwd: turn_context.cwd.to_path_buf(),
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort,
        summary: turn_context.reasoning_summary,
        user_instructions: None,
        developer_instructions: None,
        final_output_json_schema: None,
        truncation_policy: Some(turn_context.truncation_policy),
    };
    let turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "forked seed".to_string(),
                images: None,
                image_details: Vec::new(),
                local_images: Vec::new(),
                local_image_details: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(previous_context_item.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            codex_protocol::protocol::TurnCompleteEvent {
                turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Forked(rollout_items))
        .await;

    let history = session.clone_history().await;
    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(history.raw_items(), &[]);
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize fork reference context item"),
        serde_json::to_value(Some(previous_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn thread_rollback_drops_last_turn_from_history() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    let rollout_path = attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    let turn_1 = vec![
        user_message("turn 1 user"),
        assistant_message("turn 1 assistant"),
    ];
    let turn_2 = vec![
        user_message("turn 2 user"),
        assistant_message("turn 2 assistant"),
    ];
    let mut full_history = Vec::new();
    full_history.extend(initial_context.clone());
    full_history.extend(turn_1.clone());
    full_history.extend(turn_2);
    sess.replace_history(full_history.clone(), Some(tc.to_turn_context_item()))
        .await;
    let rollout_items: Vec<RolloutItem> = full_history
        .into_iter()
        .map(RolloutItem::ResponseItem)
        .collect();
    sess.persist_rollout_items(&rollout_items).await;
    sess.set_previous_turn_settings(Some(PreviousTurnSettings {
        model: "stale-model".to_string(),
        realtime_active: Some(tc.realtime_active),
    }))
    .await;
    {
        let mut state = sess.state.lock().await;
        state.set_reference_context_item(Some(tc.to_turn_context_item()));
    }

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;

    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 1);

    let mut expected = Vec::new();
    expected.extend(initial_context);
    expected.extend(turn_1);

    let history = sess.clone_history().await;
    assert_eq!(expected, history.raw_items());
    assert_eq!(sess.previous_turn_settings().await, None);
    assert!(sess.reference_context_item().await.is_none());

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    assert!(resumed.history.iter().any(|item| {
        matches!(
            item,
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(rollback))
            if rollback.num_turns == 1
        )
    }));
}

#[tokio::test]
async fn thread_rollback_clears_history_when_num_turns_exceeds_existing_turns() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    let turn_1 = vec![user_message("turn 1 user")];
    let mut full_history = Vec::new();
    full_history.extend(initial_context.clone());
    full_history.extend(turn_1);
    sess.replace_history(full_history.clone(), Some(tc.to_turn_context_item()))
        .await;
    let rollout_items: Vec<RolloutItem> = full_history
        .into_iter()
        .map(RolloutItem::ResponseItem)
        .collect();
    sess.persist_rollout_items(&rollout_items).await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 99).await;

    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 99);

    let history = sess.clone_history().await;
    assert_eq!(initial_context, history.raw_items());
}

#[tokio::test]
async fn thread_rollback_fails_without_persisted_thread_history() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    sess.record_into_history(&initial_context, tc.as_ref())
        .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;

    let error_event = wait_for_thread_rollback_failed(&rx).await;
    assert_eq!(
        error_event.message,
        "thread rollback requires persisted thread history"
    );
    assert_eq!(
        error_event.codex_error_info,
        Some(CodexErrorInfo::ThreadRollbackFailed)
    );
    assert_eq!(sess.clone_history().await.raw_items(), initial_context);
}

#[tokio::test]
async fn thread_rollback_recomputes_previous_turn_settings_and_reference_context_from_replay() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let first_context_item = tc.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let mut rolled_back_context_item = first_context_item.clone();
    rolled_back_context_item.turn_id = Some("rolled-back-turn".to_string());
    rolled_back_context_item.model = "rolled-back-model".to_string();
    let rolled_back_turn_id = rolled_back_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");
    let turn_two_assistant = assistant_message("turn 2 assistant");

    sess.persist_rollout_items(&[
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 1 user".to_string(),
                images: None,
                image_details: Vec::new(),
                local_images: Vec::new(),
                local_image_details: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: first_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: rolled_back_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            codex_protocol::protocol::UserMessageEvent {
                message: "turn 2 user".to_string(),
                images: None,
                image_details: Vec::new(),
                local_images: Vec::new(),
                local_image_details: Vec::new(),
                text_elements: Vec::new(),
            },
        )),
        RolloutItem::TurnContext(rolled_back_context_item),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::ResponseItem(turn_two_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: rolled_back_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
    ])
    .await;
    sess.replace_history(
        vec![assistant_message("stale history")],
        Some(first_context_item.clone()),
    )
    .await;
    sess.set_previous_turn_settings(Some(PreviousTurnSettings {
        model: "stale-model".to_string(),
        realtime_active: None,
    }))
    .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 1);

    assert_eq!(
        sess.clone_history().await.raw_items(),
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        sess.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: tc.model_info.slug.clone(),
            realtime_active: Some(tc.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(sess.reference_context_item().await)
            .expect("serialize replay reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn thread_rollback_restores_cleared_reference_context_item_after_compaction() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let first_context_item = tc.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("turn context should have turn_id");
    let compact_turn_id = "compact-turn".to_string();
    let rolled_back_turn_id = "rolled-back-turn".to_string();
    let compacted_history = vec![
        user_message("turn 1 user"),
        user_message("summary after compaction"),
    ];

    sess.persist_rollout_items(&[
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "turn 1 user".to_string(),
            images: None,
            image_details: Vec::new(),
            local_images: Vec::new(),
            local_image_details: Vec::new(),
            text_elements: Vec::new(),
        })),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(user_message("turn 1 user")),
        RolloutItem::ResponseItem(assistant_message("turn 1 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: first_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: compact_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: "summary after compaction".to_string(),
            replacement_history: Some(compacted_history.clone()),
        }),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: compact_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: rolled_back_turn_id.clone(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "turn 2 user".to_string(),
            images: None,
            image_details: Vec::new(),
            local_images: Vec::new(),
            local_image_details: Vec::new(),
            text_elements: Vec::new(),
        })),
        RolloutItem::TurnContext(TurnContextItem {
            turn_id: Some(rolled_back_turn_id.clone()),
            model: "rolled-back-model".to_string(),
            ..first_context_item.clone()
        }),
        RolloutItem::ResponseItem(user_message("turn 2 user")),
        RolloutItem::ResponseItem(assistant_message("turn 2 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: rolled_back_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
    ])
    .await;
    sess.replace_history(
        vec![assistant_message("stale history")],
        Some(first_context_item),
    )
    .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 1);

    assert_eq!(sess.clone_history().await.raw_items(), compacted_history);
    assert!(sess.reference_context_item().await.is_none());
}

#[tokio::test]
async fn thread_rollback_persists_marker_and_replays_cumulatively() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    let rollout_path = attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;
    let turn_context_item = tc.to_turn_context_item();

    sess.persist_rollout_items(&[
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: "turn-1".to_string(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "turn 1 user".to_string(),
            images: None,
            image_details: Vec::new(),
            local_images: Vec::new(),
            local_image_details: Vec::new(),
            text_elements: Vec::new(),
        })),
        RolloutItem::TurnContext(turn_context_item.clone()),
        RolloutItem::ResponseItem(user_message("turn 1 user")),
        RolloutItem::ResponseItem(assistant_message("turn 1 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: "turn-1".to_string(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: "turn-2".to_string(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "turn 2 user".to_string(),
            images: None,
            image_details: Vec::new(),
            local_images: Vec::new(),
            local_image_details: Vec::new(),
            text_elements: Vec::new(),
        })),
        RolloutItem::TurnContext(turn_context_item.clone()),
        RolloutItem::ResponseItem(user_message("turn 2 user")),
        RolloutItem::ResponseItem(assistant_message("turn 2 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: "turn-2".to_string(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            codex_protocol::protocol::TurnStartedEvent {
                turn_id: "turn-3".to_string(),
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            message: "turn 3 user".to_string(),
            images: None,
            image_details: Vec::new(),
            local_images: Vec::new(),
            local_image_details: Vec::new(),
            text_elements: Vec::new(),
        })),
        RolloutItem::TurnContext(turn_context_item),
        RolloutItem::ResponseItem(user_message("turn 3 user")),
        RolloutItem::ResponseItem(assistant_message("turn 3 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: "turn-3".to_string(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
    ])
    .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let first_rollback = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(first_rollback.num_turns, 1);
    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let second_rollback = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(second_rollback.num_turns, 1);

    assert_eq!(
        sess.clone_history().await.raw_items(),
        vec![
            user_message("turn 1 user"),
            assistant_message("turn 1 assistant")
        ]
    );

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let rollback_markers = resumed
        .history
        .iter()
        .filter(|item| matches!(item, RolloutItem::EventMsg(EventMsg::ThreadRolledBack(_))))
        .count();
    assert_eq!(rollback_markers, 2);
}

#[tokio::test]
async fn thread_rollback_fails_when_turn_in_progress() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    sess.record_into_history(&initial_context, tc.as_ref())
        .await;

    *sess.active_turn.lock().await = Some(crate::state::ActiveTurn::default());
    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;

    let error_event = wait_for_thread_rollback_failed(&rx).await;
    assert_eq!(
        error_event.codex_error_info,
        Some(CodexErrorInfo::ThreadRollbackFailed)
    );

    let history = sess.clone_history().await;
    assert_eq!(initial_context, history.raw_items());
}

#[tokio::test]
async fn thread_rollback_fails_when_num_turns_is_zero() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    sess.record_into_history(&initial_context, tc.as_ref())
        .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 0).await;

    let error_event = wait_for_thread_rollback_failed(&rx).await;
    assert_eq!(error_event.message, "num_turns must be >= 1");
    assert_eq!(
        error_event.codex_error_info,
        Some(CodexErrorInfo::ThreadRollbackFailed)
    );

    let history = sess.clone_history().await;
    assert_eq!(initial_context, history.raw_items());
}

#[tokio::test]
async fn build_initial_context_omits_default_image_save_location_with_image_history() {
    let (session, turn_context) = make_session_and_context().await;
    session
        .replace_history(
            vec![ResponseItem::ImageGenerationCall {
                id: "ig-test".to_string(),
                status: "completed".to_string(),
                revised_prompt: Some("a tiny blue square".to_string()),
                result: "Zm9v".to_string(),
            }],
            /*reference_context_item*/ None,
        )
        .await;

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);
    assert!(
        !developer_texts
            .iter()
            .any(|text| text.contains("Generated images are saved to")),
        "expected initial context to omit image save instructions even with image history, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_default_image_save_location_without_image_history() {
    let (session, turn_context) = make_session_and_context().await;

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);

    assert!(
        !developer_texts
            .iter()
            .any(|text| text.contains("Generated images are saved to")),
        "expected initial context to omit image save instructions without image history, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn handle_output_item_done_records_image_save_history_message() {
    let (session, turn_context) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "ig_history_records_message";
    let expected_saved_path = crate::stream_events_utils::image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.conversation_id.to_string(),
        call_id,
    );
    let _ = std::fs::remove_file(&expected_saved_path);
    let item = ResponseItem::ImageGenerationCall {
        id: call_id.to_string(),
        status: "completed".to_string(),
        revised_prompt: Some("a tiny blue square".to_string()),
        result: "Zm9v".to_string(),
    };

    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&session),
        turn_context: Arc::clone(&turn_context),
        turn_store: Arc::clone(&turn_context.extension_data),
        tool_runtime: test_tool_runtime(Arc::clone(&session), Arc::clone(&turn_context)),
        cancellation_token: CancellationToken::new(),
    };
    handle_output_item_done(&mut ctx, item.clone(), /*previously_active_item*/ None)
        .await
        .expect("image generation item should succeed");

    let history = session.clone_history().await;
    let image_output_path = crate::stream_events_utils::image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.conversation_id.to_string(),
        "<image_id>",
    );
    let image_output_dir = image_output_path
        .parent()
        .expect("generated image path should have a parent");
    let image_message: ResponseItem = crate::context::ContextualUserFragment::into(
        crate::context::ImageGenerationInstructions::new(
            image_output_dir.display(),
            image_output_path.display(),
        ),
    );
    assert_eq!(history.raw_items(), &[image_message, item]);
    assert_eq!(
        std::fs::read(&expected_saved_path).expect("saved file"),
        b"foo"
    );
    let _ = std::fs::remove_file(&expected_saved_path);
}

#[tokio::test]
async fn build_initial_context_restates_realtime_start_when_reference_context_is_missing() {
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.realtime_active = true;
    let previous_turn_settings = PreviousTurnSettings {
        model: turn_context.model_info.slug.clone(),
        realtime_active: Some(true),
    };

    session
        .set_previous_turn_settings(Some(previous_turn_settings))
        .await;
    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<realtime_conversation>")),
        "expected initial context to restate active realtime when the reference context is missing, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_injects_full_context_when_baseline_missing()
 {
    let (session, turn_context) = make_session_and_context().await;
    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    let history = session.clone_history().await;
    let initial_context = session.build_initial_context(&turn_context).await;
    assert_eq!(history.raw_items().to_vec(), initial_context);

    let current_context = session.reference_context_item().await;
    assert_eq!(
        serde_json::to_value(current_context).expect("serialize current context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected context item")
    );
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_reinjects_full_context_after_clear()
{
    let (session, turn_context) = make_session_and_context().await;
    let compacted_summary = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: format!("{}\nsummary", crate::compact::SUMMARY_PREFIX),
        }],
        phase: None,
    };
    session
        .record_into_history(std::slice::from_ref(&compacted_summary), &turn_context)
        .await;
    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(/*item*/ None);
    }
    session
        .replace_history(
            vec![compacted_summary.clone()],
            /*reference_context_item*/ None,
        )
        .await;

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;

    let history = session.clone_history().await;
    let mut expected_history = vec![compacted_summary];
    expected_history.extend(session.build_initial_context(&turn_context).await);
    assert_eq!(history.raw_items().to_vec(), expected_history);
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_persists_baseline_without_emitting_diffs()
 {
    let (mut session, previous_context) = make_session_and_context().await;
    let next_model = if previous_context.model_info.slug == "gpt-5.4" {
        "gpt-5.2"
    } else {
        "gpt-5.4"
    };
    let turn_context = previous_context
        .with_model(next_model.to_string(), &session.services.models_manager)
        .await;
    let previous_context_item = previous_context.to_turn_context_item();
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(Some(previous_context_item.clone()));
    }
    let rollout_path = attach_thread_persistence(&mut session).await;

    let update_items = session
        .build_settings_update_items(Some(&previous_context_item), &turn_context)
        .await;
    assert_eq!(update_items, Vec::new());

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;

    assert_eq!(
        session.clone_history().await.raw_items().to_vec(),
        Vec::new()
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize current context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected context item")
    );
    session.ensure_rollout_materialized().await;
    session.flush_rollout().await.expect("rollout should flush");

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let persisted_turn_context = resumed.history.iter().find_map(|item| match item {
        RolloutItem::TurnContext(ctx) => Some(ctx.clone()),
        _ => None,
    });
    assert_eq!(
        serde_json::to_value(persisted_turn_context)
            .expect("serialize persisted turn context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected turn context item")
    );
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_persists_split_file_system_policy_to_rollout()
 {
    let (mut session, mut turn_context) = make_session_and_context().await;
    let file_system_sandbox_policy = file_system_policy_with_unreadable_glob(&turn_context);
    turn_context.permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
        turn_context.permission_profile.enforcement(),
        &file_system_sandbox_policy,
        turn_context.network_sandbox_policy(),
    );
    let rollout_path = attach_thread_persistence(&mut session).await;

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    session.ensure_rollout_materialized().await;
    session.flush_rollout().await.expect("rollout should flush");

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let persisted_file_system_sandbox_policy = resumed.history.iter().find_map(|item| match item {
        RolloutItem::TurnContext(ctx) => ctx.file_system_sandbox_policy.clone(),
        _ => None,
    });
    assert_eq!(
        persisted_file_system_sandbox_policy,
        Some(file_system_sandbox_policy)
    );
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_persists_full_reinjection_to_rollout()
 {
    let (mut session, previous_context) = make_session_and_context().await;
    let next_model = if previous_context.model_info.slug == "gpt-5.4" {
        "gpt-5.2"
    } else {
        "gpt-5.4"
    };
    let turn_context = previous_context
        .with_model(next_model.to_string(), &session.services.models_manager)
        .await;
    let rollout_path = attach_thread_persistence(&mut session).await;

    session
        .persist_rollout_items(&[RolloutItem::EventMsg(EventMsg::UserMessage(
            UserMessageEvent {
                message: "seed rollout".to_string(),
                images: None,
                image_details: Vec::new(),
                local_images: Vec::new(),
                local_image_details: Vec::new(),
                text_elements: Vec::new(),
            },
        ))])
        .await;
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(/*item*/ None);
    }

    session
        .set_previous_turn_settings(Some(PreviousTurnSettings {
            model: previous_context.model_info.slug.clone(),
            realtime_active: Some(previous_context.realtime_active),
        }))
        .await;
    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    session.ensure_rollout_materialized().await;
    session.flush_rollout().await.expect("rollout should flush");

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let persisted_turn_context = resumed.history.iter().find_map(|item| match item {
        RolloutItem::TurnContext(ctx) => Some(ctx.clone()),
        _ => None,
    });

    assert_eq!(
        serde_json::to_value(persisted_turn_context)
            .expect("serialize persisted turn context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected turn context item")
    );
}

#[tokio::test]
async fn run_user_shell_command_does_not_set_reference_context_item() {
    let (session, _turn_context, rx) = make_session_and_context_with_rx().await;
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(/*item*/ None);
    }

    handlers::run_user_shell_command(&session, "sub-id".to_string(), "echo shell".to_string())
        .await;

    let deadline = StdDuration::from_secs(15);
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline.saturating_sub(start.elapsed());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        if matches!(evt.msg, EventMsg::TurnComplete(_)) {
            break;
        }
    }

    assert!(
        session.reference_context_item().await.is_none(),
        "standalone shell tasks should not mutate previous context"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abort_review_task_emits_exited_then_aborted_and_records_history() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "start review".to_string(),
        text_elements: Vec::new(),
    }];
    sess.spawn_task(Arc::clone(&tc), input, ReviewTask::new())
        .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    // Aborting a review task should exit review mode before surfacing the abort to the client.
    // We scan for these events (rather than relying on fixed ordering) since unrelated events
    // may interleave.
    let mut exited_review_mode_idx = None;
    let mut turn_aborted_idx = None;
    let mut idx = 0usize;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        let event_idx = idx;
        idx = idx.saturating_add(1);
        match evt.msg {
            EventMsg::ExitedReviewMode(ev) => {
                assert!(ev.review_output.is_none());
                exited_review_mode_idx = Some(event_idx);
            }
            EventMsg::TurnAborted(ev) => {
                assert_eq!(TurnAbortReason::Interrupted, ev.reason);
                turn_aborted_idx = Some(event_idx);
                break;
            }
            _ => {}
        }
    }
    assert!(
        exited_review_mode_idx.is_some(),
        "expected ExitedReviewMode after abort"
    );
    assert!(
        turn_aborted_idx.is_some(),
        "expected TurnAborted after abort"
    );
    assert!(
        exited_review_mode_idx.unwrap() < turn_aborted_idx.unwrap(),
        "expected ExitedReviewMode before TurnAborted"
    );

    let history = sess.clone_history().await;
    // The `<turn_aborted>` marker is silent in the event stream, so verify it is still
    // recorded in history for the model.
    assert!(
        history.raw_items().iter().any(|item| {
            let ResponseItem::Message { role, content, .. } = item else {
                return false;
            };
            if role != "user" {
                return false;
            }
            content.iter().any(|content_item| {
                let ContentItem::InputText { text } = content_item else {
                    return false;
                };
                TurnAborted::matches_text(text)
            })
        }),
        "expected a model-visible turn aborted marker in history after interrupt"
    );
}
