//! The SQLite backend
use byteorder::NativeEndian;

use super::connection::SqliteValue;
use super::query_builder::SqliteQueryBuilder;
use crate::backend::*;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::sql_types::TypeMetadata;

/// The SQLite backend
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Sqlite;

/// Determines how a bind parameter is given to SQLite
///
/// Diesel deals with bind parameters after serialization as opaque blobs of
/// bytes. However, SQLite instead has several functions where it expects the
/// relevant C types.
///
/// The variants of this struct determine what bytes are expected from
/// `ToSql` impls.
#[allow(missing_debug_implementations)]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SqliteType {
    /// Bind using `sqlite3_bind_blob`
    Binary,
    /// Bind using `sqlite3_bind_text`
    Text,
    /// `bytes` should contain an `f32`
    Float,
    /// `bytes` should contain an `f64`
    Double,
    /// `bytes` should contain an `i16`
    SmallInt,
    /// `bytes` should contain an `i32`
    Integer,
    /// `bytes` should contain an `i64`
    Long,
}

impl SqliteType {
    pub(super) fn from_raw_sqlite(tpe: i32) -> Option<SqliteType> {
        use libsqlite3_sys as ffi;

        match tpe {
            ffi::SQLITE_TEXT => Some(SqliteType::Text),
            ffi::SQLITE_INTEGER => Some(SqliteType::Long),
            ffi::SQLITE_FLOAT => Some(SqliteType::Double),
            ffi::SQLITE_BLOB => Some(SqliteType::Binary),
            ffi::SQLITE_NULL => None,
            _ => unreachable!(
                "Sqlite's documentation state that this case ({}) is not reachable. \
                 If you ever see this error message please open an issue at \
                 https://github.com/diesel-rs/diesel."
            ),
        }
    }
}

impl Backend for Sqlite {
    type QueryBuilder = SqliteQueryBuilder;
    type BindCollector = RawBytesBindCollector<Sqlite>;
    type ByteOrder = NativeEndian;
}

impl<'a> HasRawValue<'a> for Sqlite {
    type RawValue = SqliteValue<'a, 'a>;
}

impl TypeMetadata for Sqlite {
    type TypeMetadata = SqliteType;
    type MetadataLookup = ();
}

impl SupportsOnConflictClause for Sqlite {}
impl UsesAnsiSavepointSyntax for Sqlite {}
