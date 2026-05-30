use super::*;

#[tokio::test]
async fn create_goal_tool_rejects_existing_goal() {
    let (session, turn_context, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let handler = CreateGoalHandler;

    handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "create-goal-1".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Keep the watcher alive",
                    "token_budget": 123,
                })
                .to_string(),
            },
        })
        .await
        .expect("initial create_goal should succeed");

    let response = handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker,
            call_id: "create-goal-2".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Replace the watcher",
                    "token_budget": 456,
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = response else {
        panic!("expected create_goal to reject an existing goal");
    };
    assert_eq!(
        output,
        "cannot create a new goal because this thread already has a goal; use update_goal only when the existing goal is complete"
    );

    let goal = session
        .get_thread_goal()
        .await
        .expect("read thread goal")
        .expect("goal should still exist");
    assert_eq!(goal.objective, "Keep the watcher alive");
    assert_eq!(goal.token_budget, Some(123));
}

#[tokio::test]
async fn update_goal_tool_rejects_pausing_goal() {
    let (session, turn_context, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let create_handler = CreateGoalHandler;
    let update_handler = UpdateGoalHandler;

    create_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "create-goal".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Keep the watcher alive",
                    "token_budget": 123,
                })
                .to_string(),
            },
        })
        .await
        .expect("initial create_goal should succeed");

    let response = update_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker,
            call_id: "pause-goal".to_string(),
            tool_name: plain_tool_name("update_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "status": "paused",
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = response else {
        panic!("expected update_goal to reject pausing a goal");
    };
    assert_eq!(
        output,
        "update_goal can only mark the existing goal complete; pause, resume, and budget-limited status changes are controlled by the user or system"
    );

    let goal = session
        .get_thread_goal()
        .await
        .expect("read thread goal")
        .expect("goal should still exist");
    assert_eq!(goal.status, ThreadGoalStatus::Active);
}

#[tokio::test]
async fn update_goal_tool_marks_goal_complete() {
    let (session, turn_context, _rx, _codex_home) = make_goal_session_and_context_with_rx().await;
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let create_handler = CreateGoalHandler;
    let update_handler = UpdateGoalHandler;

    create_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "create-goal".to_string(),
            tool_name: plain_tool_name("create_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "objective": "Keep the watcher alive",
                    "token_budget": 123,
                })
                .to_string(),
            },
        })
        .await
        .expect("initial create_goal should succeed");

    update_handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker,
            call_id: "complete-goal".to_string(),
            tool_name: plain_tool_name("update_goal"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "status": "complete",
                })
                .to_string(),
            },
        })
        .await
        .expect("update_goal should mark the goal complete");

    let goal = session
        .get_thread_goal()
        .await
        .expect("read thread goal")
        .expect("goal should still exist");
    assert_eq!(goal.status, ThreadGoalStatus::Complete);
}
