use super::support::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn webrtc_v2_background_agent_tool_call_delegates_and_returns_function_output() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    // Phase 1: script a v2 background agent function call and a delegated Responses turn that
    // returns final assistant text.
    let mut harness = RealtimeE2eHarness::new(
        RealtimeTestVersion::V2,
        main_loop_responses(vec![create_final_assistant_message_sse_response(
            "delegated from v2",
        )?]),
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![
                session_updated("sess_v2_tool"),
                json!({
                    "type": "conversation.item.input_audio_transcription.completed",
                    "transcript": "Hi how are you"
                }),
                json!({
                    "type": "response.output_audio_transcript.done",
                    "transcript": "Doing well, what can I help you with?"
                }),
                json!({
                    "type": "conversation.item.input_audio_transcription.completed",
                    "transcript": "The secret word is strawberry"
                }),
                json!({
                    "type": "conversation.item.created",
                    "item": {
                        "type": "message",
                        "role": "user",
                        "content": [{
                            "type": "input_text",
                            "text": "<realtime_collaboration_update><voice_policy>silent_delegate</voice_policy></realtime_collaboration_update>"
                        }]
                    }
                }),
                json!({
                    "type": "response.output_audio_transcript.delta",
                    "delta": "Got it-strawberry. What's next on the menu?"
                }),
                v2_background_agent_tool_call("call_v2", "run ls"),
            ],
            vec![],
            vec![],
            vec![],
        ])]),
    )
    .await?;

    let started = harness.start_webrtc_realtime("v=offer\r\n").await?;
    assert_eq!(started.started.version, RealtimeConversationVersion::V2);

    // Phase 2: wait for the delegated turn lifecycle kicked off by the v2 function-call item.
    let turn_started = harness
        .read_notification::<TurnStartedNotification>("turn/started")
        .await?;
    assert_eq!(turn_started.thread_id, harness.thread_id);
    let turn_completed = harness
        .read_notification::<TurnCompletedNotification>("turn/completed")
        .await?;
    assert_eq!(turn_completed.thread_id, harness.thread_id);

    // Phase 3: assert the delegated prompt went to Responses and the result
    // returned as exactly one v2 function-call output event on the sideband.
    let requests = harness.main_loop_responses_requests().await?;
    assert_eq!(requests.len(), 1);
    assert!(
        response_request_contains_text(
            &requests[0],
            "<realtime_delegation>\n  <input>run ls</input>\n  <transcript_delta>user: Hi how are you\nassistant: Doing well, what can I help you with?\nuser: The secret word is strawberry\nassistant: Got it-strawberry. What's next on the menu?\nuser: run ls</transcript_delta>\n</realtime_delegation>",
        ),
        "delegated Responses request should contain realtime delegation envelope: {}",
        requests[0]
    );
    assert!(
        !response_request_contains_text(&requests[0], "<realtime_collaboration_update>"),
        "delegated Responses request should not include realtime control injects: {}",
        requests[0]
    );

    let progress = harness.sideband_outbound_request(/*request_index*/ 1).await;
    assert_v2_progress_update(&progress, "delegated from v2");

    let tool_output = harness.sideband_outbound_request(/*request_index*/ 2).await;
    assert_v2_function_call_output(&tool_output, "call_v2", V2_HANDOFF_COMPLETE_ACKNOWLEDGEMENT);
    assert_eq!(
        function_call_output_sideband_requests(&harness.realtime_server).len(),
        1
    );

    // Phase 4: after the final function-call output, realtime needs an explicit
    // `response.create` to produce the next user-visible response.
    assert_v2_response_create(&harness.sideband_outbound_request(/*request_index*/ 3).await);

    harness.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn webrtc_v2_tool_call_delegated_turn_can_execute_shell_tool() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: keep the two mocked OpenAI conversations explicit. The realtime sideband only
    // calls the `background_agent` function; the shell command is requested by the delegated
    // background agent Responses turn that app-server starts after receiving that function call.
    let main_loop = main_loop_responses(vec![
        create_shell_command_sse_response(
            realtime_tool_ok_command(),
            /*workdir*/ None,
            // Windows CI can spend several seconds starting the nested PowerShell command. This
            // test verifies delegated shell-tool plumbing, not timeout enforcement.
            Some(DELEGATED_SHELL_TOOL_TIMEOUT_MS),
            "shell_call",
        )?,
        create_final_assistant_message_sse_response("shell tool finished")?,
    ]);
    let realtime = realtime_sideband(vec![realtime_sideband_connection(vec![
        vec![
            session_updated("sess_v2_shell"),
            v2_background_agent_tool_call("call_shell", "run shell through delegated turn"),
        ],
        vec![],
        vec![],
    ])]);

    let mut harness = RealtimeE2eHarness::new_with_sandbox(
        RealtimeTestVersion::V2,
        main_loop,
        realtime,
        RealtimeTestSandbox::DangerFullAccess,
    )
    .await?;

    let _ = harness.start_webrtc_realtime("v=offer\r\n").await?;

    // Phase 2: observe the delegated background agent turn executing the requested shell command.
    let started_command = wait_for_started_command_execution(&mut harness.mcp).await?;
    let ThreadItem::CommandExecution { id, status, .. } = started_command.item else {
        unreachable!("helper returns command execution items");
    };
    assert_eq!(
        (id.as_str(), status),
        ("shell_call", CommandExecutionStatus::InProgress)
    );

    let completed_command = wait_for_completed_command_execution(&mut harness.mcp).await?;
    let ThreadItem::CommandExecution {
        id,
        status,
        aggregated_output,
        ..
    } = completed_command.item
    else {
        unreachable!("helper returns command execution items");
    };
    assert_eq!(id.as_str(), "shell_call");
    assert_eq!(status, CommandExecutionStatus::Completed);
    assert_eq!(aggregated_output.as_deref(), Some("realtime-tool-ok"));

    // Phase 3: verify the shell output reached Responses and the final delegated answer returned
    // to realtime as a single function-call-output item.
    let turn_completed = harness
        .read_notification::<TurnCompletedNotification>("turn/completed")
        .await?;
    assert_eq!(turn_completed.thread_id, harness.thread_id);

    let requests = harness.main_loop_responses_requests().await?;
    assert_eq!(requests.len(), 2);
    assert!(
        response_request_contains_text(&requests[1], "realtime-tool-ok"),
        "follow-up Responses request should contain shell output: {}",
        requests[1]
    );

    let progress = harness.sideband_outbound_request(/*request_index*/ 1).await;
    assert_v2_progress_update(&progress, "shell tool finished");

    let tool_output = harness.sideband_outbound_request(/*request_index*/ 2).await;
    assert_v2_function_call_output(
        &tool_output,
        "call_shell",
        V2_HANDOFF_COMPLETE_ACKNOWLEDGEMENT,
    );
    assert_eq!(
        function_call_output_sideband_requests(&harness.realtime_server).len(),
        1
    );

    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn webrtc_v2_tool_call_does_not_block_sideband_audio() -> Result<()> {
    skip_if_no_network!(Ok(()));

    // Phase 1: gate the delegated Responses stream so the sideband can send audio while the tool
    // call is still waiting on its delegated turn.
    let main_loop_responses_server = responses::start_mock_server().await;
    let (gate_completed_tx, gate_completed_rx) = mpsc::channel();
    let gated_response = responses::sse(vec![
        responses::ev_response_created("resp-1"),
        responses::ev_assistant_message("msg-1", "late delegated result"),
        responses::ev_completed("resp-1"),
    ]);
    Mock::given(method("POST"))
        .and(path_regex(".*/responses$"))
        .respond_with(GatedSseResponse {
            gate_rx: Mutex::new(Some(gate_completed_rx)),
            response: gated_response,
        })
        .expect(1)
        .mount(&main_loop_responses_server)
        .await;

    let mut harness = RealtimeE2eHarness::new_with_main_loop_responses_server(
        RealtimeTestVersion::V2,
        main_loop_responses_server,
        realtime_sideband(vec![realtime_sideband_connection(vec![
            vec![
                session_updated("sess_v2_nonblocking"),
                v2_background_agent_tool_call("call_audio", "delegate while audio continues"),
                json!({
                    "type": "response.output_audio.delta",
                    "delta": "CQoL",
                    "sample_rate": 24_000,
                    "channels": 1,
                    "samples_per_channel": 256
                }),
            ],
            vec![],
            vec![],
        ])]),
    )
    .await?;

    let _ = harness.start_webrtc_realtime("v=offer\r\n").await?;
    let _ = harness
        .read_notification::<TurnStartedNotification>("turn/started")
        .await?;

    // Phase 2: require app-server to fan out sideband audio before the delegated tool call is
    // allowed to finish.
    let audio = harness
        .read_notification::<ThreadRealtimeOutputAudioDeltaNotification>(
            "thread/realtime/outputAudio/delta",
        )
        .await?;
    assert_eq!(audio.audio.data, "CQoL");

    // Phase 3: release the delegated turn and assert the sideband function-call output is delivered
    // after the nonblocking audio.
    let _ = gate_completed_tx.send(());
    let turn_completed = harness
        .read_notification::<TurnCompletedNotification>("turn/completed")
        .await?;
    assert_eq!(turn_completed.thread_id, harness.thread_id);

    let progress = harness.sideband_outbound_request(/*request_index*/ 1).await;
    assert_v2_progress_update(&progress, "late delegated result");

    let tool_output = harness.sideband_outbound_request(/*request_index*/ 2).await;
    assert_v2_function_call_output(
        &tool_output,
        "call_audio",
        V2_HANDOFF_COMPLETE_ACKNOWLEDGEMENT,
    );

    harness.shutdown().await;
    Ok(())
}
