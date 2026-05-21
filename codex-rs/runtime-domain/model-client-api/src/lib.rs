//! Model client abstractions for runtime adapters.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Protocol-neutral model request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRequest {
    /// Model identifier selected by the caller.
    pub model: String,
    /// Serialized prompt or request body supplied to the model adapter.
    pub input: String,
}

/// Protocol-neutral model response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelResponse {
    /// Serialized model output returned by the adapter.
    pub output: String,
}

/// Sends model requests through a concrete model provider.
///
/// Implementations should own provider-specific transport, authentication, and
/// serialization while exposing a stable runtime-facing boundary.
pub trait ModelClient {
    /// Error type returned by the concrete model provider.
    type Error;

    /// Sends one model request and returns the provider response.
    fn send_model_request(&self, request: ModelRequest) -> Result<ModelResponse, Self::Error>;
}
