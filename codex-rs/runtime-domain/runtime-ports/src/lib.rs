//! Runtime boundary traits for infrastructure adapters.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Sends requests across a runtime-owned boundary.
///
/// Implementations should adapt concrete transports, queues, or process
/// boundaries while keeping domain callers independent from infrastructure.
pub trait RuntimePort {
    /// Request shape accepted by the port.
    type Request;
    /// Response shape returned by the port.
    type Response;
    /// Error type returned by the concrete runtime adapter.
    type Error;

    /// Sends one request through the runtime port.
    fn send(&self, request: Self::Request) -> Result<Self::Response, Self::Error>;
}

/// Supplies runtime time to domain code.
///
/// Implementations should centralize clock access so tests and deterministic
/// runtimes can control time without coupling callers to system APIs.
pub trait RuntimeClock {
    /// Returns the current Unix timestamp in whole seconds.
    fn unix_timestamp_seconds(&self) -> i64;
}
