use super::fuzzy_file_search::*;
use crate::RequestId;
use crate::export::GeneratedSchema;
use crate::export::write_json_schema;
use crate::protocol::v1;
use crate::protocol::v2;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use ts_rs::TS;

macro_rules! experimental_reason_expr {
    // If a request variant is explicitly marked experimental, that reason wins.
    (variant $variant:ident, #[experimental($reason:expr)] $params:ident $(, $inspect_params:tt)?) => {
        Some($reason)
    };
    // `inspect_params: true` is used when a method is mostly stable but needs
    // field-level gating from its params type (for example, ThreadStart).
    (variant $variant:ident, $params:ident, true) => {
        crate::experimental_api::ExperimentalApi::experimental_reason($params)
    };
    (variant $variant:ident, $params:ident $(, $inspect_params:tt)?) => {
        None
    };
}

macro_rules! experimental_method_entry {
    (#[experimental($reason:expr)] => $wire:literal) => {
        $wire
    };
    (#[experimental($reason:expr)]) => {
        $reason
    };
    ($($tt:tt)*) => {
        ""
    };
}

macro_rules! experimental_type_entry {
    (#[experimental($reason:expr)] $ty:ty) => {
        stringify!($ty)
    };
    ($ty:ty) => {
        ""
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientRequestSerializationScope {
    Global(&'static str),
    GlobalSharedRead(&'static str),
    Thread { thread_id: String },
    ThreadPath { path: PathBuf },
    CommandExecProcess { process_id: String },
    Process { process_handle: String },
    FuzzyFileSearchSession { session_id: String },
    FsWatch { watch_id: String },
    McpOauth { server_name: String },
}

macro_rules! serialization_scope_expr {
    ($actual_params:ident, None) => {
        None
    };
    ($actual_params:ident, global($key:literal)) => {
        Some(ClientRequestSerializationScope::Global($key))
    };
    ($actual_params:ident, global_shared_read($key:literal)) => {
        Some(ClientRequestSerializationScope::GlobalSharedRead($key))
    };
    ($actual_params:ident, thread_id($params:ident . $field:ident)) => {
        Some(ClientRequestSerializationScope::Thread {
            thread_id: $actual_params.$field.clone(),
        })
    };
    ($actual_params:ident, optional_thread_id($params:ident . $field:ident)) => {
        $actual_params
            .$field
            .clone()
            .map(|thread_id| ClientRequestSerializationScope::Thread { thread_id })
    };
    ($actual_params:ident, thread_or_path($params:ident . $thread_field:ident, $params2:ident . $path_field:ident)) => {
        if !$actual_params.$thread_field.is_empty() {
            Some(ClientRequestSerializationScope::Thread {
                thread_id: $actual_params.$thread_field.clone(),
            })
        } else if let Some(path) = $actual_params.$path_field.clone() {
            Some(ClientRequestSerializationScope::ThreadPath { path })
        } else {
            Some(ClientRequestSerializationScope::Thread {
                thread_id: $actual_params.$thread_field.clone(),
            })
        }
    };
    ($actual_params:ident, optional_command_process_id($params:ident . $field:ident)) => {
        $actual_params
            .$field
            .clone()
            .map(|process_id| ClientRequestSerializationScope::CommandExecProcess { process_id })
    };
    ($actual_params:ident, command_process_id($params:ident . $field:ident)) => {
        Some(ClientRequestSerializationScope::CommandExecProcess {
            process_id: $actual_params.$field.clone(),
        })
    };
    ($actual_params:ident, process_handle($params:ident . $field:ident)) => {
        Some(ClientRequestSerializationScope::Process {
            process_handle: $actual_params.$field.clone(),
        })
    };
    ($actual_params:ident, fuzzy_session_id($params:ident . $field:ident)) => {
        Some(ClientRequestSerializationScope::FuzzyFileSearchSession {
            session_id: $actual_params.$field.clone(),
        })
    };
    ($actual_params:ident, fs_watch_id($params:ident . $field:ident)) => {
        Some(ClientRequestSerializationScope::FsWatch {
            watch_id: $actual_params.$field.clone(),
        })
    };
    ($actual_params:ident, mcp_oauth_server($params:ident . $field:ident)) => {
        Some(ClientRequestSerializationScope::McpOauth {
            server_name: $actual_params.$field.clone(),
        })
    };
}

/// Generates an `enum ClientRequest` where each variant is a request that the
/// client can send to the server. Each variant has associated `params` and
/// `response` types. Also generates a `export_client_responses()` function to
/// export all response types to TypeScript.
macro_rules! client_request_definitions {
    (
        $(
            $(#[experimental($reason:expr)])?
            $(#[doc = $variant_doc:literal])*
            $variant:ident $(=> $wire:literal)? {
                params: $(#[$params_meta:meta])* $params:ty,
                $(inspect_params: $inspect_params:tt,)?
                serialization: $serialization:ident $( ( $($serialization_args:tt)* ) )?,
                $(manual_payload_conversion: $manual_payload_conversion:ident,)?
                response: $response:ty,
            }
        ),* $(,)?
    ) => {
        /// Request from the client to the server.
        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
        #[serde(tag = "method", rename_all = "camelCase")]
        pub enum ClientRequest {
            $(
                $(#[doc = $variant_doc])*
                $(#[serde(rename = $wire)] #[ts(rename = $wire)])?
                $variant {
                    #[serde(rename = "id")]
                    request_id: RequestId,
                    $(#[$params_meta])*
                    params: $params,
                },
            )*
        }

        impl ClientRequest {
            pub fn id(&self) -> &RequestId {
                match self {
                    $(Self::$variant { request_id, .. } => request_id,)*
                }
            }

            pub fn method(&self) -> String {
                serde_json::to_value(self)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("method")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "<unknown>".to_string())
            }

            pub fn serialization_scope(&self) -> Option<ClientRequestSerializationScope> {
                match self {
                    $(
                        Self::$variant { params, .. } => {
                            let _ = params;
                            serialization_scope_expr!(
                                params, $serialization $( ( $($serialization_args)* ) )?
                            )
                        }
                    )*
                }
            }
        }

        /// Typed response from the server to the client.
        #[derive(Serialize, Deserialize, Debug, Clone)]
        #[allow(clippy::large_enum_variant)]
        #[serde(tag = "method", rename_all = "camelCase")]
        pub enum ClientResponse {
            $(
                $(#[doc = $variant_doc])*
                $(#[serde(rename = $wire)])?
                $variant {
                    #[serde(rename = "id")]
                    request_id: RequestId,
                    response: $response,
                },
            )*
        }

        impl ClientResponse {
            pub fn id(&self) -> &RequestId {
                match self {
                    $(Self::$variant { request_id, .. } => request_id,)*
                }
            }

            pub fn method(&self) -> String {
                serde_json::to_value(self)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("method")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "<unknown>".to_string())
            }

            pub fn into_jsonrpc_parts(
                self,
            ) -> std::result::Result<(RequestId, crate::Result), serde_json::Error> {
                match self {
                    $(
                        Self::$variant { request_id, response } => {
                            serde_json::to_value(response).map(|result| (request_id, result))
                        }
                    )*
                }
            }
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[allow(clippy::large_enum_variant)]
        pub enum ClientResponsePayload {
            $( $variant($response), )*
            InterruptConversation(v1::InterruptConversationResponse),
        }

        impl ClientResponsePayload {
            pub fn into_jsonrpc_parts_and_payload(
                self,
                request_id: RequestId,
            ) -> std::result::Result<
                (RequestId, crate::Result, Option<ClientResponsePayload>),
                serde_json::Error,
            > {
                match self {
                    $(
                        Self::$variant(response) => {
                            let result = serde_json::to_value(&response)?;
                            Ok((request_id, result, Some(Self::$variant(response))))
                        }
                    )*
                    Self::InterruptConversation(response) => {
                        serde_json::to_value(response).map(|result| (request_id, result, None))
                    }
                }
            }

            pub fn into_client_response(self, request_id: RequestId) -> Option<ClientResponse> {
                match self {
                    $(
                        Self::$variant(response) => {
                            Some(ClientResponse::$variant {
                                request_id,
                                response,
                            })
                        }
                    )*
                    Self::InterruptConversation(_) => None,
                }
            }

            pub fn into_jsonrpc_parts(
                self,
                request_id: RequestId,
            ) -> std::result::Result<(RequestId, crate::Result), serde_json::Error> {
                self.to_jsonrpc_parts(request_id)
            }

            pub fn to_jsonrpc_parts(
                &self,
                request_id: RequestId,
            ) -> std::result::Result<(RequestId, crate::Result), serde_json::Error> {
                match self {
                    $(
                        Self::$variant(response) => {
                            serde_json::to_value(response).map(|result| (request_id, result))
                        }
                    )*
                    Self::InterruptConversation(response) => {
                        serde_json::to_value(response).map(|result| (request_id, result))
                    }
                }
            }
        }

        impl From<v1::InterruptConversationResponse> for ClientResponsePayload {
            fn from(response: v1::InterruptConversationResponse) -> Self {
                Self::InterruptConversation(response)
            }
        }

        $(
            client_response_payload_from_impl!(
                $variant,
                $response
                $(, $manual_payload_conversion)?
            );
        )*

        impl crate::experimental_api::ExperimentalApi for ClientRequest {
            fn experimental_reason(&self) -> Option<&'static str> {
                match self {
                    $(
                        Self::$variant { params: _params, .. } => {
                            experimental_reason_expr!(
                                variant $variant,
                                $(#[experimental($reason)])?
                                _params
                                $(, $inspect_params)?
                            )
                        }
                    )*
                }
            }
        }

        pub(crate) const EXPERIMENTAL_CLIENT_METHODS: &[&str] = &[
            $(
                experimental_method_entry!($(#[experimental($reason)])? $(=> $wire)?),
            )*
        ];
        pub(crate) const EXPERIMENTAL_CLIENT_METHOD_PARAM_TYPES: &[&str] = &[
            $(
                experimental_type_entry!($(#[experimental($reason)])? $params),
            )*
        ];
        pub(crate) const EXPERIMENTAL_CLIENT_METHOD_RESPONSE_TYPES: &[&str] = &[
            $(
                experimental_type_entry!($(#[experimental($reason)])? $response),
            )*
        ];

        pub fn export_client_responses(
            out_dir: &::std::path::Path,
        ) -> ::std::result::Result<(), ::ts_rs::ExportError> {
            $(
                <$response as ::ts_rs::TS>::export_all_to(out_dir)?;
            )*
            Ok(())
        }

        pub(crate) fn visit_client_response_types(v: &mut impl ::ts_rs::TypeVisitor) {
            $(
                v.visit::<$response>();
            )*
        }

        #[allow(clippy::vec_init_then_push)]
        pub fn export_client_response_schemas(
            out_dir: &::std::path::Path,
        ) -> ::anyhow::Result<Vec<GeneratedSchema>> {
            let mut schemas = Vec::new();
            $(
                schemas.push(write_json_schema::<$response>(out_dir, stringify!($response))?);
            )*
            Ok(schemas)
        }

        #[allow(clippy::vec_init_then_push)]
        pub fn export_client_param_schemas(
            out_dir: &::std::path::Path,
        ) -> ::anyhow::Result<Vec<GeneratedSchema>> {
            let mut schemas = Vec::new();
            $(
                schemas.push(write_json_schema::<$params>(out_dir, stringify!($params))?);
            )*
            Ok(schemas)
        }
    };
}

macro_rules! client_response_payload_from_impl {
    ($variant:ident, $response:ty) => {
        impl From<$response> for ClientResponsePayload {
            fn from(response: $response) -> Self {
                Self::$variant(response)
            }
        }
    };
    ($variant:ident, $response:ty, manual) => {};
}

client_request_definitions! {
    Initialize {
        params: v1::InitializeParams,
        serialization: None,
        response: v1::InitializeResponse,
    },

    /// NEW APIs
    // Thread lifecycle
    // Uses `inspect_params` because only some fields are experimental.
    ThreadStart => "thread/start" {
        params: v2::ThreadStartParams,
        inspect_params: true,
        serialization: None,
        response: v2::ThreadStartResponse,
    },
    ThreadResume => "thread/resume" {
        params: v2::ThreadResumeParams,
        inspect_params: true,
        serialization: thread_or_path(params.thread_id, params.path),
        response: v2::ThreadResumeResponse,
    },
    ThreadFork => "thread/fork" {
        params: v2::ThreadForkParams,
        inspect_params: true,
        serialization: thread_or_path(params.thread_id, params.path),
        response: v2::ThreadForkResponse,
    },
    ThreadArchive => "thread/archive" {
        params: v2::ThreadArchiveParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadArchiveResponse,
    },
    ThreadUnsubscribe => "thread/unsubscribe" {
        params: v2::ThreadUnsubscribeParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadUnsubscribeResponse,
    },
    #[experimental("thread/increment_elicitation")]
    /// Increment the thread-local out-of-band elicitation counter.
    ///
    /// This is used by external helpers to pause timeout accounting while a user
    /// approval or other elicitation is pending outside the app-server request flow.
    ThreadIncrementElicitation => "thread/increment_elicitation" {
        params: v2::ThreadIncrementElicitationParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadIncrementElicitationResponse,
    },
    #[experimental("thread/decrement_elicitation")]
    /// Decrement the thread-local out-of-band elicitation counter.
    ///
    /// When the count reaches zero, timeout accounting resumes for the thread.
    ThreadDecrementElicitation => "thread/decrement_elicitation" {
        params: v2::ThreadDecrementElicitationParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadDecrementElicitationResponse,
    },
    ThreadSetName => "thread/name/set" {
        params: v2::ThreadSetNameParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadSetNameResponse,
    },
    ThreadGoalSet => "thread/goal/set" {
        params: v2::ThreadGoalSetParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadGoalSetResponse,
    },
    ThreadGoalGet => "thread/goal/get" {
        params: v2::ThreadGoalGetParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadGoalGetResponse,
    },
    ThreadGoalClear => "thread/goal/clear" {
        params: v2::ThreadGoalClearParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadGoalClearResponse,
    },
    ThreadMetadataUpdate => "thread/metadata/update" {
        params: v2::ThreadMetadataUpdateParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadMetadataUpdateResponse,
    },
    #[experimental("thread/settings/update")]
    ThreadSettingsUpdate => "thread/settings/update" {
        params: v2::ThreadSettingsUpdateParams,
        inspect_params: true,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadSettingsUpdateResponse,
    },
    #[experimental("thread/memoryMode/set")]
    ThreadMemoryModeSet => "thread/memoryMode/set" {
        params: v2::ThreadMemoryModeSetParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadMemoryModeSetResponse,
    },
    #[experimental("memory/reset")]
    MemoryReset => "memory/reset" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("memory"),
        response: v2::MemoryResetResponse,
    },
    #[experimental("memory/status")]
    MemoryStatus => "memory/status" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("memory"),
        response: v2::MemoryStatusResponse,
    },
    ThreadUnarchive => "thread/unarchive" {
        params: v2::ThreadUnarchiveParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadUnarchiveResponse,
    },
    ThreadCompactStart => "thread/compact/start" {
        params: v2::ThreadCompactStartParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadCompactStartResponse,
    },
    ThreadShellCommand => "thread/shellCommand" {
        params: v2::ThreadShellCommandParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadShellCommandResponse,
    },
    ThreadApproveGuardianDeniedAction => "thread/approveGuardianDeniedAction" {
        params: v2::ThreadApproveGuardianDeniedActionParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadApproveGuardianDeniedActionResponse,
    },
    #[experimental("thread/backgroundTerminals/clean")]
    ThreadBackgroundTerminalsClean => "thread/backgroundTerminals/clean" {
        params: v2::ThreadBackgroundTerminalsCleanParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadBackgroundTerminalsCleanResponse,
    },
    ThreadRollback => "thread/rollback" {
        params: v2::ThreadRollbackParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadRollbackResponse,
    },
    ThreadList => "thread/list" {
        params: v2::ThreadListParams,
        serialization: None,
        response: v2::ThreadListResponse,
    },
    #[experimental("thread/search")]
    ThreadSearch => "thread/search" {
        params: v2::ThreadSearchParams,
        serialization: None,
        response: v2::ThreadSearchResponse,
    },
    ThreadLoadedList => "thread/loaded/list" {
        params: v2::ThreadLoadedListParams,
        serialization: None,
        response: v2::ThreadLoadedListResponse,
    },
    ThreadRead => "thread/read" {
        params: v2::ThreadReadParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadReadResponse,
    },
    #[experimental("thread/turns/list")]
    ThreadTurnsList => "thread/turns/list" {
        params: v2::ThreadTurnsListParams,
        // Explicitly concurrent: this primarily reads append-only rollout storage.
        serialization: None,
        response: v2::ThreadTurnsListResponse,
    },
    #[experimental("thread/turns/items/list")]
    ThreadTurnsItemsList => "thread/turns/items/list" {
        params: v2::ThreadTurnsItemsListParams,
        // Explicitly concurrent: this primarily reads append-only rollout storage.
        serialization: None,
        response: v2::ThreadTurnsItemsListResponse,
    },
    /// Append raw Responses API items to the thread history without starting a user turn.
    ThreadInjectItems => "thread/inject_items" {
        params: v2::ThreadInjectItemsParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadInjectItemsResponse,
    },
    SkillsList => "skills/list" {
        params: v2::SkillsListParams,
        serialization: global_shared_read("config"),
        response: v2::SkillsListResponse,
    },
    SkillsExtraRootsSet => "skills/extraRoots/set" {
        params: v2::SkillsExtraRootsSetParams,
        serialization: global("config"),
        response: v2::SkillsExtraRootsSetResponse,
    },
    HooksList => "hooks/list" {
        params: v2::HooksListParams,
        serialization: global("config"),
        response: v2::HooksListResponse,
    },
    MarketplaceAdd => "marketplace/add" {
        params: v2::MarketplaceAddParams,
        serialization: global("config"),
        response: v2::MarketplaceAddResponse,
    },
    MarketplaceRemove => "marketplace/remove" {
        params: v2::MarketplaceRemoveParams,
        serialization: global("config"),
        response: v2::MarketplaceRemoveResponse,
    },
    MarketplaceUpgrade => "marketplace/upgrade" {
        params: v2::MarketplaceUpgradeParams,
        serialization: global("config"),
        response: v2::MarketplaceUpgradeResponse,
    },
    PluginList => "plugin/list" {
        params: v2::PluginListParams,
        serialization: None,
        response: v2::PluginListResponse,
    },
    PluginInstalled => "plugin/installed" {
        params: v2::PluginInstalledParams,
        serialization: None,
        response: v2::PluginInstalledResponse,
    },
    PluginRead => "plugin/read" {
        params: v2::PluginReadParams,
        serialization: None,
        response: v2::PluginReadResponse,
    },
    PluginSkillRead => "plugin/skill/read" {
        params: v2::PluginSkillReadParams,
        serialization: global("config"),
        response: v2::PluginSkillReadResponse,
    },
    PluginShareSave => "plugin/share/save" {
        params: v2::PluginShareSaveParams,
        serialization: global("config"),
        response: v2::PluginShareSaveResponse,
    },
    PluginShareUpdateTargets => "plugin/share/updateTargets" {
        params: v2::PluginShareUpdateTargetsParams,
        serialization: global("config"),
        response: v2::PluginShareUpdateTargetsResponse,
    },
    PluginShareList => "plugin/share/list" {
        params: v2::PluginShareListParams,
        serialization: global("config"),
        response: v2::PluginShareListResponse,
    },
    PluginShareCheckout => "plugin/share/checkout" {
        params: v2::PluginShareCheckoutParams,
        serialization: global("config"),
        response: v2::PluginShareCheckoutResponse,
    },
    PluginShareDelete => "plugin/share/delete" {
        params: v2::PluginShareDeleteParams,
        serialization: global("config"),
        response: v2::PluginShareDeleteResponse,
    },
    AppsList => "app/list" {
        params: v2::AppsListParams,
        serialization: None,
        response: v2::AppsListResponse,
    },
    // File system requests are intentionally concurrent. Desktop already treats local
    // file system operations as concurrent, and app-server remote fs mirrors that model.
    FsReadFile => "fs/readFile" {
        params: v2::FsReadFileParams,
        serialization: None,
        response: v2::FsReadFileResponse,
    },
    FsWriteFile => "fs/writeFile" {
        params: v2::FsWriteFileParams,
        serialization: None,
        response: v2::FsWriteFileResponse,
    },
    FsCreateDirectory => "fs/createDirectory" {
        params: v2::FsCreateDirectoryParams,
        serialization: None,
        response: v2::FsCreateDirectoryResponse,
    },
    FsGetMetadata => "fs/getMetadata" {
        params: v2::FsGetMetadataParams,
        serialization: None,
        response: v2::FsGetMetadataResponse,
    },
    FsReadDirectory => "fs/readDirectory" {
        params: v2::FsReadDirectoryParams,
        serialization: None,
        response: v2::FsReadDirectoryResponse,
    },
    FsRemove => "fs/remove" {
        params: v2::FsRemoveParams,
        serialization: None,
        response: v2::FsRemoveResponse,
    },
    FsCopy => "fs/copy" {
        params: v2::FsCopyParams,
        serialization: None,
        response: v2::FsCopyResponse,
    },
    FsWatch => "fs/watch" {
        params: v2::FsWatchParams,
        serialization: fs_watch_id(params.watch_id),
        response: v2::FsWatchResponse,
    },
    FsUnwatch => "fs/unwatch" {
        params: v2::FsUnwatchParams,
        serialization: fs_watch_id(params.watch_id),
        response: v2::FsUnwatchResponse,
    },
    SkillsConfigWrite => "skills/config/write" {
        params: v2::SkillsConfigWriteParams,
        serialization: global("config"),
        response: v2::SkillsConfigWriteResponse,
    },
    PluginInstall => "plugin/install" {
        params: v2::PluginInstallParams,
        serialization: global("config"),
        response: v2::PluginInstallResponse,
    },
    PluginUninstall => "plugin/uninstall" {
        params: v2::PluginUninstallParams,
        serialization: global("config"),
        response: v2::PluginUninstallResponse,
    },
    TurnStart => "turn/start" {
        params: v2::TurnStartParams,
        inspect_params: true,
        serialization: thread_id(params.thread_id),
        response: v2::TurnStartResponse,
    },
    TurnSteer => "turn/steer" {
        params: v2::TurnSteerParams,
        inspect_params: true,
        serialization: thread_id(params.thread_id),
        response: v2::TurnSteerResponse,
    },
    TurnInterrupt => "turn/interrupt" {
        params: v2::TurnInterruptParams,
        serialization: thread_id(params.thread_id),
        response: v2::TurnInterruptResponse,
    },
    #[experimental("thread/realtime/start")]
    ThreadRealtimeStart => "thread/realtime/start" {
        params: v2::ThreadRealtimeStartParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadRealtimeStartResponse,
    },
    #[experimental("thread/realtime/appendAudio")]
    ThreadRealtimeAppendAudio => "thread/realtime/appendAudio" {
        params: v2::ThreadRealtimeAppendAudioParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadRealtimeAppendAudioResponse,
    },
    #[experimental("thread/realtime/appendText")]
    ThreadRealtimeAppendText => "thread/realtime/appendText" {
        params: v2::ThreadRealtimeAppendTextParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadRealtimeAppendTextResponse,
    },
    #[experimental("thread/realtime/stop")]
    ThreadRealtimeStop => "thread/realtime/stop" {
        params: v2::ThreadRealtimeStopParams,
        serialization: thread_id(params.thread_id),
        response: v2::ThreadRealtimeStopResponse,
    },
    #[experimental("thread/realtime/listVoices")]
    ThreadRealtimeListVoices => "thread/realtime/listVoices" {
        params: v2::ThreadRealtimeListVoicesParams,
        serialization: None,
        response: v2::ThreadRealtimeListVoicesResponse,
    },
    ReviewStart => "review/start" {
        params: v2::ReviewStartParams,
        serialization: thread_id(params.thread_id),
        response: v2::ReviewStartResponse,
    },

    ModelList => "model/list" {
        params: v2::ModelListParams,
        serialization: None,
        response: v2::ModelListResponse,
    },
    ModelProviderCapabilitiesRead => "modelProvider/capabilities/read" {
        params: v2::ModelProviderCapabilitiesReadParams,
        serialization: None,
        response: v2::ModelProviderCapabilitiesReadResponse,
    },
    ExperimentalFeatureList => "experimentalFeature/list" {
        params: v2::ExperimentalFeatureListParams,
        serialization: global("config"),
        response: v2::ExperimentalFeatureListResponse,
    },
    PermissionProfileList => "permissionProfile/list" {
        params: v2::PermissionProfileListParams,
        serialization: global_shared_read("config"),
        response: v2::PermissionProfileListResponse,
    },
    ExperimentalFeatureEnablementSet => "experimentalFeature/enablement/set" {
        params: v2::ExperimentalFeatureEnablementSetParams,
        serialization: global("config"),
        response: v2::ExperimentalFeatureEnablementSetResponse,
    },
    #[experimental("remoteControl/enable")]
    RemoteControlEnable => "remoteControl/enable" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("remote-control"),
        response: v2::RemoteControlEnableResponse,
    },
    #[experimental("remoteControl/disable")]
    RemoteControlDisable => "remoteControl/disable" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("remote-control"),
        response: v2::RemoteControlDisableResponse,
    },
    #[experimental("remoteControl/status/read")]
    RemoteControlStatusRead => "remoteControl/status/read" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global_shared_read("remote-control"),
        response: v2::RemoteControlStatusReadResponse,
    },
    #[experimental("remoteControl/pairing/start")]
    RemoteControlPairingStart => "remoteControl/pairing/start" {
        params: v2::RemoteControlPairingStartParams,
        serialization: global("remote-control-pairing"),
        response: v2::RemoteControlPairingStartResponse,
    },
    #[experimental("remoteControl/pairing/status")]
    RemoteControlPairingStatus => "remoteControl/pairing/status" {
        params: v2::RemoteControlPairingStatusParams,
        serialization: global_shared_read("remote-control-pairing"),
        response: v2::RemoteControlPairingStatusResponse,
    },
    #[experimental("remoteControl/client/list")]
    RemoteControlClientsList => "remoteControl/client/list" {
        params: v2::RemoteControlClientsListParams,
        serialization: global_shared_read("remote-control-clients"),
        response: v2::RemoteControlClientsListResponse,
    },
    #[experimental("remoteControl/client/revoke")]
    RemoteControlClientsRevoke => "remoteControl/client/revoke" {
        params: v2::RemoteControlClientsRevokeParams,
        serialization: global("remote-control-clients"),
        response: v2::RemoteControlClientsRevokeResponse,
    },
    #[experimental("collaborationMode/list")]
    /// Lists collaboration mode presets.
    CollaborationModeList => "collaborationMode/list" {
        params: v2::CollaborationModeListParams,
        serialization: None,
        response: v2::CollaborationModeListResponse,
    },
    #[experimental("mock/experimentalMethod")]
    /// Test-only method used to validate experimental gating.
    MockExperimentalMethod => "mock/experimentalMethod" {
        params: v2::MockExperimentalMethodParams,
        serialization: None,
        response: v2::MockExperimentalMethodResponse,
    },
    #[experimental("environment/add")]
    /// Adds or replaces a remote environment by id for later selection.
    EnvironmentAdd => "environment/add" {
        params: v2::EnvironmentAddParams,
        serialization: global("environment"),
        response: v2::EnvironmentAddResponse,
    },

    McpServerOauthLogin => "mcpServer/oauth/login" {
        params: v2::McpServerOauthLoginParams,
        serialization: mcp_oauth_server(params.name),
        response: v2::McpServerOauthLoginResponse,
    },

    McpServerRefresh => "config/mcpServer/reload" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("mcp-registry"),
        response: v2::McpServerRefreshResponse,
    },

    McpServerStatusList => "mcpServerStatus/list" {
        params: v2::ListMcpServerStatusParams,
        serialization: global("mcp-registry"),
        response: v2::ListMcpServerStatusResponse,
    },
    #[experimental("mcp/cache/status")]
    McpCacheStatus => "mcp/cache/status" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("mcp-registry"),
        response: v2::McpCacheStatusResponse,
    },

    McpResourceRead => "mcpServer/resource/read" {
        params: v2::McpResourceReadParams,
        serialization: optional_thread_id(params.thread_id),
        response: v2::McpResourceReadResponse,
    },

    McpServerToolCall => "mcpServer/tool/call" {
        params: v2::McpServerToolCallParams,
        serialization: thread_id(params.thread_id),
        response: v2::McpServerToolCallResponse,
    },

    WindowsSandboxSetupStart => "windowsSandbox/setupStart" {
        params: v2::WindowsSandboxSetupStartParams,
        serialization: global("windows-sandbox-setup"),
        response: v2::WindowsSandboxSetupStartResponse,
    },
    WindowsSandboxReadiness => "windowsSandbox/readiness" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("config"),
        response: v2::WindowsSandboxReadinessResponse,
    },

    LoginAccount => "account/login/start" {
        params: v2::LoginAccountParams,
        inspect_params: true,
        serialization: global("account-auth"),
        response: v2::LoginAccountResponse,
    },

    CancelLoginAccount => "account/login/cancel" {
        params: v2::CancelLoginAccountParams,
        serialization: global("account-auth"),
        response: v2::CancelLoginAccountResponse,
    },

    LogoutAccount => "account/logout" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("account-auth"),
        response: v2::LogoutAccountResponse,
    },

    GetAccountRateLimits => "account/rateLimits/read" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: None,
        response: v2::GetAccountRateLimitsResponse,
    },

    GetAccountTokenUsage => "account/usage/read" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: None,
        response: v2::GetAccountTokenUsageResponse,
    },

    SendAddCreditsNudgeEmail => "account/sendAddCreditsNudgeEmail" {
        params: v2::SendAddCreditsNudgeEmailParams,
        serialization: global("account-auth"),
        response: v2::SendAddCreditsNudgeEmailResponse,
    },

    FeedbackUpload => "feedback/upload" {
        params: v2::FeedbackUploadParams,
        serialization: None,
        response: v2::FeedbackUploadResponse,
    },

    /// Execute a standalone command (argv vector) under the server's sandbox.
    OneOffCommandExec => "command/exec" {
        params: v2::CommandExecParams,
        inspect_params: true,
        serialization: optional_command_process_id(params.process_id),
        response: v2::CommandExecResponse,
    },
    /// Write stdin bytes to a running `command/exec` session or close stdin.
    CommandExecWrite => "command/exec/write" {
        params: v2::CommandExecWriteParams,
        serialization: command_process_id(params.process_id),
        response: v2::CommandExecWriteResponse,
    },
    /// Terminate a running `command/exec` session by client-supplied `processId`.
    CommandExecTerminate => "command/exec/terminate" {
        params: v2::CommandExecTerminateParams,
        serialization: command_process_id(params.process_id),
        response: v2::CommandExecTerminateResponse,
    },
    /// Resize a running PTY-backed `command/exec` session by client-supplied `processId`.
    CommandExecResize => "command/exec/resize" {
        params: v2::CommandExecResizeParams,
        serialization: command_process_id(params.process_id),
        response: v2::CommandExecResizeResponse,
    },
    #[experimental("process/spawn")]
    /// Spawn a standalone process (argv vector) without a Codex sandbox.
    ProcessSpawn => "process/spawn" {
        params: v2::ProcessSpawnParams,
        serialization: process_handle(params.process_handle),
        response: v2::ProcessSpawnResponse,
    },
    #[experimental("process/writeStdin")]
    /// Write stdin bytes to a running `process/spawn` session or close stdin.
    ProcessWriteStdin => "process/writeStdin" {
        params: v2::ProcessWriteStdinParams,
        serialization: process_handle(params.process_handle),
        response: v2::ProcessWriteStdinResponse,
    },
    #[experimental("process/kill")]
    /// Terminate a running `process/spawn` session by client-supplied `processHandle`.
    ProcessKill => "process/kill" {
        params: v2::ProcessKillParams,
        serialization: process_handle(params.process_handle),
        response: v2::ProcessKillResponse,
    },
    #[experimental("process/resizePty")]
    /// Resize a running PTY-backed `process/spawn` session by client-supplied `processHandle`.
    ProcessResizePty => "process/resizePty" {
        params: v2::ProcessResizePtyParams,
        serialization: process_handle(params.process_handle),
        response: v2::ProcessResizePtyResponse,
    },

    ConfigRead => "config/read" {
        params: v2::ConfigReadParams,
        serialization: global_shared_read("config"),
        response: v2::ConfigReadResponse,
    },
    ExternalAgentConfigDetect => "externalAgentConfig/detect" {
        params: v2::ExternalAgentConfigDetectParams,
        serialization: global("config"),
        response: v2::ExternalAgentConfigDetectResponse,
    },
    ExternalAgentConfigImport => "externalAgentConfig/import" {
        params: v2::ExternalAgentConfigImportParams,
        serialization: global("config"),
        response: v2::ExternalAgentConfigImportResponse,
    },
    ConfigValueWrite => "config/value/write" {
        params: v2::ConfigValueWriteParams,
        serialization: global("config"),
        manual_payload_conversion: manual,
        response: v2::ConfigWriteResponse,
    },
    ConfigBatchWrite => "config/batchWrite" {
        params: v2::ConfigBatchWriteParams,
        serialization: global("config"),
        manual_payload_conversion: manual,
        response: v2::ConfigWriteResponse,
    },

    ConfigRequirementsRead => "configRequirements/read" {
        params: #[ts(type = "undefined")] #[serde(skip_serializing_if = "Option::is_none")] Option<()>,
        serialization: global("config"),
        response: v2::ConfigRequirementsReadResponse,
    },

    GetAccount => "account/read" {
        params: v2::GetAccountParams,
        serialization: global("account-auth"),
        response: v2::GetAccountResponse,
    },

    /// DEPRECATED APIs below
    GetConversationSummary {
        params: v1::GetConversationSummaryParams,
        serialization: None,
        response: v1::GetConversationSummaryResponse,
    },
    GitDiffToRemote {
        params: v1::GitDiffToRemoteParams,
        serialization: None,
        response: v1::GitDiffToRemoteResponse,
    },
    /// DEPRECATED in favor of GetAccount
    GetAuthStatus {
        params: v1::GetAuthStatusParams,
        serialization: global("account-auth"),
        response: v1::GetAuthStatusResponse,
    },
    // Legacy fuzzy search cancellation is intentionally concurrent: clients reuse a
    // cancellation token so a newer request can cancel an older in-flight search.
    FuzzyFileSearch {
        params: FuzzyFileSearchParams,
        serialization: None,
        response: FuzzyFileSearchResponse,
    },
    #[experimental("fuzzyFileSearch/sessionStart")]
    FuzzyFileSearchSessionStart => "fuzzyFileSearch/sessionStart" {
        params: FuzzyFileSearchSessionStartParams,
        serialization: fuzzy_session_id(params.session_id),
        response: FuzzyFileSearchSessionStartResponse,
    },
    #[experimental("fuzzyFileSearch/sessionUpdate")]
    FuzzyFileSearchSessionUpdate => "fuzzyFileSearch/sessionUpdate" {
        params: FuzzyFileSearchSessionUpdateParams,
        serialization: fuzzy_session_id(params.session_id),
        response: FuzzyFileSearchSessionUpdateResponse,
    },
    #[experimental("fuzzyFileSearch/sessionStop")]
    FuzzyFileSearchSessionStop => "fuzzyFileSearch/sessionStop" {
        params: FuzzyFileSearchSessionStopParams,
        serialization: fuzzy_session_id(params.session_id),
        response: FuzzyFileSearchSessionStopResponse,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use codex_protocol::ThreadId;
    use codex_protocol::account::PlanType;
    use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_READ_ONLY;
    use codex_protocol::protocol::RealtimeOutputModality;
    use codex_protocol::protocol::RealtimeVoice;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use codex_utils_absolute_path::test_support::PathBufExt;
    use codex_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::path::PathBuf;

    fn absolute_path_string(path: &str) -> String {
        let path = format!("/{}", path.trim_start_matches('/'));
        test_path_buf(&path).display().to_string()
    }

    fn absolute_path(path: &str) -> AbsolutePathBuf {
        let path = format!("/{}", path.trim_start_matches('/'));
        test_path_buf(&path).abs()
    }

    fn request_id() -> RequestId {
        const REQUEST_ID: i64 = 1;
        RequestId::Integer(REQUEST_ID)
    }

    #[test]
    fn client_request_serialization_scope_covers_keyed_families() {
        let thread_id = "thread-1".to_string();
        let thread_resume = ClientRequest::ThreadResume {
            request_id: request_id(),
            params: v2::ThreadResumeParams {
                thread_id: thread_id.clone(),
                ..Default::default()
            },
        };
        assert_eq!(
            thread_resume.serialization_scope(),
            Some(ClientRequestSerializationScope::Thread {
                thread_id: thread_id.clone()
            })
        );

        let thread_resume_with_path = ClientRequest::ThreadResume {
            request_id: request_id(),
            params: v2::ThreadResumeParams {
                thread_id: thread_id.clone(),
                path: Some(PathBuf::from("/tmp/resume-thread.jsonl")),
                ..Default::default()
            },
        };
        assert_eq!(
            thread_resume_with_path.serialization_scope(),
            Some(ClientRequestSerializationScope::Thread {
                thread_id: thread_id.clone()
            })
        );

        let thread_fork = ClientRequest::ThreadFork {
            request_id: request_id(),
            params: v2::ThreadForkParams {
                thread_id: thread_id.clone(),
                path: Some(PathBuf::from("/tmp/source-thread.jsonl")),
                ..Default::default()
            },
        };
        assert_eq!(
            thread_fork.serialization_scope(),
            Some(ClientRequestSerializationScope::Thread { thread_id })
        );

        let command_exec = ClientRequest::OneOffCommandExec {
            request_id: request_id(),
            params: v2::CommandExecParams {
                command: vec!["sleep".to_string(), "10".to_string()],
                process_id: Some("proc-1".to_string()),
                tty: false,
                stream_stdin: false,
                stream_stdout_stderr: false,
                output_bytes_cap: None,
                disable_output_cap: false,
                disable_timeout: false,
                timeout_ms: None,
                cwd: None,
                env: None,
                size: None,
                sandbox_policy: None,
                permission_profile: None,
            },
        };
        assert_eq!(
            command_exec.serialization_scope(),
            Some(ClientRequestSerializationScope::CommandExecProcess {
                process_id: "proc-1".to_string()
            })
        );

        let fuzzy_update = ClientRequest::FuzzyFileSearchSessionUpdate {
            request_id: request_id(),
            params: FuzzyFileSearchSessionUpdateParams {
                session_id: "search-1".to_string(),
                query: "lib".to_string(),
            },
        };
        assert_eq!(
            fuzzy_update.serialization_scope(),
            Some(ClientRequestSerializationScope::FuzzyFileSearchSession {
                session_id: "search-1".to_string()
            })
        );

        let fs_watch = ClientRequest::FsWatch {
            request_id: request_id(),
            params: v2::FsWatchParams {
                watch_id: "watch-1".to_string(),
                path: absolute_path("/tmp/repo"),
            },
        };
        assert_eq!(
            fs_watch.serialization_scope(),
            Some(ClientRequestSerializationScope::FsWatch {
                watch_id: "watch-1".to_string()
            })
        );

        let plugin_install = ClientRequest::PluginInstall {
            request_id: request_id(),
            params: v2::PluginInstallParams {
                marketplace_path: Some(absolute_path("/tmp/marketplace")),
                remote_marketplace_name: None,
                plugin_name: "plugin-a".to_string(),
            },
        };
        assert_eq!(
            plugin_install.serialization_scope(),
            Some(ClientRequestSerializationScope::Global("config"))
        );

        let skills_list = ClientRequest::SkillsList {
            request_id: request_id(),
            params: v2::SkillsListParams {
                cwds: Vec::new(),
                force_reload: false,
            },
        };
        assert_eq!(
            skills_list.serialization_scope(),
            Some(ClientRequestSerializationScope::GlobalSharedRead("config"))
        );

        let skills_extra_roots_set = ClientRequest::SkillsExtraRootsSet {
            request_id: request_id(),
            params: v2::SkillsExtraRootsSetParams {
                extra_roots: vec![absolute_path("/tmp/skills")],
            },
        };
        assert_eq!(
            skills_extra_roots_set.serialization_scope(),
            Some(ClientRequestSerializationScope::Global("config"))
        );

        let plugin_list = ClientRequest::PluginList {
            request_id: request_id(),
            params: v2::PluginListParams {
                cwds: None,
                marketplace_kinds: None,
            },
        };
        assert_eq!(plugin_list.serialization_scope(), None);

        let plugin_read = ClientRequest::PluginRead {
            request_id: request_id(),
            params: v2::PluginReadParams {
                marketplace_path: Some(absolute_path("/tmp/marketplace")),
                remote_marketplace_name: None,
                plugin_name: "plugin-a".to_string(),
            },
        };
        assert_eq!(plugin_read.serialization_scope(), None);

        let plugin_installed = ClientRequest::PluginInstalled {
            request_id: request_id(),
            params: v2::PluginInstalledParams {
                cwds: None,
                install_suggestion_plugin_names: None,
            },
        };
        assert_eq!(plugin_installed.serialization_scope(), None);

        let plugin_uninstall = ClientRequest::PluginUninstall {
            request_id: request_id(),
            params: v2::PluginUninstallParams {
                plugin_id: "plugin-a".to_string(),
            },
        };
        assert_eq!(
            plugin_uninstall.serialization_scope(),
            Some(ClientRequestSerializationScope::Global("config"))
        );

        let mcp_oauth = ClientRequest::McpServerOauthLogin {
            request_id: request_id(),
            params: v2::McpServerOauthLoginParams {
                name: "server-a".to_string(),
                scopes: None,
                timeout_secs: None,
            },
        };
        assert_eq!(
            mcp_oauth.serialization_scope(),
            Some(ClientRequestSerializationScope::McpOauth {
                server_name: "server-a".to_string()
            })
        );

        let mcp_resource_read = ClientRequest::McpResourceRead {
            request_id: request_id(),
            params: v2::McpResourceReadParams {
                thread_id: Some("thread-1".to_string()),
                server: "server-a".to_string(),
                uri: "file:///tmp/resource".to_string(),
            },
        };
        assert_eq!(
            mcp_resource_read.serialization_scope(),
            Some(ClientRequestSerializationScope::Thread {
                thread_id: "thread-1".to_string()
            })
        );

        let config_read = ClientRequest::ConfigRead {
            request_id: request_id(),
            params: v2::ConfigReadParams {
                include_layers: false,
                cwd: None,
            },
        };
        assert_eq!(
            config_read.serialization_scope(),
            Some(ClientRequestSerializationScope::GlobalSharedRead("config"))
        );

        let account_read = ClientRequest::GetAccount {
            request_id: request_id(),
            params: v2::GetAccountParams {
                refresh_token: false,
            },
        };
        assert_eq!(
            account_read.serialization_scope(),
            Some(ClientRequestSerializationScope::Global("account-auth"))
        );

        let thread_goal_set = ClientRequest::ThreadGoalSet {
            request_id: request_id(),
            params: v2::ThreadGoalSetParams {
                thread_id: "goal-thread".to_string(),
                objective: Some("ship it".to_string()),
                status: None,
                token_budget: None,
            },
        };
        assert_eq!(
            thread_goal_set.serialization_scope(),
            Some(ClientRequestSerializationScope::Thread {
                thread_id: "goal-thread".to_string()
            })
        );

        let guardian_approval = ClientRequest::ThreadApproveGuardianDeniedAction {
            request_id: request_id(),
            params: v2::ThreadApproveGuardianDeniedActionParams {
                thread_id: "guardian-thread".to_string(),
                event: json!({ "type": "guardian" }),
            },
        };
        assert_eq!(
            guardian_approval.serialization_scope(),
            Some(ClientRequestSerializationScope::Thread {
                thread_id: "guardian-thread".to_string()
            })
        );

        let marketplace_remove = ClientRequest::MarketplaceRemove {
            request_id: request_id(),
            params: v2::MarketplaceRemoveParams {
                marketplace_name: "marketplace".to_string(),
            },
        };
        assert_eq!(
            marketplace_remove.serialization_scope(),
            Some(ClientRequestSerializationScope::Global("config"))
        );

        let add_credits_nudge = ClientRequest::SendAddCreditsNudgeEmail {
            request_id: request_id(),
            params: v2::SendAddCreditsNudgeEmailParams {
                credit_type: v2::AddCreditsNudgeCreditType::Credits,
            },
        };
        assert_eq!(
            add_credits_nudge.serialization_scope(),
            Some(ClientRequestSerializationScope::Global("account-auth"))
        );

        let environment_add = ClientRequest::EnvironmentAdd {
            request_id: request_id(),
            params: v2::EnvironmentAddParams {
                environment_id: "remote-a".to_string(),
                exec_server_url: "ws://127.0.0.1:8765".to_string(),
            },
        };
        assert_eq!(
            environment_add.serialization_scope(),
            Some(ClientRequestSerializationScope::Global("environment"))
        );
    }

    #[test]
    fn client_request_serialization_scope_covers_unkeyed_representatives() {
        let initialize = ClientRequest::Initialize {
            request_id: request_id(),
            params: v1::InitializeParams {
                client_info: v1::ClientInfo {
                    name: "test".to_string(),
                    title: None,
                    version: "0.1.0".to_string(),
                },
                capabilities: None,
            },
        };
        assert_eq!(initialize.serialization_scope(), None);

        let thread_start = ClientRequest::ThreadStart {
            request_id: request_id(),
            params: v2::ThreadStartParams::default(),
        };
        assert_eq!(thread_start.serialization_scope(), None);

        let command_exec = ClientRequest::OneOffCommandExec {
            request_id: request_id(),
            params: v2::CommandExecParams {
                command: vec!["true".to_string()],
                process_id: None,
                tty: false,
                stream_stdin: false,
                stream_stdout_stderr: false,
                output_bytes_cap: None,
                disable_output_cap: false,
                disable_timeout: false,
                timeout_ms: None,
                cwd: None,
                env: None,
                size: None,
                sandbox_policy: None,
                permission_profile: None,
            },
        };
        assert_eq!(command_exec.serialization_scope(), None);

        let fs_read = ClientRequest::FsReadFile {
            request_id: request_id(),
            params: v2::FsReadFileParams {
                path: absolute_path("/tmp/file.txt"),
            },
        };
        assert_eq!(fs_read.serialization_scope(), None);

        let thread_turns_list = ClientRequest::ThreadTurnsList {
            request_id: request_id(),
            params: v2::ThreadTurnsListParams {
                thread_id: "thread-1".to_string(),
                cursor: None,
                limit: None,
                sort_direction: None,
                items_view: None,
            },
        };
        assert_eq!(thread_turns_list.serialization_scope(), None);

        let thread_turns_items_list = ClientRequest::ThreadTurnsItemsList {
            request_id: request_id(),
            params: v2::ThreadTurnsItemsListParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                cursor: None,
                limit: None,
                sort_direction: None,
            },
        };
        assert_eq!(thread_turns_items_list.serialization_scope(), None);

        let mcp_resource_read = ClientRequest::McpResourceRead {
            request_id: request_id(),
            params: v2::McpResourceReadParams {
                thread_id: None,
                server: "server-a".to_string(),
                uri: "file:///tmp/resource".to_string(),
            },
        };
        assert_eq!(mcp_resource_read.serialization_scope(), None);

        let remote_control_pairing_start = ClientRequest::RemoteControlPairingStart {
            request_id: request_id(),
            params: v2::RemoteControlPairingStartParams::default(),
        };
        assert_eq!(
            remote_control_pairing_start.serialization_scope(),
            Some(ClientRequestSerializationScope::Global(
                "remote-control-pairing"
            ))
        );
        let remote_control_clients_list = ClientRequest::RemoteControlClientsList {
            request_id: request_id(),
            params: v2::RemoteControlClientsListParams::default(),
        };
        assert_eq!(
            remote_control_clients_list.serialization_scope(),
            Some(ClientRequestSerializationScope::GlobalSharedRead(
                "remote-control-clients"
            ))
        );
        let remote_control_clients_revoke = ClientRequest::RemoteControlClientsRevoke {
            request_id: request_id(),
            params: v2::RemoteControlClientsRevokeParams {
                environment_id: "environment-id".to_string(),
                client_id: "client-id".to_string(),
            },
        };
        assert_eq!(
            remote_control_clients_revoke.serialization_scope(),
            Some(ClientRequestSerializationScope::Global(
                "remote-control-clients"
            ))
        );
    }

    #[test]
    fn serialize_get_conversation_summary() -> Result<()> {
        let request = ClientRequest::GetConversationSummary {
            request_id: RequestId::Integer(42),
            params: v1::GetConversationSummaryParams::ThreadId {
                conversation_id: ThreadId::from_string("67e55044-10b1-426f-9247-bb680e5fe0c8")?,
            },
        };
        assert_eq!(
            json!({
                "method": "getConversationSummary",
                "id": 42,
                "params": {
                    "conversationId": "67e55044-10b1-426f-9247-bb680e5fe0c8"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_initialize_with_opt_out_notification_methods() -> Result<()> {
        let request = ClientRequest::Initialize {
            request_id: RequestId::Integer(42),
            params: v1::InitializeParams {
                client_info: v1::ClientInfo {
                    name: "codex_vscode".to_string(),
                    title: Some("Codex VS Code Extension".to_string()),
                    version: "0.1.0".to_string(),
                },
                capabilities: Some(v1::InitializeCapabilities {
                    experimental_api: true,
                    request_attestation: true,
                    opt_out_notification_methods: Some(vec![
                        "thread/started".to_string(),
                        "item/agentMessage/delta".to_string(),
                    ]),
                }),
            },
        };

        assert_eq!(
            json!({
                "method": "initialize",
                "id": 42,
                "params": {
                    "clientInfo": {
                        "name": "codex_vscode",
                        "title": "Codex VS Code Extension",
                        "version": "0.1.0"
                    },
                    "capabilities": {
                        "experimentalApi": true,
                        "requestAttestation": true,
                        "optOutNotificationMethods": [
                            "thread/started",
                            "item/agentMessage/delta"
                        ]
                    }
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn deserialize_initialize_with_opt_out_notification_methods() -> Result<()> {
        let request: ClientRequest = serde_json::from_value(json!({
            "method": "initialize",
            "id": 42,
            "params": {
                "clientInfo": {
                    "name": "codex_vscode",
                    "title": "Codex VS Code Extension",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "experimentalApi": true,
                    "requestAttestation": true,
                    "optOutNotificationMethods": [
                        "thread/started",
                        "item/agentMessage/delta"
                    ]
                }
            }
        }))?;

        assert_eq!(
            request,
            ClientRequest::Initialize {
                request_id: RequestId::Integer(42),
                params: v1::InitializeParams {
                    client_info: v1::ClientInfo {
                        name: "codex_vscode".to_string(),
                        title: Some("Codex VS Code Extension".to_string()),
                        version: "0.1.0".to_string(),
                    },
                    capabilities: Some(v1::InitializeCapabilities {
                        experimental_api: true,
                        request_attestation: true,
                        opt_out_notification_methods: Some(vec![
                            "thread/started".to_string(),
                            "item/agentMessage/delta".to_string(),
                        ]),
                    }),
                },
            }
        );
        Ok(())
    }

    #[test]
    fn conversation_id_serializes_as_plain_string() -> Result<()> {
        let id = ThreadId::from_string("67e55044-10b1-426f-9247-bb680e5fe0c8")?;

        assert_eq!(
            json!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            serde_json::to_value(id)?
        );
        Ok(())
    }

    #[test]
    fn conversation_id_deserializes_from_plain_string() -> Result<()> {
        let id: ThreadId = serde_json::from_value(json!("67e55044-10b1-426f-9247-bb680e5fe0c8"))?;

        assert_eq!(
            ThreadId::from_string("67e55044-10b1-426f-9247-bb680e5fe0c8")?,
            id,
        );
        Ok(())
    }

    #[test]
    fn serialize_get_account_rate_limits() -> Result<()> {
        let request = ClientRequest::GetAccountRateLimits {
            request_id: RequestId::Integer(1),
            params: None,
        };
        assert_eq!(request.id(), &RequestId::Integer(1));
        assert_eq!(request.method(), "account/rateLimits/read");
        assert_eq!(
            json!({
                "method": "account/rateLimits/read",
                "id": 1,
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_client_response() -> Result<()> {
        let cwd = absolute_path("/tmp");
        let response = ClientResponse::ThreadStart {
            request_id: RequestId::Integer(7),
            response: v2::ThreadStartResponse {
                thread: v2::Thread {
                    id: "67e55044-10b1-426f-9247-bb680e5fe0c8".to_string(),
                    session_id: "67e55044-10b1-426f-9247-bb680e5fe0c7".to_string(),
                    forked_from_id: None,
                    preview: "first prompt".to_string(),
                    ephemeral: true,
                    model_provider: "openai".to_string(),
                    created_at: 1,
                    updated_at: 2,
                    status: v2::ThreadStatus::Idle,
                    path: None,
                    cwd: cwd.clone(),
                    cli_version: "0.0.0".to_string(),
                    source: v2::SessionSource::Exec,
                    thread_source: None,
                    agent_nickname: None,
                    agent_role: None,
                    git_info: None,
                    name: None,
                    turns: Vec::new(),
                },
                model: "gpt-5".to_string(),
                model_provider: "openai".to_string(),
                service_tier: None,
                cwd,
                runtime_workspace_roots: Vec::new(),
                instruction_sources: vec![absolute_path("/tmp/AGENTS.md")],
                approval_policy: v2::AskForApproval::OnFailure,
                approvals_reviewer: v2::ApprovalsReviewer::User,
                sandbox: v2::SandboxPolicy::DangerFullAccess,
                active_permission_profile: None,
                reasoning_effort: None,
            },
        };

        assert_eq!(response.id(), &RequestId::Integer(7));
        assert_eq!(response.method(), "thread/start");
        assert_eq!(
            json!({
                "method": "thread/start",
                "id": 7,
                "response": {
                    "thread": {
                        "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
                        "sessionId": "67e55044-10b1-426f-9247-bb680e5fe0c7",
                        "forkedFromId": null,
                        "preview": "first prompt",
                        "ephemeral": true,
                        "modelProvider": "openai",
                        "createdAt": 1,
                        "updatedAt": 2,
                        "status": {
                            "type": "idle"
                        },
                        "path": null,
                        "cwd": absolute_path_string("tmp"),
                        "cliVersion": "0.0.0",
                        "source": "exec",
                        "threadSource": null,
                        "agentNickname": null,
                        "agentRole": null,
                        "gitInfo": null,
                        "name": null,
                        "turns": []
                    },
                    "model": "gpt-5",
                    "modelProvider": "openai",
                    "serviceTier": null,
                    "cwd": absolute_path_string("tmp"),
                    "runtimeWorkspaceRoots": [],
                    "instructionSources": [absolute_path_string("tmp/AGENTS.md")],
                    "approvalPolicy": "on-failure",
                    "approvalsReviewer": "user",
                    "sandbox": {
                        "type": "dangerFullAccess"
                    },
                    "activePermissionProfile": null,
                    "reasoningEffort": null
                }
            }),
            serde_json::to_value(&response)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_config_requirements_read() -> Result<()> {
        let request = ClientRequest::ConfigRequirementsRead {
            request_id: RequestId::Integer(1),
            params: None,
        };
        assert_eq!(
            json!({
                "method": "configRequirements/read",
                "id": 1,
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_account_login_api_key() -> Result<()> {
        let request = ClientRequest::LoginAccount {
            request_id: RequestId::Integer(2),
            params: v2::LoginAccountParams::ApiKey {
                api_key: "secret".to_string(),
            },
        };
        assert_eq!(
            json!({
                "method": "account/login/start",
                "id": 2,
                "params": {
                    "type": "apiKey",
                    "apiKey": "secret"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_account_login_chatgpt() -> Result<()> {
        let request = ClientRequest::LoginAccount {
            request_id: RequestId::Integer(3),
            params: v2::LoginAccountParams::Chatgpt {
                codex_streamlined_login: false,
            },
        };
        assert_eq!(
            json!({
                "method": "account/login/start",
                "id": 3,
                "params": {
                    "type": "chatgpt"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_account_login_chatgpt_streamlined() -> Result<()> {
        let request = ClientRequest::LoginAccount {
            request_id: RequestId::Integer(3),
            params: v2::LoginAccountParams::Chatgpt {
                codex_streamlined_login: true,
            },
        };
        assert_eq!(
            json!({
                "method": "account/login/start",
                "id": 3,
                "params": {
                    "type": "chatgpt",
                    "codexStreamlinedLogin": true
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_account_login_chatgpt_device_code() -> Result<()> {
        let request = ClientRequest::LoginAccount {
            request_id: RequestId::Integer(4),
            params: v2::LoginAccountParams::ChatgptDeviceCode,
        };
        assert_eq!(
            json!({
                "method": "account/login/start",
                "id": 4,
                "params": {
                    "type": "chatgptDeviceCode"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_account_logout() -> Result<()> {
        let request = ClientRequest::LogoutAccount {
            request_id: RequestId::Integer(5),
            params: None,
        };
        assert_eq!(
            json!({
                "method": "account/logout",
                "id": 5,
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_account_login_chatgpt_auth_tokens() -> Result<()> {
        let request = ClientRequest::LoginAccount {
            request_id: RequestId::Integer(6),
            params: v2::LoginAccountParams::ChatgptAuthTokens {
                access_token: "access-token".to_string(),
                chatgpt_account_id: "org-123".to_string(),
                chatgpt_plan_type: Some("business".to_string()),
            },
        };
        assert_eq!(
            json!({
                "method": "account/login/start",
                "id": 6,
                "params": {
                    "type": "chatgptAuthTokens",
                    "accessToken": "access-token",
                    "chatgptAccountId": "org-123",
                    "chatgptPlanType": "business"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_get_account() -> Result<()> {
        let request = ClientRequest::GetAccount {
            request_id: RequestId::Integer(6),
            params: v2::GetAccountParams {
                refresh_token: false,
            },
        };
        assert_eq!(
            json!({
                "method": "account/read",
                "id": 6,
                "params": {}
            }),
            serde_json::to_value(&request)?,
        );
        let request = ClientRequest::GetAccount {
            request_id: RequestId::Integer(7),
            params: v2::GetAccountParams {
                refresh_token: true,
            },
        };
        assert_eq!(
            json!({
                "method": "account/read",
                "id": 7,
                "params": {
                    "refreshToken": true
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn account_serializes_fields_in_camel_case() -> Result<()> {
        let api_key = v2::Account::ApiKey {};
        assert_eq!(
            json!({
                "type": "apiKey",
            }),
            serde_json::to_value(&api_key)?,
        );

        let chatgpt = v2::Account::Chatgpt {
            email: "user@example.com".to_string(),
            plan_type: PlanType::Plus,
        };
        assert_eq!(
            json!({
                "type": "chatgpt",
                "email": "user@example.com",
                "planType": "plus",
            }),
            serde_json::to_value(&chatgpt)?,
        );

        Ok(())
    }

    #[test]
    fn serialize_list_models() -> Result<()> {
        let request = ClientRequest::ModelList {
            request_id: RequestId::Integer(6),
            params: v2::ModelListParams::default(),
        };
        assert_eq!(
            json!({
                "method": "model/list",
                "id": 6,
                "params": {
                    "limit": null,
                    "cursor": null,
                    "includeHidden": null
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_model_provider_capabilities_read() -> Result<()> {
        let request = ClientRequest::ModelProviderCapabilitiesRead {
            request_id: RequestId::Integer(7),
            params: v2::ModelProviderCapabilitiesReadParams {},
        };
        assert_eq!(
            json!({
                "method": "modelProvider/capabilities/read",
                "id": 7,
                "params": {}
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_list_collaboration_modes() -> Result<()> {
        let request = ClientRequest::CollaborationModeList {
            request_id: RequestId::Integer(7),
            params: v2::CollaborationModeListParams::default(),
        };
        assert_eq!(
            json!({
                "method": "collaborationMode/list",
                "id": 7,
                "params": {}
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_list_apps() -> Result<()> {
        let request = ClientRequest::AppsList {
            request_id: RequestId::Integer(8),
            params: v2::AppsListParams::default(),
        };
        assert_eq!(
            json!({
                "method": "app/list",
                "id": 8,
                "params": {
                    "cursor": null,
                    "limit": null,
                    "threadId": null
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_environment_add() -> Result<()> {
        let request = ClientRequest::EnvironmentAdd {
            request_id: RequestId::Integer(9),
            params: v2::EnvironmentAddParams {
                environment_id: "remote-a".to_string(),
                exec_server_url: "ws://127.0.0.1:8765".to_string(),
            },
        };
        assert_eq!(
            json!({
                "method": "environment/add",
                "id": 9,
                "params": {
                    "environmentId": "remote-a",
                    "execServerUrl": "ws://127.0.0.1:8765"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_fs_get_metadata() -> Result<()> {
        let request = ClientRequest::FsGetMetadata {
            request_id: RequestId::Integer(10),
            params: v2::FsGetMetadataParams {
                path: absolute_path("tmp/example"),
            },
        };
        assert_eq!(
            json!({
                "method": "fs/getMetadata",
                "id": 10,
                "params": {
                    "path": absolute_path_string("tmp/example")
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_fs_watch() -> Result<()> {
        let request = ClientRequest::FsWatch {
            request_id: RequestId::Integer(10),
            params: v2::FsWatchParams {
                watch_id: "watch-git".to_string(),
                path: absolute_path("tmp/repo/.git"),
            },
        };
        assert_eq!(
            json!({
                "method": "fs/watch",
                "id": 10,
                "params": {
                    "watchId": "watch-git",
                    "path": absolute_path_string("tmp/repo/.git")
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_list_experimental_features() -> Result<()> {
        let request = ClientRequest::ExperimentalFeatureList {
            request_id: RequestId::Integer(8),
            params: v2::ExperimentalFeatureListParams::default(),
        };
        assert_eq!(
            json!({
                "method": "experimentalFeature/list",
                "id": 8,
                "params": {
                    "cursor": null,
                    "limit": null,
                    "threadId": null
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_list_experimental_features_with_thread_id() -> Result<()> {
        let request = ClientRequest::ExperimentalFeatureList {
            request_id: RequestId::Integer(8),
            params: v2::ExperimentalFeatureListParams {
                cursor: Some("3".to_string()),
                limit: Some(2),
                thread_id: Some("00000000-0000-4000-8000-000000000001".to_string()),
            },
        };
        assert_eq!(
            json!({
                "method": "experimentalFeature/list",
                "id": 8,
                "params": {
                    "cursor": "3",
                    "limit": 2,
                    "threadId": "00000000-0000-4000-8000-000000000001"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_thread_background_terminals_clean() -> Result<()> {
        let request = ClientRequest::ThreadBackgroundTerminalsClean {
            request_id: RequestId::Integer(8),
            params: v2::ThreadBackgroundTerminalsCleanParams {
                thread_id: "thr_123".to_string(),
            },
        };
        assert_eq!(
            json!({
                "method": "thread/backgroundTerminals/clean",
                "id": 8,
                "params": {
                    "threadId": "thr_123"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_thread_realtime_start() -> Result<()> {
        let request = ClientRequest::ThreadRealtimeStart {
            request_id: RequestId::Integer(9),
            params: v2::ThreadRealtimeStartParams {
                thread_id: "thr_123".to_string(),
                output_modality: RealtimeOutputModality::Audio,
                prompt: Some(Some("You are on a call".to_string())),
                realtime_session_id: Some("sess_456".to_string()),
                transport: None,
                voice: Some(RealtimeVoice::Marin),
            },
        };
        assert_eq!(
            json!({
                "method": "thread/realtime/start",
                "id": 9,
                "params": {
                    "threadId": "thr_123",
                    "outputModality": "audio",
                    "prompt": "You are on a call",
                    "realtimeSessionId": "sess_456",
                    "transport": null,
                    "voice": "marin"
                }
            }),
            serde_json::to_value(&request)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_thread_realtime_start_prompt_default_and_null() -> Result<()> {
        let default_prompt_request = ClientRequest::ThreadRealtimeStart {
            request_id: RequestId::Integer(9),
            params: v2::ThreadRealtimeStartParams {
                thread_id: "thr_123".to_string(),
                output_modality: RealtimeOutputModality::Audio,
                prompt: None,
                realtime_session_id: None,
                transport: None,
                voice: None,
            },
        };
        assert_eq!(
            json!({
                "method": "thread/realtime/start",
                "id": 9,
                "params": {
                    "threadId": "thr_123",
                    "outputModality": "audio",
                    "realtimeSessionId": null,
                    "transport": null,
                    "voice": null
                }
            }),
            serde_json::to_value(&default_prompt_request)?,
        );

        let null_prompt_request = ClientRequest::ThreadRealtimeStart {
            request_id: RequestId::Integer(9),
            params: v2::ThreadRealtimeStartParams {
                thread_id: "thr_123".to_string(),
                output_modality: RealtimeOutputModality::Audio,
                prompt: Some(None),
                realtime_session_id: None,
                transport: None,
                voice: None,
            },
        };
        assert_eq!(
            json!({
                "method": "thread/realtime/start",
                "id": 9,
                "params": {
                    "threadId": "thr_123",
                    "outputModality": "audio",
                    "prompt": null,
                    "realtimeSessionId": null,
                    "transport": null,
                    "voice": null
                }
            }),
            serde_json::to_value(&null_prompt_request)?,
        );

        let default_prompt_value = json!({
            "method": "thread/realtime/start",
            "id": 9,
            "params": {
                "threadId": "thr_123",
                "outputModality": "audio",
                "realtimeSessionId": null,
                "transport": null,
                "voice": null
            }
        });
        assert_eq!(
            serde_json::from_value::<ClientRequest>(default_prompt_value)?,
            default_prompt_request,
        );

        let null_prompt_value = json!({
            "method": "thread/realtime/start",
            "id": 9,
            "params": {
                "threadId": "thr_123",
                "outputModality": "audio",
                "prompt": null,
                "realtimeSessionId": null,
                "transport": null,
                "voice": null
            }
        });
        assert_eq!(
            serde_json::from_value::<ClientRequest>(null_prompt_value)?,
            null_prompt_request,
        );

        Ok(())
    }

    #[test]
    fn mock_experimental_method_is_marked_experimental() {
        let request = ClientRequest::MockExperimentalMethod {
            request_id: RequestId::Integer(1),
            params: v2::MockExperimentalMethodParams::default(),
        };
        let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&request);
        assert_eq!(reason, Some("mock/experimentalMethod"));
    }

    #[test]
    fn environment_add_is_marked_experimental() {
        let request = ClientRequest::EnvironmentAdd {
            request_id: RequestId::Integer(1),
            params: v2::EnvironmentAddParams {
                environment_id: "remote-a".to_string(),
                exec_server_url: "ws://127.0.0.1:8765".to_string(),
            },
        };
        let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&request);
        assert_eq!(reason, Some("environment/add"));
    }

    #[test]
    fn command_exec_permission_profile_is_marked_experimental() {
        let request = ClientRequest::OneOffCommandExec {
            request_id: RequestId::Integer(1),
            params: v2::CommandExecParams {
                command: vec!["pwd".to_string()],
                process_id: None,
                tty: false,
                stream_stdin: false,
                stream_stdout_stderr: false,
                output_bytes_cap: None,
                disable_output_cap: false,
                disable_timeout: false,
                timeout_ms: None,
                cwd: None,
                env: None,
                size: None,
                sandbox_policy: None,
                permission_profile: Some(BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_string()),
            },
        };

        let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&request);
        assert_eq!(reason, Some("command/exec.permissionProfile"));
    }

    #[test]
    fn thread_realtime_start_is_marked_experimental() {
        let request = ClientRequest::ThreadRealtimeStart {
            request_id: RequestId::Integer(1),
            params: v2::ThreadRealtimeStartParams {
                thread_id: "thr_123".to_string(),
                output_modality: RealtimeOutputModality::Audio,
                prompt: Some(Some("You are on a call".to_string())),
                realtime_session_id: None,
                transport: None,
                voice: None,
            },
        };
        let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&request);
        assert_eq!(reason, Some("thread/realtime/start"));
    }

    #[test]
    fn thread_goal_methods_are_not_marked_experimental() {
        let set_request = ClientRequest::ThreadGoalSet {
            request_id: RequestId::Integer(1),
            params: v2::ThreadGoalSetParams {
                thread_id: "thr_123".to_string(),
                objective: Some("ship goal mode".to_string()),
                status: Some(v2::ThreadGoalStatus::Active),
                token_budget: Some(Some(10_000)),
            },
        };
        let get_request = ClientRequest::ThreadGoalGet {
            request_id: RequestId::Integer(2),
            params: v2::ThreadGoalGetParams {
                thread_id: "thr_123".to_string(),
            },
        };
        let clear_request = ClientRequest::ThreadGoalClear {
            request_id: RequestId::Integer(3),
            params: v2::ThreadGoalClearParams {
                thread_id: "thr_123".to_string(),
            },
        };

        assert_eq!(
            crate::experimental_api::ExperimentalApi::experimental_reason(&set_request),
            None
        );
        assert_eq!(
            crate::experimental_api::ExperimentalApi::experimental_reason(&get_request),
            None
        );
        assert_eq!(
            crate::experimental_api::ExperimentalApi::experimental_reason(&clear_request),
            None
        );
    }
}
