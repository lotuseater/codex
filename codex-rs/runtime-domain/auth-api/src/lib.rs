//! Authentication abstractions for runtime adapters.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Authentication mode for OpenAI-backed providers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthMode {
    /// OpenAI API key provided by the caller and stored by Codex.
    ApiKey,
    /// ChatGPT OAuth managed by Codex.
    Chatgpt,
    /// ChatGPT auth tokens supplied by an external host app.
    ChatgptAuthTokens,
    /// Programmatic Codex auth backed by a registered Agent Identity.
    AgentIdentity,
}

/// Credential material supplied to a runtime adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthCredential {
    /// Stable account or profile identifier.
    pub account_id: String,
    /// Opaque secret material owned by the concrete auth provider.
    pub secret: String,
}

/// Provides authentication material to runtime infrastructure.
///
/// Implementations should own refresh, storage, and account-selection logic and
/// return only the credential material needed by the caller.
pub trait AuthProvider {
    /// Error type returned by the concrete auth provider.
    type Error;

    /// Returns the current credential for runtime use.
    fn credential(&self) -> Result<AuthCredential, Self::Error>;
}
