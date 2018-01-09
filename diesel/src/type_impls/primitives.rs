use std::error::Error;
use std::io::Write;

use backend::Backend;
use deserialize::{self, FromSql, FromSqlRow, Queryable};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::{self, BigInt, Binary, Bool, Double, Float, Integer, NotNull, SmallInt, Text};

#[allow(dead_code)]
mod foreign_impls {
    use super::*;

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Bool"]
    struct BoolProxy(bool);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "mysql", sql_type = "::sql_types::Tinyint")]
    struct I8Proxy(i8);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "SmallInt"]
    struct I16Proxy(i16);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Integer"]
    struct I32Proxy(i32);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "BigInt"]
    struct I64Proxy(i64);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "postgres", sql_type = "::sql_types::Oid")]
    struct U32Proxy(u32);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Float"]
    struct F32Proxy(f32);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Double"]
    struct F64Proxy(f64);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Text"]
    #[cfg_attr(feature = "sqlite", sql_type = "::sql_types::Date")]
    #[cfg_attr(feature = "sqlite", sql_type = "::sql_types::Time")]
    #[cfg_attr(feature = "sqlite", sql_type = "::sql_types::Timestamp")]
    struct StringProxy(String);

    #[derive(AsExpression)]
    #[diesel(foreign_derive, not_sized)]
    #[sql_type = "Text"]
    #[cfg_attr(feature = "sqlite", sql_type = "::sql_types::Date")]
    #[cfg_attr(feature = "sqlite", sql_type = "::sql_types::Time")]
    #[cfg_attr(feature = "sqlite", sql_type = "::sql_types::Timestamp")]
    struct StrProxy(str);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    struct VecProxy<T>(Vec<T>);

    #[derive(AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Binary"]
    struct BinaryVecProxy(Vec<u8>);

    #[derive(AsExpression)]
    #[diesel(foreign_derive, not_sized)]
    #[sql_type = "Binary"]
    struct BinarySliceProxy([u8]);
}

impl NotNull for () {}

impl<DB: Backend<RawValue = [u8]>> FromSql<sql_types::Text, DB> for String {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);
        String::from_utf8(bytes.into()).map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend> ToSql<sql_types::Text, DB> for str {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for String
where
    DB: Backend,
    str: ToSql<sql_types::Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        (self as &str).to_sql(out)
    }
}

impl<DB: Backend<RawValue = [u8]>> FromSql<sql_types::Binary, DB> for Vec<u8> {
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        Ok(not_none!(bytes).into())
    }
}

impl<DB> ToSql<sql_types::Binary, DB> for Vec<u8>
where
    DB: Backend,
    [u8]: ToSql<sql_types::Binary, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        (self as &[u8]).to_sql(out)
    }
}

impl<DB: Backend> ToSql<sql_types::Binary, DB> for [u8] {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_all(self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

use std::borrow::{Cow, ToOwned};
impl<'a, T: ?Sized, ST, DB> ToSql<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned + ToSql<ST, DB>,
    DB: Backend,
    T::Owned: ToSql<ST, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        match *self {
            Cow::Borrowed(t) => t.to_sql(out),
            Cow::Owned(ref t) => t.to_sql(out),
        }
    }
}

impl<'a, T: ?Sized, ST, DB> FromSql<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    DB: Backend,
    T::Owned: FromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        T::Owned::from_sql(bytes).map(Cow::Owned)
    }
}

impl<'a, T: ?Sized, ST, DB> FromSqlRow<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    DB: Backend,
    Cow<'a, T>: FromSql<ST, DB>,
{
    fn build_from_row<R: ::row::Row<DB>>(row: &mut R) -> deserialize::Result<Self> {
        FromSql::<ST, DB>::from_sql(row.take())
    }
}

impl<'a, T: ?Sized, ST, DB> Queryable<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    DB: Backend,
    Self: FromSqlRow<ST, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> Self {
        row
    }
}

use expression::bound::Bound;
use expression::{AsExpression, Expression};

impl<'a, T: ?Sized, ST> AsExpression<ST> for Cow<'a, T>
where
    T: 'a + ToOwned,
    Bound<ST, Cow<'a, T>>: Expression<SqlType = ST>,
{
    type Expression = Bound<ST, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'a, 'b, T: ?Sized, ST> AsExpression<ST> for &'b Cow<'a, T>
where
    T: 'a + ToOwned,
    Bound<ST, &'b T>: Expression<SqlType = ST>,
{
    type Expression = Bound<ST, &'b T>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(&**self)
    }
}
