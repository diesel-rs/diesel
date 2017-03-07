use std::error::Error;
use std::io::Write;

use backend::Backend;
use types::{self, HasSqlType, FromSql, ToSql, IsNull, NotNull};

primitive_impls!(Bool -> (bool, pg: (16, 1000), sqlite: (Integer), mysql: (Tiny)));

primitive_impls!(SmallInt -> (i16, pg: (21, 1005), sqlite: (SmallInt), mysql: (Short)));
primitive_impls!(Integer -> (i32, pg: (23, 1007), sqlite: (Integer), mysql: (Long)));
primitive_impls!(BigInt -> (i64, pg: (20, 1016), sqlite: (Long), mysql: (LongLong)));
primitive_impls!(UInt2 -> (u16, mysql: (Short)));
primitive_impls!(UInt4 -> (u32, mysql: (Long)));
primitive_impls!(UInt8 -> (u64, mysql: (LongLong)));

primitive_impls!(Float -> (f32, pg: (700, 1021), sqlite: (Float), mysql: (Float)));
primitive_impls!(Double -> (f64, pg: (701, 1022), sqlite: (Double), mysql: (Double)));

primitive_impls!(Text -> (String, pg: (25, 1009), sqlite: (Text), mysql: (String)));

primitive_impls!(Binary -> (Vec<u8>, pg: (17, 1001), sqlite: (Binary), mysql: (Blob)));

primitive_impls!(Date);
primitive_impls!(Time);
primitive_impls!(Timestamp);

expression_impls! {
    Text -> &'a str,

    Binary -> &'a [u8],
}

impl NotNull for () {}

impl<DB: Backend<RawValue=[u8]>> FromSql<types::Text, DB> for String {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
        let bytes = not_none!(bytes);
        String::from_utf8(bytes.into()).map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

impl<'a, DB: Backend> ToSql<types::Text, DB> for &'a str {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

impl<DB> ToSql<types::Text, DB> for String where
    DB: Backend,
    for<'a> &'a str: ToSql<types::Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        (self as &str).to_sql(out)
    }
}

impl<DB: Backend<RawValue=[u8]>> FromSql<types::Binary, DB> for Vec<u8> {
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error+Send+Sync>> {
        Ok(not_none!(bytes).into())
    }
}

impl<DB> ToSql<types::Binary, DB> for Vec<u8> where
    DB: Backend,
    for<'a> &'a [u8]: ToSql<types::Binary, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        (self as &[u8]).to_sql(out)
    }
}

impl<'a, DB: Backend> ToSql<types::Binary, DB> for &'a [u8] {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        out.write_all(self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error+Send+Sync>)
    }
}

use std::borrow::{Cow, ToOwned};
impl<'a, T: ?Sized, ST, DB> ToSql<ST, DB> for Cow<'a, T> where
    T: 'a + ToOwned + ToSql<ST, DB>,
    DB: Backend + HasSqlType<ST>,
    T::Owned: ToSql<ST, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        match *self {
            Cow::Borrowed(t) => t.to_sql(out),
            Cow::Owned(ref t) => t.to_sql(out),
        }
    }
}

impl<'a, T: ?Sized, ST, DB> FromSql<ST, DB> for Cow<'a, T> where
    T: 'a + ToOwned,
    DB: Backend + HasSqlType<ST>,
    T::Owned: FromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> Result<Self, Box<Error+Send+Sync>> {
        T::Owned::from_sql(bytes).map(Cow::Owned)
    }
}

impl <'a, T: ?Sized, ST, DB> ::types::FromSqlRow<ST, DB> for Cow<'a, T> where
    T: 'a + ToOwned,
    DB: Backend + HasSqlType<ST>,
    Cow<'a, T>: FromSql<ST, DB>,
{
    fn build_from_row<R: ::row::Row<DB>>(row: &mut R) -> Result<Self, Box<Error+Send+Sync>> {
        FromSql::<ST, DB>::from_sql(row.take())
    }
}
