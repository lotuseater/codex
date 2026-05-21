//! State database abstractions for runtime adapters.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Opaque state record stored by a concrete database implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateRecord {
    /// Logical collection containing the record.
    pub collection: String,
    /// Stable key within the collection.
    pub key: String,
    /// Opaque serialized record value.
    pub value: Vec<u8>,
}

/// Reads and writes opaque state records.
///
/// Implementations should own database connections, transactions, and encoding
/// details while exposing a small runtime-facing boundary.
pub trait StateDb {
    /// Error type returned by the concrete database adapter.
    type Error;

    /// Reads one record by collection and key.
    fn get_state_record(
        &self,
        collection: &str,
        key: &str,
    ) -> Result<Option<StateRecord>, Self::Error>;

    /// Writes one record.
    fn put_state_record(&mut self, record: StateRecord) -> Result<(), Self::Error>;
}
