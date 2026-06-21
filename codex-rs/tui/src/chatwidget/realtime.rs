use super::*;
use codex_app_server_protocol::ThreadRealtimeAudioChunk;
use codex_app_server_protocol::ThreadRealtimeClosedNotification;
use codex_app_server_protocol::ThreadRealtimeErrorNotification;
use codex_app_server_protocol::ThreadRealtimeItemAddedNotification;
use codex_app_server_protocol::ThreadRealtimeOutputAudioDeltaNotification;
use codex_app_server_protocol::ThreadRealtimeStartTransport;
use codex_app_server_protocol::ThreadRealtimeStartedNotification;
use codex_config::config_toml::RealtimeTransport;
use codex_realtime_webrtc::RealtimeWebrtcEvent;
use codex_realtime_webrtc::RealtimeWebrtcSession;
use codex_realtime_webrtc::RealtimeWebrtcSessionHandle;
#[cfg(not(target_os = "linux"))]
use std::sync::atomic::AtomicU16;
#[cfg(not(target_os = "linux"))]
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RealtimeConversationPhase {
    #[default]
    Inactive,
    Starting,
    Active,
    Stopping,
}

#[derive(Default)]
pub(super) struct RealtimeConversationUiState {
    pub(super) phase: RealtimeConversationPhase,
    requested_close: bool,
    realtime_session_id: Option<String>,
    transport: RealtimeConversationUiTransport,
    #[cfg(not(target_os = "linux"))]
    pub(super) meter_placeholder_id: Option<String>,
    #[cfg(not(target_os = "linux"))]
    capture_stop_flag: Option<Arc<AtomicBool>>,
    #[cfg(not(target_os = "linux"))]
    capture: Option<crate::voice::VoiceCapture>,
    #[cfg(not(target_os = "linux"))]
    audio_player: Option<crate::voice::RealtimeAudioPlayer>,
}

#[derive(Debug, Default)]
enum RealtimeConversationUiTransport {
    #[default]
    Websocket,
    Webrtc {
        handle: Option<RealtimeWebrtcSessionHandle>,
    },
}

impl RealtimeConversationUiState {
    pub(super) fn is_live(&self) -> bool {
        matches!(
            self.phase,
            RealtimeConversationPhase::Starting
                | RealtimeConversationPhase::Active
                | RealtimeConversationPhase::Stopping
        )
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn is_active(&self) -> bool {
        matches!(self.phase, RealtimeConversationPhase::Active)
    }
}

impl ChatWidget {
    fn realtime_footer_hint_items() -> Vec<(String, String)> {
        vec![("/realtime".to_string(), "stop live voice".to_string())]
    }

    pub(super) fn stop_realtime_conversation_from_ui(&mut self) {
        self.request_realtime_conversation_close(/*info_message*/ None);
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn stop_realtime_conversation_for_deleted_meter(&mut self, id: &str) -> bool {
        if self.realtime_conversation.is_live()
            && self.realtime_conversation.meter_placeholder_id.as_deref() == Some(id)
        {
            self.realtime_conversation.meter_placeholder_id = None;
            self.stop_realtime_conversation_from_ui();
            return true;
        }

        false
    }

    pub(super) fn start_realtime_conversation(&mut self) {
        self.realtime_conversation.phase = RealtimeConversationPhase::Starting;
        self.realtime_conversation.requested_close = false;
        self.realtime_conversation.realtime_session_id = None;
        self.set_footer_hint_override(Some(Self::realtime_footer_hint_items()));
        match self.config.realtime.transport {
            RealtimeTransport::Websocket => {
                self.realtime_conversation.transport = RealtimeConversationUiTransport::Websocket;
                self.submit_realtime_conversation_start(/*transport*/ None);
            }
            RealtimeTransport::WebRtc => {
                self.realtime_conversation.transport =
                    RealtimeConversationUiTransport::Webrtc { handle: None };
                start_realtime_webrtc_offer_task(self.app_event_tx.clone());
            }
        }
        self.request_redraw();
    }

    fn submit_realtime_conversation_start(
        &mut self,
        transport: Option<ThreadRealtimeStartTransport>,
    ) {
        self.submit_op(AppCommand::realtime_conversation_start(
            transport,
            self.config
                .realtime
                .voice
                .and_then(|voice| serde_json::to_value(voice).ok()),
        ));
    }

    pub(super) fn request_realtime_conversation_close(&mut self, info_message: Option<String>) {
        if !self.realtime_conversation.is_live() {
            if let Some(message) = info_message {
                self.add_info_message(message, /*hint*/ None);
            }
            return;
        }

        self.realtime_conversation.requested_close = true;
        self.realtime_conversation.phase = RealtimeConversationPhase::Stopping;
        self.submit_op(AppCommand::realtime_conversation_close());
        self.stop_realtime_local_audio();
        self.close_realtime_webrtc_transport();
        self.set_footer_hint_override(/*items*/ None);

        if let Some(message) = info_message {
            self.add_info_message(message, /*hint*/ None);
        } else {
            self.request_redraw();
        }
    }

    pub(super) fn reset_realtime_conversation_state(&mut self) {
        self.stop_realtime_local_audio();
        self.close_realtime_webrtc_transport();
        self.set_footer_hint_override(/*items*/ None);
        self.realtime_conversation.phase = RealtimeConversationPhase::Inactive;
        self.realtime_conversation.requested_close = false;
        self.realtime_conversation.realtime_session_id = None;
        self.realtime_conversation.transport = RealtimeConversationUiTransport::Websocket;
    }

    fn fail_realtime_conversation(&mut self, message: String) {
        self.add_error_message(message);
        if self.realtime_conversation.is_live() {
            self.request_realtime_conversation_close(/*info_message*/ None);
        } else {
            self.reset_realtime_conversation_state();
            self.request_redraw();
        }
    }

    pub(super) fn on_realtime_conversation_started(
        &mut self,
        notification: ThreadRealtimeStartedNotification,
    ) {
        if !self.realtime_conversation_enabled() {
            self.request_realtime_conversation_close(/*info_message*/ None);
            return;
        }
        self.realtime_conversation.realtime_session_id = notification.realtime_session_id;
        self.set_footer_hint_override(Some(Self::realtime_footer_hint_items()));
        if self.realtime_conversation_uses_webrtc() {
            self.realtime_conversation.phase = RealtimeConversationPhase::Starting;
        } else {
            self.realtime_conversation.phase = RealtimeConversationPhase::Active;
            self.start_realtime_local_audio();
        }
        self.request_redraw();
    }

    pub(super) fn on_realtime_output_audio_delta(
        &mut self,
        notification: ThreadRealtimeOutputAudioDeltaNotification,
    ) {
        if self.realtime_conversation_uses_webrtc() {
            return;
        }
        self.enqueue_realtime_audio_out(&notification.audio);
    }

    pub(super) fn on_realtime_item_added(
        &mut self,
        notification: ThreadRealtimeItemAddedNotification,
    ) {
        if self.realtime_conversation_uses_webrtc() {
            return;
        }
        if matches!(
            notification
                .item
                .get("type")
                .and_then(|value| value.as_str()),
            Some("input_audio_buffer.speech_started" | "response.cancelled")
        ) {
            self.interrupt_realtime_audio_playback();
        }
    }

    pub(super) fn on_realtime_error(&mut self, notification: ThreadRealtimeErrorNotification) {
        self.fail_realtime_conversation(format!("Realtime voice error: {}", notification.message));
    }

    pub(super) fn on_realtime_conversation_closed(
        &mut self,
        notification: ThreadRealtimeClosedNotification,
    ) {
        if self.realtime_conversation_uses_webrtc()
            && self.realtime_conversation.is_live()
            && notification.reason.as_deref() == Some("transport_closed")
        {
            return;
        }

        let requested = self.realtime_conversation.requested_close;
        let reason = notification.reason;
        self.reset_realtime_conversation_state();
        if !requested
            && let Some(reason) = reason
            && reason != "error"
        {
            self.add_info_message(
                format!("Realtime voice mode closed: {reason}"),
                /*hint*/ None,
            );
        }
        self.request_redraw();
    }

    pub(super) fn on_realtime_conversation_sdp(&mut self, sdp: String) {
        let RealtimeConversationUiTransport::Webrtc {
            handle: Some(handle),
        } = &self.realtime_conversation.transport
        else {
            return;
        };

        if let Err(err) = handle.apply_answer_sdp(sdp) {
            self.fail_realtime_conversation(format!("Failed to connect realtime WebRTC: {err}"));
        }
    }

    pub(crate) fn on_realtime_webrtc_offer_created(
        &mut self,
        result: Result<crate::app_event::RealtimeWebrtcOffer, String>,
    ) {
        if self.realtime_conversation.phase != RealtimeConversationPhase::Starting
            || !matches!(
                self.realtime_conversation.transport,
                RealtimeConversationUiTransport::Webrtc { handle: None }
            )
        {
            return;
        }

        let offer = match result {
            Ok(offer) => offer,
            Err(err) => {
                self.fail_realtime_conversation(format!("Failed to start realtime WebRTC: {err}"));
                return;
            }
        };

        self.realtime_conversation.transport = RealtimeConversationUiTransport::Webrtc {
            handle: Some(offer.handle),
        };
        self.submit_realtime_conversation_start(Some(ThreadRealtimeStartTransport::Webrtc {
            sdp: offer.offer_sdp,
        }));
        self.request_redraw();
    }

    pub(crate) fn on_realtime_webrtc_event(&mut self, event: RealtimeWebrtcEvent) {
        if !self.realtime_conversation_uses_webrtc() {
            return;
        }

        match event {
            RealtimeWebrtcEvent::Connected => {
                if self.realtime_conversation.phase != RealtimeConversationPhase::Starting {
                    return;
                }
                self.realtime_conversation.phase = RealtimeConversationPhase::Active;
                self.set_footer_hint_override(Some(Self::realtime_footer_hint_items()));
                self.request_redraw();
            }
            RealtimeWebrtcEvent::Closed => {
                self.reset_realtime_conversation_state();
                self.request_redraw();
            }
            RealtimeWebrtcEvent::Failed(message) => {
                self.fail_realtime_conversation(format!("Realtime WebRTC error: {message}"));
            }
            RealtimeWebrtcEvent::LocalAudioLevel(_) => {}
        }
    }

    pub(crate) fn on_realtime_webrtc_local_audio_level(&mut self, peak: u16) {
        if !self.realtime_conversation_uses_webrtc() || peak == 0 {
            return;
        }

        #[cfg(target_os = "linux")]
        {
            let _ = peak;
        }

        #[cfg(not(target_os = "linux"))]
        {
            let RealtimeConversationUiTransport::Webrtc {
                handle: Some(handle),
            } = &self.realtime_conversation.transport
            else {
                return;
            };
            let peak = handle.local_audio_peak();
            if self.realtime_conversation.meter_placeholder_id.is_none() {
                self.start_realtime_webrtc_meter(peak);
            }
        }
    }

    fn realtime_conversation_uses_webrtc(&self) -> bool {
        matches!(
            self.realtime_conversation.transport,
            RealtimeConversationUiTransport::Webrtc { .. }
        )
    }

    fn close_realtime_webrtc_transport(&mut self) {
        if let RealtimeConversationUiTransport::Webrtc { handle } =
            &mut self.realtime_conversation.transport
            && let Some(handle) = handle.take()
        {
            handle.close();
        }
    }

    fn enqueue_realtime_audio_out(&mut self, frame: &ThreadRealtimeAudioChunk) {
        #[cfg(not(target_os = "linux"))]
        {
            if self.realtime_conversation.audio_player.is_none() {
                self.realtime_conversation.audio_player =
                    crate::voice::RealtimeAudioPlayer::start(&self.config).ok();
            }
            if let Some(player) = &self.realtime_conversation.audio_player
                && let Err(err) = player.enqueue_frame(frame)
            {
                warn!("failed to play realtime audio: {err}");
            }
        }
        #[cfg(target_os = "linux")]
        {
            let _ = frame;
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn interrupt_realtime_audio_playback(&mut self) {
        if let Some(player) = &self.realtime_conversation.audio_player {
            player.clear();
        }
    }

    #[cfg(target_os = "linux")]
    fn interrupt_realtime_audio_playback(&mut self) {}

    #[cfg(not(target_os = "linux"))]
    fn start_realtime_local_audio(&mut self) {
        if self.realtime_conversation.capture_stop_flag.is_some() {
            return;
        }

        let capture = match crate::voice::VoiceCapture::start_realtime(
            &self.config,
            self.app_event_tx.clone(),
        ) {
            Ok(capture) => capture,
            Err(err) => {
                self.fail_realtime_conversation(format!(
                    "Failed to start microphone capture: {err}"
                ));
                return;
            }
        };

        let stop_flag = capture.stopped_flag();
        let peak = capture.last_peak_arc();
        self.start_realtime_meter(stop_flag.clone(), peak);
        self.realtime_conversation.capture_stop_flag = Some(stop_flag);
        self.realtime_conversation.capture = Some(capture);
        if self.realtime_conversation.audio_player.is_none() {
            self.realtime_conversation.audio_player =
                crate::voice::RealtimeAudioPlayer::start(&self.config).ok();
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn start_realtime_webrtc_meter(&mut self, peak: Arc<AtomicU16>) {
        if self.realtime_conversation.capture_stop_flag.is_some() {
            return;
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        self.start_realtime_meter(stop_flag.clone(), peak);
        self.realtime_conversation.capture_stop_flag = Some(stop_flag);
    }

    #[cfg(not(target_os = "linux"))]
    fn start_realtime_meter(&mut self, stop_flag: Arc<AtomicBool>, peak: Arc<AtomicU16>) {
        let placeholder_id = self.bottom_pane.insert_recording_meter_placeholder("⠤⠤⠤⠤");
        self.realtime_conversation.meter_placeholder_id = Some(placeholder_id.clone());
        self.request_redraw();

        start_realtime_meter_task(placeholder_id, self.app_event_tx.clone(), stop_flag, peak);
    }

    #[cfg(target_os = "linux")]
    fn start_realtime_local_audio(&mut self) {}

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn restart_realtime_audio_device(&mut self, kind: RealtimeAudioDeviceKind) {
        if !self.realtime_conversation.is_active() {
            return;
        }

        match kind {
            RealtimeAudioDeviceKind::Microphone => {
                self.stop_realtime_microphone();
                self.start_realtime_local_audio();
            }
            RealtimeAudioDeviceKind::Speaker => {
                self.stop_realtime_speaker();
                match crate::voice::RealtimeAudioPlayer::start(&self.config) {
                    Ok(player) => {
                        self.realtime_conversation.audio_player = Some(player);
                    }
                    Err(err) => {
                        self.fail_realtime_conversation(format!(
                            "Failed to start speaker output: {err}"
                        ));
                    }
                }
            }
        }
        self.request_redraw();
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn restart_realtime_audio_device(&mut self, kind: RealtimeAudioDeviceKind) {
        let _ = kind;
    }

    #[cfg(not(target_os = "linux"))]
    fn stop_realtime_local_audio(&mut self) {
        self.stop_realtime_microphone();
        self.stop_realtime_speaker();
    }

    #[cfg(target_os = "linux")]
    fn stop_realtime_local_audio(&mut self) {}

    #[cfg(not(target_os = "linux"))]
    fn stop_realtime_microphone(&mut self) {
        if let Some(flag) = self.realtime_conversation.capture_stop_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }
        if let Some(capture) = self.realtime_conversation.capture.take() {
            capture.stop();
        }
        if let Some(id) = self.realtime_conversation.meter_placeholder_id.take() {
            self.remove_recording_meter_placeholder(&id);
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn stop_realtime_speaker(&mut self) {
        if let Some(player) = self.realtime_conversation.audio_player.take() {
            player.clear();
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn update_recording_meter_in_place(&mut self, id: &str, text: &str) -> bool {
        let updated = self.bottom_pane.update_recording_meter_in_place(id, text);
        if updated {
            self.request_redraw();
        }
        updated
    }
}

fn start_realtime_webrtc_offer_task(app_event_tx: AppEventSender) {
    std::thread::spawn(move || {
        let result = match RealtimeWebrtcSession::start() {
            Ok(started) => {
                let event_tx = app_event_tx.clone();
                let local_audio_peak = started.handle.local_audio_peak();
                std::thread::spawn(move || {
                    for event in started.events {
                        if let RealtimeWebrtcEvent::LocalAudioLevel(peak) = event {
                            local_audio_peak.store(peak, Ordering::Relaxed);
                            event_tx.send(AppEvent::RealtimeWebrtcLocalAudioLevel(peak));
                        } else {
                            event_tx.send(AppEvent::RealtimeWebrtcEvent(event));
                        }
                    }
                });
                Ok(crate::app_event::RealtimeWebrtcOffer {
                    offer_sdp: started.offer_sdp,
                    handle: started.handle,
                })
            }
            Err(err) => Err(err.to_string()),
        };
        app_event_tx.send(AppEvent::RealtimeWebrtcOfferCreated { result });
    });
}

#[cfg(not(target_os = "linux"))]
fn start_realtime_meter_task(
    meter_placeholder_id: String,
    app_event_tx: AppEventSender,
    stop_flag: Arc<AtomicBool>,
    peak: Arc<AtomicU16>,
) {
    std::thread::spawn(move || {
        let mut meter = crate::voice::RecordingMeterState::new();

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let meter_text = meter.next_text(peak.load(Ordering::Relaxed));
            app_event_tx.send(AppEvent::UpdateRecordingMeter {
                id: meter_placeholder_id.clone(),
                text: meter_text,
            });

            std::thread::sleep(Duration::from_millis(60));
        }
    });
}

// fork-local: realtime audio-device settings + popup surfaces, consolidated here from the fork's
// chatwidget/settings.rs and chatwidget/settings_popups.rs (the upstream merge severed the realtime
// feature out of those modules). Kept verbatim from fork-parent b342f16013 so the call-sites in
// app/event_dispatch.rs and chatwidget/slash_dispatch.rs resolve. Inherent impls merge across
// modules, so behavior is identical to the original split.
impl ChatWidget {
    pub(crate) fn set_realtime_audio_device(
        &mut self,
        kind: RealtimeAudioDeviceKind,
        name: Option<String>,
    ) {
        match kind {
            RealtimeAudioDeviceKind::Microphone => self.config.realtime_audio.microphone = name,
            RealtimeAudioDeviceKind::Speaker => self.config.realtime_audio.speaker = name,
        }
    }

    pub(crate) fn realtime_conversation_is_live(&self) -> bool {
        self.realtime_conversation.is_live()
    }

    pub(super) fn current_realtime_audio_device_name(
        &self,
        kind: RealtimeAudioDeviceKind,
    ) -> Option<String> {
        match kind {
            RealtimeAudioDeviceKind::Microphone => self.config.realtime_audio.microphone.clone(),
            RealtimeAudioDeviceKind::Speaker => self.config.realtime_audio.speaker.clone(),
        }
    }

    pub(super) fn current_realtime_audio_selection_label(
        &self,
        kind: RealtimeAudioDeviceKind,
    ) -> String {
        self.current_realtime_audio_device_name(kind)
            .unwrap_or_else(|| "System default".to_string())
    }

    pub(crate) fn open_realtime_audio_popup(&mut self) {
        let items = [
            RealtimeAudioDeviceKind::Microphone,
            RealtimeAudioDeviceKind::Speaker,
        ]
        .into_iter()
        .map(|kind| {
            let description = Some(format!(
                "Current: {}",
                self.current_realtime_audio_selection_label(kind)
            ));
            let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenRealtimeAudioDeviceSelection { kind });
            })];
            SelectionItem {
                name: kind.title().to_string(),
                description,
                actions,
                dismiss_on_select: true,
                ..Default::default()
            }
        })
        .collect();

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Settings".to_string()),
            subtitle: Some("Configure settings for Codex.".to_string()),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn open_realtime_audio_device_selection(&mut self, kind: RealtimeAudioDeviceKind) {
        match list_realtime_audio_device_names(kind) {
            Ok(device_names) => {
                self.open_realtime_audio_device_selection_with_names(kind, device_names);
            }
            Err(err) => {
                self.add_error_message(format!(
                    "Failed to load realtime {} devices: {err}",
                    kind.noun()
                ));
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn open_realtime_audio_device_selection(&mut self, kind: RealtimeAudioDeviceKind) {
        let _ = kind;
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn open_realtime_audio_device_selection_with_names(
        &mut self,
        kind: RealtimeAudioDeviceKind,
        device_names: Vec<String>,
    ) {
        let current_selection = self.current_realtime_audio_device_name(kind);
        let current_available = current_selection
            .as_deref()
            .is_some_and(|name| device_names.iter().any(|device_name| device_name == name));
        let mut items = vec![SelectionItem {
            name: "System default".to_string(),
            description: Some("Use your operating system default device.".to_string()),
            is_current: current_selection.is_none(),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::PersistRealtimeAudioDeviceSelection { kind, name: None });
            })],
            dismiss_on_select: true,
            ..Default::default()
        }];

        if let Some(selection) = current_selection.as_deref()
            && !current_available
        {
            items.push(SelectionItem {
                name: format!("Unavailable: {selection}"),
                description: Some("Configured device is not currently available.".to_string()),
                is_current: true,
                is_disabled: true,
                disabled_reason: Some("Reconnect the device or choose another one.".to_string()),
                ..Default::default()
            });
        }

        items.extend(device_names.into_iter().map(|device_name| {
            let persisted_name = device_name.clone();
            let actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
                tx.send(AppEvent::PersistRealtimeAudioDeviceSelection {
                    kind,
                    name: Some(persisted_name.clone()),
                });
            })];
            SelectionItem {
                is_current: current_selection.as_deref() == Some(device_name.as_str()),
                name: device_name,
                actions,
                dismiss_on_select: true,
                ..Default::default()
            }
        }));

        let mut header = ColumnRenderable::new();
        header.push(Line::from(format!("Select {}", kind.title()).bold()));
        header.push(Line::from(
            "Saved devices apply to realtime voice only.".dim(),
        ));

        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    pub(crate) fn open_realtime_audio_restart_prompt(&mut self, kind: RealtimeAudioDeviceKind) {
        let restart_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::RestartRealtimeAudioDevice { kind });
        })];
        let items = vec![
            SelectionItem {
                name: "Restart now".to_string(),
                description: Some(format!("Restart local {} audio now.", kind.noun())),
                actions: restart_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Apply later".to_string(),
                description: Some(format!(
                    "Keep the current {} until local audio starts again.",
                    kind.noun()
                )),
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        let mut header = ColumnRenderable::new();
        header.push(Line::from(format!("Restart {} now?", kind.title()).bold()));
        header.push(Line::from(
            "Configuration is saved. Restart local audio to use it immediately.".dim(),
        ));

        self.bottom_pane.show_selection_view(SelectionViewParams {
            header: Box::new(header),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }
}
