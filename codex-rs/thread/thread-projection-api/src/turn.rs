use codex_protocol::ThreadId;
use codex_protocol::protocol::CodexErrorInfo;
use serde::Deserialize;
use serde::Serialize;

/// Status of a projected turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnStatus {
    Completed,
    Interrupted,
    Failed,
    InProgress,
}

/// Amount of item detail loaded into a projected turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnItemsView {
    /// `items` was not loaded for this turn.
    NotLoaded,
    /// `items` contains only a display summary for this turn.
    Summary,
    /// `items` contains every item available from projected history.
    #[default]
    Full,
}

/// Error details for a failed projected turn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedTurnError {
    pub message: String,
    pub codex_error_info: Option<CodexErrorInfo>,
    #[serde(default)]
    pub additional_details: Option<String>,
}

/// App-server-neutral representation of a turn in a thread history projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedTurn<TurnItem> {
    pub id: String,
    pub items: Vec<TurnItem>,
    #[serde(default)]
    pub items_view: TurnItemsView,
    pub status: TurnStatus,
    pub error: Option<ProjectedTurnError>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
}

/// App-server-neutral thread history projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedThread<TurnItem> {
    pub id: ThreadId,
    pub turns: Vec<ProjectedTurn<TurnItem>>,
}

pub type ThreadHistoryProjection<TurnItem> = Vec<ProjectedTurn<TurnItem>>;
