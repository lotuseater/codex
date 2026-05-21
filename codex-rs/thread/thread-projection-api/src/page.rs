use codex_protocol::ThreadId;
use serde::Deserialize;
use serde::Serialize;

use crate::TurnItemsView;
use crate::turn::ProjectedTurn;

/// Sort direction for cursor-based thread projection pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionSortDirection {
    Asc,
    Desc,
}

/// Parameters for listing projected turns from a thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnListParams {
    pub thread_id: ThreadId,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub sort_direction: Option<ProjectionSortDirection>,
    pub items_view: Option<TurnItemsView>,
}

/// Parameters for listing projected items within one turn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnItemsListParams {
    pub thread_id: ThreadId,
    pub turn_id: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub sort_direction: Option<ProjectionSortDirection>,
}

/// A cursor-paginated projection page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionPage<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub backwards_cursor: Option<String>,
}

pub type TurnProjectionPage<TurnItem> = ProjectionPage<ProjectedTurn<TurnItem>>;
pub type TurnItemProjectionPage<TurnItem> = ProjectionPage<TurnItem>;
