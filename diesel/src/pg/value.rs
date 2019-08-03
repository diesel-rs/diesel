use super::Pg;
use backend::BinaryRawValue;
use std::num::NonZeroU32;

/// Marker trait for types with oid's known at compile time
pub trait StaticSqlType {
    /// Type oid
    const OID: NonZeroU32;
    /// Array oid
    const ARRAY_OID: NonZeroU32;
}

/// Raw postgres value as received from the database
#[derive(Clone, Copy)]
#[allow(missing_debug_implementations)]
pub struct PgValue<'a> {
    raw_value: &'a [u8],
    type_oid: NonZeroU32,
}

impl<'a> BinaryRawValue<'a> for Pg {
    fn as_bytes(value: PgValue<'a>) -> &'a [u8] {
        value.raw_value
    }
}

impl<'a> PgValue<'a> {
    pub(crate) fn new(raw_value: &'a [u8], type_oid: NonZeroU32) -> Self {
        Self {
            raw_value,
            type_oid,
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

    pub(crate) fn with_new_oid(self, type_oid: NonZeroU32) -> Self {
        Self { type_oid, ..self }
    }
}
