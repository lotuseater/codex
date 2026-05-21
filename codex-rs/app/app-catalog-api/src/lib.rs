use std::future::Future;
use std::pin::Pin;

use codex_app_catalog_types::AppInfo;

pub type AppCatalogFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Provides app catalog entries without exposing transport or storage details.
pub trait AppCatalogProvider: Send + Sync {
    fn list_apps(&self) -> AppCatalogFuture<'_, Result<Vec<AppInfo>, AppCatalogError>>;
}

/// Provides accessible app catalog entries for a configured runtime context.
pub trait AccessibleAppCatalogProvider: Send + Sync {
    fn list_accessible_apps(&self) -> AppCatalogFuture<'_, Result<Vec<AppInfo>, AppCatalogError>>;
}

#[derive(Debug, thiserror::Error)]
pub enum AppCatalogError {
    #[error("{0}")]
    Message(String),
}
