//! Thread persistence implementations.
//!
//! Application code should treat [`codex_protocol::ThreadId`] as the only durable thread handle.
//! Storage-neutral contracts live in `codex-thread-store-api`; this crate owns concrete store
//! implementations only.

mod factory;
mod in_memory;
mod live_thread;
mod local;
mod thread_metadata_sync;

pub use codex_thread_store_api::SearchThreadsParams;
pub use codex_thread_store_api::SortDirection;
pub use codex_thread_store_api::StoredThreadSearchResult;
pub use codex_thread_store_api::ThreadSearchPage;
pub use codex_thread_store_api::ThreadSortKey;
pub use codex_thread_store_api::ThreadStoreError;
pub use codex_thread_store_api::ThreadStoreResult;
pub use codex_thread_store_api::ThreadStoreSelection;
pub use factory::thread_store_from_config;
pub use in_memory::InMemoryThreadStore;
pub use in_memory::InMemoryThreadStoreCalls;
pub use live_thread::LiveThread;
pub use live_thread::StoreLiveThreadFactory;
pub use local::LocalThreadStore;
pub use local::LocalThreadStoreConfig;
