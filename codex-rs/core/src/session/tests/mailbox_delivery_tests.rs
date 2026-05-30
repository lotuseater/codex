use super::*;

#[tokio::test]
async fn queue_only_mailbox_mail_waits_for_next_turn_after_answer_boundary() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "late queue-only update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;

    assert!(
        !sess.input_queue.has_pending_input(&sess.active_turn).await,
        "queue-only mailbox mail should stay buffered once the current turn emitted its answer"
    );
    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        Vec::new()
    );

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![TurnInput::ResponseInputItem(
            communication.to_response_input_item()
        )],
    );
}

#[tokio::test]
async fn trigger_turn_mailbox_mail_waits_for_next_turn_after_answer_boundary() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(InterAgentCommunication::new(
            AgentPath::try_from("/root/worker").expect("worker path should parse"),
            AgentPath::root(),
            Vec::new(),
            "late trigger update".to_string(),
            /*trigger_turn*/ true,
        ))
        .await;

    assert!(
        !sess.input_queue.has_pending_input(&sess.active_turn).await,
        "trigger-turn mailbox mail should not extend the current turn after its answer boundary"
    );

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert!(sess.input_queue.has_trigger_turn_mailbox_items().await);
}

#[tokio::test]
async fn steered_input_reopens_mailbox_delivery_for_current_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;
    sess.steer_input(
        vec![UserInput::Text {
            text: "follow up".to_string(),
            text_elements: Vec::new(),
        }],
        Some(&tc.sub_id),
        /*responsesapi_client_metadata*/ None,
    )
    .await
    .expect("steered input should be accepted");

    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![
            TurnInput::UserInput(vec![UserInput::Text {
                text: "follow up".to_string(),
                text_elements: Vec::new(),
            }]),
            TurnInput::ResponseInputItem(communication.to_response_input_item()),
        ],
    );
}

#[tokio::test]
async fn stale_defer_mailbox_delivery_does_not_override_steered_input() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;
    sess.steer_input(
        vec![UserInput::Text {
            text: "follow up".to_string(),
            text_elements: Vec::new(),
        }],
        Some(&tc.sub_id),
        /*responsesapi_client_metadata*/ None,
    )
    .await
    .expect("steered input should be accepted");

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;

    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![
            TurnInput::UserInput(vec![UserInput::Text {
                text: "follow up".to_string(),
                text_elements: Vec::new(),
            }]),
            TurnInput::ResponseInputItem(communication.to_response_input_item()),
        ],
    );
}

#[tokio::test]
async fn tool_calls_reopen_mailbox_delivery_for_current_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;

    let item = ResponseItem::FunctionCall {
        id: None,
        name: "test_tool".to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        call_id: "call-1".to_string(),
    };
    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&sess),
        turn_context: Arc::clone(&tc),
        turn_store: Arc::new(codex_extension_api::ExtensionData::new(tc.sub_id.clone())),
        tool_runtime: test_tool_runtime(Arc::clone(&sess), Arc::clone(&tc)),
        cancellation_token: CancellationToken::new(),
    };

    let output = handle_output_item_done(&mut ctx, item, /*previously_active_item*/ None)
        .await
        .expect("tool call should be handled");

    assert!(output.needs_follow_up);
    assert!(output.tool_future.is_some());
    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![TurnInput::ResponseInputItem(
            communication.to_response_input_item()
        )],
    );
}
