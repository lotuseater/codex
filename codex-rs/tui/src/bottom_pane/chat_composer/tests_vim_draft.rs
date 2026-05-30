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
    fn esc_hint_stays_hidden_with_draft_content() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        type_chars_humanlike(&mut composer, &['d']);

        assert!(!composer.is_empty());
        assert_eq!(composer.current_text(), "d");
        assert_eq!(composer.footer_mode, FooterMode::ComposerEmpty);
        assert!(matches!(composer.active_popup, ActivePopup::None));

        let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(composer.footer_mode, FooterMode::ComposerEmpty);
        assert!(!composer.esc_backtrack_hint);
    }

    #[test]
    fn empty_vim_insert_escape_enters_normal_without_esc_hint() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_vim_enabled(/*enabled*/ true);
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));

        assert!(composer.is_empty());
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Insert".green())
        );

        let (result, needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(matches!(result, InputResult::None));
        assert!(needs_redraw);
        assert!(composer.is_empty());
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Normal".magenta())
        );
        assert_eq!(composer.footer_mode, FooterMode::ComposerEmpty);
        assert!(!composer.esc_backtrack_hint);
    }

    #[test]
    fn slash_opens_command_popup_in_vim_normal_mode() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ true,
        );
        composer.set_vim_enabled(/*enabled*/ true);

        let (result, needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));

        assert!(matches!(result, InputResult::None));
        assert!(needs_redraw);
        assert_eq!(composer.textarea.text(), "/");
        assert_eq!(composer.textarea.cursor(), "/".len());
        assert!(matches!(composer.active_popup, ActivePopup::Command(_)));
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Insert".green())
        );
    }

    #[test]
    fn slash_command_can_be_typed_and_dispatched_after_vim_normal_slash() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ true,
        );
        composer.set_vim_enabled(/*enabled*/ true);

        for ch in ['/', 'd', 'i', 'f', 'f'] {
            let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        assert_eq!(composer.textarea.text(), "/diff");
        assert!(matches!(composer.active_popup, ActivePopup::Command(_)));

        let (result, needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(needs_redraw);
        assert!(composer.is_empty());
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Normal".magenta())
        );
        assert!(matches!(result, InputResult::Command(SlashCommand::Diff)));
    }

    #[test]
    fn inline_slash_command_dispatch_resets_vim_mode_to_normal() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ true,
        );
        composer.set_collaboration_modes_enabled(/*enabled*/ true);
        composer.set_vim_enabled(/*enabled*/ true);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        composer.set_text_content("/plan investigate this".to_string(), Vec::new(), Vec::new());
        composer.active_popup = ActivePopup::None;
        let (result, needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(needs_redraw);
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Normal".magenta())
        );
        match result {
            InputResult::CommandWithArgs(cmd, args, text_elements) => {
                assert_eq!(cmd, SlashCommand::Plan);
                assert_eq!(args, "investigate this");
                assert!(text_elements.is_empty());
            }
            _ => panic!("expected CommandWithArgs"),
        }
    }

    #[test]
    fn bang_enters_shell_mode_in_vim_normal_mode() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ true,
        );
        composer.set_vim_enabled(/*enabled*/ true);

        let (result, needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE));

        assert!(matches!(result, InputResult::None));
        assert!(needs_redraw);
        assert!(composer.is_bash_mode);
        assert_eq!(composer.current_text(), "!");
        assert_eq!(composer.textarea.text(), "");
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Insert".green())
        );
    }

    #[test]
    fn shell_command_can_be_typed_after_vim_normal_bang() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ true,
        );
        composer.set_vim_enabled(/*enabled*/ true);

        for ch in ['!', 'e', 'c', 'h', 'o'] {
            let _ = composer.handle_key_event(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }

        assert!(composer.is_bash_mode);
        assert_eq!(composer.current_text(), "!echo");
        assert_eq!(composer.textarea.text(), "echo");
        assert!(matches!(composer.active_popup, ActivePopup::None));
    }

    #[test]
    fn base_footer_mode_tracks_empty_state_after_quit_hint_expires() {
        use crossterm::event::KeyCode;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        type_chars_humanlike(&mut composer, &['d']);
        composer
            .show_quit_shortcut_hint(key_hint::ctrl(KeyCode::Char('c')), /*has_focus*/ true);
        composer.quit_shortcut_expires_at =
            Some(Instant::now() - std::time::Duration::from_secs(1));

        assert_eq!(composer.footer_mode(), FooterMode::ComposerHasDraft);

        composer.set_text_content(String::new(), Vec::new(), Vec::new());
        assert_eq!(composer.footer_mode(), FooterMode::ComposerEmpty);
    }

    #[test]
    fn clear_for_ctrl_c_records_cleared_draft() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        composer.set_text_content("draft text".to_string(), Vec::new(), Vec::new());
        assert_eq!(composer.clear_for_ctrl_c(), Some("draft text".to_string()));
        assert!(composer.is_empty());

        assert_eq!(
            composer.history.navigate_up(&composer.app_event_tx),
            Some(HistoryEntry::new("draft text".to_string()))
        );
    }

    #[test]
    fn clear_for_ctrl_c_preserves_pending_paste_history_entry() {
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
        composer.handle_paste(large.clone());
        let char_count = large.chars().count();
        let placeholder = format!("[Pasted Content {char_count} chars]");
        assert_eq!(composer.textarea.text(), placeholder);
        assert_eq!(
            composer.pending_pastes,
            vec![(placeholder.clone(), large.clone())]
        );

        composer.clear_for_ctrl_c();
        assert!(composer.is_empty());

        let history_entry = composer
            .history
            .navigate_up(&composer.app_event_tx)
            .expect("expected history entry");
        let text_elements = vec![TextElement::new(
            (0..placeholder.len()).into(),
            Some(placeholder.clone()),
        )];
        assert_eq!(
            history_entry,
            HistoryEntry::with_pending(
                placeholder.clone(),
                text_elements,
                Vec::new(),
                vec![(placeholder.clone(), large.clone())]
            )
        );

        composer.apply_history_entry(history_entry);
        assert_eq!(composer.textarea.text(), placeholder);
        assert_eq!(composer.pending_pastes, vec![(placeholder.clone(), large)]);
        assert_eq!(composer.textarea.element_payloads(), vec![placeholder]);

        let (result, _needs_redraw) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        match result {
            InputResult::Submitted {
                text,
                text_elements,
            } => {
                assert_eq!(text, "x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 5));
                assert!(text_elements.is_empty());
            }
            _ => panic!("expected Submitted"),
        }
    }

    #[test]
    fn large_paste_numbering_reuses_after_ctrl_c_clear() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        let paste = "x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 4);
        let base = format!("[Pasted Content {} chars]", paste.chars().count());

        composer.handle_paste(paste.clone());
        assert_eq!(composer.textarea.text(), base);
        assert_eq!(composer.pending_pastes.len(), 1);

        assert_eq!(composer.clear_for_ctrl_c(), Some(base.clone()));
        assert!(composer.textarea.text().is_empty());
        assert!(composer.pending_pastes.is_empty());

        composer.handle_paste(paste);
        assert_eq!(composer.textarea.text(), base);
        assert_eq!(composer.pending_pastes.len(), 1);
        assert_eq!(composer.pending_pastes[0].0, base);
    }

    #[test]
    fn vim_mode_resets_to_normal_after_submission() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_steer_enabled(/*enabled*/ true);
        composer.set_vim_enabled(/*enabled*/ true);

        assert!(composer.textarea.is_vim_enabled());
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Normal".magenta())
        );

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        composer.set_text_content("h".to_string(), Vec::new(), Vec::new());
        let (result, _) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(composer.textarea.is_vim_enabled());
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Normal".magenta())
        );
        assert!(composer.is_empty());
        match result {
            InputResult::Submitted { text, .. } => assert_eq!(text, "h"),
            _ => panic!("expected Submitted"),
        }
    }

    #[test]
    fn vim_mode_resets_to_normal_after_queued_submission() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_steer_enabled(/*enabled*/ true);
        composer.set_task_running(/*running*/ true);
        composer.set_vim_enabled(/*enabled*/ true);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        composer.set_text_content("queued".to_string(), Vec::new(), Vec::new());
        let (result, _) = composer.handle_submission(/*should_queue*/ true);

        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Normal".magenta())
        );
        assert!(composer.is_empty());
        match result {
            InputResult::Queued { text, .. } => assert_eq!(text, "queued"),
            _ => panic!("expected Queued"),
        }
    }

    #[test]
    fn vim_mode_stays_insert_after_suppressed_submission() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_steer_enabled(/*enabled*/ true);
        composer.set_vim_enabled(/*enabled*/ true);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        composer.set_text_content("/not-a-command".to_string(), Vec::new(), Vec::new());
        let (result, _) =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(result, InputResult::None));
        assert_eq!(composer.textarea.text(), "/not-a-command");
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Insert".green())
        );
    }

    #[test]
    fn esc_switches_vim_insert_to_normal() {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        composer.set_vim_enabled(/*enabled*/ true);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        composer.set_text_content("hey".to_string(), Vec::new(), Vec::new());
        composer.textarea.set_cursor(composer.textarea.text().len());
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Insert".green())
        );
        assert_eq!(composer.textarea.cursor(), "hey".len());

        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(
            composer.vim_mode_indicator_span(),
            Some("Vim: Normal".magenta())
        );
        assert_eq!(composer.textarea.cursor(), "he".len());
    }

    #[test]
    fn vim_insert_uses_bar_cursor_style() {
        use crate::render::renderable::Renderable;
        use crossterm::cursor::SetCursorStyle;
        use crossterm::event::KeyCode;
        use crossterm::event::KeyEvent;
        use crossterm::event::KeyModifiers;
        use crossterm::queue;

        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        let area = Rect::new(0, 0, 80, 10);
        let style_output = |style| {
            let mut output = Vec::new();
            queue!(output, style).expect("queue cursor style");
            output
        };
        let default = style_output(SetCursorStyle::DefaultUserShape);
        let steady_bar = style_output(SetCursorStyle::SteadyBar);

        assert_eq!(style_output(composer.cursor_style(area)), default,);

        composer.set_vim_enabled(/*enabled*/ true);
        assert_eq!(style_output(composer.cursor_style(area)), default,);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        composer.set_text_content("hey".to_string(), Vec::new(), Vec::new());
        assert_eq!(style_output(composer.cursor_style(area)), steady_bar);

        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(style_output(composer.cursor_style(area)), default,);
    }

    #[test]
    fn clear_for_ctrl_c_preserves_image_draft_state() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        let path = PathBuf::from("example.png");
        composer.attach_image(path.clone());
        let placeholder = local_image_label_text(/*label_number*/ 1);

        composer.clear_for_ctrl_c();
        assert!(composer.is_empty());

        let history_entry = composer
            .history
            .navigate_up(&composer.app_event_tx)
            .expect("expected history entry");
        let text_elements = vec![TextElement::new(
            (0..placeholder.len()).into(),
            Some(placeholder.clone()),
        )];
        assert_eq!(
            history_entry,
            HistoryEntry::with_pending(
                placeholder.clone(),
                text_elements,
                vec![path.clone()],
                Vec::new()
            )
        );

        composer.apply_history_entry(history_entry);
        assert_eq!(composer.textarea.text(), placeholder);
        assert_eq!(composer.local_image_paths(), vec![path]);
        assert_eq!(composer.textarea.element_payloads(), vec![placeholder]);
    }

    #[test]
    fn clear_for_ctrl_c_preserves_remote_offset_image_labels() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );
        let remote_image_url = "https://example.com/one.png".to_string();
        composer.set_remote_image_urls(vec![remote_image_url.clone()]);
        let text = "[Image #2] draft".to_string();
        let text_elements = vec![TextElement::new(
            (0.."[Image #2]".len()).into(),
            Some("[Image #2]".to_string()),
        )];
        let local_image_path = PathBuf::from("/tmp/local-draft.png");
        composer.set_text_content(text, text_elements, vec![local_image_path.clone()]);
        let expected_text = composer.current_text();
        let expected_elements = composer.text_elements();
        assert_eq!(expected_text, "[Image #2] draft");
        assert_eq!(
            expected_elements[0].placeholder(&expected_text),
            Some("[Image #2]")
        );

        assert_eq!(composer.clear_for_ctrl_c(), Some(expected_text.clone()));

        assert_eq!(
            composer.history.navigate_up(&composer.app_event_tx),
            Some(HistoryEntry::with_pending_and_remote(
                expected_text,
                expected_elements,
                vec![local_image_path],
                Vec::new(),
                vec![remote_image_url],
            ))
        );
    }

    #[test]
    fn apply_history_entry_preserves_local_placeholders_after_remote_prefix() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let sender = AppEventSender::new(tx);
        let mut composer = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ false,
            "Ask Codex to do anything".to_string(),
            /*disable_paste_burst*/ false,
        );

        let remote_image_url = "https://example.com/one.png".to_string();
        let local_image_path = PathBuf::from("/tmp/local-draft.png");
        composer.apply_history_entry(HistoryEntry::with_pending_and_remote(
            "[Image #2] draft".to_string(),
            vec![TextElement::new(
                (0.."[Image #2]".len()).into(),
                Some("[Image #2]".to_string()),
            )],
            vec![local_image_path.clone()],
            Vec::new(),
            vec![remote_image_url.clone()],
        ));

        let restored_text = composer.current_text();
        assert_eq!(restored_text, "[Image #2] draft");
        let restored_elements = composer.text_elements();
        assert_eq!(restored_elements.len(), 1);
        assert_eq!(
            restored_elements[0].placeholder(&restored_text),
            Some("[Image #2]")
        );
        assert_eq!(composer.local_image_paths(), vec![local_image_path]);
        assert_eq!(composer.remote_image_urls(), vec![remote_image_url]);
    }

    /// Behavior: `?` toggles the shortcut overlay only when the composer is otherwise empty. After
    /// any typing has occurred, `?` should be inserted as a literal character.
