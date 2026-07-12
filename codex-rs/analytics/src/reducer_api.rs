//! The protocol-free seam between the analytics queue (this crate) and a
//! pluggable reducer that turns [`AnalyticsFact`]s into wire-ready
//! [`TrackEvent`]s.
//!
//! `codex-analytics` stays protocol-free: it owns the background queue and a
//! default [`CustomFactReducer`] that handles only the core-facing custom
//! facts. The app-server-facing reducer (which consumes the app-server RPC
//! protocol types) lives in the upper crate `codex-analytics-appserver` and is
//! injected through the [`AnalyticsReducer`] trait.

use crate::events::CodexAppMentionedEventRequest;
use crate::events::CodexAppUsedEventRequest;
use crate::events::CodexHookRunEventRequest;
use crate::events::CodexPluginEventRequest;
use crate::events::CodexPluginInstallRequestedEventRequest;
use crate::events::CodexPluginUsedEventRequest;
use crate::events::SkillInvocationEventParams;
use crate::events::SkillInvocationEventRequest;
use crate::events::codex_app_metadata;
use crate::events::codex_hook_run_metadata;
use crate::events::codex_plugin_install_requested_metadata;
use crate::events::codex_plugin_metadata;
use crate::events::codex_plugin_used_metadata;
use crate::events::plugin_state_event_type;
use crate::events::skill_id_for_local_skill;
use crate::events::subagent_thread_started_event_request;
use crate::facts::AnalyticsFact;
use crate::facts::AppMentionedInput;
use crate::facts::AppUsedInput;
use crate::facts::CustomAnalyticsFact;
use crate::facts::HookRunInput;
use crate::facts::PluginInstallRequestedInput;
use crate::facts::PluginState;
use crate::facts::PluginStateChangedInput;
use crate::facts::PluginUsedInput;
use crate::facts::SkillInvokedInput;
use crate::facts::SubAgentThreadStartedInput;
use codex_git_utils::collect_git_info;
use codex_git_utils::get_git_repo_root;
use codex_login::default_client::originator;
use codex_protocol::protocol::SkillScope;
use serde::Serialize;

/// A serialized, wire-ready analytics event.
///
/// The queue in this crate never inspects the body; it only forwards it to the
/// analytics endpoint. Both the lower [`CustomFactReducer`] and the upper
/// app-server reducer build these. `isolated` mirrors the previous
/// `should_send_in_isolated_request` behavior so the queue can keep batching
/// identically.
#[derive(Clone, Debug)]
pub struct TrackEvent {
    body: serde_json::Value,
    isolated: bool,
}

impl TrackEvent {
    /// Build a [`TrackEvent`] from any serializable event-request body.
    pub fn from_serializable<T: Serialize>(body: &T, isolated: bool) -> Self {
        Self {
            body: serde_json::to_value(body).unwrap_or(serde_json::Value::Null),
            isolated,
        }
    }

    /// Build a [`TrackEvent`] from an already-serialized JSON body.
    pub fn from_value(body: serde_json::Value, isolated: bool) -> Self {
        Self { body, isolated }
    }

    /// Whether this event must be sent in its own isolated request (preserves
    /// the previous accepted-line-fingerprints batching behavior).
    pub fn should_send_in_isolated_request(&self) -> bool {
        self.isolated
    }

    /// The serialized event body, consumed for sending.
    pub fn into_body(self) -> serde_json::Value {
        self.body
    }
}

/// A pluggable reducer that converts [`AnalyticsFact`]s into [`TrackEvent`]s.
///
/// `codex-core` injects the protocol-free [`CustomFactReducer`];
/// `codex-app-server` injects the protocol-aware reducer from
/// `codex-analytics-appserver`.
#[async_trait::async_trait]
pub trait AnalyticsReducer: Send {
    async fn ingest(&mut self, input: AnalyticsFact, out: &mut Vec<TrackEvent>);
}

/// The default, protocol-free reducer used by `codex-core`.
///
/// It handles only the custom facts that core emits and that do not depend on
/// app-server connection/thread state. The connection-gated custom facts
/// (compaction, guardian review, turn events) require analytics context that is
/// only populated from app-server RPC facts; core never supplies that context,
/// so — exactly as before this split — those events are not emitted here.
/// App-server-shaped facts ([`AnalyticsFact::AppServer`]) are opaque to this
/// crate and are ignored by this reducer (the app-server reducer handles them).
#[derive(Default)]
pub struct CustomFactReducer;

#[async_trait::async_trait]
impl AnalyticsReducer for CustomFactReducer {
    async fn ingest(&mut self, input: AnalyticsFact, out: &mut Vec<TrackEvent>) {
        let AnalyticsFact::Custom(custom) = input else {
            return;
        };
        match custom {
            CustomAnalyticsFact::SubAgentThreadStarted(input) => {
                ingest_subagent_thread_started(input, out);
            }
            CustomAnalyticsFact::SkillInvoked(input) => {
                ingest_skill_invoked(input, out).await;
            }
            CustomAnalyticsFact::AppMentioned(input) => ingest_app_mentioned(input, out),
            CustomAnalyticsFact::AppUsed(input) => ingest_app_used(input, out),
            CustomAnalyticsFact::HookRun(input) => ingest_hook_run(input, out),
            CustomAnalyticsFact::PluginUsed(input) => ingest_plugin_used(input, out),
            CustomAnalyticsFact::PluginStateChanged(input) => {
                ingest_plugin_state_changed(input, out);
            }
            CustomAnalyticsFact::PluginInstallRequested(input) => {
                ingest_plugin_install_requested(input, out);
            }
            // Connection-gated facts: dropped without app-server analytics
            // context (unchanged behavior for core).
            CustomAnalyticsFact::Compaction(_)
            | CustomAnalyticsFact::GuardianReview(_)
            | CustomAnalyticsFact::TurnResolvedConfig(_)
            | CustomAnalyticsFact::TurnTokenUsage(_)
            | CustomAnalyticsFact::TurnProfile(_)
            | CustomAnalyticsFact::TurnCodexError(_) => {}
            // Goal events are connection-gated (require app-server thread context);
            // the app-server reducer handles them. No-op here exactly like the
            // other connection-gated variants above.
            CustomAnalyticsFact::Goal(_) => {}
            // Plugin install failure and external-agent config import events are
            // not yet consumed by the protocol-free reducer; drop as no-ops
            // consistent with the connection-gated variants above.
            CustomAnalyticsFact::PluginInstallFailed(_)
            | CustomAnalyticsFact::ExternalAgentConfigImportCompleted(_)
            | CustomAnalyticsFact::ExternalAgentConfigImportFailure(_) => {}
        }
    }
}

fn ingest_subagent_thread_started(input: SubAgentThreadStartedInput, out: &mut Vec<TrackEvent>) {
    out.push(TrackEvent::from_serializable(
        &subagent_thread_started_event_request(input),
        false,
    ));
}

async fn ingest_skill_invoked(input: SkillInvokedInput, out: &mut Vec<TrackEvent>) {
    let SkillInvokedInput {
        tracking,
        invocations,
    } = input;
    for invocation in invocations {
        let skill_scope = match invocation.skill_scope {
            SkillScope::User => "user",
            SkillScope::Repo => "repo",
            SkillScope::System => "system",
            SkillScope::Admin => "admin",
        };
        let repo_root = get_git_repo_root(invocation.skill_path.as_path());
        let repo_url = if let Some(root) = repo_root.as_ref() {
            collect_git_info(root)
                .await
                .and_then(|info| info.repository_url)
        } else {
            None
        };
        let skill_id = skill_id_for_local_skill(
            repo_url.as_deref(),
            repo_root.as_deref(),
            invocation.skill_path.as_path(),
            invocation.skill_name.as_str(),
        );
        out.push(TrackEvent::from_serializable(
            &SkillInvocationEventRequest {
                event_type: "skill_invocation",
                skill_id,
                skill_name: invocation.skill_name.clone(),
                event_params: SkillInvocationEventParams {
                    thread_id: Some(tracking.thread_id.clone()),
                    turn_id: Some(tracking.turn_id.clone()),
                    invoke_type: Some(invocation.invocation_type),
                    model_slug: Some(tracking.model_slug.clone()),
                    product_client_id: Some(originator().value),
                    repo_url,
                    skill_scope: Some(skill_scope.to_string()),
                    plugin_id: invocation.plugin_id,
                },
            },
            false,
        ));
    }
}

fn ingest_app_mentioned(input: AppMentionedInput, out: &mut Vec<TrackEvent>) {
    let AppMentionedInput { tracking, mentions } = input;
    for mention in mentions {
        let event_params = codex_app_metadata(&tracking, mention);
        out.push(TrackEvent::from_serializable(
            &CodexAppMentionedEventRequest {
                event_type: "codex_app_mentioned",
                event_params,
            },
            false,
        ));
    }
}

fn ingest_app_used(input: AppUsedInput, out: &mut Vec<TrackEvent>) {
    let AppUsedInput { tracking, app } = input;
    let event_params = codex_app_metadata(&tracking, app);
    out.push(TrackEvent::from_serializable(
        &CodexAppUsedEventRequest {
            event_type: "codex_app_used",
            event_params,
        },
        false,
    ));
}

fn ingest_hook_run(input: HookRunInput, out: &mut Vec<TrackEvent>) {
    let HookRunInput { tracking, hook } = input;
    out.push(TrackEvent::from_serializable(
        &CodexHookRunEventRequest {
            event_type: "codex_hook_run",
            event_params: codex_hook_run_metadata(&tracking, hook),
        },
        false,
    ));
}

fn ingest_plugin_used(input: PluginUsedInput, out: &mut Vec<TrackEvent>) {
    let PluginUsedInput { tracking, plugin } = input;
    out.push(TrackEvent::from_serializable(
        &CodexPluginUsedEventRequest {
            event_type: "codex_plugin_used",
            event_params: codex_plugin_used_metadata(&tracking, plugin),
        },
        false,
    ));
}

fn ingest_plugin_install_requested(input: PluginInstallRequestedInput, out: &mut Vec<TrackEvent>) {
    let PluginInstallRequestedInput { tracking, request } = input;
    out.push(TrackEvent::from_serializable(
        &CodexPluginInstallRequestedEventRequest {
            event_type: "codex_plugin_install_requested",
            event_params: codex_plugin_install_requested_metadata(&tracking, request),
        },
        false,
    ));
}

fn ingest_plugin_state_changed(input: PluginStateChangedInput, out: &mut Vec<TrackEvent>) {
    let PluginStateChangedInput { plugin, state } = input;
    let event = CodexPluginEventRequest {
        event_type: plugin_state_event_type(state),
        event_params: codex_plugin_metadata(plugin),
    };
    let _ = match state {
        PluginState::Installed
        | PluginState::Uninstalled
        | PluginState::Enabled
        | PluginState::Disabled => (),
    };
    out.push(TrackEvent::from_serializable(&event, false));
}
