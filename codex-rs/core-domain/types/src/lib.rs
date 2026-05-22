//! Protocol-neutral identifiers shared by Codex domain crates.
//!
//! This crate owns value types only. It must stay independent from runtime, UI,
//! persistence, transport protocols, and concrete core implementations.
//! Serialization derives are allowed so edge DTOs can carry these identifiers
//! without depending on protocol crates for identity types.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

macro_rules! string_id {
    (
        $(#[$meta:meta])*
        pub struct $name:ident;
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Creates an identifier from its canonical string representation.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consumes the identifier and returns the owned string.
            #[must_use]
            pub fn into_string(self) -> String {
                self.0
            }

            /// Returns true when the identifier is empty.
            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

string_id! {
    /// Stable identifier for a Codex conversation.
    pub struct ConversationId;
}

string_id! {
    /// Stable identifier for a Codex thread.
    pub struct ThreadId;
}

string_id! {
    /// Stable identifier for a Codex session.
    pub struct SessionId;
}

string_id! {
    /// Stable external identifier for one model interaction turn.
    pub struct TurnId;
}

string_id! {
    /// Stable identifier for a tool invocation.
    pub struct ToolCallId;
}
