use super::Turn;
use super::shared::v2_enum_from_core;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use ts_rs::TS;

v2_enum_from_core!(
    pub enum ReviewDelivery from codex_protocol::protocol::ReviewDelivery {
        Inline, Detached
    }
);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReviewStartParams {
    pub thread_id: String,
    pub target: ReviewTarget,

    /// Where to run the review: inline (default) on the current thread or
    /// detached on a new thread (returned in `reviewThreadId`).
    #[serde(default)]
    #[ts(optional = nullable)]
    pub delivery: Option<ReviewDelivery>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReviewStartResponse {
    pub turn: Turn,
    /// Identifies the thread where the review runs.
    ///
    /// For inline reviews, this is the original thread id.
    /// For detached reviews, this is the id of the new review thread.
    pub review_thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type", export_to = "v2/")]
pub enum ReviewTarget {
    /// Review the working tree: staged, unstaged, and untracked files.
    UncommittedChanges,

    /// Review changes between the current branch and the given base branch.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    BaseBranch { branch: String },

    /// Review the changes introduced by a specific commit.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Commit {
        sha: String,
        /// Optional human-readable label (e.g., commit subject) for UIs.
        #[ts(optional = nullable)]
        title: Option<String>,
    },

    /// Arbitrary instructions, equivalent to the old free-form prompt.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Custom { instructions: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReviewOutput {
    pub findings: Vec<ReviewFinding>,
    pub overall_correctness: String,
    pub overall_explanation: String,
    pub overall_confidence_score: f32,
}

impl From<&codex_protocol::protocol::ReviewOutputEvent> for ReviewOutput {
    fn from(value: &codex_protocol::protocol::ReviewOutputEvent) -> Self {
        Self {
            findings: value.findings.iter().map(ReviewFinding::from).collect(),
            overall_correctness: value.overall_correctness.clone(),
            overall_explanation: value.overall_explanation.clone(),
            overall_confidence_score: value.overall_confidence_score,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReviewFinding {
    pub title: String,
    pub body: String,
    pub confidence_score: f32,
    pub priority: i32,
    pub code_location: ReviewCodeLocation,
}

impl From<&codex_protocol::protocol::ReviewFinding> for ReviewFinding {
    fn from(value: &codex_protocol::protocol::ReviewFinding) -> Self {
        Self {
            title: value.title.clone(),
            body: value.body.clone(),
            confidence_score: value.confidence_score,
            priority: value.priority,
            code_location: ReviewCodeLocation::from(&value.code_location),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReviewCodeLocation {
    pub absolute_file_path: PathBuf,
    pub line_range: ReviewLineRange,
}

impl From<&codex_protocol::protocol::ReviewCodeLocation> for ReviewCodeLocation {
    fn from(value: &codex_protocol::protocol::ReviewCodeLocation) -> Self {
        Self {
            absolute_file_path: value.absolute_file_path.clone(),
            line_range: ReviewLineRange::from(&value.line_range),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReviewLineRange {
    pub start: u32,
    pub end: u32,
}

impl From<&codex_protocol::protocol::ReviewLineRange> for ReviewLineRange {
    fn from(value: &codex_protocol::protocol::ReviewLineRange) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}
