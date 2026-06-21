//! Storage-neutral thread persistence API.
//!
//! This crate owns the thread-store contracts and data transfer types. Concrete
//! stores live in implementation crates and should depend on this crate, not the
//! other way around.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

mod error;
mod live_thread;
mod recording;
mod store;
mod types;

pub use error::ThreadStoreError;
pub use error::ThreadStoreResult;
pub use live_thread::LiveThreadFactory;
pub use live_thread::LiveThreadHandle;
pub use live_thread::ThreadPersistenceServices;
pub use live_thread::UnsupportedLiveThreadFactory;
pub use recording::RecordingLiveThreadFactory;
pub use recording::RecordingThreadStore;
pub use recording::RecordingThreadStoreCalls;
pub use store::ThreadStore;
pub use store::ThreadStoreFuture;
pub use store::UnsupportedThreadStore;
pub use types::AppendThreadItemsParams;
pub use types::ArchiveThreadParams;
pub use types::ClearableField;
pub use types::CreateThreadParams;
pub use types::DeleteThreadParams;
pub use types::ExtraConfig;
pub use types::GitInfoPatch;
pub use types::ItemPage;
pub use types::ListItemsParams;
pub use types::ListThreadsParams;
pub use types::ListTurnsParams;
pub use types::LoadThreadHistoryParams;
pub use types::ReadThreadByRolloutPathParams;
pub use types::ReadThreadDynamicToolsParams;
pub use types::ReadThreadParams;
pub use types::ResumeThreadParams;
pub use types::SearchThreadsParams;
pub use types::SortDirection;
pub use types::StoredThread;
pub use types::StoredThreadHistory;
pub use types::StoredThreadItem;
pub use types::StoredThreadSearchResult;
pub use types::StoredTurn;
pub use types::StoredTurnError;
pub use types::StoredTurnItemsView;
pub use types::StoredTurnStatus;
pub use types::ThreadEventPersistenceMode;
pub use types::ThreadMetadataPatch;
pub use types::ThreadPage;
pub use types::ThreadPersistenceMetadata;
pub use types::ThreadSearchPage;
pub use types::ThreadSortKey;
pub use types::ThreadStoreSelection;
pub use types::TurnPage;
pub use types::UpdateThreadMetadataParams;
