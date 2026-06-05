//! The chat composer is the bottom-pane text input state machine.
//!
//! It is responsible for:
//!
//! - Editing the input buffer (a [`TextArea`]), including placeholder "elements" for attachments.
//! - Routing keys to the active popup (slash commands, file search, skill/apps mentions).
//! - Promoting typed slash commands into atomic elements when the command name is completed.
//! - Handling submit vs newline on Enter.
//! - Turning raw key streams into explicit paste operations on platforms where terminals
//!   don't provide reliable bracketed paste (notably Windows).
//!
//! # Key Event Routing
//!
//! Most key handling goes through [`ChatComposer::handle_key_event`], which dispatches to a
//! popup-specific handler if a popup is visible and otherwise to
//! [`ChatComposer::handle_key_event_without_popup`]. After every handled key, we call
//! [`ChatComposer::sync_popups`] so UI state follows the latest buffer/cursor.
//!
//! # History Navigation (↑/↓)
//!
//! The Up/Down history path is managed by [`ChatComposerHistory`]. It merges:
//!
//! - Persistent cross-session history (text-only; no element ranges or attachments).
//! - Local in-session history (full text + text elements + local/remote image attachments).
//!
//! When recalling a local entry, the composer rehydrates text elements and both attachment kinds
//! (local image paths + remote image URLs).
//! When recalling a persistent entry, only the text is restored.
//! Recalled entries move the cursor to end-of-line so repeated Up/Down presses keep shell-like
//! history traversal semantics instead of dropping to column 0.
//! `Ctrl+R` opens a reverse incremental search mode. The footer becomes the search input; once the
//! query is non-empty, the composer body previews the current match. `Enter` accepts the preview as
//! an editable draft and `Esc` restores the draft that was active when search started.
//!
//! Slash commands are staged for local history instead of being recorded immediately. Command
//! recall is a two-phase handoff: stage the submitted slash text here, then record it after
//! `ChatWidget` dispatches the command.
//!
//! # Submission and Prompt Expansion
//!
//! `Enter` submits immediately. `Tab` requests queuing while a task is running; if no task is
//! running, `Tab` submits just like Enter so input is never dropped.
//! `Tab` does not submit when entering a `!` shell command.
//!
//! On submit/queue paths, the composer:
//!
//! - Expands pending paste placeholders so element ranges align with the final text.
//! - Trims whitespace and rebases text elements accordingly.
//! - Prunes local attached images so only placeholders that survive expansion are sent.
//! - Preserves remote image URLs as separate attachments even when text is empty.
//!
//! When these paths clear the visible textarea after a successful submit or slash-command
//! dispatch, they intentionally preserve the textarea kill buffer. That lets users `Ctrl+K` part
//! of a draft, perform a composer action such as changing reasoning level, and then `Ctrl+Y` the
//! killed text back into the now-empty draft.
//!
//! The numeric auto-submit path used by the slash popup performs the same pending-paste expansion
//! and attachment pruning, and clears pending paste state on success.
//! Slash commands with arguments (like `/plan` and `/review`) reuse the same preparation path so
//! pasted content and text elements are preserved when extracting args.
//!
//! # Large Paste Placeholders
//!
//! Large pastes insert an element placeholder in the buffer and store the full text in
//! `pending_pastes`. The placeholder label is derived from the pasted character count:
//!
//! - First paste of a given size uses `[Pasted Content N chars]`.
//! - Additional pending pastes of the same size add a numeric suffix (`#2`, `#3`, ...), where the
//!   next suffix is computed from the placeholders that still exist in `pending_pastes`.
//! - When all placeholders for a size are cleared or deleted, the next paste of that size reuses
//!   the base label without a suffix.
//!
//! # Remote Image Rows (Up/Down/Delete)
//!
//! Remote image URLs are rendered as non-editable `[Image #N]` rows above the textarea (inside the
//! same composer block). These rows represent image attachments rehydrated from app-server/backtrack
//! history; TUI users can remove them, but cannot type into that row region.
//!
//! Keyboard behavior:
//!
//! - `Up` at textarea cursor `0` enters remote-row selection at the last remote image.
//! - `Up`/`Down` move selection between remote rows.
//! - `Down` on the last row clears selection and returns control to the textarea.
//! - `Delete`/`Backspace` remove the selected remote image row.
//!
//! Placeholder numbering is unified across remote and local images:
//!
//! - Remote rows occupy `[Image #1]..[Image #M]`.
//! - Local placeholders are offset after that range (`[Image #M+1]..`).
//! - Deleting a remote row relabels local placeholders to keep numbering contiguous.
//!
//! # Non-bracketed Paste Bursts
//!
//! On some terminals (especially on Windows), pastes arrive as a rapid sequence of
//! `KeyCode::Char` and `KeyCode::Enter` key events instead of a single paste event.
//!
//! To avoid misinterpreting these bursts as real typing (and to prevent transient UI effects like
//! shortcut overlays toggling on a pasted `?`), we feed "plain" character events into
//! [`PasteBurst`](super::paste_burst::PasteBurst), which buffers bursts and later flushes them
//! through [`ChatComposer::handle_paste`].
//!
//! The burst detector intentionally treats ASCII and non-ASCII differently:
//!
//! - ASCII: we briefly hold the first fast char (flicker suppression) until we know whether the
//!   stream is paste-like.
//! - non-ASCII: we do not hold the first char (IME input would feel dropped), but we still allow
//!   burst detection for actual paste streams.
//!
//! The burst detector can also be disabled (`disable_paste_burst`), which bypasses the state
//! machine and treats the key stream as normal typing. When toggling from enabled → disabled, the
//! composer flushes/clears any in-flight burst state so it cannot leak into subsequent input.
//!
//! For the detailed burst state machine, see `codex-rs/tui/src/bottom_pane/paste_burst.rs`.
//!
//! # PasteBurst Integration Points
//!
//! The burst detector is consulted in a few specific places:
//!
//! - [`ChatComposer::handle_input_basic`]: flushes any due burst first, then intercepts plain char
//!   input to either buffer it or insert normally.
//! - [`ChatComposer::handle_non_ascii_char`]: handles the non-ASCII/IME path without holding the
//!   first char, while still allowing paste detection via retro-capture.
//! - [`ChatComposer::flush_paste_burst_if_due`]/[`ChatComposer::handle_paste_burst_flush`]: called
//!   from UI ticks to turn a pending burst into either an explicit paste (`handle_paste`) or a
//!   normal typed character.
//!
//! # Input Disabled Mode
//!
//! The composer can be temporarily read-only (`input_enabled = false`). In that mode it ignores
//! edits and renders a placeholder prompt instead of the editable textarea. This is part of the
//! overall state machine, since it affects which transitions are even possible from a given UI
//! state.
//!
use crate::key_hint;
use crate::key_hint::KeyBinding;
use crate::key_hint::has_ctrl_or_alt;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::ui_consts::FOOTER_INDENT_COLS;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Margin;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidgetRef;
use ratatui::widgets::WidgetRef;

use super::chat_composer_history::ChatComposerHistory;
use super::chat_composer_history::HistoryEntry;
use super::chat_composer_history::HistoryEntryResponse;
use super::command_popup::CommandItem;
use super::command_popup::CommandPopup;
use super::command_popup::CommandPopupFlags;
use super::file_search_popup::FileSearchPopup;
use super::footer::CollaborationModeIndicator;
use super::footer::FooterKeyHints;
use super::footer::FooterMode;
use super::footer::FooterProps;
use super::footer::GoalStatusIndicator;
use super::footer::SummaryLeft;
use super::footer::can_show_left_with_context;
use super::footer::context_window_line;
use super::footer::esc_hint_mode;
use super::footer::footer_height;
use super::footer::footer_hint_items_width;
use super::footer::footer_line_width;
use super::footer::inset_footer_hint_area;
use super::footer::max_left_width_for_right;
use super::footer::passive_footer_status_line;
use super::footer::render_context_right;
use super::footer::render_footer_from_props;
use super::footer::render_footer_hint_items;
use super::footer::render_footer_line;
use super::footer::reset_mode_after_activity;
use super::footer::side_conversation_context_line;
use super::footer::single_line_footer_layout;
use super::footer::status_line_right_indicator_line;
use super::footer::toggle_shortcut_mode;
use super::footer::uses_passive_footer_status_layout;
use super::mentions_v2::MentionV2Popup;
use super::mentions_v2::MentionV2Selection;
use super::paste_burst::CharDecision;
use super::paste_burst::PasteBurst;
use super::skill_popup::MentionItem;
use super::skill_popup::SkillPopup;
use super::prompt_args::parse_slash_name;
use super::slash_commands::BuiltinCommandFlags;
use super::slash_commands::ServiceTierCommand;
use super::slash_commands::SlashCommandItem;
use super::slash_commands::find_slash_command;
use super::slash_commands::has_slash_command_prefix;
use crate::bottom_pane::paste_burst::FlushResult;
use crate::key_hint::KeyBindingListExt;
use crate::keymap::EditorKeymap;
use crate::keymap::RuntimeKeymap;
use crate::keymap::VimNormalKeymap;
use crate::keymap::primary_binding;
use crate::onboarding::mark_underlined_hyperlink;
use crate::render::Insets;
use crate::render::RectExt;
use crate::render::renderable::Renderable;
use crate::slash_command::SlashCommand;
use crate::style::user_message_style;
use codex_protocol::ThreadId;
use codex_protocol::models::local_image_label_text;
use codex_protocol::user_input::ByteRange;
use codex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;
use codex_protocol::user_input::TextElement;

mod history_search;
mod bash_mode;
mod footer;
mod layout;
mod recording;
mod render;
mod state_setup;
mod text_editing;
mod input;
mod mentions;
mod submission;
mod remote_images;
mod input_basic;
mod popups_sync;

use self::history_search::HistorySearchSession;
use crate::app_event::AppEvent;
use crate::app_event::ConnectorsSnapshot;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::LocalImageAttachment;
use crate::bottom_pane::MentionBinding;
use crate::bottom_pane::textarea::TextArea;
use crate::bottom_pane::textarea::TextAreaState;
use crate::clipboard_paste::normalize_pasted_path;
use crate::clipboard_paste::pasted_image_format;
use crate::history_cell;
use crate::skills_helpers::skill_display_name;
use crate::tui::FrameRequester;
use crate::ui_consts::LIVE_PREFIX_COLS;
use codex_app_server_protocol::AppInfo;
#[cfg(test)]
use codex_core_skills::model::SkillInterface;
use codex_core_skills::model::SkillMetadata;
use codex_file_search::FileMatch;
use codex_input_queue::QueuedInputAction;
#[cfg(test)]
use codex_plugin::AppConnectorId;
use codex_plugin::PluginCapabilitySummary;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::ops::Range;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use ratatui::style::Color;

/// If the pasted content exceeds this number of characters, replace it with a
/// placeholder in the UI.
const LARGE_PASTE_CHAR_THRESHOLD: usize = 1000;

fn user_input_too_large_message(actual_chars: usize) -> String {
    format!(
        "Message exceeds the maximum length of {MAX_USER_INPUT_TEXT_CHARS} characters ({actual_chars} provided)."
    )
}

/// Result returned when the user interacts with the text area.
#[derive(Debug, PartialEq)]
pub enum InputResult {
    Submitted {
        text: String,
        text_elements: Vec<TextElement>,
    },
    Queued {
        text: String,
        text_elements: Vec<TextElement>,
        action: QueuedInputAction,
    },
    /// A bare slash command parsed by the composer.
    ///
    /// Callers that dispatch this variant are also responsible for resolving any pending local
    /// command-history entry that the composer staged before clearing the visible input.
    Command(SlashCommand),
    /// A bare model service-tier command parsed by the composer.
    ServiceTierCommand(ServiceTierCommand),
    /// An inline slash command and its trimmed argument text.
    ///
    /// The `TextElement` ranges are rebased into the argument string, while any pending local
    /// command-history entry still represents the original command invocation that should be
    /// committed only if dispatch accepts it.
    CommandWithArgs(SlashCommand, String, Vec<TextElement>),
    None,
}

#[derive(Clone, Debug, PartialEq)]
struct AttachedImage {
    placeholder: String,
    path: PathBuf,
}

/// Feature flags for reusing the chat composer in other bottom-pane surfaces.
///
/// The default keeps today's behavior intact. Other call sites can opt out of
/// specific behaviors by constructing a config with those flags set to `false`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChatComposerConfig {
    /// Whether command/file/skill popups are allowed to appear.
    pub(crate) popups_enabled: bool,
    /// Whether `/...` input is parsed and dispatched as slash commands.
    pub(crate) slash_commands_enabled: bool,
    /// Whether pasting a file path can attach local images.
    pub(crate) image_paste_enabled: bool,
}

impl Default for ChatComposerConfig {
    fn default() -> Self {
        Self {
            popups_enabled: true,
            slash_commands_enabled: true,
            image_paste_enabled: true,
        }
    }
}

impl ChatComposerConfig {
    /// A minimal preset for plain-text inputs embedded in other surfaces.
    ///
    /// This disables popups, slash commands, and image-path attachment behavior
    /// so the composer behaves like a simple notes field.
    pub(crate) const fn plain_text() -> Self {
        Self {
            popups_enabled: false,
            slash_commands_enabled: false,
            image_paste_enabled: false,
        }
    }
}

pub(crate) struct ChatComposer {
    textarea: TextArea,
    textarea_state: RefCell<TextAreaState>,
    is_bash_mode: bool,
    active_popup: ActivePopup,
    app_event_tx: AppEventSender,
    history: ChatComposerHistory,
    quit_shortcut_expires_at: Option<Instant>,
    quit_shortcut_key: KeyBinding,
    esc_backtrack_hint: bool,
    use_shift_enter_hint: bool,
    dismissed_file_popup_token: Option<String>,
    current_file_query: Option<String>,
    pending_pastes: Vec<(String, String)>,
    has_focus: bool,
    frame_requester: Option<FrameRequester>,
    /// Invariant: attached images are labeled in vec order as
    /// `[Image #M+1]..[Image #N]`, where `M` is the number of remote images.
    attached_images: Vec<AttachedImage>,
    placeholder_text: String,
    is_task_running: bool,
    /// When false, the composer is temporarily read-only (e.g. during sandbox setup).
    input_enabled: bool,
    input_disabled_placeholder: Option<String>,
    /// Non-bracketed paste burst tracker (see `bottom_pane/paste_burst.rs`).
    paste_burst: PasteBurst,
    // When true, disables paste-burst logic and inserts characters immediately.
    disable_paste_burst: bool,
    footer_mode: FooterMode,
    footer_hint_override: Option<Vec<(String, String)>>,
    /// Whether the ambient footer row is currently replaced by the Plan-mode nudge.
    ///
    /// Eligibility is decided by `ChatWidget`; the composer only owns presentation so enabling
    /// the nudge never changes layout height or reimplements mode-selection policy here.
    plan_mode_nudge_visible: bool,
    remote_image_urls: Vec<String>,
    /// Tracks keyboard selection for the remote-image rows so Up/Down + Delete/Backspace
    /// can highlight and remove remote attachments from the composer UI.
    selected_remote_image_index: Option<usize>,
    queue_submissions: bool,
    /// Slash-command draft staged for local recall after application-level dispatch.
    ///
    /// This slot is intentionally separate from `ChatComposerHistory` so inline slash commands can
    /// prepare their argument text without also double-recording the full command invocation.
    pending_slash_command_history: Option<HistoryEntry>,
    footer_flash: Option<FooterFlash>,
    context_window_percent: Option<i64>,
    // Monotonically increasing identifier for textarea elements we insert.
    #[cfg(not(target_os = "linux"))]
    next_element_id: u64,
    context_window_used_tokens: Option<i64>,
    skills: Option<Vec<SkillMetadata>>,
    plugins: Option<Vec<PluginCapabilitySummary>>,
    connectors_snapshot: Option<ConnectorsSnapshot>,
    dismissed_mention_popup_token: Option<String>,
    mention_bindings: HashMap<u64, ComposerMentionBinding>,
    recent_submission_mention_bindings: Vec<MentionBinding>,
    collaboration_modes_enabled: bool,
    config: ChatComposerConfig,
    collaboration_mode_indicator: Option<CollaborationModeIndicator>,
    goal_status_indicator: Option<GoalStatusIndicator>,
    ide_context_active: bool,
    connectors_enabled: bool,
    plugins_command_enabled: bool,
    service_tier_commands_enabled: bool,
    service_tier_commands: Vec<ServiceTierCommand>,
    mentions_v2_enabled: bool,
    goal_command_enabled: bool,
    personality_command_enabled: bool,
    realtime_conversation_enabled: bool,
    audio_device_selection_enabled: bool,
    windows_degraded_sandbox_active: bool,
    side_conversation_active: bool,
    status_line_value: Option<Line<'static>>,
    status_line_hyperlink_url: Option<String>,
    status_line_enabled: bool,
    session_limit_status_line: Option<Line<'static>>,
    side_conversation_context_label: Option<String>,
    // Agent label injected into the footer's contextual row when multi-agent mode is active.
    active_agent_label: Option<String>,
    history_search: Option<HistorySearchSession>,
    submit_keys: Vec<KeyBinding>,
    queue_keys: Vec<KeyBinding>,
    toggle_shortcuts_keys: Vec<KeyBinding>,
    history_search_previous_keys: Vec<KeyBinding>,
    history_search_next_keys: Vec<KeyBinding>,
    editor_keymap: EditorKeymap,
    vim_normal_keymap: VimNormalKeymap,
    footer_external_editor_key: Option<KeyBinding>,
    footer_show_transcript_key: Option<KeyBinding>,
    footer_insert_newline_key: Option<KeyBinding>,
    footer_queue_key: Option<KeyBinding>,
    footer_toggle_shortcuts_key: Option<KeyBinding>,
    footer_history_search_key: Option<KeyBinding>,
    footer_reasoning_down_key: Option<KeyBinding>,
    footer_reasoning_up_key: Option<KeyBinding>,
}

#[derive(Clone, Debug)]
struct FooterFlash {
    line: Line<'static>,
    expires_at: Instant,
}

#[derive(Clone, Debug)]
struct ComposerDraft {
    text: String,
    text_elements: Vec<TextElement>,
    local_image_paths: Vec<PathBuf>,
    remote_image_urls: Vec<String>,
    mention_bindings: Vec<MentionBinding>,
    pending_pastes: Vec<(String, String)>,
    cursor: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ComposerDraftSnapshot {
    pub(crate) text: String,
    pub(crate) text_elements: Vec<TextElement>,
    pub(crate) local_images: Vec<PathBuf>,
    pub(crate) remote_image_urls: Vec<String>,
    pub(crate) mention_bindings: Vec<MentionBinding>,
    pub(crate) pending_pastes: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
struct ComposerMentionBinding {
    /// Visible mention sigil (`$` or `@`). Composer-inserted mentions are always `$`.
    sigil: char,
    mention: String,
    path: String,
}

/// Popup state – at most one can be visible at any time.
enum ActivePopup {
    None,
    Command(CommandPopup),
    File(FileSearchPopup),
    Skill(SkillPopup),
    MentionV2(MentionV2Popup),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlashValidation {
    Immediate,
    Deferred,
}

const FOOTER_SPACING_HEIGHT: u16 = 0;

/// Builds the one-line nudge that replaces the ambient footer without adding layout height.
fn plan_mode_nudge_line() -> Line<'static> {
    Line::from(vec![
        "Create a plan?".magenta(),
        "  ".into(),
        key_hint::shift(KeyCode::Tab).into(),
        " use Plan mode".into(),
        "   ".into(),
        key_hint::plain(KeyCode::Esc).into(),
        " dismiss".into(),
    ])
}

fn combine_right_context_lines(
    primary: Option<Line<'static>>,
    session_limits: Option<Line<'static>>,
) -> Option<Line<'static>> {
    match (primary, session_limits) {
        (Some(mut primary), Some(session_limits)) => {
            primary.spans.push(" · ".into());
            primary.spans.extend(session_limits.spans);
            Some(primary)
        }
        (Some(primary), None) => Some(primary),
        (None, Some(session_limits)) => Some(session_limits),
        (None, None) => None,
    }
}

impl ChatComposer {
    #[cfg(test)]
    pub(crate) fn cursor(&self) -> usize {
        self.current_cursor()
    }

    fn set_has_focus(&mut self, has_focus: bool) {
        self.has_focus = has_focus;
    }

    #[allow(dead_code)]
    pub(crate) fn set_input_enabled(&mut self, enabled: bool, placeholder: Option<String>) {
        self.input_enabled = enabled;
        self.input_disabled_placeholder = if enabled { None } else { placeholder };

        // Avoid leaving interactive popups open while input is blocked.
        if !enabled && !matches!(self.active_popup, ActivePopup::None) {
            self.active_popup = ActivePopup::None;
        }
    }

    pub fn set_task_running(&mut self, running: bool) {
        self.is_task_running = running;
    }

    pub(crate) fn set_queue_submissions(&mut self, queue_submissions: bool) {
        self.queue_submissions = queue_submissions;
    }

    pub(crate) fn set_context_window(&mut self, percent: Option<i64>, used_tokens: Option<i64>) {
        if self.context_window_percent == percent && self.context_window_used_tokens == used_tokens
        {
            return;
        }
        self.context_window_percent = percent;
        self.context_window_used_tokens = used_tokens;
    }

    pub(crate) fn set_esc_backtrack_hint(&mut self, show: bool) {
        self.esc_backtrack_hint = show;
        if show {
            self.footer_mode = esc_hint_mode(self.footer_mode, self.is_task_running);
        } else {
            self.footer_mode = reset_mode_after_activity(self.footer_mode);
        }
    }

    pub(crate) fn set_status_line(&mut self, status_line: Option<Line<'static>>) -> bool {
        if self.status_line_value == status_line {
            return false;
        }
        self.status_line_value = status_line;
        true
    }

    pub(crate) fn set_status_line_hyperlink(&mut self, url: Option<String>) -> bool {
        if self.status_line_hyperlink_url == url {
            return false;
        }
        self.status_line_hyperlink_url = url;
        true
    }

    pub(crate) fn set_status_line_enabled(&mut self, enabled: bool) -> bool {
        if self.status_line_enabled == enabled {
            return false;
        }
        self.status_line_enabled = enabled;
        true
    }

    pub(crate) fn set_session_limit_status_line(&mut self, line: Option<Line<'static>>) -> bool {
        if self.session_limit_status_line == line {
            return false;
        }
        self.session_limit_status_line = line;
        true
    }

    pub(crate) fn set_side_conversation_context_label(&mut self, label: Option<String>) -> bool {
        if self.side_conversation_context_label == label {
            return false;
        }
        self.side_conversation_context_label = label;
        true
    }

    /// Replaces the contextual footer label for the currently viewed agent.
    ///
    /// Returning `false` means the value was unchanged, so callers can skip redraw work. This
    /// field is intentionally just cached presentation state; `ChatComposer` does not infer which
    /// thread is active on its own.
    pub(crate) fn set_active_agent_label(&mut self, active_agent_label: Option<String>) -> bool {
        if self.active_agent_label == active_agent_label {
            return false;
        }
        self.active_agent_label = active_agent_label;
        true
    }
}

fn footer_insert_newline_key(
    bindings: &[KeyBinding],
    enhanced_keys_supported: bool,
) -> Option<KeyBinding> {
    let shift_enter = key_hint::shift(KeyCode::Enter);
    if enhanced_keys_supported && bindings.contains(&shift_enter) {
        return Some(shift_enter);
    }

    let plain_enter = key_hint::plain(KeyCode::Enter);
    bindings
        .iter()
        .copied()
        .find(|binding| *binding != plain_enter)
        .or_else(|| bindings.first().copied())
}

fn skill_description(skill: &SkillMetadata) -> Option<String> {
    let description = skill
        .interface
        .as_ref()
        .and_then(|interface| interface.short_description.as_deref())
        .or(skill.short_description.as_deref())
        .unwrap_or(&skill.description);
    let trimmed = description.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn is_mention_name_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
}

fn ends_plaintext_at_mention(bytes: &[u8], index: usize) -> bool {
    bytes.get(index).is_none_or(|byte| {
        byte.is_ascii_whitespace()
            || *byte == b'.'
                && bytes.get(index + 1).is_none_or(|next| {
                    next.is_ascii_whitespace()
                        || !next.is_ascii_alphanumeric() && *next != b'_' && *next != b'-'
                })
            || !matches!(*byte, b'.' | b'/' | b'\\')
                && !byte.is_ascii_alphanumeric()
                && *byte != b'_'
                && *byte != b'-'
    })
}

fn starts_plaintext_at_mention(text: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }

    text.get(..index)
        .and_then(|prefix| prefix.chars().next_back())
        .is_some_and(|ch| ch.is_whitespace() || !is_mention_name_char_char(ch))
}

fn is_mention_name_char_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}

fn find_next_mention_token_range(text: &str, token: &str, from: usize) -> Option<Range<usize>> {
    if token.is_empty() || from >= text.len() {
        return None;
    }
    let bytes = text.as_bytes();
    let token_bytes = token.as_bytes();
    let sigil = *token_bytes.first()?;
    let mut index = from;

    while index < bytes.len() {
        if bytes[index] != sigil {
            index += 1;
            continue;
        }

        let end = index.saturating_add(token_bytes.len());
        if end > bytes.len() {
            return None;
        }
        if &bytes[index..end] != token_bytes {
            index += 1;
            continue;
        }

        // Fix for restored `@` mentions: rebinding must not attach to embedded substrings such
        // as email addresses, while preserving the existing `$` mention matching behavior.
        let starts_plaintext_mention = if sigil == b'@' {
            starts_plaintext_at_mention(text, index)
        } else {
            true
        };
        // Fix for restored `@` mentions: mirror history encoding's trailing boundary so path-like
        // text such as `@sample/pkg` is not rebound as the plain `@sample` mention.
        let ends_plaintext_mention = if sigil == b'@' {
            ends_plaintext_at_mention(bytes, end)
        } else {
            bytes
                .get(end)
                .is_none_or(|byte| !is_mention_name_char(*byte))
        };

        if starts_plaintext_mention && ends_plaintext_mention {
            return Some(index..end);
        }

        index = end;
    }

    None
}

#[cfg(test)]
mod tests_support;
#[cfg(test)]
mod tests_footer;
#[cfg(test)]
mod tests_vim_draft;
#[cfg(test)]
mod tests_mentions;
#[cfg(test)]
mod tests_paste_burst;
#[cfg(test)]
mod tests_snapshots;
#[cfg(test)]
mod tests_slash;
#[cfg(test)]
mod tests_placeholders;
#[cfg(test)]
mod tests_history_attach;
#[cfg(test)]
mod tests_misc;
#[cfg(test)]
mod tests_external_edit;
