use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use ts_rs::TS;

/// Authentication mode for OpenAI-backed providers.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Display, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    /// OpenAI API key provided by the caller and stored by Codex.
    ApiKey,
    /// ChatGPT OAuth managed by Codex (tokens persisted and refreshed by Codex).
    Chatgpt,
    /// [UNSTABLE] FOR OPENAI INTERNAL USE ONLY - DO NOT USE.
    ///
    /// ChatGPT auth tokens are supplied by an external host app and are only
    /// stored in memory. Token refresh must be handled by the external host app.
    #[serde(rename = "chatgptAuthTokens")]
    #[ts(rename = "chatgptAuthTokens")]
    #[strum(serialize = "chatgptAuthTokens")]
    ChatgptAuthTokens,
    /// Programmatic Codex auth backed by a registered Agent Identity.
    #[serde(rename = "agentIdentity")]
    #[ts(rename = "agentIdentity")]
    #[strum(serialize = "agentIdentity")]
    AgentIdentity,
    /// Programmatic Codex auth backed by a personal access token.
    #[serde(rename = "personalAccessToken")]
    #[ts(rename = "personalAccessToken")]
    #[strum(serialize = "personalAccessToken")]
    PersonalAccessToken,
}

impl AuthMode {
    /// Returns whether this mode represents an authenticated human ChatGPT account.
    pub fn has_chatgpt_account(self) -> bool {
        match self {
            Self::Chatgpt | Self::ChatgptAuthTokens | Self::PersonalAccessToken => true,
            Self::ApiKey | Self::AgentIdentity => false,
        }
    }
}
