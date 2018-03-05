//! MySQL specific types

#[cfg(feature = "chrono")]
mod date_and_time;
mod numeric;

use std::io::Write;
use byteorder::{ReadBytesExt, WriteBytesExt};

use row::Row;
use deserialize::{self, FromSql, FromSqlRow, Queryable};
use backend::Backend;
use mysql::{Mysql, MysqlType};
use serialize::{self, IsNull, Output, ToSql};
use expression::{AppearsOnTable, Expression, SelectableExpression};
use query_builder::{AstPass, QueryFragment, QueryId};
use result::QueryResult;
use sql_types::*;

impl ToSql<Tinyint, Mysql> for i8 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        out.write_i8(*self).map(|_| IsNull::No).map_err(Into::into)
    }
}

impl FromSql<Tinyint, Mysql> for i8 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        Ok(bytes[0] as i8)
    }
}

/// Represents the MySQL unsigned type.
#[derive(Debug, Clone, Copy, Default, SqlType)]
pub struct Unsigned<ST: NotNull + SingleValue + QueryId>(ST);

impl ToSql<Unsigned<Integer>, Mysql> for u16 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        out.write_u16::<<Mysql as Backend>::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

impl FromSql<Unsigned<SmallInt>, Mysql> for u16 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 2,
            "Received more than 2 bytes decoding u16. \
             Was a Integer expression accidentally identified as SmallInt?"
        );
        debug_assert!(
            bytes.len() >= 2,
            "Received fewer than 2 bytes decoding u16. \
             Was the expression accidentally identified as SmallInt?"
        );
        bytes
            .read_u16::<<Mysql as Backend>::ByteOrder>()
            .map_err(Into::into)
    }
}

impl ToSql<Unsigned<Integer>, Mysql> for u32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        out.write_u32::<<Mysql as Backend>::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

impl FromSql<Unsigned<Integer>, Mysql> for u32 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 4,
            "Received more than 4 bytes decoding u32. \
             Was a BigInteger expression accidentally identified as Integer?"
        );
        debug_assert!(
            bytes.len() >= 4,
            "Received fewer than 4 bytes decoding u32. \
             Was a SmallInteger expression accidentally identified as Integer?"
        );
        bytes
            .read_u32::<<Mysql as Backend>::ByteOrder>()
            .map_err(Into::into)
    }
}

impl ToSql<Unsigned<Integer>, Mysql> for u64 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        out.write_u64::<<Mysql as Backend>::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}

impl FromSql<Unsigned<BigInt>, Mysql> for u64 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 8,
            "Received more than 8 bytes decoding u64. \
             Was the expression accidentally identified as BigInt?"
        );
        debug_assert!(
            bytes.len() >= 8,
            "Received fewer than 8 bytes decoding u64. \
             Was a Integer expression accidentally identified as BigInt?"
        );
        bytes
            .read_u64::<<Mysql as Backend>::ByteOrder>()
            .map_err(Into::into)
    }
}

impl ToSql<Bool, Mysql> for bool {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Mysql>) -> serialize::Result {
        let int_value = if *self { 1 } else { 0 };
        <i32 as ToSql<Integer, Mysql>>::to_sql(&int_value, out)
    }
}

impl FromSql<Bool, Mysql> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        Ok(not_none!(bytes).iter().any(|x| *x != 0))
    }
}

impl<ST> HasSqlType<Unsigned<ST>> for Mysql
where
    ST: NotNull + SingleValue + QueryId,
    Mysql: HasSqlType<ST>,
{
    fn metadata(lookup: &()) -> MysqlType {
        <Mysql as HasSqlType<ST>>::metadata(lookup)
    }
}

impl<ST> QueryId for Unsigned<ST>
where
    ST: NotNull + SingleValue + QueryId,
    <ST as QueryId>::QueryId: SingleValue + NotNull + QueryId,
{
    type QueryId = Unsigned<<ST as QueryId>::QueryId>;
}

impl QueryFragment<Mysql> for u16 {
    fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
        let value = *self as i16;
        out.push_sql(&value.to_string());
        Ok(())
    }
}

impl QueryFragment<Mysql> for u32 {
    fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
        let value = *self as i32;
        out.push_sql(&value.to_string());
        Ok(())
    }
}

impl QueryFragment<Mysql> for u64 {
    fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
        let value = *self as i64;
        out.push_sql(&value.to_string());
        Ok(())
    }
}

impl Expression for u16 {
    type SqlType = Unsigned<SmallInt>;
}

impl Expression for u32 {
    type SqlType = Unsigned<Integer>;
}

impl Expression for u64 {
    type SqlType = Unsigned<BigInt>;
}

impl SelectableExpression<()> for u16 {}
impl SelectableExpression<()> for u32 {}
impl SelectableExpression<()> for u64 {}

impl AppearsOnTable<()> for u16 {}
impl AppearsOnTable<()> for u32 {}
impl AppearsOnTable<()> for u64 {}

impl QueryId for u16 {
    type QueryId = ();
}

impl QueryId for u32 {
    type QueryId = ();
}

impl QueryId for u64 {
    type QueryId = ();
}

impl Queryable<Unsigned<SmallInt>, Mysql> for u16 {
    type Row = u16;

    fn build(row: Self::Row) -> Self {
        row
    }
}

impl Queryable<Unsigned<BigInt>, Mysql> for u64 {
    type Row = u64;

    fn build(row: Self::Row) -> Self {
        row
    }
}

impl FromSqlRow<Unsigned<SmallInt>, Mysql> for u16 {
    fn build_from_row<T: Row<Mysql>>(row: &mut T) -> deserialize::Result<Self> {
        Self::from_sql(row.take())
    }
}

impl FromSqlRow<Unsigned<BigInt>, Mysql> for u64 {
    fn build_from_row<T: Row<Mysql>>(row: &mut T) -> deserialize::Result<Self> {
        Self::from_sql(row.take())
    }
}

/// Represents the MySQL datetime type.
///
/// ### [`ToSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
///
/// ### [`FromSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
///
/// [`ToSql`]: ../../serialize/trait.ToSql.html
/// [`FromSql`]: ../../deserialize/trait.FromSql.html
/// [`chrono::NaiveDateTime`]: ../../../chrono/naive/struct.NaiveDateTime.html
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[mysql_type = "DateTime"]
pub struct Datetime;
