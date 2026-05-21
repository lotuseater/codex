//! Policy abstractions for deciding when prompt context should be compacted.
//!
//! The crate is intentionally independent from compaction executors and model
//! clients; it only represents policy inputs and decisions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Decision returned by a compaction policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompactionDecision {
    /// Keep the context as-is.
    Keep,
    /// Compact the context toward the requested token target.
    Compact { target_tokens: u64 },
}

/// Decides whether a context value should be compacted.
///
/// Implementations should be deterministic for the same policy configuration
/// and context input. Executing the compaction is owned by a separate layer.
pub trait CompactionPolicy {
    /// Context representation evaluated by the policy.
    type Context;

    /// Evaluates the supplied context and returns a compaction decision.
    fn evaluate(&self, context: &Self::Context) -> CompactionDecision;
}
