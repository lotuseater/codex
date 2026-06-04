use crate::events::GuardianReviewAnalyticsResult;
use crate::events::GuardianReviewTrackContext;
use crate::facts::AnalyticsFact;
use crate::facts::AppInvocation;
use crate::facts::AppMentionedInput;
use crate::facts::AppUsedInput;
use crate::facts::CustomAnalyticsFact;
use crate::facts::HookRunFact;
use crate::facts::HookRunInput;
use crate::facts::PluginState;
use crate::facts::PluginStateChangedInput;
use crate::facts::SkillInvocation;
use crate::facts::SkillInvokedInput;
use crate::facts::SubAgentThreadStartedInput;
use crate::facts::TrackEventsContext;
use crate::facts::TurnCodexErrorFact;
use crate::facts::TurnResolvedConfigFact;
use crate::facts::TurnTokenUsageFact;
use crate::reducer_api::AnalyticsReducer;
use crate::reducer_api::TrackEvent;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::default_client::create_client;
use codex_plugin::PluginTelemetryMetadata;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

const ANALYTICS_EVENTS_QUEUE_SIZE: usize = 256;
const ANALYTICS_EVENTS_TIMEOUT: Duration = Duration::from_secs(10);
const ANALYTICS_EVENT_DEDUPE_MAX_KEYS: usize = 4096;

#[derive(Clone)]
pub(crate) struct AnalyticsEventsQueue {
    pub(crate) sender: mpsc::Sender<AnalyticsFact>,
    pub(crate) app_used_emitted_keys: Arc<Mutex<HashSet<(String, String)>>>,
    pub(crate) plugin_used_emitted_keys: Arc<Mutex<HashSet<(String, String)>>>,
}

#[derive(Clone)]
pub struct AnalyticsEventsClient {
    queue: Option<AnalyticsEventsQueue>,
}

impl AnalyticsEventsQueue {
    pub(crate) fn new(
        auth_manager: Arc<AuthManager>,
        base_url: String,
        mut reducer: Box<dyn AnalyticsReducer>,
    ) -> Self {
        let (sender, mut receiver) = mpsc::channel(ANALYTICS_EVENTS_QUEUE_SIZE);
        tokio::spawn(async move {
            while let Some(input) = receiver.recv().await {
                let mut events = Vec::new();
                reducer.ingest(input, &mut events).await;
                send_track_events(&auth_manager, &base_url, events).await;
            }
        });
        Self {
            sender,
            app_used_emitted_keys: Arc::new(Mutex::new(HashSet::new())),
            plugin_used_emitted_keys: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn try_send(&self, input: AnalyticsFact) {
        if self.sender.try_send(input).is_err() {
            //TODO: add a metric for this
            tracing::warn!("dropping analytics events: queue is full");
        }
    }

    pub(crate) fn should_enqueue_app_used(
        &self,
        tracking: &TrackEventsContext,
        app: &AppInvocation,
    ) -> bool {
        let Some(connector_id) = app.connector_id.as_ref() else {
            return true;
        };
        let mut emitted = self
            .app_used_emitted_keys
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if emitted.len() >= ANALYTICS_EVENT_DEDUPE_MAX_KEYS {
            emitted.clear();
        }
        emitted.insert((tracking.turn_id.clone(), connector_id.clone()))
    }

    pub(crate) fn should_enqueue_plugin_used(
        &self,
        tracking: &TrackEventsContext,
        plugin: &PluginTelemetryMetadata,
    ) -> bool {
        let mut emitted = self
            .plugin_used_emitted_keys
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if emitted.len() >= ANALYTICS_EVENT_DEDUPE_MAX_KEYS {
            emitted.clear();
        }
        emitted.insert((tracking.turn_id.clone(), plugin.plugin_id.as_key()))
    }
}

impl AnalyticsEventsClient {
    pub fn new(
        auth_manager: Arc<AuthManager>,
        base_url: String,
        analytics_enabled: Option<bool>,
        reducer: Box<dyn AnalyticsReducer>,
    ) -> Self {
        Self {
            queue: (analytics_enabled != Some(false)).then(|| {
                AnalyticsEventsQueue::new(Arc::clone(&auth_manager), base_url, reducer)
            }),
        }
    }

    pub fn disabled() -> Self {
        Self { queue: None }
    }

    pub fn track_skill_invocations(
        &self,
        tracking: TrackEventsContext,
        invocations: Vec<SkillInvocation>,
    ) {
        if invocations.is_empty() {
            return;
        }
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::SkillInvoked(
            SkillInvokedInput {
                tracking,
                invocations,
            },
        )));
    }

    pub fn track_subagent_thread_started(&self, input: SubAgentThreadStartedInput) {
        self.record_fact(AnalyticsFact::Custom(
            CustomAnalyticsFact::SubAgentThreadStarted(input),
        ));
    }

    pub fn track_guardian_review(
        &self,
        tracking: &GuardianReviewTrackContext,
        result: GuardianReviewAnalyticsResult,
        completed_at_ms: u64,
    ) {
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::GuardianReview(
            Box::new(tracking.event_params(result, completed_at_ms)),
        )));
    }

    pub fn track_app_mentioned(&self, tracking: TrackEventsContext, mentions: Vec<AppInvocation>) {
        if mentions.is_empty() {
            return;
        }
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::AppMentioned(
            AppMentionedInput { tracking, mentions },
        )));
    }

    pub fn track_app_used(&self, tracking: TrackEventsContext, app: AppInvocation) {
        let Some(queue) = self.queue.as_ref() else {
            return;
        };
        if !queue.should_enqueue_app_used(&tracking, &app) {
            return;
        }
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::AppUsed(
            AppUsedInput { tracking, app },
        )));
    }

    pub fn track_hook_run(&self, tracking: TrackEventsContext, hook: HookRunFact) {
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::HookRun(
            HookRunInput { tracking, hook },
        )));
    }

    pub fn track_plugin_used(&self, tracking: TrackEventsContext, plugin: PluginTelemetryMetadata) {
        let Some(queue) = self.queue.as_ref() else {
            return;
        };
        if !queue.should_enqueue_plugin_used(&tracking, &plugin) {
            return;
        }
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::PluginUsed(
            crate::facts::PluginUsedInput { tracking, plugin },
        )));
    }

    pub fn track_compaction(&self, event: crate::facts::CodexCompactionEvent) {
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::Compaction(
            Box::new(event),
        )));
    }

    pub fn track_turn_resolved_config(&self, fact: TurnResolvedConfigFact) {
        self.record_fact(AnalyticsFact::Custom(
            CustomAnalyticsFact::TurnResolvedConfig(Box::new(fact)),
        ));
    }

    pub fn track_turn_token_usage(&self, fact: TurnTokenUsageFact) {
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::TurnTokenUsage(
            Box::new(fact),
        )));
    }

    pub fn track_turn_codex_error(&self, fact: TurnCodexErrorFact) {
        self.record_fact(AnalyticsFact::Custom(CustomAnalyticsFact::TurnCodexError(
            Box::new(fact),
        )));
    }

    pub fn track_plugin_installed(&self, plugin: PluginTelemetryMetadata) {
        self.record_fact(AnalyticsFact::Custom(
            CustomAnalyticsFact::PluginStateChanged(PluginStateChangedInput {
                plugin,
                state: PluginState::Installed,
            }),
        ));
    }

    pub fn track_plugin_uninstalled(&self, plugin: PluginTelemetryMetadata) {
        self.record_fact(AnalyticsFact::Custom(
            CustomAnalyticsFact::PluginStateChanged(PluginStateChangedInput {
                plugin,
                state: PluginState::Uninstalled,
            }),
        ));
    }

    pub fn track_plugin_enabled(&self, plugin: PluginTelemetryMetadata) {
        self.record_fact(AnalyticsFact::Custom(
            CustomAnalyticsFact::PluginStateChanged(PluginStateChangedInput {
                plugin,
                state: PluginState::Enabled,
            }),
        ));
    }

    pub fn track_plugin_disabled(&self, plugin: PluginTelemetryMetadata) {
        self.record_fact(AnalyticsFact::Custom(
            CustomAnalyticsFact::PluginStateChanged(PluginStateChangedInput {
                plugin,
                state: PluginState::Disabled,
            }),
        ));
    }

    /// Enqueue a fact for the background reducer. Public so the
    /// `codex-analytics-appserver` crate's app-server tracking extension can
    /// submit its opaque [`AnalyticsFact::AppServer`] payloads.
    pub fn record_fact(&self, input: AnalyticsFact) {
        if let Some(queue) = self.queue.as_ref() {
            queue.try_send(input);
        }
    }
}

async fn send_track_events(auth_manager: &AuthManager, base_url: &str, events: Vec<TrackEvent>) {
    if events.is_empty() {
        return;
    }

    let Some(auth) = auth_manager.auth().await else {
        return;
    };
    if !auth.uses_codex_backend() {
        return;
    }

    let base_url = base_url.trim_end_matches('/');
    let url = format!("{base_url}/codex/analytics-events/events");
    for events in track_event_request_batches(events) {
        send_track_events_request(&auth, &url, events).await;
    }
}

fn track_event_request_batches(events: Vec<TrackEvent>) -> Vec<Vec<TrackEvent>> {
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();

    for event in events {
        if event.should_send_in_isolated_request() {
            if !current_batch.is_empty() {
                batches.push(current_batch);
                current_batch = Vec::new();
            }
            batches.push(vec![event]);
        } else {
            current_batch.push(event);
        }
    }

    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}

async fn send_track_events_request(auth: &CodexAuth, url: &str, events: Vec<TrackEvent>) {
    if events.is_empty() {
        return;
    }

    let event_bodies: Vec<serde_json::Value> =
        events.into_iter().map(TrackEvent::into_body).collect();
    let payload = serde_json::json!({ "events": event_bodies });

    let response = create_client()
        .post(url)
        .timeout(ANALYTICS_EVENTS_TIMEOUT)
        .headers(codex_model_provider::auth_provider_from_auth(auth).to_auth_headers())
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await;

    match response {
        Ok(response) if response.status().is_success() => {}
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("events failed with status {status}: {body}");
        }
        Err(err) => {
            tracing::warn!("failed to send events request: {err}");
        }
    }
}

#[cfg(test)]
mod queue_dedupe_tests {
    //! Per-turn dedupe behavior of [`AnalyticsEventsQueue`]. These live here
    //! (rather than in `codex-analytics-appserver`) because they construct the
    //! crate-private `AnalyticsEventsQueue` by field, which is only possible
    //! within this crate.

    use super::AnalyticsEventsQueue;
    use crate::facts::AppInvocation;
    use crate::facts::InvocationType;
    use crate::facts::TrackEventsContext;
    use codex_plugin::AppConnectorId;
    use codex_plugin::PluginCapabilitySummary;
    use codex_plugin::PluginId;
    use codex_plugin::PluginTelemetryMetadata;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    fn sample_plugin_metadata() -> PluginTelemetryMetadata {
        PluginTelemetryMetadata {
            plugin_id: PluginId::parse("sample@test").expect("valid plugin id"),
            remote_plugin_id: None,
            capability_summary: Some(PluginCapabilitySummary {
                config_name: "sample@test".to_string(),
                display_name: "sample".to_string(),
                description: None,
                has_skills: true,
                mcp_server_names: vec!["mcp-1".to_string(), "mcp-2".to_string()],
                app_connector_ids: vec![
                    AppConnectorId("calendar".to_string()),
                    AppConnectorId("drive".to_string()),
                ],
            }),
        }
    }

    #[test]
    fn app_used_dedupe_is_keyed_by_turn_and_connector() {
        let (sender, _receiver) = mpsc::channel(1);
        let queue = AnalyticsEventsQueue {
            sender,
            app_used_emitted_keys: Arc::new(Mutex::new(HashSet::new())),
            plugin_used_emitted_keys: Arc::new(Mutex::new(HashSet::new())),
        };
        let app = AppInvocation {
            connector_id: Some("calendar".to_string()),
            app_name: Some("Calendar".to_string()),
            invocation_type: Some(InvocationType::Implicit),
        };

        let turn_1 = TrackEventsContext {
            model_slug: "gpt-5".to_string(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
        };
        let turn_2 = TrackEventsContext {
            model_slug: "gpt-5".to_string(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-2".to_string(),
        };

        assert_eq!(queue.should_enqueue_app_used(&turn_1, &app), true);
        assert_eq!(queue.should_enqueue_app_used(&turn_1, &app), false);
        assert_eq!(queue.should_enqueue_app_used(&turn_2, &app), true);
    }

    #[test]
    fn plugin_used_dedupe_is_keyed_by_turn_and_plugin() {
        let (sender, _receiver) = mpsc::channel(1);
        let queue = AnalyticsEventsQueue {
            sender,
            app_used_emitted_keys: Arc::new(Mutex::new(HashSet::new())),
            plugin_used_emitted_keys: Arc::new(Mutex::new(HashSet::new())),
        };
        let plugin = sample_plugin_metadata();

        let turn_1 = TrackEventsContext {
            model_slug: "gpt-5".to_string(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
        };
        let turn_2 = TrackEventsContext {
            model_slug: "gpt-5".to_string(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-2".to_string(),
        };

        assert_eq!(queue.should_enqueue_plugin_used(&turn_1, &plugin), true);
        assert_eq!(queue.should_enqueue_plugin_used(&turn_1, &plugin), false);
        assert_eq!(queue.should_enqueue_plugin_used(&turn_2, &plugin), true);
    }
}
