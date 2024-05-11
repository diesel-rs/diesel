use std::error::Error;
use std::io::Write;

use crate::backend::Backend;
use crate::deserialize::{self, FromSql, Queryable};
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::{
    self, BigInt, Binary, Bool, Double, Float, Integer, SingleValue, SmallInt, Text,
};
use std::borrow::Cow;
use std::fmt;

#[allow(dead_code)]
mod foreign_impls {
    use super::*;
    use crate::deserialize::FromSqlRow;

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Bool)]
    struct BoolProxy(bool);

    #[derive(FromSqlRow)]
    #[cfg_attr(feature = "mysql_backend", derive(AsExpression))]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "mysql_backend", diesel(sql_type = crate::sql_types::TinyInt))]
    struct I8Proxy(i8);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = SmallInt)]
    struct I16Proxy(i16);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Integer)]
    struct I32Proxy(i32);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = BigInt)]
    struct I64Proxy(i64);

    #[derive(FromSqlRow)]
    #[cfg_attr(
        any(feature = "mysql_backend", feature = "postgres_backend"),
        derive(AsExpression)
    )]
    #[diesel(foreign_derive)]
    #[cfg_attr(
        feature = "mysql_backend",
        diesel(sql_type = crate::sql_types::Unsigned<crate::sql_types::TinyInt>)
    )]
    #[cfg_attr(feature = "postgres_backend", diesel(foreign_derive, sql_type = crate::sql_types::CChar))]
    struct U8Proxy(u8);

    #[derive(FromSqlRow)]
    #[cfg_attr(feature = "mysql_backend", derive(AsExpression))]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "mysql_backend", diesel(sql_type = crate::sql_types::Unsigned<SmallInt>))]
    struct U16Proxy(u16);

    #[derive(FromSqlRow)]
    #[cfg_attr(
        any(feature = "mysql_backend", feature = "postgres_backend"),
        derive(AsExpression)
    )]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "mysql_backend", diesel(sql_type = crate::sql_types::Unsigned<Integer>))]
    #[cfg_attr(feature = "postgres_backend", diesel(sql_type = crate::sql_types::Oid))]
    struct U32Proxy(u32);

    #[derive(FromSqlRow)]
    #[cfg_attr(feature = "mysql_backend", derive(AsExpression))]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "mysql_backend", diesel(sql_type = crate::sql_types::Unsigned<BigInt>))]
    struct U64Proxy(u64);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Float)]
    struct F32Proxy(f32);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Double)]
    struct F64Proxy(f64);

    #[derive(AsExpression, FromSqlRow)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Text)]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = crate::sql_types::Date))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = crate::sql_types::Time))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = crate::sql_types::Timestamp))]
    #[cfg_attr(feature = "postgres_backend", diesel(sql_type = crate::sql_types::Citext))]
    struct StringProxy(String);

    #[derive(AsExpression)]
    #[diesel(foreign_derive, not_sized)]
    #[diesel(sql_type = Text)]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = crate::sql_types::Date))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = crate::sql_types::Time))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = crate::sql_types::Timestamp))]
    #[cfg_attr(feature = "postgres_backend", diesel(sql_type = crate::sql_types::Citext))]
    struct StrProxy(str);

    #[derive(FromSqlRow)]
    #[diesel(foreign_derive)]
    struct VecProxy<T>(Vec<T>);

    #[derive(AsExpression)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Binary)]
    struct BinaryVecProxy(Vec<u8>);

    #[derive(AsExpression)]
    #[diesel(foreign_derive, not_sized)]
    #[diesel(sql_type = Binary)]
    struct BinarySliceProxy([u8]);

    #[derive(AsExpression)]
    #[diesel(foreign_derive)]
    #[diesel(sql_type = Binary)]
    struct BinaryArrayProxy<const N: usize>([u8; N]);
}

impl<ST, DB> FromSql<ST, DB> for String
where
    DB: Backend,
    *const str: FromSql<ST, DB>,
{
    #[allow(unsafe_code)] // ptr dereferencing
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let str_ptr = <*const str as FromSql<ST, DB>>::from_sql(bytes)?;
        // We know that the pointer impl will never return null
        let string = unsafe { &*str_ptr };
        Ok(string.to_owned())
    }
}

impl<DB> ToSql<sql_types::Text, DB> for str
where
    for<'a> DB: Backend<BindCollector<'a> = RawBytesBindCollector<DB>>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

impl<DB> ToSql<sql_types::Text, DB> for String
where
    DB: Backend,
    str: ToSql<sql_types::Text, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        (self as &str).to_sql(out)
    }
}

impl<ST, DB> FromSql<ST, DB> for Vec<u8>
where
    DB: Backend,
    *const [u8]: FromSql<ST, DB>,
{
    #[allow(unsafe_code)] // ptr dereferencing
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let slice_ptr = <*const [u8] as FromSql<ST, DB>>::from_sql(bytes)?;
        // We know that the pointer impl will never return null
        let bytes = unsafe { &*slice_ptr };
        Ok(bytes.to_owned())
    }
}

impl<DB> ToSql<sql_types::Binary, DB> for Vec<u8>
where
    DB: Backend,
    [u8]: ToSql<sql_types::Binary, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        (self as &[u8]).to_sql(out)
    }
}

impl<DB, const N: usize> ToSql<sql_types::Binary, DB> for [u8; N]
where
    DB: Backend,
    [u8]: ToSql<sql_types::Binary, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.as_slice().to_sql(out)
    }
}

impl<DB> ToSql<sql_types::Binary, DB> for [u8]
where
    for<'a> DB: Backend<BindCollector<'a> = RawBytesBindCollector<DB>>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_all(self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

impl<'a, T: ?Sized, ST, DB> ToSql<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned + ToSql<ST, DB>,
    DB: Backend,
    Self: fmt::Debug,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        ToSql::<ST, DB>::to_sql(&**self, out)
    }
}

impl<'a, T: ?Sized, ST, DB> FromSql<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    DB: Backend,
    T::Owned: FromSql<ST, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        T::Owned::from_sql(bytes).map(Cow::Owned)
    }
}

impl<'a, T: ?Sized, ST, DB> Queryable<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    ST: SingleValue,
    DB: Backend,
    Self: FromSql<ST, DB>,
{
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

use crate::expression::bound::Bound;
use crate::expression::{AsExpression, Expression, TypedExpressionType};
use sql_types::SqlType;

impl<'a, T: ?Sized, ST> AsExpression<ST> for Cow<'a, T>
where
    T: 'a + ToOwned,
    Bound<ST, Cow<'a, T>>: Expression<SqlType = ST>,
    ST: SqlType + TypedExpressionType,
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
    ST: SqlType + TypedExpressionType,
{
    type Expression = Bound<ST, &'b T>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(&**self)
    }
}
