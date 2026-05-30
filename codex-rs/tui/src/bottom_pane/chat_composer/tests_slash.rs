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
    fn service_tier_slash_command_dispatches_from_catalog_name() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_service_tier_commands_enabled(/*enabled*/ true);
        composer.set_service_tier_commands(vec![ServiceTierCommand {
            id: "priority".to_string(),
            name: "fast".to_string(),
            description: "Fastest inference with increased plan usage".to_string(),
        }]);
        type_chars_humanlike(&mut composer, &['/', 'f', 'a', 's', 't']);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(
            result,
            InputResult::ServiceTierCommand(ServiceTierCommand {
                id: "priority".to_string(),
                name: "fast".to_string(),
                description: "Fastest inference with increased plan usage".to_string(),
            })
        );
    }


    #[test]
    fn slash_init_dispatches_command_and_does_not_submit_literal_text() {
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

        // Type the slash command.
        type_chars_humanlike(&mut composer, &['/', 'i', 'n', 'i', 't']);

        // Press Enter to dispatch the selected command.
        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        // When a slash command is dispatched, the composer should return a
        // Command result (not submit literal text) and clear its textarea.
        match result {
            InputResult::Command(cmd) => {
                assert_eq!(cmd.command(), "init");
            }
            InputResult::CommandWithArgs(_, _, _) => {
                panic!("expected command dispatch without args for '/init'")
            }
            InputResult::ServiceTierCommand(command) => {
                panic!("expected init command, got service tier {command:?}")
            }
            InputResult::Submitted { text, .. } => {
                panic!("expected command dispatch, but composer submitted literal text: {text}")
            }
            InputResult::Queued { .. } => {
                panic!("expected command dispatch, but composer queued literal text")
            }
            InputResult::None => panic!("expected Command result for '/init'"),
        }
        assert!(composer.textarea.is_empty(), "composer should be cleared");
    }

    #[test]
    fn kill_buffer_persists_after_submit() {
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
        composer.set_steer_enabled(true);
        composer.textarea.insert_str("restore me");
        composer.textarea.set_cursor(/*pos*/ 0);

        let (_result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(composer.textarea.is_empty());

        composer.textarea.insert_str("hello");
        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(result, InputResult::Submitted { .. }));
        assert!(composer.textarea.is_empty());

        let (_result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(composer.textarea.text(), "restore me");
    }

    #[test]
    fn kill_buffer_persists_after_slash_command_dispatch() {
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
        composer.textarea.insert_str("restore me");
        composer.textarea.set_cursor(/*pos*/ 0);

        let (_result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(composer.textarea.is_empty());

        composer.textarea.insert_str("/diff");
        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        match result {
            InputResult::Command(cmd) => {
                assert_eq!(cmd.command(), "diff");
            }
            _ => panic!("expected Command result for '/diff'"),
        }
        assert!(composer.textarea.is_empty());

        let (_result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(composer.textarea.text(), "restore me");
    }

    #[test]
    fn slash_command_disabled_while_task_running_keeps_text() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, mut rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_task_running(/*running*/ true);
        composer
            .textarea
            .set_text_clearing_elements("/review these changes");

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(InputResult::None, result);
        assert_eq!("/review these changes", composer.textarea.text());

        let mut found_error = false;
        while let Ok(event) = rx.try_recv() {
            if let AppEvent::InsertHistoryCell(cell) = event {
                let message = cell
                    .display_lines(/*width*/ 80)
                    .into_iter()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(message.contains("disabled while a task is in progress"));
                found_error = true;
                break;
            }
        }
        assert!(found_error, "expected error history cell to be sent");
    }

    #[test]
    fn enter_queues_when_queue_submissions_is_enabled() {
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
        composer.set_queue_submissions(/*queue_submissions*/ true);
        composer
            .draft
            .textarea
            .set_text_clearing_elements("queued before session");

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(
            result,
            InputResult::Queued {
                text: "queued before session".to_string(),
                text_elements: Vec::new(),
                action: QueuedInputAction::Plain,
            }
        );
    }

    #[test]
    fn tab_queues_slash_led_prompts_while_task_running_without_validation() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        fn assert_queued_slash(input: &str) {
            let (tx, mut rx) = unbounded_channel::<AppEvent>();
            let sender = AppEventSender::new(tx);
            let mut composer = ChatComposer::new(
                /*has_input_focus*/ true,
                sender,
                /*enhanced_keys_supported*/ false,
                "Ask Codex to do anything".to_string(),
                /*disable_paste_burst*/ false,
            );
            composer.set_task_running(/*running*/ true);
            composer.textarea.set_text_clearing_elements(input);

            let (result, _needs_redraw) =
                composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

            match result {
                InputResult::Queued {
                    text,
                    text_elements,
                    action,
                } => {
                    assert_eq!(text, input);
                    assert!(text_elements.is_empty());
                    assert_eq!(action, QueuedInputAction::ParseSlash);
                }
                other => panic!("expected slash-led input to queue, got {other:?}"),
            }
            assert!(composer.textarea.is_empty());
            assert!(
                rx.try_recv().is_err(),
                "queueing should not report slash errors"
            );
        }

        assert_queued_slash("/compact");
        assert_queued_slash("/review check regressions");
        assert_queued_slash("/fast");
        assert_queued_slash("/does-not-exist");
    }

    #[test]
    fn remapped_submit_does_not_fall_back_to_enter() {
        use crate::key_hint;
        use crate::keymap::RuntimeKeymap;
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
        composer
            .textarea
            .set_text_clearing_elements("explain the change");
        composer.textarea.set_cursor(composer.textarea.text().len());
        let mut keymap = RuntimeKeymap::defaults();
        keymap.composer.submit = vec![key_hint::ctrl(KeyCode::Char('j'))];
        composer.set_keymap_bindings(&keymap);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(InputResult::None, result);
        assert_eq!("explain the change\n", composer.textarea.text());
    }

    #[test]
    fn remapped_queue_does_not_fall_back_to_tab() {
        use crate::key_hint;
        use crate::keymap::RuntimeKeymap;
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
        composer.set_task_running(/*running*/ true);
        composer.textarea.set_text_clearing_elements("queue me");
        let mut keymap = RuntimeKeymap::defaults();
        keymap.composer.queue = vec![key_hint::ctrl(KeyCode::Char('q'))];
        composer.set_keymap_bindings(&keymap);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(InputResult::None, result);
        assert_eq!("queue me", composer.textarea.text());
    }

    #[test]
    fn remapped_history_search_does_not_fall_back_to_ctrl_r() {
        use crate::key_hint;
        use crate::keymap::RuntimeKeymap;
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
        let mut keymap = RuntimeKeymap::defaults();
        keymap.composer.history_search_previous = vec![key_hint::plain(KeyCode::F(2))];
        composer.set_keymap_bindings(&keymap);

        let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert!(!composer.history_search_active());

        let _ = composer.handle_key_event(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert!(composer.history_search_active());
    }

    #[test]
    fn tab_queues_leading_space_slash_as_plain_text_while_task_running() {
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
        composer.set_task_running(/*running*/ true);
        composer
            .textarea
            .set_text_clearing_elements(" /does-not-exist");

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        match result {
            InputResult::Queued { text, action, .. } => {
                assert_eq!(text, "/does-not-exist");
                assert_eq!(action, QueuedInputAction::Plain);
            }
            other => panic!("expected leading-space slash input to queue, got {other:?}"),
        }
    }

    #[test]
    fn tab_queues_bang_shell_prompts_while_task_running_without_execution() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        fn assert_queued_shell(input: &str, expected_text: &str) {
            let (tx, mut rx) = unbounded_channel::<AppEvent>();
            let sender = AppEventSender::new(tx);
            let mut composer = ChatComposer::new(
                /*has_input_focus*/ true,
                sender,
                /*enhanced_keys_supported*/ false,
                "Ask Codex to do anything".to_string(),
                /*disable_paste_burst*/ false,
            );
            composer.set_task_running(/*running*/ true);
            composer.textarea.set_text_clearing_elements(input);

            let (result, _needs_redraw) =
                composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

            match result {
                InputResult::Queued {
                    text,
                    text_elements,
                    action,
                } => {
                    assert_eq!(text, expected_text);
                    assert!(text_elements.is_empty());
                    assert_eq!(action, QueuedInputAction::RunShell);
                }
                other => panic!("expected bang shell input to queue, got {other:?}"),
            }
            assert!(composer.textarea.is_empty());
            assert!(
                rx.try_recv().is_err(),
                "queueing should not show shell help immediately"
            );
        }

        assert_queued_shell("!echo hi", "!echo hi");
        assert_queued_shell("!", "!");
        assert_queued_shell(" !echo hi", "!echo hi");
    }

    #[test]
    fn slash_tab_completion_moves_cursor_to_end() {
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

        type_chars_humanlike(&mut composer, &['/', 'c']);

        let (_result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(composer.textarea.text(), "/compact ");
        assert_eq!(composer.textarea.cursor(), composer.textarea.text().len());
    }

    #[test]
    fn slash_tab_completion_wins_over_queueing_while_task_running() {
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
        composer.set_task_running(/*running*/ true);

        type_chars_humanlike(&mut composer, &['/', 'm', 'o']);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(result, InputResult::None);
        assert_eq!(composer.textarea.text(), "/model ");
        assert_eq!(composer.textarea.cursor(), composer.textarea.text().len());
    }

    #[test]
    fn slash_key_completes_selected_slash_command_as_text() {
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

        type_chars_humanlike(&mut composer, &['/', 'm']);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));

        assert_eq!(result, InputResult::None);
        assert_eq!(composer.textarea.text(), "/model ");
        assert_eq!(composer.textarea.cursor(), composer.textarea.text().len());
    }

    #[test]
    fn slash_tab_then_enter_dispatches_builtin_command() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        // Type a prefix and complete with Tab, which inserts a trailing space
        // and moves the cursor beyond the '/name' token (hides the popup).
        type_chars_humanlike(&mut composer, &['/', 'd', 'i']);
        let (_res, _redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(composer.textarea.text(), "/diff ");

        // Press Enter: should dispatch the command, not submit literal text.
        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        match result {
            InputResult::Command(cmd) => assert_eq!(cmd.command(), "diff"),
            InputResult::CommandWithArgs(_, _, _) => {
                panic!("expected command dispatch without args for '/diff'")
            }
            InputResult::ServiceTierCommand(command) => {
                panic!("expected diff command, got service tier {command:?}")
            }
            InputResult::Submitted { text, .. } => {
                panic!("expected command dispatch after Tab completion, got literal submit: {text}")
            }
            InputResult::Queued { .. } => {
                panic!("expected command dispatch after Tab completion, got literal queue")
            }
            InputResult::None => panic!("expected Command result for '/diff'"),
        }
        assert!(composer.textarea.is_empty());
    }

    #[test]
    fn slash_command_elementizes_on_space() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_collaboration_modes_enabled(/*enabled*/ true);

        type_chars_humanlike(&mut composer, &['/', 'p', 'l', 'a', 'n', ' ']);

        let text = composer.textarea.text().to_string();
        let elements = composer.textarea.text_elements();
        assert_eq!(text, "/plan ");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].placeholder(&text), Some("/plan"));
    }

    #[test]
    fn slash_command_elementizes_only_known_commands() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_collaboration_modes_enabled(/*enabled*/ true);

        type_chars_humanlike(&mut composer, &['/', 'U', 's', 'e', 'r', 's', ' ']);

        let text = composer.textarea.text().to_string();
        let elements = composer.textarea.text_elements();
        assert_eq!(text, "/Users ");
        assert!(elements.is_empty());
    }

    #[test]
    fn slash_command_element_removed_when_not_at_start() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        type_chars_humanlike(&mut composer, &['/', 'r', 'e', 'v', 'i', 'e', 'w', ' ']);

        let text = composer.textarea.text().to_string();
        let elements = composer.textarea.text_elements();
        assert_eq!(text, "/review ");
        assert_eq!(elements.len(), 1);

        composer.textarea.set_cursor(/*pos*/ 0);
        type_chars_humanlike(&mut composer, &['x']);

        let text = composer.textarea.text().to_string();
        let elements = composer.textarea.text_elements();
        assert_eq!(text, "x/review ");
        assert!(elements.is_empty());
    }

    #[test]
    fn tab_submits_when_no_task_running() {
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

        type_chars_humanlike(&mut composer, &['h', 'i']);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        assert!(matches!(
            result,
            InputResult::Submitted { ref text, .. } if text == "hi"
        ));
        assert!(composer.textarea.is_empty());
    }

    #[test]
    fn tab_does_not_submit_for_bang_shell_command() {
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
        composer.set_task_running(/*running*/ false);

        type_chars_humanlike(&mut composer, &['!', 'l', 's']);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        assert!(matches!(result, InputResult::None));
        assert!(
            composer.current_text().starts_with("!ls"),
            "expected Tab not to submit or clear a `!` command"
        );
    }

    #[test]
    fn bang_prefixed_slash_text_submits_literal_shell_command() {
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

        type_chars_humanlike(&mut composer, &['!', '/', 'd', 'i', 'f', 'f']);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(
            result,
            InputResult::Submitted { ref text, .. } if text == "!/diff"
        ));
    }

    #[test]
    fn slash_mention_dispatches_command_and_inserts_at() {
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

        type_chars_humanlike(&mut composer, &['/', 'm', 'e', 'n', 't', 'i', 'o', 'n']);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        match result {
            InputResult::Command(cmd) => {
                assert_eq!(cmd.command(), "mention");
            }
            InputResult::CommandWithArgs(_, _, _) => {
                panic!("expected command dispatch without args for '/mention'")
            }
            InputResult::ServiceTierCommand(command) => {
                panic!("expected mention command, got service tier {command:?}")
            }
            InputResult::Submitted { text, .. } => {
                panic!("expected command dispatch, but composer submitted literal text: {text}")
            }
            InputResult::Queued { .. } => {
                panic!("expected command dispatch, but composer queued literal text")
            }
            InputResult::None => panic!("expected Command result for '/mention'"),
        }
        assert!(composer.textarea.is_empty(), "composer should be cleared");
        composer.insert_str("@");
        assert_eq!(composer.textarea.text(), "@");
    }

    #[test]
    fn slash_plan_args_preserve_text_elements() {
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
        composer.set_collaboration_modes_enabled(/*enabled*/ true);

        type_chars_humanlike(&mut composer, &['/', 'p', 'l', 'a', 'n', ' ']);
        let placeholder = local_image_label_text(/*label_number*/ 1);
        composer.attach_image(PathBuf::from("/tmp/plan.png"));

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        match result {
            InputResult::CommandWithArgs(cmd, args, text_elements) => {
                assert_eq!(cmd.command(), "plan");
                assert_eq!(args, placeholder);
                assert_eq!(text_elements.len(), 1);
                assert_eq!(
                    text_elements[0].placeholder(&args),
                    Some(placeholder.as_str())
                );
            }
            _ => panic!("expected CommandWithArgs for /plan with args"),
        }
    }

    #[test]
    fn file_completion_preserves_large_paste_placeholder_elements() {
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

        let large = "x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 5);
        let placeholder = format!("[Pasted Content {} chars]", large.chars().count());

        composer.handle_paste(large.clone());
        composer.insert_str(" @ma");
        composer.on_file_search_result(
            "ma".to_string(),
            vec![FileMatch {
                score: 1,
                path: PathBuf::from("src/main.rs"),
                match_type: codex_file_search::MatchType::File,
                root: PathBuf::from("/tmp"),
                indices: None,
            }],
        );

        let (_result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        let text = composer.textarea.text().to_string();
        assert_eq!(text, format!("{placeholder} src/main.rs "));
        let elements = composer.textarea.text_elements();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].placeholder(&text), Some(placeholder.as_str()));

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        match result {
            InputResult::Submitted {
                text,
                text_elements,
            } => {
                assert_eq!(text, format!("{large} src/main.rs"));
                assert!(text_elements.is_empty());
            }
            _ => panic!("expected Submitted"),
        }
    }

    /// Behavior: multiple paste operations can coexist; placeholders should be expanded to their
    /// original content on submission.
