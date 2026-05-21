//! Tool registry abstractions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Protocol-neutral description of an available tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    /// Stable tool name used by callers.
    pub name: String,
    /// Human-readable description of the tool contract.
    pub description: String,
}

/// Exposes the tool set available to a caller.
///
/// Implementations should describe registered tools without coupling the
/// caller to the concrete handler, execution engine, or protocol serializer.
pub trait ToolRegistry {
    /// Returns the tool descriptors available for registration.
    fn tool_descriptors(&self) -> Vec<ToolDescriptor>;
}
