//! The PostgreSQL backend

use byteorder::NetworkEndian;

use super::query_builder::PgQueryBuilder;
use super::PgMetadataLookup;
use backend::*;
use prelude::Queryable;
use query_builder::bind_collector::RawBytesBindCollector;
use sql_types::{Oid, TypeMetadata};

use std::slice;

/// The PostgreSQL backend
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pg;

/// The raw value representation of the postgres backend
#[derive(Debug)]
pub struct PgValue<'a> {
    raw_bytes: &'a [u8],
    oid: u32,
}

impl<'a> PgValue<'a> {
    pub(crate) fn new(value_ptr: *mut u8, byte_count: usize) -> PgValue<'a> {
        unsafe {
            Self {
                raw_bytes: slice::from_raw_parts_mut(value_ptr, byte_count),
                oid: 0
            }
        }
    }

    pub(crate) fn with_oid(value_ptr: *mut u8, byte_count: usize, oid: u32) -> PgValue<'a> {
        unsafe {
            Self {
                raw_bytes: slice::from_raw_parts_mut(value_ptr, byte_count),
                oid
            }
        }
    }

    /// Get the bytes associated with this raw value
    pub fn bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    /// Get the type oid of the value represented by this raw value
    pub fn oid(&self) -> u32 {
        self.oid
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct PgValueProxy;

impl<'a> FamilyLt<'a> for PgValueProxy {
    type Out = PgValue<'a>;
}

/// The [OIDs] for a SQL type
///
/// [OIDs]: https://www.postgresql.org/docs/current/static/datatype-oid.html
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct PgTypeMetadata {
    /// The [OID] of `T`
    ///
    /// [OID]: https://www.postgresql.org/docs/current/static/datatype-oid.html
    pub oid: u32,
    /// The [OID] of `T[]`
    ///
    /// [OID]: https://www.postgresql.org/docs/current/static/datatype-oid.html
    pub array_oid: u32,
}

impl Queryable<(Oid, Oid), Pg> for PgTypeMetadata {
    type Row = (u32, u32);

    fn build((oid, array_oid): Self::Row) -> Self {
        PgTypeMetadata { oid, array_oid }
    }
}

impl Backend for Pg {
    type QueryBuilder = PgQueryBuilder;
    type BindCollector = RawBytesBindCollector<Pg>;
    type RawValue = PgValueProxy;
    type ByteOrder = NetworkEndian;
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
    type MetadataLookup = PgMetadataLookup;
}

impl SupportsReturningClause for Pg {}
impl SupportsDefaultKeyword for Pg {}
impl UsesAnsiSavepointSyntax for Pg {}
