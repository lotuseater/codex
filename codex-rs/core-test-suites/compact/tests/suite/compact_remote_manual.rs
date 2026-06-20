#![allow(clippy::expect_used)]

use crate::compact_remote_support::*;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_manual_compact_emits_context_compaction_items() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing()),
    )
    .await?;
    let codex = harness.test().codex.clone();

    mount_sse_once(
        harness.server(),
        sse(vec![
            responses::ev_assistant_message("m1", "REMOTE_REPLY"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;

    let compact_mock = responses::mount_compact_user_history_with_summary_once(
        harness.server(),
        "REMOTE_COMPACTED_SUMMARY",
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "manual remote compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await?;

    let mut started_item = None;
    let mut completed_item = None;
    let mut legacy_event = false;
    let mut saw_turn_complete = false;

    while !saw_turn_complete || started_item.is_none() || completed_item.is_none() || !legacy_event
    {
        let event = codex.next_event().await.unwrap();
        match event.msg {
            EventMsg::ItemStarted(ItemStartedEvent {
                item: TurnItem::ContextCompaction(item),
                ..
            }) => {
                started_item = Some(item);
            }
            EventMsg::ItemCompleted(ItemCompletedEvent {
                item: TurnItem::ContextCompaction(item),
                ..
            }) => {
                completed_item = Some(item);
            }
            EventMsg::ContextCompacted(_) => {
                legacy_event = true;
            }
            EventMsg::TurnComplete(_) => {
                saw_turn_complete = true;
            }
            _ => {}
        }
    }

    let started_item = started_item.expect("context compaction item started");
    let completed_item = completed_item.expect("context compaction item completed");
    assert_eq!(started_item.id, completed_item.id);
    assert!(legacy_event);
    assert_eq!(compact_mock.requests().len(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_manual_compact_failure_emits_task_error_event() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing()),
    )
    .await?;
    let codex = harness.test().codex.clone();

    mount_sse_once(
        harness.server(),
        sse(vec![
            responses::ev_assistant_message("m1", "REMOTE_REPLY"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;

    let compact_mock = responses::mount_compact_json_once(
        harness.server(),
        serde_json::json!({ "output": "invalid compact payload shape" }),
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "manual remote compact".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await?;

    let error_message = wait_for_event_match(&codex, |event| match event {
        EventMsg::Error(err) => Some(err.message.clone()),
        _ => None,
    })
    .await;
    assert!(
        error_message.contains("Error running remote compact task"),
        "expected remote compact task error prefix, got {error_message}"
    );
    assert!(
        error_message.contains("invalid compact payload shape")
            || error_message.contains("invalid type: string"),
        "expected invalid compact payload details, got {error_message}"
    );
    wait_for_event(&codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// TODO(ccunningham): Re-enable after the follow-up compaction behavior PR lands.
// Current main behavior for rollout replacement-history persistence is known-incorrect.
#[ignore = "behavior change covered in follow-up compaction PR"]
async fn remote_compact_persists_replacement_history_in_rollout() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = TestCodexHarness::with_builder(
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing()),
    )
    .await?;
    let codex = harness.test().codex.clone();
    let rollout_path = harness
        .test()
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    let responses_mock = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_assistant_message("m1", "COMPACT_BASELINE_REPLY"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;

    let compacted_history = vec![
        ResponseItem::Compaction {
            encrypted_content: "ENCRYPTED_COMPACTION_SUMMARY".to_string(),
        },
        ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText {
                text: "COMPACTED_ASSISTANT_NOTE".to_string(),
            }],
            phase: None,
        },
    ];
    let compact_mock = responses::mount_compact_json_once(
        harness.server(),
        serde_json::json!({ "output": compacted_history.clone() }),
    )
    .await;

    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "needs compaction".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Compact).await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    codex.submit(Op::Shutdown).await?;
    wait_for_event(&codex, |ev| matches!(ev, EventMsg::ShutdownComplete)).await;

    assert_eq!(responses_mock.requests().len(), 1);
    assert_eq!(compact_mock.requests().len(), 1);

    let rollout_text = fs::read_to_string(&rollout_path)?;
    let mut saw_compacted_history = false;
    for line in rollout_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        let Ok(entry) = serde_json::from_str::<RolloutLine>(line) else {
            continue;
        };
        if let RolloutItem::Compacted(compacted) = entry.item
            && compacted.message.is_empty()
            && let Some(replacement_history) = compacted.replacement_history.as_ref()
        {
            let has_compaction_item = replacement_history.iter().any(|item| {
                matches!(
                    item,
                    ResponseItem::Compaction { encrypted_content }
                        if encrypted_content == "ENCRYPTED_COMPACTION_SUMMARY"
                )
            });
            let has_compacted_assistant_note = replacement_history.iter().any(|item| {
                matches!(
                    item,
                    ResponseItem::Message { role, content, .. }
                        if role == "assistant"
                            && content.iter().any(|part| matches!(
                                part,
                                ContentItem::OutputText { text } if text == "COMPACTED_ASSISTANT_NOTE"
                            ))
                )
            });
            let has_permissions_developer_message = replacement_history.iter().any(|item| {
                matches!(
                    item,
                    ResponseItem::Message { role, content, .. }
                        if role == "developer"
                            && content.iter().any(|part| matches!(
                                part,
                                ContentItem::InputText { text }
                                    if text.contains("<permissions instructions>")
                            ))
                )
            });

            if has_compaction_item && has_compacted_assistant_note {
                assert!(
                    !has_permissions_developer_message,
                    "manual remote compact rollout replacement history should not inject permissions context"
                );
                saw_compacted_history = true;
                break;
            }
        }
    }

    assert!(
        saw_compacted_history,
        "expected rollout to persist remote compaction history"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_and_resume_refresh_stale_developer_instructions() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let stale_developer_message = "STALE_DEVELOPER_INSTRUCTIONS_SHOULD_BE_REMOVED";

    let mut start_builder =
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    let initial = start_builder.build(&server).await?;
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    let responses_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "BASELINE_REPLY"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "AFTER_COMPACT_REPLY"),
                responses::ev_completed("resp-2"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m3", "AFTER_RESUME_REPLY"),
                responses::ev_completed("resp-3"),
            ]),
        ],
    )
    .await;

    let compacted_history = vec![
        ResponseItem::Message {
            id: None,
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: stale_developer_message.to_string(),
            }],
            phase: None,
        },
        ResponseItem::Compaction {
            encrypted_content: "ENCRYPTED_COMPACTION_SUMMARY".to_string(),
        },
    ];
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({ "output": compacted_history }),
    )
    .await;

    initial
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "start remote compact flow".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&initial.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    initial.codex.submit(Op::Compact).await?;
    wait_for_event(&initial.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    initial
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after compact in same session".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&initial.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    initial.codex.submit(Op::Shutdown).await?;
    wait_for_event(&initial.codex, |ev| {
        matches!(ev, EventMsg::ShutdownComplete)
    })
    .await;

    let mut resume_builder =
        test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;

    resumed
        .codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after resume".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&resumed.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(requests.len(), 3, "expected three model requests");

    let after_compact_request = &requests[1];
    let after_resume_request = &requests[2];

    let after_compact_body = after_compact_request.body_json().to_string();
    assert!(
        !after_compact_body.contains(stale_developer_message),
        "stale developer instructions should be removed immediately after compaction"
    );
    assert!(
        after_compact_body.contains("<permissions instructions>"),
        "fresh developer instructions should be present after compaction"
    );
    assert!(
        after_compact_body.contains("ENCRYPTED_COMPACTION_SUMMARY"),
        "compaction item should be present after compaction"
    );

    let after_resume_body = after_resume_request.body_json().to_string();
    assert!(
        !after_resume_body.contains(stale_developer_message),
        "stale developer instructions should be removed after resume"
    );
    assert!(
        after_resume_body.contains("<permissions instructions>"),
        "fresh developer instructions should be present after resume"
    );
    assert!(
        after_resume_body.contains("ENCRYPTED_COMPACTION_SUMMARY"),
        "compaction item should persist after resume"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_compact_refreshes_stale_developer_instructions_without_resume() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = wiremock::MockServer::start().await;
    let stale_developer_message = "STALE_DEVELOPER_INSTRUCTIONS_SHOULD_BE_REMOVED";

    let mut builder = test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    let test = builder.build(&server).await?;

    let responses_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_assistant_message("m1", "BASELINE_REPLY"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_assistant_message("m2", "AFTER_COMPACT_REPLY"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    let compacted_history = vec![
        ResponseItem::Message {
            id: None,
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: stale_developer_message.to_string(),
            }],
            phase: None,
        },
        ResponseItem::Compaction {
            encrypted_content: "ENCRYPTED_COMPACTION_SUMMARY".to_string(),
        },
    ];
    let compact_mock = responses::mount_compact_json_once(
        &server,
        serde_json::json!({ "output": compacted_history }),
    )
    .await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "start remote compact flow".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex.submit(Op::Compact).await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after compact in same session".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
        })
        .await?;
    wait_for_event(&test.codex, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    assert_eq!(compact_mock.requests().len(), 1);
    let requests = responses_mock.requests();
    assert_eq!(requests.len(), 2, "expected two model requests");

    let after_compact_body = requests[1].body_json().to_string();
    assert!(
        !after_compact_body.contains(stale_developer_message),
        "stale developer instructions should be removed immediately after compaction"
    );
    assert!(
        after_compact_body.contains("<permissions instructions>"),
        "fresh developer instructions should be present after compaction"
    );
    assert!(
        after_compact_body.contains("ENCRYPTED_COMPACTION_SUMMARY"),
        "compaction item should be present after compaction"
    );

    Ok(())
}
