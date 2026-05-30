use super::*;
use super::tests_support::*;
use crate::test_support::PathBufExt;
use crate::test_support::test_path_buf;
use image::ImageBuffer;
use image::Rgba;
use pretty_assertions::assert_eq;
use std::path::PathBuf;
use tempfile::tempdir;
use crate::app_event::AppEvent;
use crate::bottom_pane::AppEventSender;
use crate::bottom_pane::ChatComposer;
use crate::bottom_pane::InputResult;
use crate::bottom_pane::chat_composer::AttachedImage;
use crate::bottom_pane::chat_composer::LARGE_PASTE_CHAR_THRESHOLD;
use crate::bottom_pane::textarea::TextArea;
use tokio::sync::mpsc::unbounded_channel;

    #[test]
    fn footer_hint_row_is_separated_from_composer() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        composer.render(area, &mut buf);

        let row_to_string = |y: u16| {
            let mut row = String::new();
            for x in 0..area.width {
                row.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
            row
        };

        let mut hint_row: Option<(u16, String)> = None;
        for y in 0..area.height {
            let row = row_to_string(y);
            if row.contains("? for shortcuts") {
                hint_row = Some((y, row));
                break;
            }
        }

        let (hint_row_idx, hint_row_contents) =
            hint_row.expect("expected footer hint row to be rendered");
        assert_eq!(
            hint_row_idx,
            area.height - 1,
            "hint row should occupy the bottom line: {hint_row_contents:?}",
        );

        assert!(
            hint_row_idx > 0,
            "expected a spacing row above the footer hints",
        );

        let spacing_row = row_to_string(hint_row_idx - 1);
        assert_eq!(
            spacing_row.trim(),
            "",
            "expected blank spacing row above hints but saw: {spacing_row:?}",
        );
    }

    #[test]
    fn footer_flash_overrides_footer_hint_override() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_footer_hint_override(Some(vec![("K".to_string(), "label".to_string())]));
        composer.show_footer_flash(Line::from("FLASH"), Duration::from_secs(10));

        let area = Rect::new(0, 0, 60, 6);
        let mut buf = Buffer::empty(area);
        composer.render(area, &mut buf);

        let mut bottom_row = String::new();
        for x in 0..area.width {
            bottom_row.push(
                buf[(x, area.height - 1)]
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' '),
            );
        }
        assert!(
            bottom_row.contains("FLASH"),
            "expected flash content to render in footer row, saw: {bottom_row:?}",
        );
        assert!(
            !bottom_row.contains("K label"),
            "expected flash to override hint override, saw: {bottom_row:?}",
        );
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn remove_recording_meter_placeholder_clears_placeholder_text() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        let id = composer.insert_recording_meter_placeholder("⠤⠤⠤⠤");
        composer.remove_recording_meter_placeholder(&id);

        assert_eq!(composer.textarea.text(), "");
        assert!(composer.textarea.named_element_range(&id).is_none());
    }

    #[test]
    fn footer_flash_expires_and_falls_back_to_hint_override() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_footer_hint_override(Some(vec![("K".to_string(), "label".to_string())]));
        composer.show_footer_flash(Line::from("FLASH"), Duration::from_secs(10));
        composer.footer_flash.as_mut().unwrap().expires_at =
            Instant::now() - Duration::from_secs(1);

        let area = Rect::new(0, 0, 60, 6);
        let mut buf = Buffer::empty(area);
        composer.render(area, &mut buf);

        let mut bottom_row = String::new();
        for x in 0..area.width {
            bottom_row.push(
                buf[(x, area.height - 1)]
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' '),
            );
        }
        assert!(
            bottom_row.contains("K label"),
            "expected hint override to render after flash expired, saw: {bottom_row:?}",
        );
        assert!(
            !bottom_row.contains("FLASH"),
            "expected expired flash to be hidden, saw: {bottom_row:?}",
        );
    }


    #[test]
    fn footer_mode_snapshots() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        snapshot_composer_state(
            "footer_mode_shortcut_overlay",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer.set_esc_backtrack_hint(/*show*/ true);
                let _ = composer
                    .handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
            },
        );

        snapshot_composer_state(
            "footer_mode_ctrl_c_quit",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer.show_quit_shortcut_hint(
                    key_hint::ctrl(KeyCode::Char('c')),
                    /*has_focus*/ true,
                );
            },
        );

        snapshot_composer_state(
            "footer_mode_ctrl_c_interrupt",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer.set_task_running(/*running*/ true);
                composer.show_quit_shortcut_hint(
                    key_hint::ctrl(KeyCode::Char('c')),
                    /*has_focus*/ true,
                );
            },
        );

        snapshot_composer_state(
            "footer_mode_ctrl_c_then_esc_hint",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer.show_quit_shortcut_hint(
                    key_hint::ctrl(KeyCode::Char('c')),
                    /*has_focus*/ true,
                );
                let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            },
        );

        snapshot_composer_state(
            "footer_mode_esc_hint_from_overlay",
            /*enhanced_keys_supported*/ true,
            |composer| {
                let _ = composer
                    .handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
                let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            },
        );

        snapshot_composer_state(
            "footer_mode_esc_hint_backtrack",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer.set_esc_backtrack_hint(/*show*/ true);
                let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            },
        );

        snapshot_composer_state(
            "footer_mode_overlay_then_external_esc_hint",
            /*enhanced_keys_supported*/ true,
            |composer| {
                let _ = composer
                    .handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
                composer.set_esc_backtrack_hint(/*show*/ true);
            },
        );

        snapshot_composer_state(
            "footer_mode_hidden_while_typing",
            /*enhanced_keys_supported*/ true,
            |composer| {
                type_chars_humanlike(composer, &['h']);
            },
        );

        snapshot_composer_state(
            "footer_mode_history_search",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer
                    .history
                    .record_local_submission(HistoryEntry::new("cargo test".to_string()));
                let _ = composer
                    .handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
                let _ = composer
                    .handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
            },
        );

        snapshot_composer_state(
            "footer_mode_shell_command_absorbs_bang",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer.set_status_line_enabled(/*enabled*/ true);
                composer.set_status_line(Some(Line::from(
                    "gpt-5.4 high fast · ~/code/codex-1 · Context 0% used",
                )));
                composer.set_text_content("!git status".to_string(), Vec::new(), Vec::new());
            },
        );

        snapshot_composer_state(
            "footer_mode_shell_command_escape_exits_empty_mode",
            /*enhanced_keys_supported*/ true,
            |composer| {
                composer.set_status_line_enabled(/*enabled*/ true);
                composer.set_status_line(Some(Line::from(
                    "gpt-5.4 high fast · ~/code/codex-1 · Context 0% used",
                )));
                composer.set_text_content("!".to_string(), Vec::new(), Vec::new());
                let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            },
        );
    }

    #[test]
    fn shell_command_cursor_uses_absorbed_prefix() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        let area = Rect::new(0, 0, 40, 5);

        composer.set_text_content("!git".to_string(), Vec::new(), Vec::new());
        composer.move_cursor_to_end();
        assert_eq!(composer.cursor_pos(area), Some((5, 1)));

        composer.set_text_content("! git".to_string(), Vec::new(), Vec::new());
        composer.move_cursor_to_end();
        assert_eq!(composer.cursor_pos(area), Some((6, 1)));
    }

    #[test]
    fn shell_command_uses_shell_accent_style() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_status_line_enabled(/*enabled*/ true);
        composer.set_status_line(Some(Line::from(
            "gpt-5.4 high fast · ~/code/codex-1 · Context 0% used",
        )));
        composer.set_text_content("!git status".to_string(), Vec::new(), Vec::new());

        let area = Rect::new(0, 0, 100, 9);
        let mut buf = Buffer::empty(area);
        composer.render(area, &mut buf);

        let prompt_cell = &buf[(0, 1)];
        assert_eq!(prompt_cell.symbol(), "!");
        assert_eq!(prompt_cell.style().fg, Some(Color::LightRed));

        let footer_y = area.height - 1;
        let footer_text = (0..area.width)
            .map(|x| buf[(x, footer_y)].symbol().chars().next().unwrap_or(' '))
            .collect::<String>();
        let shell_label_x = footer_text
            .find("Shell mode")
            .expect("expected shell mode footer label");
        assert_eq!(
            buf[(shell_label_x as u16, footer_y)].style().fg,
            Some(Color::LightRed)
        );
    }

    #[test]
    fn status_line_hyperlink_marks_pr_number_cells() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        let url = "https://github.com/openai/codex/pull/20252";
        composer.set_status_line_enabled(/*enabled*/ true);
        composer.set_status_line(Some(Line::from(Span::styled(
            "PR #20252",
            Style::default().cyan().underlined(),
        ))));
        composer.set_status_line_hyperlink(Some(url.to_string()));

        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        composer.render(area, &mut buf);

        let marked_cells = (area.top()..area.bottom())
            .flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
            .filter(|&(x, y)| buf[(x, y)].symbol().contains(url))
            .count();
        assert_eq!(
            marked_cells,
            "PR #20252".chars().filter(|ch| !ch.is_whitespace()).count()
        );
    }

    #[test]
    fn esc_exits_empty_shell_mode() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        type_chars_humanlike(&mut composer, &['!']);
        assert!(composer.is_bash_mode);
        assert_eq!(composer.current_text(), "!");

        let (result, needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(matches!(result, InputResult::None));
        assert!(needs_redraw);
        assert!(!composer.is_bash_mode);
        assert_eq!(composer.current_text(), "");
    }

    #[test]
    fn esc_keeps_shell_mode_when_paste_burst_flushes_pending_text() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        type_chars_humanlike(&mut composer, &['!']);
        let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert!(composer.is_in_paste_burst());
        assert_eq!(composer.current_text(), "!");

        let (result, needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(matches!(result, InputResult::None));
        assert!(needs_redraw);
        assert!(composer.is_bash_mode);
        assert_eq!(composer.current_text(), "!g");
    }

    #[test]
    fn footer_collapse_snapshots() {
        fn setup_collab_footer(
            composer: &mut ChatComposer,
            context_percent: i64,
            indicator: Option<CollaborationModeIndicator>,
        ) {
            composer.set_collaboration_modes_enabled(/*enabled*/ true);
            composer.set_collaboration_mode_indicator(indicator);
            composer.set_context_window(Some(context_percent), /*used_tokens*/ None);
        }

        // Empty textarea, agent idle: shortcuts hint can show, and cycle hint is hidden.
        snapshot_composer_state_with_width(
            "footer_collapse_empty_full",
            /*width*/ 120,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 100, /*indicator*/ None,
                );
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_empty_mode_cycle_with_context",
            /*width*/ 60,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 100, /*indicator*/ None,
                );
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_empty_mode_cycle_without_context",
            /*width*/ 44,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 100, /*indicator*/ None,
                );
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_empty_mode_only",
            /*width*/ 26,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 100, /*indicator*/ None,
                );
            },
        );

        // Empty textarea, plan mode idle: shortcuts hint and cycle hint are available.
        snapshot_composer_state_with_width(
            "footer_collapse_plan_empty_full",
            /*width*/ 120,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 100,
                    Some(CollaborationModeIndicator::Plan),
                );
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_plan_empty_mode_cycle_with_context",
            /*width*/ 60,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 100,
                    Some(CollaborationModeIndicator::Plan),
                );
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_plan_empty_mode_cycle_without_context",
            /*width*/ 44,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 100,
                    Some(CollaborationModeIndicator::Plan),
                );
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_plan_empty_mode_only",
            /*width*/ 26,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 100,
                    Some(CollaborationModeIndicator::Plan),
                );
            },
        );

        // Textarea has content, agent running: queue hint is shown.
        snapshot_composer_state_with_width(
            "footer_collapse_queue_full",
            /*width*/ 120,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 98, /*indicator*/ None,
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_queue_short_with_context",
            /*width*/ 50,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 98, /*indicator*/ None,
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_queue_message_without_context",
            /*width*/ 40,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 98, /*indicator*/ None,
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_queue_short_without_context",
            /*width*/ 30,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 98, /*indicator*/ None,
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_queue_mode_only",
            /*width*/ 20,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer, /*context_percent*/ 98, /*indicator*/ None,
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );

        // Textarea has content, plan mode active, agent running: queue hint + mode.
        snapshot_composer_state_with_width(
            "footer_collapse_plan_queue_full",
            /*width*/ 120,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 98,
                    Some(CollaborationModeIndicator::Plan),
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_plan_queue_short_with_context",
            /*width*/ 50,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 98,
                    Some(CollaborationModeIndicator::Plan),
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_plan_queue_message_without_context",
            /*width*/ 40,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 98,
                    Some(CollaborationModeIndicator::Plan),
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_plan_queue_short_without_context",
            /*width*/ 30,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 98,
                    Some(CollaborationModeIndicator::Plan),
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
        snapshot_composer_state_with_width(
            "footer_collapse_plan_queue_mode_only",
            /*width*/ 20,
            /*enhanced_keys_supported*/ true,
            |composer| {
                setup_collab_footer(
                    composer,
                    /*context_percent*/ 98,
                    Some(CollaborationModeIndicator::Plan),
                );
                composer.set_task_running(/*running*/ true);
                composer.set_text_content("Test".to_string(), Vec::new(), Vec::new());
            },
        );
    }
