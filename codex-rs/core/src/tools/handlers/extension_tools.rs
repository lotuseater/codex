use std::sync::Arc;
use std::sync::Weak;

use codex_extension_api::ExtensionToolExecutor;
use codex_protocol::items::TurnItem;
use codex_tool_execution_api::FunctionCallError;
use codex_tools::ConversationHistory;
use codex_tools::ExtensionTurnItem;
use codex_tools::ImageGenerationCompletionFuture;
use codex_tools::ToolCall as ExtensionToolCall;
use codex_tools::ToolEnvironment;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use codex_tools::TurnItemEmissionFuture;
use codex_tools::TurnItemEmitter;
use serde_json::Value;

use crate::context::ContextualUserFragment;
use crate::context::ImageGenerationInstructions;
use crate::sandboxing::SandboxPermissions;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::stream_events_utils::persist_image_generation_item;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::flat_tool_name;
use crate::tools::handlers::apply_granted_turn_permissions;
use crate::tools::hook_names::HookToolName;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::PreToolUsePayload;
use crate::tools::registry::ToolExecutor;

pub(crate) struct ExtensionToolHandler {
    executor: Arc<dyn ExtensionToolExecutor>,
}

impl ExtensionToolHandler {
    pub(crate) fn new(executor: Arc<dyn ExtensionToolExecutor>) -> Self {
        Self { executor }
    }

    fn arguments_from_payload<'a>(&self, payload: &'a ToolPayload) -> Option<&'a str> {
        let ToolPayload::Function { arguments } = payload else {
            return None;
        };
        Some(arguments)
    }
}

impl ToolExecutor<ToolInvocation> for ExtensionToolHandler {
    type Output = Box<dyn ToolOutput>;

    fn tool_name(&self) -> ToolName {
        self.executor.tool_name()
    }

    fn spec(&self) -> Option<ToolSpec> {
        self.executor.spec()
    }

    async fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        self.executor
            .handle(to_extension_call(&invocation).await)
            .await
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        self.executor.supports_parallel_tool_calls()
    }

    fn exposure(&self) -> crate::tools::registry::ToolExposure {
        self.executor.exposure()
    }
}

impl CoreToolRuntime for ExtensionToolHandler {
    fn matches_kind(&self, payload: &ToolPayload) -> bool {
        self.arguments_from_payload(payload).is_some()
    }

    fn pre_tool_use_payload(&self, invocation: &ToolInvocation) -> Option<PreToolUsePayload> {
        let arguments = self.arguments_from_payload(&invocation.payload)?;
        Some(PreToolUsePayload {
            tool_name: HookToolName::new(flat_tool_name(&self.tool_name()).into_owned()),
            tool_input: extension_tool_hook_input(arguments),
        })
    }

    fn post_tool_use_payload(
        &self,
        invocation: &ToolInvocation,
        result: &dyn crate::tools::context::ToolOutput,
    ) -> Option<PostToolUsePayload> {
        let arguments = self.arguments_from_payload(&invocation.payload)?;
        Some(PostToolUsePayload {
            tool_name: HookToolName::new(flat_tool_name(&self.tool_name()).into_owned()),
            tool_use_id: invocation.call_id.clone(),
            tool_input: extension_tool_hook_input(arguments),
            tool_response: result
                .post_tool_use_response(&invocation.call_id, &invocation.payload)?,
        })
    }
}

async fn to_extension_call(invocation: &ToolInvocation) -> ExtensionToolCall {
    let conversation_history =
        ConversationHistory::new(invocation.session.clone_history().await.into_raw_items());
    let mut environments = Vec::with_capacity(invocation.turn.environments.turn_environments.len());
    for environment in &invocation.turn.environments.turn_environments {
        // TODO(anp): Migrate extension ToolEnvironment and granted-permission lookup to PathUri
        // so extensions can receive foreign environment cwd values.
        let Ok(native_cwd) = environment.cwd().to_abs_path() else {
            continue;
        };
        let additional_permissions = apply_granted_turn_permissions(
            invocation.session.as_ref(),
            &environment.environment_id,
            native_cwd.as_path(),
            SandboxPermissions::UseDefault,
            /*additional_permissions*/ None,
        )
        .await
        .additional_permissions;
        let file_system_sandbox_context = invocation
            .turn
            .file_system_sandbox_context(additional_permissions, environment.cwd());
        environments.push(ToolEnvironment {
            environment_id: environment.environment_id.clone(),
            cwd: native_cwd,
            file_system: environment.environment.get_filesystem(),
            file_system_sandbox_context,
        });
    }
    ExtensionToolCall {
        turn_id: invocation.turn.sub_id.clone(),
        call_id: invocation.call_id.clone(),
        tool_name: invocation.tool_name.clone(),
        model: invocation.turn.model_info.slug.clone(),
        truncation_policy: invocation.turn.model_info.truncation_policy.into(),
        conversation_history,
        turn_item_emitter: Arc::new(CoreTurnItemEmitter {
            session: Arc::downgrade(&invocation.session),
            turn: Arc::downgrade(&invocation.turn),
        }),
        environments,
        payload: invocation.payload.clone(),
    }
}

/// Host-side bridge that routes extension turn-item lifecycle events through the
/// session's normal item event pipeline (persistence + client delivery).
struct CoreTurnItemEmitter {
    session: Weak<Session>,
    turn: Weak<TurnContext>,
}

impl TurnItemEmitter for CoreTurnItemEmitter {
    fn emit_started<'a>(&'a self, item: ExtensionTurnItem) -> TurnItemEmissionFuture<'a> {
        Box::pin(async move {
            let (Some(session), Some(turn)) = (self.session.upgrade(), self.turn.upgrade()) else {
                return;
            };
            let item = extension_turn_item(item);
            session.emit_turn_item_started(turn.as_ref(), &item).await;
        })
    }

    fn emit_completed<'a>(&'a self, item: ExtensionTurnItem) -> TurnItemEmissionFuture<'a> {
        Box::pin(async move {
            let (Some(session), Some(turn)) = (self.session.upgrade(), self.turn.upgrade()) else {
                return;
            };
            let item = extension_turn_item(item);
            session.emit_turn_item_completed(turn.as_ref(), item).await;
        })
    }

    fn image_generation_completed<'a>(
        &'a self,
        call_id: String,
        prompt: String,
        result: String,
    ) -> ImageGenerationCompletionFuture<'a> {
        Box::pin(async move {
            let (Some(session), Some(turn)) = (self.session.upgrade(), self.turn.upgrade()) else {
                return None;
            };
            let mut item = codex_protocol::items::ImageGenerationItem {
                id: call_id,
                status: "completed".to_string(),
                revised_prompt: Some(prompt),
                result,
                saved_path: None,
            };
            let output_hint =
                persist_image_generation_item(session.as_ref(), turn.as_ref(), &mut item)
                    .await
                    .map(|saved_path| {
                        let output_dir = saved_path
                            .parent()
                            .unwrap_or_else(|| turn.config.codex_home.clone());
                        ImageGenerationInstructions::new(output_dir.display(), saved_path.display())
                            .body()
                    });
            let started_item = codex_protocol::items::ImageGenerationItem {
                id: item.id.clone(),
                status: "in_progress".to_string(),
                revised_prompt: None,
                result: String::new(),
                saved_path: None,
            };
            session
                .emit_turn_item_started(turn.as_ref(), &TurnItem::ImageGeneration(started_item))
                .await;
            session
                .emit_turn_item_completed(turn.as_ref(), TurnItem::ImageGeneration(item))
                .await;
            output_hint
        })
    }
}

fn extension_turn_item(item: ExtensionTurnItem) -> TurnItem {
    match item {
        ExtensionTurnItem::WebSearch(item) => TurnItem::WebSearch(item),
        ExtensionTurnItem::ImageGeneration(item) => TurnItem::ImageGeneration(item),
    }
}

fn extension_tool_hook_input(arguments: &str) -> Value {
    if arguments.trim().is_empty() {
        return Value::Object(serde_json::Map::new());
    }

    serde_json::from_str(arguments).unwrap_or_else(|_| Value::String(arguments.to_string()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use codex_protocol::models::ContentItem;
    use codex_protocol::models::ResponseItem;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tokio::sync::Mutex;

    use super::ExtensionToolHandler;
    use crate::tools::context::ToolCallSource;
    use crate::tools::context::ToolInvocation;
    use crate::tools::context::ToolPayload;
    use crate::tools::hook_names::HookToolName;
    use crate::tools::registry::CoreToolRuntime;
    use crate::tools::registry::PostToolUsePayload;
    use crate::tools::registry::PreToolUsePayload;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use codex_tools::ResponsesApiTool;
    use codex_tools::ToolName;
    use codex_tools::ToolSpec;
    use codex_tools::parse_tool_input_schema;

    struct StubExtensionExecutor;

    // fork-local: extensions implement the shared `ToolExecutor<ToolCall>` contract directly
    // (RPITIT `handle`, `Option<ToolSpec>`, associated `Output`); the `ExtensionToolExecutor`
    // blanket impl makes them dynamically dispatchable. Upstream's old `#[async_trait]` /
    // `ToolSpec` (non-Option) form no longer matches the live trait.
    impl codex_extension_api::ToolExecutor<ExtensionToolCall> for StubExtensionExecutor {
        type Output = Box<dyn codex_tools::ToolOutput>;

        fn tool_name(&self) -> ToolName {
            ToolName::plain("extension_echo")
        }

        fn spec(&self) -> Option<ToolSpec> {
            Some(ToolSpec::Function(ResponsesApiTool {
                name: "extension_echo".to_string(),
                description: "Echoes arguments.".to_string(),
                strict: true,
                parameters: parse_tool_input_schema(&json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" },
                    },
                    "required": ["message"],
                    "additionalProperties": false,
                }))
                .expect("extension schema should parse"),
                output_schema: None,
                defer_loading: None,
            }))
        }

        async fn handle(
            &self,
            _call: ExtensionToolCall,
        ) -> Result<Box<dyn codex_tools::ToolOutput>, codex_tools::FunctionCallError> {
            Ok(Box::new(codex_tools::JsonToolOutput::new(
                json!({ "ok": true }),
            )))
        }
    }

    struct CapturingExtensionExecutor {
        captured_call: Arc<Mutex<Option<ExtensionToolCall>>>,
    }

    // fork-local: see `StubExtensionExecutor` — extensions implement `ToolExecutor<ToolCall>`
    // directly with the live RPITIT / `Option<ToolSpec>` shape.
    impl codex_extension_api::ToolExecutor<ExtensionToolCall> for CapturingExtensionExecutor {
        type Output = Box<dyn codex_tools::ToolOutput>;

        fn tool_name(&self) -> codex_tools::ToolName {
            codex_tools::ToolName::plain("extension_echo")
        }

        fn spec(&self) -> Option<codex_tools::ToolSpec> {
            Some(codex_tools::ToolSpec::Function(
                codex_tools::ResponsesApiTool {
                    name: "extension_echo".to_string(),
                    description: "Captures arguments.".to_string(),
                    strict: false,
                    parameters: codex_tools::JsonSchema::default(),
                    output_schema: None,
                    defer_loading: None,
                },
            ))
        }

        async fn handle(
            &self,
            call: codex_tools::ToolCall,
        ) -> Result<Box<dyn codex_tools::ToolOutput>, codex_tools::FunctionCallError> {
            self.handle_call(call).await
        }
    }

    impl CapturingExtensionExecutor {
        async fn handle_call(
            &self,
            call: ExtensionToolCall,
        ) -> Result<Box<dyn codex_tools::ToolOutput>, codex_tools::FunctionCallError> {
            *self.captured_call.lock().await = Some(call);
            Ok(
                Box::new(codex_tools::JsonToolOutput::new(json!({ "ok": true })))
                    as Box<dyn codex_tools::ToolOutput>,
            )
        }
    }

    #[tokio::test]
    async fn exposes_generic_hook_payloads() {
        let handler = ExtensionToolHandler::new(Arc::new(StubExtensionExecutor));
        let (session, turn) = crate::session::tests::make_session_and_context().await;
        let invocation = ToolInvocation {
            session: session.into(),
            turn: turn.into(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
            call_id: "call-extension".to_string(),
            tool_name: ToolName::plain("extension_echo"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: json!({ "message": "hello" }).to_string(),
            },
        };
        let output = codex_tools::JsonToolOutput::new(json!({ "ok": true }));

        assert_eq!(
            CoreToolRuntime::pre_tool_use_payload(&handler, &invocation),
            Some(PreToolUsePayload {
                tool_name: HookToolName::new("extension_echo"),
                tool_input: json!({ "message": "hello" }),
            })
        );
        assert_eq!(
            CoreToolRuntime::post_tool_use_payload(&handler, &invocation, &output),
            Some(PostToolUsePayload {
                tool_name: HookToolName::new("extension_echo"),
                tool_use_id: "call-extension".to_string(),
                tool_input: json!({ "message": "hello" }),
                tool_response: json!({ "ok": true }),
            })
        );
    }

    #[tokio::test]
    async fn passes_turn_fields_to_extension_call() {
        let captured_call = Arc::new(Mutex::new(None));
        let handler = ExtensionToolHandler::new(Arc::new(CapturingExtensionExecutor {
            captured_call: Arc::clone(&captured_call),
        }));
        let (session, turn) = crate::session::tests::make_session_and_context().await;
        let turn_id = turn.sub_id.clone();
        let _model = turn.model_info.slug.clone();
        let truncation_policy = turn.model_info.truncation_policy.into();
        let expected_sandbox_cwds = turn
            .environments
            .turn_environments
            .iter()
            .map(|environment| Some(environment.cwd().clone()))
            .collect::<Vec<_>>();
        let history_item = ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: "extension history".to_string(),
            }],
            phase: None,
            metadata: None,
        };
        session
            .record_into_history(std::slice::from_ref(&history_item), &turn)
            .await;
        let invocation = ToolInvocation {
            session: session.into(),
            turn: turn.into(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
            call_id: "call-extension".to_string(),
            tool_name: codex_tools::ToolName::plain("extension_echo"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: json!({ "message": "hello" }).to_string(),
            },
        };

        crate::tools::registry::ToolExecutor::handle(&handler, invocation)
            .await
            .expect("extension call should succeed");

        let captured_call = captured_call.lock().await.clone().expect("captured call");
        assert_eq!(captured_call.turn_id, turn_id);
        assert_eq!(captured_call.call_id, "call-extension");
        assert_eq!(
            captured_call.tool_name,
            codex_tools::ToolName::plain("extension_echo")
        );
        assert_eq!(captured_call.truncation_policy, truncation_policy);
        assert_eq!(
            captured_call
                .environments
                .iter()
                .map(|environment| environment.file_system_sandbox_context.cwd.clone())
                .collect::<Vec<_>>(),
            expected_sandbox_cwds
        );
        assert_eq!(
            captured_call.conversation_history.items(),
            std::slice::from_ref(&history_item)
        );
        match captured_call.payload {
            ToolPayload::Function { arguments } => {
                assert_eq!(arguments, json!({ "message": "hello" }).to_string());
            }
            payload => panic!("expected function payload, got {payload:?}"),
        }
    }

    struct ImageGenerationExtensionExecutor;

    #[derive(Debug)]
    struct ExtensionTurnItemContributorRan;

    struct RecordExtensionTurnItemContributor;

    impl TurnItemContributor for RecordExtensionTurnItemContributor {
        fn contribute<'a>(
            &'a self,
            _thread_store: &'a ExtensionData,
            turn_store: &'a ExtensionData,
            _item: &'a mut TurnItem,
        ) -> codex_extension_api::ExtensionFuture<'a, Result<(), String>> {
            Box::pin(async move {
                turn_store.insert(ExtensionTurnItemContributorRan);
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn extension_completion_runs_turn_item_contributors() {
        let (mut session, turn) = crate::session::tests::make_session_and_context().await;
        let mut builder = codex_extension_api::ExtensionRegistryBuilder::new();
        builder.turn_item_contributor(Arc::new(RecordExtensionTurnItemContributor));
        session.services.extensions = Arc::new(builder.build());
        let session = Arc::new(session);
        let turn = Arc::new(turn);
        let emitter = CoreTurnItemEmitter {
            session: Arc::downgrade(&session),
            turn: Arc::downgrade(&turn),
        };

        codex_tools::TurnItemEmitter::emit_completed(
            &emitter,
            ExtensionTurnItem::WebSearch(WebSearchItem {
                id: "search-1".to_string(),
                query: "contributors".to_string(),
                action: WebSearchAction::Other,
            }),
        )
        .await;

        assert!(
            turn.extension_data
                .get::<ExtensionTurnItemContributorRan>()
                .is_some()
        );
    }

    impl codex_extension_api::ToolExecutor<codex_tools::ToolCall> for ImageGenerationExtensionExecutor {
        type Output = Box<dyn codex_tools::ToolOutput>;

        fn tool_name(&self) -> codex_tools::ToolName {
            codex_tools::ToolName::namespaced("image_gen", "imagegen")
        }

        fn spec(&self) -> Option<codex_tools::ToolSpec> {
            Some(codex_tools::ToolSpec::Function(
                codex_tools::ResponsesApiTool {
                    name: "imagegen".to_string(),
                    description: "Generates an image.".to_string(),
                    strict: false,
                    parameters: codex_tools::JsonSchema::default(),
                    output_schema: None,
                    defer_loading: None,
                },
            ))
        }

        async fn handle(
            &self,
            call: codex_tools::ToolCall,
        ) -> Result<Box<dyn codex_tools::ToolOutput>, codex_tools::FunctionCallError> {
            self.handle_call(call).await
        }
    }

    impl ImageGenerationExtensionExecutor {
        async fn handle_call(
            &self,
            call: codex_tools::ToolCall,
        ) -> Result<Box<dyn codex_tools::ToolOutput>, codex_tools::FunctionCallError> {
            call.turn_item_emitter
                .emit_started(ExtensionTurnItem::ImageGeneration(
                    codex_protocol::items::ImageGenerationItem {
                        id: call.call_id.clone(),
                        status: "in_progress".to_string(),
                        revised_prompt: None,
                        result: String::new(),
                        saved_path: None,
                    },
                ))
                .await;
            call.turn_item_emitter
                .emit_completed(ExtensionTurnItem::ImageGeneration(
                    codex_protocol::items::ImageGenerationItem {
                        id: call.call_id,
                        status: "completed".to_string(),
                        revised_prompt: Some("A tiny blue square".to_string()),
                        result: "cG5n".to_string(),
                        saved_path: Some(test_path_buf("/tmp/extension-claimed.png").abs()),
                    },
                ))
                .await;
            Ok(
                Box::new(codex_tools::JsonToolOutput::new(json!({ "ok": true })))
                    as Box<dyn codex_tools::ToolOutput>,
            )
        }
    }

    #[tokio::test]
    async fn image_generation_publication_is_finalized_by_core() {
        let handler = ExtensionToolHandler::new(Arc::new(ImageGenerationExtensionExecutor));
        let (session, turn, rx) = crate::session::tests::make_session_and_context_with_rx().await;
        let expected_path = crate::stream_events_utils::image_generation_artifact_path(
            &turn.config.codex_home,
            &session.thread_id.to_string(),
            "call-image",
        );
        let invocation = ToolInvocation {
            session,
            turn,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
            call_id: "call-image".to_string(),
            tool_name: codex_tools::ToolName::namespaced("image_gen", "imagegen"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: "{}".to_string(),
            },
        };

        crate::tools::registry::ToolExecutor::handle(&handler, invocation)
            .await
            .expect("extension call should succeed");

        let started = rx.recv().await.expect("item started event");
        let EventMsg::ItemStarted(started) = started.msg else {
            panic!("expected item started event");
        };
        let TurnItem::ImageGeneration(started_item) = started.item else {
            panic!("expected image generation item");
        };
        let begin = rx.recv().await.expect("legacy image start event");
        assert!(matches!(begin.msg, EventMsg::ImageGenerationBegin(_)));
        let completed = rx.recv().await.expect("item completed event");
        let EventMsg::ItemCompleted(completed) = completed.msg else {
            panic!("expected item completed event");
        };
        let TurnItem::ImageGeneration(completed_item) = completed.item else {
            panic!("expected image generation item");
        };
        let end = rx.recv().await.expect("legacy image end event");
        assert!(matches!(end.msg, EventMsg::ImageGenerationEnd(_)));

        assert_eq!(
            started_item,
            codex_protocol::items::ImageGenerationItem {
                id: "call-image".to_string(),
                status: "in_progress".to_string(),
                revised_prompt: None,
                result: String::new(),
                saved_path: None,
            }
        );
        assert_eq!(
            completed_item,
            codex_protocol::items::ImageGenerationItem {
                id: "call-image".to_string(),
                status: "completed".to_string(),
                revised_prompt: Some("A tiny blue square".to_string()),
                result: "cG5n".to_string(),
                saved_path: Some(expected_path.clone()),
            }
        );
        assert_eq!(
            std::fs::read(&expected_path).expect("generated artifact should be saved"),
            b"png"
        );
    }
}
