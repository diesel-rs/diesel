use std::error::Error;
use std::io::Write;

use backend::{Backend, Pg};
use data_types::PgNumeric;
use expression::bound::Bound;
use expression::AsExpression;
use super::option::UnexpectedNullError;
use types::{HasSqlType, FromSql, ToSql, IsNull, NotNull};
use {Queryable, types};

primitive_impls! {
    Bool -> (bool, 16, 1000),

    SmallInt -> (i16, 21, 1005),
    Integer -> (i32, 23, 1007),
    BigInt -> (i64, 20, 1016),

    Oid -> (u32, 26, 1018),

    Float -> (f32, 700, 1021),
    Double -> (f64, 701, 1022),
    Numeric -> (PgNumeric, 1700, 1231),

    VarChar -> (String, 1043, 1015),
    Text -> (String, 25, 1009),

    Binary -> (Vec<u8>, 17, 1001),
}

expression_impls! {
    VarChar -> &'a str,
    Text -> &'a str,

    Binary -> &'a [u8],
}

impl NotNull for () {}

impl FromSql<types::Bool, Pg> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        match bytes {
            Some(bytes) => Ok(bytes[0] != 0),
            None => Ok(false),
        }
    }
}

impl ToSql<types::Bool, Pg> for bool {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let write_result = if *self {
            out.write_all(&[1])
        } else {
            out.write_all(&[0])
        };
        write_result
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB: Backend> FromSql<types::VarChar, DB> for String {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let bytes = not_none!(bytes);
        String::from_utf8(bytes.into()).map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB> ToSql<types::VarChar, DB> for String where
    DB: Backend,
    for<'a> &'a str: ToSql<types::VarChar, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        (&self as &str).to_sql(out)
    }
}

impl<'a, DB: Backend> ToSql<types::VarChar, DB> for &'a str {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<DB> FromSql<types::Text, DB> for String where
    DB: Backend,
    String: FromSql<types::VarChar, DB>,
{
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::VarChar, DB>>::from_sql(bytes)
    }
}

impl<DB> ToSql<types::Text, DB> for String where
    DB: Backend,
    for<'a> &'a str: ToSql<types::Text, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        (&self as &str).to_sql(out)
    }
}

impl<'a, DB> ToSql<types::Text, DB> for &'a str where
    DB: Backend,
    &'a str: ToSql<types::VarChar, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::VarChar, DB>::to_sql(self, out)
    }
}

impl<DB: Backend> FromSql<types::Binary, DB> for Vec<u8> {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        Ok(not_none!(bytes).into())
    }
}

impl<DB> ToSql<types::Binary, DB> for Vec<u8> where
    DB: Backend,
    for<'a> &'a [u8]: ToSql<types::Binary, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        (&self as &[u8]).to_sql(out)
    }
}

impl<'a, DB: Backend> ToSql<types::Binary, DB> for &'a [u8] {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_all(self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

use std::borrow::{Cow, ToOwned};
impl<'a, T: ?Sized, ST, DB> ToSql<ST, DB> for Cow<'a, T> where
    T: 'a + ToOwned + ToSql<ST, DB>,
    DB: Backend + HasSqlType<ST>,
    T::Owned: ToSql<ST, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        match self {
            &Cow::Borrowed(ref t) => t.to_sql(out),
            &Cow::Owned(ref t) => t.to_sql(out),
        }
    }
}

impl<'a, T: ?Sized, ST, DB> FromSql<ST, DB> for Cow<'a, T> where
    T: 'a + ToOwned,
    DB: Backend + HasSqlType<ST>,
    T::Owned: FromSql<ST, DB>,
{
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        T::Owned::from_sql(bytes).map(Cow::Owned)
    }
}

impl <'a, T: ?Sized, ST, DB> ::types::FromSqlRow<ST, DB> for Cow<'a, T> where
    T: 'a + ToOwned,
    DB: Backend + HasSqlType<ST>,
    Cow<'a, T>: FromSql<ST, DB>,
{
    fn build_from_row<R: ::row::Row>(row: &mut R) -> Result<Self, Box<Error>> {
        FromSql::<ST, DB>::from_sql(row.take())
    }
}

#[test]
fn bool_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::Bool, Pg>::to_sql(&true, &mut bytes).unwrap();
    ToSql::<types::Bool, Pg>::to_sql(&false, &mut bytes).unwrap();
    assert_eq!(bytes, vec![1u8, 0u8]);
}

#[test]
fn bool_from_sql_treats_null_as_false() {
    let result = <bool as FromSql<types::Bool, Pg>>::from_sql(None).unwrap();
    assert!(!result);
}
