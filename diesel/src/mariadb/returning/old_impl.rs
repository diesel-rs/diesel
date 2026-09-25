//! `UPDATE ... RETURNING OLD_VALUE(col)` support for MariaDB 13.0 and later.

use crate::backend::Backend;
use crate::deserialize::{self, FromSql, FromSqlRef};
use crate::expression::{
    AppearsOnTable, Expression, SelectableExpression, ValidGrouping, is_aggregate,
};
use crate::mariadb::Mariadb;
use crate::query_builder::returning::{OldIdent, ReturningQuerySource, UpdateStmt};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_source::{AppearsInFromClause, Column};
use crate::result::QueryResult;
use crate::sql_types::{self, HasSqlType, SingleValue, SqlType, TypeMetadata};
use core::marker::PhantomData;

/// Wraps a column to refer to its pre-modification value in the `RETURNING`
/// clause of a Mariadb `UPDATE` statement.
///
/// This is the type returned by [`old_value()`](old_value()).
#[derive(Debug, Clone, Copy, QueryId)]
pub struct OldValue<C> {
    _column: C,
}

impl<C> OldValue<C> {
    pub(crate) fn new(c: C) -> Self {
        OldValue { _column: c }
    }
}

/// Refer to the pre-modification value of `col` in a Mariadb `RETURNING`
/// clause.
///
/// This corresponds to the SQL `RETURNING OLD_VALUE(col)` syntax introduced in
/// Mariadb 13.0.
///
/// # Requires Mariadb 13.0 or newer
///
/// Diesel emits `OLD_VALUE(col)` in the SQL it sends to the database. Earlier
/// versions of Mariadb will reject the query at execution time.
///
/// # Statement compatibility
///
/// `old_value(col)` is valid inside the `RETURNING` clause of:
///
/// * an `UPDATE` statement, as a whole `RETURNING` item that loads into the
///   same Rust types as `col` (since every returned row necessarily came from a
///   pre-existing row).
///
/// Use of `old_value(col)` in `INSERT` or `DELETE` `RETURNING`
/// is rejected at compile time, because it is invalid
/// there. (Note that `ON CONFLICT DO NOTHING` never returns untouched rows.)
///
/// # Example
///
/// ```rust
/// # include!("../../doctest_setup.rs");
/// #
/// # #[cfg(feature = "mariadb")]
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     use diesel::mariadb::returning::old_value;
/// #     let connection = &mut establish_connection();
/// #     // `RETURNING OLD_VALUE(col)` requires Mariadb 13.0+
/// #     if !mariadb_server_supports_update_returning(connection) { return; }
/// let was_and_now = diesel::update(users.find(1))
///     .set(name.eq("Updated"))
///     .returning((old_value(name), name))
///     .get_result::<(String, String)>(connection);
/// assert_eq!(Ok(("Sean".to_string(), "Updated".to_string())), was_and_now);
/// # }
/// # #[cfg(not(feature = "mariadb"))]
/// # fn main() {}
/// ```
pub fn old_value<C: Column>(col: C) -> old_value<C> {
    OldValue::new(col)
}

impl<C> Expression for OldValue<C>
where
    C: Column + Expression,
    C::SqlType: SingleValue,
{
    type SqlType = OldValueOf<<C as Expression>::SqlType>;
}

/// SQL type of [`old_value(col)`](old_value()), loaded like `ST` but accepted by
/// no literal, column or `ST`-typed function, the positions where
/// [MDEV-40126](https://jira.mariadb.org/browse/MDEV-40126) breaks `OLD_VALUE`.
/// `.is_null()` and `old_value(a).eq(old_value(b))` still compile, although
/// MariaDB rejects `OLD_VALUE` as an operand there too.
///
/// A type with a hand-written `FromSql<ST, Mariadb>` impl also needs
/// `FromSql<OldValueOf<ST>, Mariadb>` to load from `old_value(col)`, which
/// `#[derive(Enum)]` already generates.
#[derive(Debug, Clone, Copy, Default, QueryId)]
pub struct OldValueOf<ST>(PhantomData<ST>);

impl<ST: SqlType> SqlType for OldValueOf<ST> {
    type IsNull = ST::IsNull;
}

impl<ST: SingleValue> SingleValue for OldValueOf<ST> {}

impl<ST> HasSqlType<OldValueOf<ST>> for Mariadb
where
    Mariadb: HasSqlType<ST>,
{
    fn metadata(lookup: &mut Self::MetadataLookup) -> <Mariadb as TypeMetadata>::TypeMetadata {
        <Mariadb as HasSqlType<ST>>::metadata(lookup)
    }
}

// Mirrors every MySQL-like leaf impl, owned types reach them through blanket impls.
macro_rules! old_value_from_sql {
    ($($(#[$meta:meta])* $st:ty => $rust:ty),* $(,)?) => {$(
        $(#[$meta])*
        #[diagnostic::do_not_recommend]
        impl FromSql<OldValueOf<$st>, Mariadb> for $rust {
            fn from_sql(value: <Mariadb as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
                <$rust as FromSql<$st, Mariadb>>::from_sql(value)
            }
        }
    )*};
}

old_value_from_sql!(
    sql_types::TinyInt => i8,
    sql_types::SmallInt => i16,
    sql_types::Integer => i32,
    sql_types::BigInt => i64,
    sql_types::Unsigned<sql_types::TinyInt> => u8,
    sql_types::Unsigned<sql_types::SmallInt> => u16,
    sql_types::Unsigned<sql_types::Integer> => u32,
    sql_types::Unsigned<sql_types::BigInt> => u64,
    sql_types::Bool => bool,
    sql_types::Float => f32,
    sql_types::Double => f64,
    sql_types::Text => *const str,
    sql_types::Binary => *const [u8],
    sql_types::Datetime => crate::mysql_like::data_types::MysqlTime,
    sql_types::Timestamp => crate::mysql_like::data_types::MysqlTime,
    sql_types::Time => crate::mysql_like::data_types::MysqlTime,
    sql_types::Date => crate::mysql_like::data_types::MysqlTime,
    #[cfg(feature = "chrono")]
    sql_types::Datetime => chrono::NaiveDateTime,
    #[cfg(feature = "chrono")]
    sql_types::Timestamp => chrono::NaiveDateTime,
    #[cfg(feature = "chrono")]
    sql_types::Time => chrono::NaiveTime,
    #[cfg(feature = "chrono")]
    sql_types::Date => chrono::NaiveDate,
    #[cfg(feature = "time")]
    sql_types::Datetime => time::PrimitiveDateTime,
    #[cfg(feature = "time")]
    sql_types::Timestamp => time::PrimitiveDateTime,
    #[cfg(feature = "time")]
    sql_types::Datetime => time::OffsetDateTime,
    #[cfg(feature = "time")]
    sql_types::Timestamp => time::OffsetDateTime,
    #[cfg(feature = "time")]
    sql_types::Time => time::Time,
    #[cfg(feature = "time")]
    sql_types::Date => time::Date,
    #[cfg(feature = "numeric")]
    sql_types::Numeric => bigdecimal::BigDecimal,
    #[cfg(feature = "serde_json")]
    sql_types::Json => serde_json::Value,
);

macro_rules! old_value_from_sql_ref {
    ($($st:ty => $rust:ty),* $(,)?) => {$(
        #[diagnostic::do_not_recommend]
        impl<'a> FromSqlRef<'a, OldValueOf<$st>, Mariadb> for &'a $rust {
            fn from_sql(
                bytes: &'a mut <Mariadb as Backend>::RawValue<'_>,
            ) -> deserialize::Result<Self> {
                <&'a $rust as FromSqlRef<'a, $st, Mariadb>>::from_sql(bytes)
            }
        }
    )*};
}

old_value_from_sql_ref!(sql_types::Text => str, sql_types::Binary => [u8]);

#[diagnostic::do_not_recommend]
impl<T, ST> FromSql<OldValueOf<sql_types::Nullable<ST>>, Mariadb> for Option<T>
where
    T: FromSql<OldValueOf<ST>, Mariadb>,
    ST: SqlType<IsNull = sql_types::is_nullable::NotNull>,
{
    fn from_sql(value: <Mariadb as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        T::from_sql(value).map(Some)
    }

    fn from_nullable_sql(
        value: Option<<Mariadb as Backend>::RawValue<'_>>,
    ) -> deserialize::Result<Self> {
        value.map(T::from_sql).transpose()
    }
}

impl<C> ValidGrouping<()> for OldValue<C>
where
    C: Column,
{
    type IsAggregate = is_aggregate::No;
}

// `OldValue<C>` is selectable on a `RETURNING` clause whose statement-kind marker
// is `UpdateStmt`. Since `OLD_VALUE` is only valid in `UPDATE ... RETURNING`
//
// It's not selectable on any subqueries in the returning clause
impl<C, QS> AppearsOnTable<ReturningQuerySource<UpdateStmt, QS>> for OldValue<C>
where
    C: Column,
    Self: Expression,
    // Check that we have exactly one `old` identifier in the `RETURNING` clause.
    ReturningQuerySource<UpdateStmt, QS>:
        AppearsInFromClause<OldIdent, Count = crate::query_source::Once>,
    // Check that the `old` identifier relates the table of that column.
    ReturningQuerySource<UpdateStmt, QS>: AppearsInFromClause<
            ReturningQuerySource<OldIdent, C::Table>,
            Count = crate::query_source::Once,
        >,
{
}

// `old_value(col)` is only valid in `UPDATE ... RETURNING` for Mariadb,
// so we don't need to implement `SelectableExpression` for any other statement kinds.
impl<C> SelectableExpression<ReturningQuerySource<UpdateStmt, C::Table>> for OldValue<C>
where
    C: Column,
    Self: AppearsOnTable<ReturningQuerySource<UpdateStmt, C::Table>>,
{
}

impl<C> QueryFragment<Mariadb> for OldValue<C>
where
    C: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Mariadb>) -> QueryResult<()> {
        out.push_sql("OLD_VALUE(");
        out.push_identifier(C::NAME)?;
        out.push_sql(")");
        Ok(())
    }
}

pub use return_type_helpers_reexported::*;

pub(crate) mod return_type_helpers_reexported {
    use super::OldValue;

    /// The return type of [`old_value(col)`](super::old_value()).
    #[allow(non_camel_case_types)]
    pub type old_value<C> = OldValue<C>;
}
