use std::future::Future;
use std::pin::Pin;

use codex_protocol::protocol::ReviewDecision;

use crate::ExtensionData;

/// Future returned by one claimed approval-review contribution.
pub type ApprovalReviewFuture<'a> = Pin<Box<dyn Future<Output = ReviewDecision> + Send + 'a>>;

/// Extension contribution that can claim rendered approval-review prompts.
pub trait ApprovalReviewContributor: Send + Sync {
    fn contribute<'a>(
        &'a self,
        session_store: &'a ExtensionData,
        thread_store: &'a ExtensionData,
        prompt: &'a str,
    ) -> Option<ApprovalReviewFuture<'a>>;
}
