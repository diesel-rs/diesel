use super::{Pg, PgMetadataLookup};
use backend::BinaryRawValue;
use std::num::NonZeroU32;
use std::ops::Range;

/// Raw postgres value as received from the database
#[derive(Clone, Copy)]
#[allow(missing_debug_implementations)]
pub struct PgValue<'a> {
    raw_value: &'a [u8],
    type_oid: NonZeroU32,
    metadata: &'a PgMetadataLookup,
}

impl<'a> BinaryRawValue<'a> for Pg {
    fn as_bytes(value: PgValue<'a>) -> &'a [u8] {
        value.raw_value
    }
}

impl<'a> PgValue<'a> {
    pub(crate) fn new(
        raw_value: &'a [u8],
        type_oid: NonZeroU32,
        metadata: &'a PgMetadataLookup,
    ) -> Self {
        Self {
            raw_value,
            type_oid,
            metadata,
        }
    }

    /// Get the underlying raw byte representation
    pub fn as_bytes(&self) -> &[u8] {
        self.raw_value
    }

    /// Get the type oid of this value
    pub fn get_oid(&self) -> NonZeroU32 {
        self.type_oid
    }

    /// Get a instance for type lookup
    pub fn get_metadata_lookup(&self) -> &PgMetadataLookup {
        self.metadata
    }

    pub(crate) fn subslice(&self, range: Range<usize>) -> Self {
        Self {
            raw_value: &self.raw_value[range],
            ..*self
        }
    }
}
