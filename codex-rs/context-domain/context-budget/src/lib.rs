//! Context budget abstractions for prompt assembly.
//!
//! This crate describes budget calculations without knowing how context is
//! collected, compacted, serialized, or sent to a model.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Token capacity available to context assembly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenBudget {
    /// Total token capacity for the request.
    pub total_tokens: u64,
    /// Tokens reserved for model output or system overhead.
    pub reserved_tokens: u64,
}

/// Provides token budget information to context builders.
///
/// Implementations should return the budget that applies to the current
/// model, configuration, and request shape without mutating context state.
pub trait ContextBudget {
    /// Returns the token budget currently available for prompt context.
    fn token_budget(&self) -> TokenBudget;
}
