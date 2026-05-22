use std::future::Future;
use std::pin::Pin;

use codex_protocol::items::TurnItem;

use crate::ExtensionData;

/// Future returned by one ordered turn-item contribution.
pub type TurnItemContributionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Ordered post-processing contribution for one parsed turn item.
///
/// Implementations may mutate the item before it is emitted and may use the
/// explicitly exposed thread- and turn-lifetime stores when they need durable
/// extension-private state.
pub trait TurnItemContributor: Send + Sync {
    fn contribute<'a>(
        &'a self,
        thread_store: &'a ExtensionData,
        turn_store: &'a ExtensionData,
        item: &'a mut TurnItem,
    ) -> TurnItemContributionFuture<'a>;
}
