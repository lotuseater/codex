mod client;
mod events;
mod facts;
mod reducer_api;

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub use client::AnalyticsEventsClient;
pub use events::AppServerRpcTransport;
pub use events::normalize_path_for_skill_id;
pub use events::skill_id_for_local_skill;
pub use events::GuardianApprovalRequestSource;
pub use events::GuardianReviewAnalyticsResult;
pub use events::GuardianReviewDecision;
pub use events::GuardianReviewEventParams;
pub use events::GuardianReviewFailureReason;
pub use events::GuardianReviewSessionKind;
pub use events::GuardianReviewTerminalStatus;
pub use events::GuardianReviewTrackContext;
pub use events::GuardianReviewedAction;
pub use facts::AcceptedLineFingerprint;
pub use facts::AnalyticsFact;
pub use facts::AnalyticsJsonRpcError;
pub use facts::AppInvocation;
pub use facts::AppMentionedInput;
pub use facts::AppUsedInput;
pub use facts::HookRunInput;
pub use facts::PluginState;
pub use facts::PluginStateChangedInput;
pub use facts::PluginUsedInput;
pub use facts::SkillInvokedInput;
pub use facts::CodexCompactionEvent;
pub use facts::CodexTurnSteerEvent;
pub use facts::CompactionImplementation;
pub use facts::CompactionPhase;
pub use facts::CompactionReason;
pub use facts::CompactionStatus;
pub use facts::CompactionStrategy;
pub use facts::CompactionTrigger;
pub use facts::CustomAnalyticsFact;
pub use facts::HookRunFact;
pub use facts::InputError;
pub use facts::InvocationType;
pub use facts::SkillInvocation;
pub use facts::SubAgentThreadStartedInput;
pub use facts::ThreadInitializationMode;
pub use facts::TrackEventsContext;
pub use facts::TurnResolvedConfigFact;
pub use facts::TurnStatus;
pub use facts::TurnSteerRejectionReason;
pub use facts::TurnSteerRequestError;
pub use facts::TurnSteerResult;
pub use facts::TurnSubmissionType;
pub use facts::TurnTokenUsageFact;
pub use facts::build_track_events_context;
pub use reducer_api::AnalyticsReducer;
pub use reducer_api::CustomFactReducer;
pub use reducer_api::TrackEvent;

pub fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn now_unix_millis() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}
