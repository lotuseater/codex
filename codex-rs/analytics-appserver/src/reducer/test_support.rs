//! Test-only adapter that lets the app-server analytics test suite drive
//! [`AppServerReducer`] through the rich, pre-split fact shape it was authored
//! against.
//!
//! Background: the analytics crate split moved the app-server RPC fact variants
//! out of `codex_analytics::AnalyticsFact` (now just `AppServer(Value)` +
//! `Custom(..)`) into [`crate::rpc_fact::AppServerFact`], turned the reducer
//! into the `codex_analytics::AnalyticsReducer` trait implemented by
//! [`AppServerReducer`], and routed the unconditional custom facts through the
//! lower crate's `CustomFactReducer` (which emits the opaque `TrackEvent`). The
//! test suite, however, exercises the reducer at the structured
//! `Vec<TrackEventRequest>` level and constructs a single rich fact enum.
//!
//! Rather than rewrite ~120 reducer/fact call sites across 10 test files (which
//! would also lose the structured `TrackEventRequest` assertions), this module
//! re-creates exactly that surface for tests:
//!   * a `pub(crate) type AnalyticsReducer = AppServerReducer;`
//!   * the rich [`AnalyticsFact`] enum (the pre-split shape), and
//!   * a test-only `AppServerReducer::ingest(AnalyticsFact, &mut Vec<TrackEventRequest>)`
//!     that maps each variant onto the production reducer internals.
//!
//! It is `#[cfg(test)]`, so it adds no production surface.

use super::AppServerReducer;
use crate::events::CodexAppMentionedEventRequest;
use crate::events::CodexAppUsedEventRequest;
use crate::events::CodexHookRunEventRequest;
use crate::events::CodexPluginEventRequest;
use crate::events::CodexPluginUsedEventRequest;
use crate::events::SkillInvocationEventParams;
use crate::events::SkillInvocationEventRequest;
use crate::events::TrackEventRequest;
use crate::events::codex_app_metadata;
use crate::events::codex_hook_run_metadata;
use crate::events::codex_plugin_metadata;
use crate::events::codex_plugin_used_metadata;
use crate::events::plugin_state_event_type;
use crate::events::subagent_thread_started_event_request;
use crate::rpc_fact::AppServerFact;
use codex_analytics::AnalyticsJsonRpcError;
use codex_analytics::AppServerRpcTransport;
use codex_analytics::CustomAnalyticsFact;
use codex_analytics::PluginState;
use codex_analytics::skill_id_for_local_skill;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ClientResponsePayload;
use codex_app_server_protocol::InitializeParams;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ServerResponse;
use codex_protocol::protocol::SkillScope;
use codex_protocol::request_permissions::RequestPermissionsResponse;

/// The concrete reducer the test suite constructs via `AnalyticsReducer::default()`.
pub(crate) type AnalyticsReducer = AppServerReducer;

/// The pre-split analytics fact shape the test suite still constructs.
///
/// Mirrors the historical `codex_analytics::AnalyticsFact` (rich RPC variants +
/// `Custom`). Note that `Initialize` carries the `runtime` that production now
/// derives at reduce time; the test adapter injects it into connection state so
/// the suite's runtime assertions keep their meaning.
pub(crate) enum AnalyticsFact {
    Initialize {
        connection_id: u64,
        params: InitializeParams,
        product_client_id: String,
        runtime: crate::events::CodexRuntimeMetadata,
        rpc_transport: AppServerRpcTransport,
    },
    ClientRequest {
        connection_id: u64,
        request_id: RequestId,
        request: Box<ClientRequest>,
    },
    ClientResponse {
        connection_id: u64,
        request_id: RequestId,
        response: Box<ClientResponsePayload>,
    },
    ErrorResponse {
        connection_id: u64,
        request_id: RequestId,
        error: JSONRPCErrorError,
        error_type: Option<AnalyticsJsonRpcError>,
    },
    ServerRequest {
        connection_id: u64,
        request: Box<ServerRequest>,
    },
    ServerResponse {
        completed_at_ms: u64,
        response: Box<ServerResponse>,
    },
    EffectivePermissionsApprovalResponse {
        completed_at_ms: u64,
        request_id: RequestId,
        response: Box<RequestPermissionsResponse>,
    },
    ServerRequestAborted {
        completed_at_ms: u64,
        request_id: RequestId,
    },
    Notification(Box<ServerNotification>),
    Custom(CustomAnalyticsFact),
}

impl AppServerReducer {
    /// Test-only entry point mirroring the historical
    /// `AnalyticsReducer::ingest(AnalyticsFact, &mut Vec<TrackEventRequest>)`.
    pub(crate) async fn ingest(&mut self, fact: AnalyticsFact, out: &mut Vec<TrackEventRequest>) {
        match fact {
            AnalyticsFact::Initialize {
                connection_id,
                params,
                product_client_id,
                runtime,
                rpc_transport,
            } => {
                self.ingest_app_server(
                    AppServerFact::Initialize {
                        connection_id,
                        params,
                        product_client_id,
                        rpc_transport,
                    },
                    out,
                )
                .await;
                // Production derives runtime from the host at reduce time; tests
                // inject it via `Initialize.runtime` and assert on those values,
                // so overwrite the freshly-stored connection runtime with it.
                self.set_connection_runtime_for_test(connection_id, runtime);
            }
            AnalyticsFact::ClientRequest {
                connection_id,
                request_id,
                request,
            } => {
                self.ingest_app_server(
                    AppServerFact::ClientRequest {
                        connection_id,
                        request_id,
                        request,
                    },
                    out,
                )
                .await;
            }
            AnalyticsFact::ClientResponse {
                connection_id,
                request_id,
                response,
            } => {
                self.ingest_app_server(
                    AppServerFact::ClientResponse {
                        connection_id,
                        request_id,
                        response,
                    },
                    out,
                )
                .await;
            }
            AnalyticsFact::ErrorResponse {
                connection_id,
                request_id,
                error,
                error_type,
            } => {
                self.ingest_app_server(
                    AppServerFact::ErrorResponse {
                        connection_id,
                        request_id,
                        error,
                        error_type,
                    },
                    out,
                )
                .await;
            }
            AnalyticsFact::ServerRequest {
                connection_id,
                request,
            } => {
                self.ingest_app_server(
                    AppServerFact::ServerRequest {
                        connection_id,
                        request,
                    },
                    out,
                )
                .await;
            }
            AnalyticsFact::ServerResponse {
                completed_at_ms,
                response,
            } => {
                self.ingest_app_server(
                    AppServerFact::ServerResponse {
                        completed_at_ms,
                        response,
                    },
                    out,
                )
                .await;
            }
            AnalyticsFact::EffectivePermissionsApprovalResponse {
                completed_at_ms,
                request_id,
                response,
            } => {
                self.ingest_app_server(
                    AppServerFact::EffectivePermissionsApprovalResponse {
                        completed_at_ms,
                        request_id,
                        response,
                    },
                    out,
                )
                .await;
            }
            AnalyticsFact::ServerRequestAborted {
                completed_at_ms,
                request_id,
            } => {
                self.ingest_app_server(
                    AppServerFact::ServerRequestAborted {
                        completed_at_ms,
                        request_id,
                    },
                    out,
                )
                .await;
            }
            AnalyticsFact::Notification(notification) => {
                self.ingest_app_server(AppServerFact::Notification(notification), out)
                    .await;
            }
            AnalyticsFact::Custom(custom) => self.ingest_custom_for_test(custom, out).await,
        }
    }

    /// Overwrite the runtime stored for a connection so the suite's injected
    /// `Initialize.runtime` (rather than the host's) drives runtime assertions.
    fn set_connection_runtime_for_test(
        &mut self,
        connection_id: u64,
        runtime: crate::events::CodexRuntimeMetadata,
    ) {
        if let Some(connection) = self.connections.get_mut(&connection_id) {
            connection.runtime = runtime;
        }
    }

    async fn ingest_custom_for_test(
        &mut self,
        custom: CustomAnalyticsFact,
        out: &mut Vec<TrackEventRequest>,
    ) {
        match custom {
            // Connection-gated custom facts: handled by the production methods,
            // which already emit `TrackEventRequest`.
            CustomAnalyticsFact::Compaction(input) => self.ingest_compaction(*input, out),
            CustomAnalyticsFact::GuardianReview(input) => self.ingest_guardian_review(*input, out),
            CustomAnalyticsFact::TurnResolvedConfig(input) => {
                self.ingest_turn_resolved_config(*input, out).await;
            }
            CustomAnalyticsFact::TurnTokenUsage(input) => {
                self.ingest_turn_token_usage(*input, out).await;
            }
            // Unconditional custom facts: production routes these through the
            // lower crate's `CustomFactReducer` (opaque `TrackEvent`). Re-create
            // the structured `TrackEventRequest` the suite asserts on.
            CustomAnalyticsFact::SubAgentThreadStarted(input) => {
                out.push(TrackEventRequest::ThreadInitialized(
                    subagent_thread_started_event_request(input),
                ));
            }
            CustomAnalyticsFact::SkillInvoked(input) => {
                ingest_skill_invoked_for_test(input, out).await;
            }
            CustomAnalyticsFact::AppMentioned(input) => {
                let codex_analytics::AppMentionedInput { tracking, mentions } = input;
                for mention in mentions {
                    out.push(TrackEventRequest::AppMentioned(
                        CodexAppMentionedEventRequest {
                            event_type: "codex_app_mentioned",
                            event_params: codex_app_metadata(&tracking, mention),
                        },
                    ));
                }
            }
            CustomAnalyticsFact::AppUsed(input) => {
                let codex_analytics::AppUsedInput { tracking, app } = input;
                out.push(TrackEventRequest::AppUsed(CodexAppUsedEventRequest {
                    event_type: "codex_app_used",
                    event_params: codex_app_metadata(&tracking, app),
                }));
            }
            CustomAnalyticsFact::HookRun(input) => {
                let codex_analytics::HookRunInput { tracking, hook } = input;
                out.push(TrackEventRequest::HookRun(CodexHookRunEventRequest {
                    event_type: "codex_hook_run",
                    event_params: codex_hook_run_metadata(&tracking, hook),
                }));
            }
            CustomAnalyticsFact::PluginUsed(input) => {
                let codex_analytics::PluginUsedInput { tracking, plugin } = input;
                out.push(TrackEventRequest::PluginUsed(CodexPluginUsedEventRequest {
                    event_type: "codex_plugin_used",
                    event_params: codex_plugin_used_metadata(&tracking, plugin),
                }));
            }
            CustomAnalyticsFact::PluginStateChanged(input) => {
                let codex_analytics::PluginStateChangedInput { plugin, state } = input;
                let event = CodexPluginEventRequest {
                    event_type: plugin_state_event_type(state),
                    event_params: codex_plugin_metadata(plugin),
                };
                let request = match state {
                    PluginState::Installed => TrackEventRequest::PluginInstalled(event),
                    PluginState::Uninstalled => TrackEventRequest::PluginUninstalled(event),
                    PluginState::Enabled => TrackEventRequest::PluginEnabled(event),
                    PluginState::Disabled => TrackEventRequest::PluginDisabled(event),
                };
                out.push(request);
            }
        }
    }
}

async fn ingest_skill_invoked_for_test(
    input: codex_analytics::SkillInvokedInput,
    out: &mut Vec<TrackEventRequest>,
) {
    use codex_git_utils::collect_git_info;
    use codex_git_utils::get_git_repo_root;
    use codex_login::default_client::originator;

    // `codex_analytics::InvocationType` and the wire-shaped
    // `crate::events::InvocationType` are distinct (identical) enums after the
    // split; map between them.
    fn map_invocation_type(
        invocation_type: codex_analytics::InvocationType,
    ) -> crate::events::InvocationType {
        match invocation_type {
            codex_analytics::InvocationType::Explicit => crate::events::InvocationType::Explicit,
            codex_analytics::InvocationType::Implicit => crate::events::InvocationType::Implicit,
        }
    }

    let codex_analytics::SkillInvokedInput {
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
        out.push(TrackEventRequest::SkillInvocation(
            SkillInvocationEventRequest {
                event_type: "skill_invocation",
                skill_id,
                skill_name: invocation.skill_name.clone(),
                event_params: SkillInvocationEventParams {
                    thread_id: Some(tracking.thread_id.clone()),
                    turn_id: Some(tracking.turn_id.clone()),
                    invoke_type: Some(map_invocation_type(invocation.invocation_type)),
                    model_slug: Some(tracking.model_slug.clone()),
                    product_client_id: Some(originator().value),
                    repo_url,
                    skill_scope: Some(skill_scope.to_string()),
                    plugin_id: invocation.plugin_id,
                },
            },
        ));
    }
}
