//! Protocol-neutral history access abstractions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Role associated with a history entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryRole {
    /// User-authored input.
    User,
    /// Assistant-authored output.
    Assistant,
    /// System or developer instruction.
    System,
    /// Tool call or tool output content.
    Tool,
}

/// One item in conversation history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    /// Stable entry identifier.
    pub id: String,
    /// Role that produced the entry.
    pub role: HistoryRole,
    /// Text content for the entry.
    pub text: String,
}

/// Reads conversation history from a backing implementation.
///
/// Implementations should expose ordered history entries without committing
/// callers to a concrete persistence, protocol, or UI representation.
pub trait HistoryReader {
    /// Returns history entries in replay order.
    fn history_entries(&self) -> Vec<HistoryEntry>;
}

/// Appends conversation history to a backing implementation.
///
/// Implementations should preserve entry ordering and own any persistence or
/// replication behavior required by the concrete store.
pub trait HistoryWriter {
    /// Appends one entry to history.
    fn append_history_entry(&mut self, entry: HistoryEntry);
}
