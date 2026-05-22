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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_history_projection_preserves_turn_state() {
        let projected_turn = ProjectedTurn {
            id: "turn-1".to_string(),
            items: vec!["summary".to_string()],
            items_view: TurnItemsView::Summary,
            status: TurnStatus::Failed,
            error: Some(ProjectedTurnError {
                message: "failed".to_string(),
                codex_error_info: None,
                additional_details: Some("details".to_string()),
            }),
            started_at: Some(10),
            completed_at: Some(20),
            duration_ms: Some(10_000),
        };
        let projection: ThreadHistoryProjection<String> = vec![projected_turn.clone()];
        let projected_thread = ProjectedThread {
            id: ThreadId::default(),
            turns: projection,
        };

        assert_eq!(
            projected_thread.turns,
            vec![ProjectedTurn {
                id: "turn-1".to_string(),
                items: vec!["summary".to_string()],
                items_view: TurnItemsView::Summary,
                status: TurnStatus::Failed,
                error: Some(ProjectedTurnError {
                    message: "failed".to_string(),
                    codex_error_info: None,
                    additional_details: Some("details".to_string()),
                }),
                started_at: Some(10),
                completed_at: Some(20),
                duration_ms: Some(10_000),
            }]
        );
    }
}
