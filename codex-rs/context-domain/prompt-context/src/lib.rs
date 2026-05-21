//! Prompt context abstractions shared by context collection and assembly code.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// One protocol-neutral item that can be included in prompt context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptContextItem {
    /// Stable source label, such as a file path, memory key, or tool name.
    pub source: String,
    /// Textual content contributed by the source.
    pub body: String,
}

/// Supplies prompt context items to a prompt assembly layer.
///
/// Implementations should collect already-authorized context and leave token
/// selection, ordering, and model serialization to callers.
pub trait PromptContextProvider {
    /// Returns context items currently available for prompt assembly.
    fn prompt_context(&self) -> Vec<PromptContextItem>;
}
