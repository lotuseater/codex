use super::*;

#[test]
fn active_turn_not_steerable_turn_error_extracts_structured_server_error() {
    let turn_error = AppServerTurnError {
        message: "cannot steer a review turn".to_string(),
        codex_error_info: Some(AppServerCodexErrorInfo::ActiveTurnNotSteerable {
            turn_kind: AppServerNonSteerableTurnKind::Review,
        }),
        additional_details: None,
    };
    let error = TypedRequestError::Server {
        method: "turn/steer".to_string(),
        source: JSONRPCErrorError {
            code: -32602,
            message: turn_error.message.clone(),
            data: Some(serde_json::to_value(&turn_error).expect("turn error should serialize")),
        },
    };

    assert_eq!(
        active_turn_not_steerable_turn_error(&error),
        Some(turn_error)
    );
}

#[test]
fn active_turn_steer_race_detects_missing_active_turn() {
    let error = TypedRequestError::Server {
        method: "turn/steer".to_string(),
        source: JSONRPCErrorError {
            code: -32602,
            message: "no active turn to steer".to_string(),
            data: None,
        },
    };

    assert_eq!(
        active_turn_steer_race(&error),
        Some(ActiveTurnSteerRace::Missing)
    );
    assert_eq!(active_turn_not_steerable_turn_error(&error), None);
}

#[test]
fn active_turn_steer_race_extracts_actual_turn_id_from_mismatch() {
    let error = TypedRequestError::Server {
        method: "turn/steer".to_string(),
        source: JSONRPCErrorError {
            code: -32602,
            message: "expected active turn id `turn-expected` but found `turn-actual`".to_string(),
            data: None,
        },
    };

    assert_eq!(
        active_turn_steer_race(&error),
        Some(ActiveTurnSteerRace::ExpectedTurnMismatch {
            actual_turn_id: "turn-actual".to_string(),
        })
    );
}

#[tokio::test]
async fn fresh_session_config_uses_current_service_tier() {
    let mut app = make_test_app().await;
    app.chat_widget.set_service_tier(Some(
        codex_protocol::config_types::ServiceTier::Fast
            .request_value()
            .to_string(),
    ));

    let config = app.fresh_session_config();

    assert_eq!(
        config.service_tier,
        Some(
            codex_protocol::config_types::ServiceTier::Fast
                .request_value()
                .to_string()
        )
    );
}

#[tokio::test]
async fn override_turn_context_sends_thread_settings_update() {
    Box::pin(async {
        let mut app = make_test_app().await;
        let mut app_server =
            crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref())
                .await
                .expect("embedded app server");
        let started = app_server
            .start_thread(app.chat_widget.config_ref())
            .await
            .expect("thread/start should succeed");
        let thread_id = started.session.thread_id;
        let initial_model = started.session.model.clone();
        let initial_effort = started.session.reasoning_effort;
        app.enqueue_primary_thread_session(started.session, started.turns)
            .await
            .expect("primary thread should be registered");
        let service_tier = ServiceTier::Fast.request_value().to_string();
        let collaboration_mode = CollaborationMode {
            mode: ModeKind::Plan,
            settings: Settings {
                model: "gpt-5.4".to_string(),
                reasoning_effort: Some(ReasoningEffortConfig::High),
                developer_instructions: None,
            },
        };
        let op = AppCommand::override_turn_context(
            /*cwd*/ None,
            Some(AskForApproval::OnRequest),
            Some(ApprovalsReviewer::AutoReview),
            /*permission_profile*/ None,
            Some(ActivePermissionProfile::new(
                codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE,
            )),
            /*windows_sandbox_level*/ None,
            Some("gpt-5.4".to_string()),
            Some(Some(ReasoningEffortConfig::High)),
            /*summary*/ None,
            Some(Some(service_tier.clone())),
            Some(collaboration_mode.clone()),
            Some(Personality::Pragmatic),
        );

        let handled = app
            .try_submit_active_thread_op_via_app_server(&mut app_server, thread_id, &op)
            .await
            .expect("settings update submission should not fail");

        assert_eq!(handled, true);
        assert_eq!(
            app.primary_session_configured
                .as_ref()
                .expect("primary session")
                .model,
            initial_model,
            "thread/settings/update response is only an ack; cached state changes on notification"
        );

        let notification = next_thread_settings_updated(&mut app_server, thread_id).await;
        assert_eq!(notification.thread_settings.model, "gpt-5.4");
        assert_eq!(
            notification.thread_settings.effort,
            Some(ReasoningEffortConfig::High)
        );
        assert_eq!(
            notification.thread_settings.service_tier,
            Some(service_tier.clone())
        );
        assert_eq!(
            notification.thread_settings.approval_policy,
            AskForApproval::OnRequest
        );
        assert_eq!(
            notification.thread_settings.approvals_reviewer.to_core(),
            ApprovalsReviewer::AutoReview
        );
        let notified_mode = &notification.thread_settings.collaboration_mode;
        assert_eq!(notified_mode.mode, collaboration_mode.mode);
        assert_eq!(
            notified_mode.settings.model,
            collaboration_mode.settings.model
        );
        assert_eq!(
            notified_mode.settings.reasoning_effort,
            collaboration_mode.settings.reasoning_effort
        );
        assert_eq!(
            notification.thread_settings.personality,
            Some(Personality::Pragmatic)
        );

        app.handle_app_server_event(
            &app_server,
            codex_app_server_client::AppServerEvent::ServerNotification(
                ServerNotification::ThreadSettingsUpdated(notification),
            ),
        )
        .await;
        let updated_session = app
            .primary_session_configured
            .as_ref()
            .expect("primary session should be updated from notification");
        assert_eq!(updated_session.model, initial_model);
        assert_eq!(updated_session.reasoning_effort, initial_effort);
        let updated_mode = updated_session
            .collaboration_mode
            .as_deref()
            .expect("collaboration mode should be cached");
        assert_eq!(updated_mode.mode, collaboration_mode.mode);
        assert_eq!(
            updated_mode.settings.model,
            collaboration_mode.settings.model
        );
        assert_eq!(
            updated_mode.settings.reasoning_effort,
            collaboration_mode.settings.reasoning_effort
        );
        assert_eq!(updated_session.personality, Some(Personality::Pragmatic));
        assert_eq!(updated_session.service_tier, Some(service_tier));
        assert_eq!(updated_session.approval_policy, AskForApproval::OnRequest);
        assert_eq!(
            updated_session.approvals_reviewer,
            ApprovalsReviewer::AutoReview
        );
        assert_eq!(
            updated_session
                .active_permission_profile
                .as_ref()
                .expect("active profile")
                .id,
            codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE
        );
    })
    .await;
}

#[tokio::test]
async fn thread_setting_update_params_sync_model_and_default_reasoning() {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    app.active_thread_id = Some(thread_id);

    app.chat_widget.set_model("gpt-5.4");
    let params = app
        .active_thread_model_setting_update_params("gpt-5.4".to_string())
        .expect("active thread should produce update params");

    assert_eq!(params.thread_id, thread_id.to_string());
    assert_eq!(params.model, Some("gpt-5.4".to_string()));
    assert_eq!(
        params
            .collaboration_mode
            .as_ref()
            .expect("collaboration mode should sync with model")
            .settings
            .model,
        "gpt-5.4"
    );

    app.chat_widget
        .set_reasoning_effort(Some(ReasoningEffortConfig::Low));
    app.chat_widget
        .set_collaboration_mask(CollaborationModeMask {
            name: "Plan".to_string(),
            mode: Some(ModeKind::Plan),
            model: Some("gpt-plan".to_string()),
            reasoning_effort: Some(Some(ReasoningEffortConfig::Medium)),
            developer_instructions: None,
        });
    app.on_update_reasoning_effort(Some(ReasoningEffortConfig::High));

    let params = app
        .active_thread_reasoning_setting_update_params(Some(ReasoningEffortConfig::High))
        .expect("active thread should produce update params");

    assert_eq!(params.thread_id, thread_id.to_string());
    assert_eq!(params.effort, Some(ReasoningEffortConfig::High));
    let collaboration_mode = params
        .collaboration_mode
        .expect("collaboration mode should sync with reasoning");
    assert_eq!(collaboration_mode.mode, ModeKind::Default);
    assert_eq!(
        collaboration_mode.settings.reasoning_effort,
        Some(ReasoningEffortConfig::High)
    );
}
