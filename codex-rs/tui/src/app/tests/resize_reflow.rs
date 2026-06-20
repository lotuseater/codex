use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn capped_resize_reflow_renders_recent_suffix_only() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    app.config.terminal_resize_reflow.max_rows = TerminalResizeReflowMaxRows::Limit(5);
    app.transcript_cells = (0..20)
        .map(|i| plain_line_cell(format!("cell {i}")))
        .collect();

    let rendered = app.render_transcript_lines_for_reflow(/*width*/ 80);

    assert_eq!(rendered.lines.len(), 5);
    assert_eq!(
        rendered
            .lines
            .iter()
            .map(rendered_line_text)
            .collect::<Vec<_>>(),
        vec![
            "cell 17".to_string(),
            String::new(),
            "cell 18".to_string(),
            String::new(),
            "cell 19".to_string(),
        ]
    );
}

#[tokio::test]
async fn uncapped_resize_reflow_renders_all_cells_when_row_cap_absent() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    app.config.terminal_resize_reflow.max_rows = TerminalResizeReflowMaxRows::Disabled;
    app.transcript_cells = (0..20)
        .map(|i| plain_line_cell(format!("cell {i}")))
        .collect();

    let rendered = app.render_transcript_lines_for_reflow(/*width*/ 80);

    assert_eq!(rendered.lines.len(), 39);
    assert_eq!(rendered_line_text(&rendered.lines[0]), "cell 0");
    assert_eq!(rendered_line_text(&rendered.lines[38]), "cell 19");
}

#[tokio::test]
async fn resize_reflow_wraps_transcript_early_when_pet_is_enabled() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    app.config.terminal_resize_reflow.max_rows = TerminalResizeReflowMaxRows::Disabled;
    app.transcript_cells = vec![Arc::new(AgentMarkdownCell::new(
        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda".to_string(),
        Path::new("/tmp"),
    ))];

    let without_pet = app.render_transcript_lines_for_reflow(/*width*/ 40);
    app.chat_widget
        .set_pet_image_support_for_tests(crate::pets::PetImageSupport::Supported(
            crate::pets::ImageProtocol::Kitty,
        ));
    app.chat_widget
        .install_test_ambient_pet_for_tests(/*animations_enabled*/ false);
    let width = app.chat_widget.history_wrap_width(/*width*/ 40);
    assert!(width < 40);
    let with_pet = app.render_transcript_lines_for_reflow(width);

    assert!(
        with_pet.lines.len() > without_pet.lines.len(),
        "expected pet-enabled transcript reflow to wrap earlier"
    );
}

#[tokio::test]
async fn uncapped_resize_reflow_renders_all_cells_under_row_limit() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    app.config.terminal_resize_reflow.max_rows = TerminalResizeReflowMaxRows::Limit(100);
    app.transcript_cells = (0..3)
        .map(|i| plain_line_cell(format!("cell {i}")))
        .collect();

    let rendered = app.render_transcript_lines_for_reflow(/*width*/ 80);

    assert_eq!(
        rendered
            .lines
            .iter()
            .map(rendered_line_text)
            .collect::<Vec<_>>(),
        vec![
            "cell 0".to_string(),
            String::new(),
            "cell 1".to_string(),
            String::new(),
            "cell 2".to_string(),
        ]
    );
}

#[tokio::test]
async fn initial_replay_buffer_keeps_recent_rows_when_row_cap_present() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    enable_terminal_resize_reflow(&mut app);
    app.config.terminal_resize_reflow.max_rows = TerminalResizeReflowMaxRows::Limit(3);

    app.begin_initial_history_replay_buffer();
    for index in 0..5 {
        App::buffer_initial_history_replay_display_lines(
            app.initial_history_replay_buffer
                .as_mut()
                .expect("initial replay buffer active"),
            vec![Line::from(format!("line {index}"))],
            /*max_rows*/ 3,
        );
    }

    let buffer = app
        .initial_history_replay_buffer
        .as_ref()
        .expect("initial replay buffer should remain active");
    assert_eq!(
        buffer
            .retained_lines
            .iter()
            .map(rendered_line_text)
            .collect::<Vec<_>>(),
        vec![
            "line 2".to_string(),
            "line 3".to_string(),
            "line 4".to_string(),
        ]
    );
}

#[tokio::test]
async fn thread_switch_replay_buffer_uses_transcript_tail_mode_when_row_cap_present() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    enable_terminal_resize_reflow(&mut app);
    app.config.terminal_resize_reflow.max_rows = TerminalResizeReflowMaxRows::Limit(3);

    app.begin_thread_switch_history_replay_buffer();

    let buffer = app
        .initial_history_replay_buffer
        .as_ref()
        .expect("thread switch replay buffer should be active");
    assert!(buffer.render_from_transcript_tail);
    assert!(buffer.retained_lines.is_empty());
}

#[tokio::test]
async fn thread_switch_replay_buffer_is_disabled_without_row_cap() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    enable_terminal_resize_reflow(&mut app);
    app.config.terminal_resize_reflow.max_rows = TerminalResizeReflowMaxRows::Disabled;

    app.begin_thread_switch_history_replay_buffer();

    assert!(app.initial_history_replay_buffer.is_none());
}

#[tokio::test]
async fn height_shrink_schedules_resize_reflow() {
    let (mut app, _rx, _op_rx) = make_test_app_with_channels().await;
    enable_terminal_resize_reflow(&mut app);
    let frame_requester = crate::tui::FrameRequester::test_dummy();

    assert!(!app.handle_draw_size_change(
        ratatui::layout::Size::new(/*width*/ 118, /*height*/ 35),
        ratatui::layout::Size::new(/*width*/ 118, /*height*/ 35),
        &frame_requester,
    ));

    assert!(app.handle_draw_size_change(
        ratatui::layout::Size::new(/*width*/ 118, /*height*/ 24),
        ratatui::layout::Size::new(/*width*/ 118, /*height*/ 35),
        &frame_requester,
    ));
    assert!(app.transcript_reflow.has_pending_reflow());
}
