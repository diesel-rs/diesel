use std::error::Error;
use std::io::Write;

use backend::Backend;
use types::{self, BigInt, Binary, Bool, Date, Double, Float, FromSql, HasSqlType, Integer, IsNull,
            NotNull, SmallInt, Text, Time, Timestamp, ToSql, ToSqlOutput};

primitive_impls!(Bool -> (pg: (16, 1000), sqlite: (Integer), mysql: (Tiny)));

primitive_impls!(SmallInt -> (pg: (21, 1005), sqlite: (SmallInt), mysql: (Short)));
primitive_impls!(Integer -> (pg: (23, 1007), sqlite: (Integer), mysql: (Long)));
primitive_impls!(BigInt -> (pg: (20, 1016), sqlite: (Long), mysql: (LongLong)));

primitive_impls!(Float -> (pg: (700, 1021), sqlite: (Float), mysql: (Float)));
primitive_impls!(Double -> (pg: (701, 1022), sqlite: (Double), mysql: (Double)));

primitive_impls!(Text -> (pg: (25, 1009), sqlite: (Text), mysql: (String)));

primitive_impls!(Binary -> (pg: (17, 1001), sqlite: (Binary), mysql: (Blob)));

primitive_impls!(Date -> (pg: (1082, 1182)));
primitive_impls!(Time -> (pg: (1083, 1183)));
primitive_impls!(Timestamp -> (pg: (1114, 1115)));

#[allow(dead_code)]
mod foreign_impls {
    use super::*;

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[sql_type = "Bool"]
    struct BoolProxy(bool);

    #[derive(FromSqlRow, AsExpression)]
    #[diesel(foreign_derive)]
    #[cfg_attr(feature = "mysql", sql_type = "types::Tinyint")]
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
    #[cfg_attr(feature = "postgres", sql_type = "::types::Oid")]
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
    #[cfg_attr(feature = "sqlite", sql_type = "Date")]
    #[cfg_attr(feature = "sqlite", sql_type = "Time")]
    #[cfg_attr(feature = "sqlite", sql_type = "Timestamp")]
    struct StringProxy(String);

    #[derive(AsExpression)]
    #[diesel(foreign_derive, not_sized)]
    #[sql_type = "Text"]
    #[cfg_attr(feature = "sqlite", sql_type = "Date")]
    #[cfg_attr(feature = "sqlite", sql_type = "Time")]
    #[cfg_attr(feature = "sqlite", sql_type = "Timestamp")]
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

impl<DB: Backend<RawValue = [u8]>> FromSql<types::Text, DB> for String {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        let bytes = not_none!(bytes);
        String::from_utf8(bytes.into()).map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend> ToSql<types::Text, DB> for str {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB> ToSql<types::Text, DB> for String
where
    DB: Backend,
    str: ToSql<types::Text, DB>,
{
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        (self as &str).to_sql(out)
    }
}

impl<DB: Backend<RawValue = [u8]>> FromSql<types::Binary, DB> for Vec<u8> {
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error + Send + Sync>> {
        Ok(not_none!(bytes).into())
    }
}

impl<DB> ToSql<types::Binary, DB> for Vec<u8>
where
    DB: Backend,
    [u8]: ToSql<types::Binary, DB>,
{
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        (self as &[u8]).to_sql(out)
    }
}

impl<DB: Backend> ToSql<types::Binary, DB> for [u8] {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        out.write_all(self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

use std::borrow::{Cow, ToOwned};
impl<'a, T: ?Sized, ST, DB> ToSql<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned + ToSql<ST, DB>,
    DB: Backend + HasSqlType<ST>,
    T::Owned: ToSql<ST, DB>,
{
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        match *self {
            Cow::Borrowed(t) => t.to_sql(out),
            Cow::Owned(ref t) => t.to_sql(out),
        }
    }
}

impl<'a, T: ?Sized, ST, DB> FromSql<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    DB: Backend + HasSqlType<ST>,
    T::Owned: FromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error + Send + Sync>> {
        T::Owned::from_sql(bytes).map(Cow::Owned)
    }
}

impl<'a, T: ?Sized, ST, DB> ::types::FromSqlRow<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    DB: Backend + HasSqlType<ST>,
    Cow<'a, T>: FromSql<ST, DB>,
{
    fn build_from_row<R: ::row::Row<DB>>(row: &mut R) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<ST, DB>::from_sql(row.take())
    }
}

impl<'a, T: ?Sized, ST, DB> ::Queryable<ST, DB> for Cow<'a, T>
where
    T: 'a + ToOwned,
    DB: Backend + HasSqlType<ST>,
    Self: ::types::FromSqlRow<ST, DB>,
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
