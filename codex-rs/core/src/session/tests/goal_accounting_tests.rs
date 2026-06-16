use super::*;

#[tokio::test]
async fn create_thread_goal_fills_empty_thread_preview() -> anyhow::Result<()> {
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    let state_db = goal_test_state_db(sess.as_ref()).await?;

    let page = state_db
        .list_threads(
            /*page_size*/ 10,
            codex_state::ThreadFilterOptions {
                archived_only: false,
                allowed_sources: &[],
                model_providers: None,
                cwd_filters: None,
                anchor: None,
                sort_key: codex_state::SortKey::UpdatedAt,
                sort_direction: codex_state::SortDirection::Desc,
                search_term: None,
            },
        )
        .await?;
    assert!(page.items.is_empty());

    sess.create_thread_goal(
        tc.as_ref(),
        CreateGoalRequest {
            objective: "Keep improving the benchmark".to_string(),
            token_budget: None,
        },
    )
    .await?;

    let page = state_db
        .list_threads(
            /*page_size*/ 10,
            codex_state::ThreadFilterOptions {
                archived_only: false,
                allowed_sources: &[],
                model_providers: None,
                cwd_filters: None,
                anchor: None,
                sort_key: codex_state::SortKey::UpdatedAt,
                sort_direction: codex_state::SortDirection::Desc,
                search_term: None,
            },
        )
        .await?;
    let ids = page
        .items
        .iter()
        .map(|thread| thread.id)
        .collect::<Vec<_>>();
    assert_eq!(vec![sess.conversation_id], ids);
    assert_eq!(
        Some("Keep improving the benchmark"),
        page.items[0].preview.as_deref()
    );

    Ok(())
}

#[tokio::test]
async fn budget_limited_accounting_steers_active_turn_without_aborting() -> anyhow::Result<()> {
    let (sess, tc, rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.set_thread_goal(
        tc.as_ref(),
        SetGoalRequest {
            objective: Some("Keep improving the benchmark".to_string()),
            status: None,
            token_budget: Some(Some(10)),
        },
    )
    .await?;
    sess.goal_runtime_apply(GoalRuntimeEvent::TurnStarted {
        turn_context: tc.as_ref(),
        token_usage: TokenUsage::default(),
    })
    .await?;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    while rx.try_recv().is_ok() {}

    set_total_token_usage(
        &sess,
        TokenUsage {
            input_tokens: 20,
            cached_input_tokens: 0,
            output_tokens: 5,
            reasoning_output_tokens: 0,
            total_tokens: 25,
        },
    )
    .await;

    sess.goal_runtime_apply(GoalRuntimeEvent::ToolCompleted {
        turn_context: tc.as_ref(),
        tool_name: "shell_command",
    })
    .await?;

    let pending_input = sess.input_queue.get_pending_input(&sess.active_turn).await;
    let [TurnInput::ResponseInputItem(ResponseInputItem::Message { role, content, .. })] =
        pending_input.as_slice()
    else {
        panic!("expected one budget-limit steering message, got {pending_input:#?}");
    };
    assert_eq!("user", role);
    let [ContentItem::InputText { text }] = content.as_slice() else {
        panic!("expected one text span in budget-limit steering message, got {content:#?}");
    };
    assert!(text.starts_with("<goal_context>"));
    assert!(text.trim_end().ends_with("</goal_context>"));
    assert!(text.contains("budget_limited"));
    assert!(text.to_lowercase().contains("wrap up this turn soon"));
    assert!(sess.active_turn.lock().await.is_some());
    while let Ok(event) = rx.try_recv() {
        assert!(
            !matches!(event.msg, EventMsg::TurnAborted(_)),
            "budget limit should steer the active turn instead of aborting it"
        );
    }

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted after accounting");
    assert_eq!(codex_state::ThreadGoalStatus::BudgetLimited, goal.status);
    assert_eq!(25, goal.tokens_used);

    set_total_token_usage(
        &sess,
        TokenUsage {
            input_tokens: 30,
            cached_input_tokens: 0,
            output_tokens: 10,
            reasoning_output_tokens: 0,
            total_tokens: 40,
        },
    )
    .await;
    sess.goal_runtime_apply(GoalRuntimeEvent::ToolCompletedGoal {
        turn_context: tc.as_ref(),
    })
    .await?;

    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted after follow-up accounting");
    assert_eq!(codex_state::ThreadGoalStatus::BudgetLimited, goal.status);
    assert_eq!(40, goal.tokens_used);

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_limit_runtime_stops_active_goal_and_prevents_idle_continuation() -> anyhow::Result<()>
{
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.set_thread_goal(
        tc.as_ref(),
        SetGoalRequest {
            objective: Some("Keep improving the benchmark".to_string()),
            status: None,
            token_budget: Some(Some(50)),
        },
    )
    .await?;
    sess.goal_runtime_apply(GoalRuntimeEvent::TurnStarted {
        turn_context: tc.as_ref(),
        token_usage: TokenUsage::default(),
    })
    .await?;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    set_total_token_usage(&sess, post_goal_token_usage()).await;

    sess.goal_runtime_apply(GoalRuntimeEvent::UsageLimitReached {
        turn_context: tc.as_ref(),
    })
    .await?;

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted after usage limiting");
    assert_eq!(codex_state::ThreadGoalStatus::UsageLimited, goal.status);
    assert_eq!(70, goal.tokens_used);

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;
    sess.goal_runtime_apply(GoalRuntimeEvent::MaybeContinueIfIdle)
        .await?;
    assert!(sess.active_turn.lock().await.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_goal_mutation_accounts_active_turn_before_status_change() -> anyhow::Result<()> {
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.set_thread_goal(
        tc.as_ref(),
        SetGoalRequest {
            objective: Some("Keep improving the benchmark".to_string()),
            status: None,
            token_budget: None,
        },
    )
    .await?;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    set_total_token_usage(&sess, post_goal_token_usage()).await;

    sess.goal_runtime_apply(GoalRuntimeEvent::ExternalMutationStarting)
        .await?;

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted");
    assert_eq!(70, goal.tokens_used);

    let previous_goal = goal.clone();
    let goal_id = goal.goal_id.clone();
    let updated_goal = state_db
        .thread_goals()
        .update_thread_goal(
            sess.conversation_id,
            codex_state::GoalUpdate {
                objective: None,
                status: Some(codex_state::ThreadGoalStatus::Complete),
                token_budget: None,
                expected_goal_id: Some(goal_id),
            },
        )
        .await?
        .expect("goal status update should succeed");
    sess.goal_runtime_apply(GoalRuntimeEvent::ExternalSet {
        external_set: ExternalGoalSet {
            goal: updated_goal,
            previous_status: ExternalGoalPreviousStatus::from(&previous_goal),
        },
    })
    .await?;

    assert!(sess.active_turn.lock().await.is_some());
    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted");
    assert_eq!(codex_state::ThreadGoalStatus::Complete, goal.status);
    assert_eq!(70, goal.tokens_used);

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_objective_change_steers_active_turn() -> anyhow::Result<()> {
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let old_goal = state_db
        .thread_goals()
        .replace_thread_goal(
            sess.conversation_id,
            "Keep improving the benchmark",
            codex_state::ThreadGoalStatus::Active,
            /*token_budget*/ Some(10_000),
        )
        .await?;
    let new_goal = state_db
        .thread_goals()
        .replace_thread_goal(
            sess.conversation_id,
            "Write a concise benchmark summary",
            codex_state::ThreadGoalStatus::Active,
            /*token_budget*/ Some(10_000),
        )
        .await?;

    sess.goal_runtime_apply(GoalRuntimeEvent::ExternalSet {
        external_set: ExternalGoalSet {
            goal: new_goal,
            previous_status: ExternalGoalPreviousStatus::from(&old_goal),
        },
    })
    .await?;

    let pending_input = sess.input_queue.get_pending_input(&sess.active_turn).await;
    assert!(
        pending_input.iter().any(|item| {
            matches!(
                item,
                TurnInput::ResponseInputItem(ResponseInputItem::Message { role, content, .. })
                    if role == "user"
                        && content.iter().any(|content| matches!(
                            content,
                            ContentItem::InputText { text }
                                if text.starts_with("<goal_context>")
                                    && text.trim_end().ends_with("</goal_context>")
                                    && text.contains("The active thread goal objective was edited")
                                    && text.contains("Write a concise benchmark summary")
                        ))
            )
        }),
        "expected objective-updated steering prompt in pending input: {pending_input:?}"
    );

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_active_goal_set_marks_current_turn_for_accounting() -> anyhow::Result<()> {
    let (sess, tc, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;
    set_total_token_usage(&sess, post_goal_token_usage()).await;

    let state_db = goal_test_state_db(sess.as_ref()).await?;
    let goal = state_db
        .thread_goals()
        .replace_thread_goal(
            sess.conversation_id,
            "Keep improving the benchmark",
            codex_state::ThreadGoalStatus::Active,
            /*token_budget*/ None,
        )
        .await?;
    sess.goal_runtime_apply(GoalRuntimeEvent::ExternalSet {
        external_set: ExternalGoalSet {
            goal,
            previous_status: ExternalGoalPreviousStatus::NewGoal,
        },
    })
    .await?;

    set_total_token_usage(
        &sess,
        TokenUsage {
            input_tokens: 65,
            cached_input_tokens: 10,
            output_tokens: 40,
            reasoning_output_tokens: 5,
            total_tokens: 110,
        },
    )
    .await;
    sess.goal_runtime_apply(GoalRuntimeEvent::ToolCompleted {
        turn_context: tc.as_ref(),
        tool_name: "shell_command",
    })
    .await?;

    let goal = state_db
        .thread_goals()
        .get_thread_goal(sess.conversation_id)
        .await?
        .expect("goal should remain persisted");
    assert_eq!(codex_state::ThreadGoalStatus::Active, goal.status);
    assert_eq!(25, goal.tokens_used);

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_goal_accounts_current_turn_tokens_before_tool_response() -> anyhow::Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::Goals)
            .expect("goal mode should be enableable in tests");
    });
    let test = builder.build(&server).await?;
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    "call-create-goal",
                    "create_goal",
                    r#"{"objective":"write a report","token_budget":500}"#,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_function_call(
                    "call-complete-goal",
                    "update_goal",
                    r#"{"status":"complete"}"#,
                ),
                ev_completed_with_tokens("resp-2", /*total_tokens*/ 580),
            ]),
            sse(vec![
                ev_assistant_message("msg-1", "Goal complete."),
                ev_completed("resp-3"),
            ]),
        ],
    )
    .await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "write a report".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await?;

    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let event = test.codex.next_event().await?;
            if matches!(event.msg, EventMsg::TurnComplete(_)) {
                return anyhow::Ok(());
            }
        }
    })
    .await??;

    let complete_output = responses
        .function_call_output_text("call-complete-goal")
        .expect("complete tool output should be sent to the model");
    let complete_output: serde_json::Value = serde_json::from_str(&complete_output)?;
    assert_eq!(complete_output["goal"]["tokensUsed"], 580);
    assert_eq!(complete_output["goal"]["status"], "complete");
    assert_eq!(complete_output["remainingTokens"], 0);
    assert_eq!(
        complete_output["completionBudgetReport"],
        "Goal achieved. Report final usage from this tool result's structured goal fields. If `goal.tokenBudget` is present, include token usage from `goal.tokensUsed` and `goal.tokenBudget`. If `goal.timeUsedSeconds` is greater than 0, summarize elapsed time in a concise, human-friendly form appropriate to the response language."
    );
    let requests = responses.requests();
    let completion_followup_request = requests
        .last()
        .expect("completion tool output should be sent in a follow-up request");
    assert!(
        !completion_followup_request.body_contains_text("budget_limited"),
        "completion follow-up should not include budget-limit steering"
    );

    let state_db = codex_state::StateRuntime::init(
        test.config.sqlite_home.clone(),
        test.config.model_provider_id.clone(),
    )
    .await?;
    let persisted_goal = state_db
        .thread_goals()
        .get_thread_goal(test.session_configured.thread_id)
        .await?
        .expect("goal should be persisted");
    assert_eq!(
        codex_state::ThreadGoalStatus::Complete,
        persisted_goal.status
    );
    assert_eq!(580, persisted_goal.tokens_used);

    Ok(())
}
