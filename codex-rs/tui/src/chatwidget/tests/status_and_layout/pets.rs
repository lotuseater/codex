use super::super::*;
use pretty_assertions::assert_eq;
use ratatui::backend::TestBackend;
use serial_test::serial;

fn enable_test_ambient_pet(chat: &mut ChatWidget) {
    chat.set_pet_image_support_for_tests(crate::pets::PetImageSupport::Supported(
        crate::pets::ImageProtocol::Kitty,
    ));
    chat.install_test_ambient_pet_for_tests(/*animations_enabled*/ false);
}

#[tokio::test]
#[cfg_attr(target_os = "windows", ignore = "disabled on windows")]
async fn configured_pet_load_is_deferred_until_after_construction() {
    let (tx_raw, mut rx) = unbounded_channel::<AppEvent>();
    let tx = AppEventSender::new(tx_raw);
    let mut cfg = test_config().await;
    cfg.tui_pet = Some(crate::pets::DEFAULT_PET_ID.to_string());
    crate::pets::write_test_pack(&cfg.codex_home);
    let resolved_model = crate::legacy_core::test_support::get_model_offline(cfg.model.as_deref());
    let session_telemetry = test_session_telemetry(&cfg, resolved_model.as_str());
    let init = ChatWidgetInit {
        config: cfg.clone(),
        frame_requester: FrameRequester::test_dummy(),
        app_event_tx: tx,
        workspace_command_runner: None,
        initial_user_message: None,
        enhanced_keys_supported: false,
        has_chatgpt_account: false,
        model_catalog: test_model_catalog(&cfg),
        feedback: codex_feedback::CodexFeedback::new(),
        is_first_run: true,
        status_account_display: None,
        runtime_model_provider_base_url: None,
        initial_plan_type: None,
        model: Some(resolved_model),
        startup_tooltip_override: None,
        status_line_invalid_items_warned: Arc::new(AtomicBool::new(false)),
        terminal_title_invalid_items_warned: Arc::new(AtomicBool::new(false)),
        session_telemetry,
    };

    let chat = ChatWidget::new_with_app_event(init);

    assert!(!chat.ambient_pet_image_enabled());
    let event = tokio::time::timeout(std::time::Duration::from_secs(/*secs*/ 30), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_matches!(
        event,
        AppEvent::ConfiguredPetLoaded { pet_id, result } => {
            assert_eq!(pet_id, crate::pets::DEFAULT_PET_ID);
            assert!(result.unwrap().is_some());
        }
    );
}

#[tokio::test]
#[serial]
async fn ambient_pet_stays_hidden_until_a_pet_is_selected() {
    use ratatui::layout::Rect;

    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_pet_image_support_for_tests(crate::pets::PetImageSupport::Supported(
        crate::pets::ImageProtocol::Kitty,
    ));
    assert!(chat.ambient_pet.is_none());

    crate::pets::write_test_pack(&chat.config.codex_home);
    chat.set_tui_pet(Some("codex".to_string()));

    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 60, /*height*/ 20,
    );
    let draw = chat
        .ambient_pet_draw(area, area.bottom())
        .expect("ambient pet draw request");
    assert_eq!(draw.x, 51);
    assert_eq!(draw.y, 14);
    assert_eq!(draw.columns, 9);
    assert_eq!(draw.rows, 5);
    assert_eq!(
        draw.y.saturating_add(draw.rows),
        area.bottom().saturating_sub(/*rhs*/ 1)
    );

    handle_turn_started(&mut chat, "turn-1");
    handle_agent_reasoning_delta(&mut chat, "**Thinking**");
    let draw_with_status = chat
        .ambient_pet_draw(area, area.bottom())
        .expect("ambient pet draw request with status");
    assert_eq!(draw_with_status.y, draw.y);
    assert_eq!(
        draw_with_status.y.saturating_add(draw_with_status.rows),
        area.bottom().saturating_sub(/*rhs*/ 1)
    );
}

#[tokio::test]
#[serial]
async fn ambient_pet_screen_bottom_anchor_uses_terminal_bottom() {
    use codex_config::types::TuiPetAnchor;
    use ratatui::layout::Rect;

    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    enable_test_ambient_pet(&mut chat);

    let terminal_area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 24,
    );
    let composer_bottom_y = 20;
    let default_draw = chat
        .ambient_pet_draw(terminal_area, composer_bottom_y)
        .expect("composer-anchored pet draw request");
    assert_eq!(default_draw.y, 14);

    chat.config.tui_pet_anchor = TuiPetAnchor::ScreenBottom;
    let screen_bottom_draw = chat
        .ambient_pet_draw(terminal_area, composer_bottom_y)
        .expect("screen-bottom anchored pet draw request");
    assert_eq!(screen_bottom_draw.y, 18);
}

#[tokio::test]
#[serial]
async fn ambient_pet_can_be_disabled() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.set_tui_pet(Some(crate::pets::DISABLED_PET_ID.to_string()));

    assert!(chat.ambient_pet.is_none());
}

#[tokio::test]
#[serial]
async fn ambient_pet_reserves_history_wrap_width() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    enable_test_ambient_pet(&mut chat);

    assert_eq!(chat.history_wrap_width(/*width*/ 80), 69);

    chat.set_tui_pet(Some(crate::pets::DISABLED_PET_ID.to_string()));

    assert_eq!(chat.history_wrap_width(/*width*/ 80), 80);
}

#[tokio::test]
#[serial]
async fn ambient_pet_reduces_stream_width_and_composer_text_width() {
    use ratatui::Terminal;

    let (mut with_pet, _with_pet_rx, _with_pet_op_rx) =
        make_chatwidget_manual(/*model_override*/ None).await;
    enable_test_ambient_pet(&mut with_pet);
    with_pet.last_rendered_width.set(Some(80));
    let stream_width_with_pet = with_pet.current_stream_width(/*reserved_cols*/ 2);

    let (mut disabled, _disabled_rx, _disabled_op_rx) =
        make_chatwidget_manual(/*model_override*/ None).await;
    disabled.set_tui_pet(Some(crate::pets::DISABLED_PET_ID.to_string()));
    disabled.last_rendered_width.set(Some(80));
    let stream_width_without_pet = disabled.current_stream_width(/*reserved_cols*/ 2);

    assert_eq!(
        stream_width_with_pet,
        crate::width::usable_content_width(/*total_width*/ 69, /*reserved_cols*/ 2)
    );
    assert_eq!(
        stream_width_without_pet,
        crate::width::usable_content_width(/*total_width*/ 80, /*reserved_cols*/ 2)
    );
    assert!(stream_width_with_pet < stream_width_without_pet);

    let draft =
        "Minim commodo esse elit Lorem exercitation elit ipsum proident labore. Esse culpa aliqua"
            .to_string();
    with_pet
        .bottom_pane
        .set_composer_text(draft.clone(), Vec::new(), Vec::new());
    disabled
        .bottom_pane
        .set_composer_text(draft, Vec::new(), Vec::new());

    let mut with_pet_terminal =
        Terminal::new(TestBackend::new(/*width*/ 80, /*height*/ 6)).expect("create terminal");
    with_pet_terminal
        .draw(|f| with_pet.render(f.area(), f.buffer_mut()))
        .expect("draw pet-enabled chat");
    let mut disabled_terminal =
        Terminal::new(TestBackend::new(/*width*/ 80, /*height*/ 6)).expect("create terminal");
    disabled_terminal
        .draw(|f| disabled.render(f.area(), f.buffer_mut()))
        .expect("draw disabled-pet chat");

    let pet_row = buffer_row_containing(with_pet_terminal.backend().buffer(), "Minim")
        .expect("pet-enabled composer row should render draft");
    let disabled_row = buffer_row_containing(disabled_terminal.backend().buffer(), "Minim")
        .expect("disabled-pet composer row should render draft");

    assert!(row_tail_is_blank(&pet_row, /*start_col*/ 69));
    assert!(!row_tail_is_blank(&disabled_row, /*start_col*/ 69));
}

fn buffer_row_containing(buffer: &ratatui::buffer::Buffer, text: &str) -> Option<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).expect("cell should exist").symbol())
                .collect::<String>()
        })
        .find(|row| row.contains(text))
}

fn row_tail_is_blank(row: &str, start_col: usize) -> bool {
    row.chars().skip(start_col).all(char::is_whitespace)
}

#[tokio::test]
#[serial]
async fn ambient_pet_draw_uses_terminal_screen_area_not_short_inline_viewport() {
    use ratatui::layout::Rect;

    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    enable_test_ambient_pet(&mut chat);

    assert!(
        chat.ambient_pet_draw(
            Rect::new(
                /*x*/ 0, /*y*/ 21, /*width*/ 80, /*height*/ 3,
            ),
            /*composer_bottom_y*/ 24
        )
        .is_none(),
        "a normal short inline viewport cannot fit the ambient pet"
    );

    let draw = chat
        .ambient_pet_draw(
            Rect::new(
                /*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 24,
            ),
            /*composer_bottom_y*/ 24,
        )
        .expect("full terminal screen has room for the ambient pet");
    assert_eq!(draw.x, 71);
    assert_eq!(draw.y, 18);
}

#[tokio::test]
#[serial]
async fn ambient_pet_hides_notification_text_overlay() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    enable_test_ambient_pet(&mut chat);
    for (kind, label) in [
        (crate::pets::PetNotificationKind::Running, "Running"),
        (crate::pets::PetNotificationKind::Waiting, "Needs input"),
        (crate::pets::PetNotificationKind::Review, "Ready"),
        (crate::pets::PetNotificationKind::Failed, "Blocked"),
    ] {
        chat.set_ambient_pet_notification(kind, /*body*/ None);
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).expect("create terminal");
        terminal
            .draw(|f| chat.render(f.area(), f.buffer_mut()))
            .expect("draw ambient pet notification");
        assert!(
            !normalized_backend_snapshot(terminal.backend()).contains(label),
            "did not expect {label} notification text to render"
        );
    }
}
