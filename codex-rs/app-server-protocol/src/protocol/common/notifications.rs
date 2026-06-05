use super::fuzzy_file_search::*;
use crate::JSONRPCNotification;
use crate::export::GeneratedSchema;
use crate::protocol::v2;
use codex_experimental_api_macros::ExperimentalApi;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use ts_rs::TS;

/// Generates `ServerNotification` enum and helpers, including a JSON Schema
/// exporter for each notification.
macro_rules! server_notification_definitions {
    (
        $(
            $(#[$variant_meta:meta])*
            $variant:ident $(=> $wire:literal)? ( $payload:ty )
        ),* $(,)?
    ) => {
        /// Notification sent from the server to the client.
        #[derive(
            Serialize,
            Deserialize,
            Debug,
            Clone,
            JsonSchema,
            TS,
            Display,
            ExperimentalApi,
        )]
        #[allow(clippy::large_enum_variant)]
        #[serde(tag = "method", content = "params", rename_all = "camelCase")]
        #[strum(serialize_all = "camelCase")]
        pub enum ServerNotification {
            $(
                $(#[$variant_meta])*
                $(#[serde(rename = $wire)] #[ts(rename = $wire)] #[strum(serialize = $wire)])?
                $variant($payload),
            )*
        }

        impl ServerNotification {
            pub fn to_params(self) -> Result<serde_json::Value, serde_json::Error> {
                match self {
                    $(Self::$variant(params) => serde_json::to_value(params),)*
                }
            }
        }

        impl TryFrom<JSONRPCNotification> for ServerNotification {
            type Error = serde_json::Error;

            fn try_from(value: JSONRPCNotification) -> Result<Self, serde_json::Error> {
                serde_json::from_value(serde_json::to_value(value)?)
            }
        }

        #[allow(clippy::vec_init_then_push)]
        pub fn export_server_notification_schemas(
            out_dir: &::std::path::Path,
        ) -> ::anyhow::Result<Vec<GeneratedSchema>> {
            let mut schemas = Vec::new();
            $(schemas.push(crate::export::write_json_schema::<$payload>(out_dir, stringify!($payload))?);)*
            Ok(schemas)
        }
    };
}
/// Notifications sent from the client to the server.
macro_rules! client_notification_definitions {
    (
        $(
            $(#[$variant_meta:meta])*
            $variant:ident $( ( $payload:ty ) )?
        ),* $(,)?
    ) => {
        #[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, TS, Display)]
        #[serde(tag = "method", content = "params", rename_all = "camelCase")]
        #[strum(serialize_all = "camelCase")]
        pub enum ClientNotification {
            $(
                $(#[$variant_meta])*
                $variant $( ( $payload ) )?,
            )*
        }

        pub fn export_client_notification_schemas(
            _out_dir: &::std::path::Path,
        ) -> ::anyhow::Result<Vec<GeneratedSchema>> {
            let schemas = Vec::new();
            $( $(schemas.push(crate::export::write_json_schema::<$payload>(_out_dir, stringify!($payload))?);)? )*
            Ok(schemas)
        }
    };
}

server_notification_definitions! {
    /// NEW NOTIFICATIONS
    Error => "error" (v2::ErrorNotification),
    ThreadStarted => "thread/started" (v2::ThreadStartedNotification),
    ThreadStatusChanged => "thread/status/changed" (v2::ThreadStatusChangedNotification),
    ThreadArchived => "thread/archived" (v2::ThreadArchivedNotification),
    ThreadUnarchived => "thread/unarchived" (v2::ThreadUnarchivedNotification),
    ThreadClosed => "thread/closed" (v2::ThreadClosedNotification),
    SkillsChanged => "skills/changed" (v2::SkillsChangedNotification),
    ThreadNameUpdated => "thread/name/updated" (v2::ThreadNameUpdatedNotification),
    ThreadGoalUpdated => "thread/goal/updated" (v2::ThreadGoalUpdatedNotification),
    ThreadGoalCleared => "thread/goal/cleared" (v2::ThreadGoalClearedNotification),
    #[experimental("thread/settings/updated")]
    ThreadSettingsUpdated => "thread/settings/updated" (v2::ThreadSettingsUpdatedNotification),
    ThreadTokenUsageUpdated => "thread/tokenUsage/updated" (v2::ThreadTokenUsageUpdatedNotification),
    TurnStarted => "turn/started" (v2::TurnStartedNotification),
    HookStarted => "hook/started" (v2::HookStartedNotification),
    TurnCompleted => "turn/completed" (v2::TurnCompletedNotification),
    HookCompleted => "hook/completed" (v2::HookCompletedNotification),
    TurnDiffUpdated => "turn/diff/updated" (v2::TurnDiffUpdatedNotification),
    TurnPlanUpdated => "turn/plan/updated" (v2::TurnPlanUpdatedNotification),
    ItemStarted => "item/started" (v2::ItemStartedNotification),
    ItemGuardianApprovalReviewStarted => "item/autoApprovalReview/started" (v2::ItemGuardianApprovalReviewStartedNotification),
    ItemGuardianApprovalReviewCompleted => "item/autoApprovalReview/completed" (v2::ItemGuardianApprovalReviewCompletedNotification),
    ItemCompleted => "item/completed" (v2::ItemCompletedNotification),
    /// This event is internal-only. Used by Codex Cloud.
    RawResponseItemCompleted => "rawResponseItem/completed" (v2::RawResponseItemCompletedNotification),
    AgentMessageDelta => "item/agentMessage/delta" (v2::AgentMessageDeltaNotification),
    /// EXPERIMENTAL - proposed plan streaming deltas for plan items.
    PlanDelta => "item/plan/delta" (v2::PlanDeltaNotification),
    /// Stream base64-encoded stdout/stderr chunks for a running `command/exec` session.
    CommandExecOutputDelta => "command/exec/outputDelta" (v2::CommandExecOutputDeltaNotification),
    /// Stream base64-encoded stdout/stderr chunks for a running `process/spawn` session.
    #[experimental("process/outputDelta")]
    ProcessOutputDelta => "process/outputDelta" (v2::ProcessOutputDeltaNotification),
    /// Final exit notification for a `process/spawn` session.
    #[experimental("process/exited")]
    ProcessExited => "process/exited" (v2::ProcessExitedNotification),
    CommandExecutionOutputDelta => "item/commandExecution/outputDelta" (v2::CommandExecutionOutputDeltaNotification),
    TerminalInteraction => "item/commandExecution/terminalInteraction" (v2::TerminalInteractionNotification),
    /// Deprecated legacy apply_patch output stream notification.
    FileChangeOutputDelta => "item/fileChange/outputDelta" (v2::FileChangeOutputDeltaNotification),
    FileChangePatchUpdated => "item/fileChange/patchUpdated" (v2::FileChangePatchUpdatedNotification),
    ServerRequestResolved => "serverRequest/resolved" (v2::ServerRequestResolvedNotification),
    McpToolCallProgress => "item/mcpToolCall/progress" (v2::McpToolCallProgressNotification),
    McpServerOauthLoginCompleted => "mcpServer/oauthLogin/completed" (v2::McpServerOauthLoginCompletedNotification),
    McpServerStatusUpdated => "mcpServer/startupStatus/updated" (v2::McpServerStatusUpdatedNotification),
    AccountUpdated => "account/updated" (v2::AccountUpdatedNotification),
    AccountRateLimitsUpdated => "account/rateLimits/updated" (v2::AccountRateLimitsUpdatedNotification),
    AppListUpdated => "app/list/updated" (v2::AppListUpdatedNotification),
    RemoteControlStatusChanged => "remoteControl/status/changed" (v2::RemoteControlStatusChangedNotification),
    ExternalAgentConfigImportCompleted => "externalAgentConfig/import/completed" (v2::ExternalAgentConfigImportCompletedNotification),
    FsChanged => "fs/changed" (v2::FsChangedNotification),
    ReasoningSummaryTextDelta => "item/reasoning/summaryTextDelta" (v2::ReasoningSummaryTextDeltaNotification),
    ReasoningSummaryPartAdded => "item/reasoning/summaryPartAdded" (v2::ReasoningSummaryPartAddedNotification),
    ReasoningTextDelta => "item/reasoning/textDelta" (v2::ReasoningTextDeltaNotification),
    /// Deprecated: Use `ContextCompaction` item type instead.
    ContextCompacted => "thread/compacted" (v2::ContextCompactedNotification),
    ModelRerouted => "model/rerouted" (v2::ModelReroutedNotification),
    ModelVerification => "model/verification" (v2::ModelVerificationNotification),
    #[experimental("turn/moderationMetadata")]
    TurnModerationMetadata => "turn/moderationMetadata" (v2::TurnModerationMetadataNotification),
    Warning => "warning" (v2::WarningNotification),
    GuardianWarning => "guardianWarning" (v2::GuardianWarningNotification),
    DeprecationNotice => "deprecationNotice" (v2::DeprecationNoticeNotification),
    ConfigWarning => "configWarning" (v2::ConfigWarningNotification),
    FuzzyFileSearchSessionUpdated => "fuzzyFileSearch/sessionUpdated" (FuzzyFileSearchSessionUpdatedNotification),
    FuzzyFileSearchSessionCompleted => "fuzzyFileSearch/sessionCompleted" (FuzzyFileSearchSessionCompletedNotification),
    #[experimental("thread/realtime/started")]
    ThreadRealtimeStarted => "thread/realtime/started" (v2::ThreadRealtimeStartedNotification),
    #[experimental("thread/realtime/itemAdded")]
    ThreadRealtimeItemAdded => "thread/realtime/itemAdded" (v2::ThreadRealtimeItemAddedNotification),
    #[experimental("thread/realtime/transcript/delta")]
    ThreadRealtimeTranscriptDelta => "thread/realtime/transcript/delta" (v2::ThreadRealtimeTranscriptDeltaNotification),
    #[experimental("thread/realtime/transcript/done")]
    ThreadRealtimeTranscriptDone => "thread/realtime/transcript/done" (v2::ThreadRealtimeTranscriptDoneNotification),
    #[experimental("thread/realtime/outputAudio/delta")]
    ThreadRealtimeOutputAudioDelta => "thread/realtime/outputAudio/delta" (v2::ThreadRealtimeOutputAudioDeltaNotification),
    #[experimental("thread/realtime/sdp")]
    ThreadRealtimeSdp => "thread/realtime/sdp" (v2::ThreadRealtimeSdpNotification),
    #[experimental("thread/realtime/error")]
    ThreadRealtimeError => "thread/realtime/error" (v2::ThreadRealtimeErrorNotification),
    #[experimental("thread/realtime/closed")]
    ThreadRealtimeClosed => "thread/realtime/closed" (v2::ThreadRealtimeClosedNotification),

    /// Notifies the user of world-writable directories on Windows, which cannot be protected by the sandbox.
    WindowsWorldWritableWarning => "windows/worldWritableWarning" (v2::WindowsWorldWritableWarningNotification),
    WindowsSandboxSetupCompleted => "windowsSandbox/setupCompleted" (v2::WindowsSandboxSetupCompletedNotification),

    #[serde(rename = "account/login/completed")]
    #[ts(rename = "account/login/completed")]
    #[strum(serialize = "account/login/completed")]
    AccountLoginCompleted(v2::AccountLoginCompletedNotification),

}

client_notification_definitions! {
    Initialized,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use codex_utils_absolute_path::AbsolutePathBuf;
    use codex_utils_absolute_path::test_support::PathBufExt;
    use codex_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn absolute_path(path: &str) -> AbsolutePathBuf {
        let path = format!("/{}", path.trim_start_matches('/'));
        test_path_buf(&path).abs()
    }

    #[test]
    fn serialize_client_notification() -> Result<()> {
        let notification = ClientNotification::Initialized;
        // Note there is no "params" field for this notification.
        assert_eq!(
            json!({
                "method": "initialized",
            }),
            serde_json::to_value(&notification)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_thread_status_changed_notification() -> Result<()> {
        let notification =
            ServerNotification::ThreadStatusChanged(v2::ThreadStatusChangedNotification {
                thread_id: "thr_123".to_string(),
                status: v2::ThreadStatus::Idle,
            });
        assert_eq!(
            json!({
                "method": "thread/status/changed",
                "params": {
                    "threadId": "thr_123",
                    "status": {
                        "type": "idle"
                    },
                }
            }),
            serde_json::to_value(&notification)?,
        );
        Ok(())
    }

    #[test]
    fn serialize_thread_realtime_output_audio_delta_notification() -> Result<()> {
        let notification = ServerNotification::ThreadRealtimeOutputAudioDelta(
            v2::ThreadRealtimeOutputAudioDeltaNotification {
                thread_id: "thr_123".to_string(),
                audio: v2::ThreadRealtimeAudioChunk {
                    data: "AQID".to_string(),
                    sample_rate: 24_000,
                    num_channels: 1,
                    samples_per_channel: Some(512),
                    item_id: None,
                },
            },
        );
        assert_eq!(
            json!({
                "method": "thread/realtime/outputAudio/delta",
                "params": {
                    "threadId": "thr_123",
                    "audio": {
                        "data": "AQID",
                        "sampleRate": 24000,
                        "numChannels": 1,
                        "samplesPerChannel": 512,
                        "itemId": null
                    }
                }
            }),
            serde_json::to_value(&notification)?,
        );
        Ok(())
    }

    #[test]
    fn thread_goal_notifications_are_not_marked_experimental() {
        let goal = v2::ThreadGoal {
            thread_id: "thr_123".to_string(),
            objective: "ship goal mode".to_string(),
            status: v2::ThreadGoalStatus::Active,
            token_budget: Some(10_000),
            tokens_used: 123,
            time_used_seconds: 45,
            created_at: 1_700_000_000,
            updated_at: 1_700_000_123,
        };
        let updated = ServerNotification::ThreadGoalUpdated(v2::ThreadGoalUpdatedNotification {
            thread_id: "thr_123".to_string(),
            turn_id: None,
            goal,
        });
        let cleared = ServerNotification::ThreadGoalCleared(v2::ThreadGoalClearedNotification {
            thread_id: "thr_123".to_string(),
        });

        assert_eq!(
            crate::experimental_api::ExperimentalApi::experimental_reason(&updated),
            None
        );
        assert_eq!(
            crate::experimental_api::ExperimentalApi::experimental_reason(&cleared),
            None
        );
    }

    #[test]
    fn thread_settings_updated_notification_is_marked_experimental() {
        let notification =
            ServerNotification::ThreadSettingsUpdated(v2::ThreadSettingsUpdatedNotification {
                thread_id: "thr_123".to_string(),
                thread_settings: v2::ThreadSettings {
                    cwd: absolute_path("/tmp/repo"),
                    approval_policy: v2::AskForApproval::Never,
                    approvals_reviewer: v2::ApprovalsReviewer::User,
                    sandbox_policy: v2::SandboxPolicy::DangerFullAccess,
                    active_permission_profile: None,
                    model: "gpt-5.4".to_string(),
                    model_provider: "openai".to_string(),
                    service_tier: None,
                    effort: None,
                    summary: None,
                    collaboration_mode: codex_protocol::config_types::CollaborationMode {
                        mode: codex_protocol::config_types::ModeKind::Default,
                        settings: codex_protocol::config_types::Settings {
                            model: "gpt-5.4".to_string(),
                            reasoning_effort: None,
                            developer_instructions: None,
                        },
                    },
                    personality: None,
                },
            });

        assert_eq!(
            crate::experimental_api::ExperimentalApi::experimental_reason(&notification),
            Some("thread/settings/updated")
        );
    }

    #[test]
    fn thread_realtime_started_notification_is_marked_experimental() {
        let notification =
            ServerNotification::ThreadRealtimeStarted(v2::ThreadRealtimeStartedNotification {
                thread_id: "thr_123".to_string(),
                realtime_session_id: Some("sess_456".to_string()),
                version: codex_protocol::protocol::RealtimeConversationVersion::V1,
            });
        let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&notification);
        assert_eq!(reason, Some("thread/realtime/started"));
    }

    #[test]
    fn thread_realtime_output_audio_delta_notification_is_marked_experimental() {
        let notification = ServerNotification::ThreadRealtimeOutputAudioDelta(
            v2::ThreadRealtimeOutputAudioDeltaNotification {
                thread_id: "thr_123".to_string(),
                audio: v2::ThreadRealtimeAudioChunk {
                    data: "AQID".to_string(),
                    sample_rate: 24_000,
                    num_channels: 1,
                    samples_per_channel: Some(512),
                    item_id: None,
                },
            },
        );
        let reason = crate::experimental_api::ExperimentalApi::experimental_reason(&notification);
        assert_eq!(reason, Some("thread/realtime/outputAudio/delta"));
    }
}
