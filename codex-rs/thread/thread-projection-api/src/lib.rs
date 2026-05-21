//! App-server-neutral thread projection types.
//!
//! This crate owns the thread history projection surface shared by core-facing
//! code and edge adapters. Wire DTOs should convert to and from these types
//! instead of importing app-server protocol types across ownership boundaries.

mod page;
mod turn;

pub use page::ProjectionPage;
pub use page::ProjectionSortDirection;
pub use page::TurnItemProjectionPage;
pub use page::TurnItemsListParams;
pub use page::TurnListParams;
pub use page::TurnProjectionPage;
pub use turn::ProjectedThread;
pub use turn::ProjectedTurn;
pub use turn::ProjectedTurnError;
pub use turn::ThreadHistoryProjection;
pub use turn::TurnItemsView;
pub use turn::TurnStatus;
