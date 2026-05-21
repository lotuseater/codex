//! Protocol-neutral identifiers shared by Codex domain crates.
//!
//! This crate owns value types only. It must stay independent from runtime,
//! transport, persistence, UI, and concrete core implementations.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Stable identifier for a Codex conversation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConversationId(pub String);

/// Stable identifier for a Codex session.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SessionId(pub String);

/// Stable identifier for one model interaction turn.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TurnId(pub String);

/// Stable identifier for a tool invocation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ToolCallId(pub String);
