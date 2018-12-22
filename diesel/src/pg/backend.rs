//! The PostgreSQL backend

use byteorder::NetworkEndian;

use super::query_builder::PgQueryBuilder;
use super::PgMetadataLookup;
use backend::*;
use prelude::Queryable;
use query_builder::bind_collector::RawBytesBindCollector;
use sql_types::{Oid, TypeMetadata};

use std::slice;
use std::ptr::NonNull;

/// The PostgreSQL backend
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Pg;

#[derive(Debug)]
pub struct PgValue<'a> {
    raw_bytes: &'a [u8],
    oid: u32,
}

impl PgValue {
    pub unsafe fn new(value_ptr: *mut u8, byte_count: usize) -> PgValue {
        let slice = slice::from_raw_parts_mut(value_ptr, byte_count);
        Self {
            raw_bytes: NonNull::new(slice).expect("Cannot be null"),
            oid: 0 // TODO FIXFIXFIX
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.raw_bytes.as_ref() }
    }

    pub fn oid(&self) -> i32 {
        self.oid
    }
}

impl<'a> FamilyLt<'a> for PgValue {
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
    type RawValue = PgValue;
    type ByteOrder = NetworkEndian;
}

impl TypeMetadata for Pg {
    type TypeMetadata = PgTypeMetadata;
    type MetadataLookup = PgMetadataLookup;
}

impl SupportsReturningClause for Pg {}
impl SupportsDefaultKeyword for Pg {}
impl UsesAnsiSavepointSyntax for Pg {}
