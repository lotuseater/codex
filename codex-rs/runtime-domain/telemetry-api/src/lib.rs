//! Telemetry abstractions for runtime infrastructure.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::collections::BTreeMap;

/// Protocol-neutral telemetry event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryEvent {
    /// Stable event name.
    pub name: String,
    /// String attributes attached to the event.
    pub attributes: BTreeMap<String, String>,
}

/// Receives runtime telemetry events.
///
/// Implementations should own export, batching, filtering, or persistence
/// behavior without leaking telemetry backend details into domain callers.
pub trait TelemetrySink {
    /// Emits one telemetry event.
    fn emit(&self, event: TelemetryEvent);
}
