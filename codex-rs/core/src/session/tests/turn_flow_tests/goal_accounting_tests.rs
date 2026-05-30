use super::*;

#[tokio::test]
async fn interrupt_accounts_active_goal_before_pausing() -> anyhow::Result<()> {
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

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    let goal = sess
        .get_thread_goal()
        .await?
        .expect("goal should remain persisted after interrupt");
    assert_eq!(
        codex_protocol::protocol::ThreadGoalStatus::Paused,
        goal.status
    );
    assert_eq!(70, goal.tokens_used);

    assert!(sess.active_turn.lock().await.is_none());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_goal_continuation_runs_again_after_no_tool_turn() -> anyhow::Result<()> {
    let server = start_mock_server().await;
    let mut builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::Goals)
            .expect("goal mode should be enableable in tests");
    });
    let test = builder.build(&server).await?;
    let _responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    "call-create-goal",
                    "create_goal",
                    r#"{"objective":"write a benchmark note"}"#,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_assistant_message("msg-1", "Draft ready."),
                ev_completed("resp-2"),
            ]),
            sse(vec![
                ev_assistant_message("msg-2", "I am still working on the benchmark note."),
                ev_completed("resp-3"),
            ]),
            sse(vec![
                ev_response_created("resp-4"),
                ev_function_call(
                    "call-complete-goal",
                    "update_goal",
                    r#"{"status":"complete"}"#,
                ),
                ev_completed("resp-4"),
            ]),
            sse(vec![
                ev_assistant_message("msg-3", "Goal complete."),
                ev_completed("resp-5"),
            ]),
        ],
    )
    .await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "write a benchmark note".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;

    let mut completed_turns = 0;
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let event = test.codex.next_event().await?;
            if matches!(event.msg, EventMsg::TurnComplete(_)) {
                completed_turns += 1;
                if completed_turns == 3 {
                    return anyhow::Ok(());
                }
            }
        }
    })
    .await??;

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
        tool_name: "shell",
    })
    .await?;

    let pending_input = sess.get_pending_input().await;
    let [TurnInput::ResponseInputItem(ResponseInputItem::Message { role, content, .. })] =
        pending_input.as_slice()
    else {
        panic!("expected one budget-limit steering message, got {pending_input:#?}");
    };
    assert_eq!("developer", role);
    let [ContentItem::InputText { text }] = content.as_slice() else {
        panic!("expected one text span in budget-limit steering message, got {content:#?}");
    };
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

    let previous_status = ExternalGoalPreviousStatus::from(&goal);
    let goal_id = goal.goal_id.clone();
    let updated_goal = state_db
        .thread_goals()
        .update_thread_goal(
            sess.conversation_id,
            codex_state::ThreadGoalUpdate {
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
            previous_status,
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
        tool_name: "shell",
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
