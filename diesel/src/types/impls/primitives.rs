use std::error::Error;
use std::io::Write;

use data_types::PgNumeric;
use expression::bound::Bound;
use expression::{Expression, AsExpression};
use super::option::UnexpectedNullError;
use types::{NativeSqlType, FromSql, ToSql, IsNull, NotNull};
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

impl NativeSqlType for () {
    fn oid(&self) -> u32 {
        0
    }

    fn array_oid(&self) -> u32 {
        0
    }

    fn new() -> Self {
        ()
    }
}

impl NotNull for () {}

impl FromSql<types::Bool> for bool {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        match bytes {
            Some(bytes) => Ok(bytes[0] != 0),
            None => Ok(false),
        }
    }
}

impl ToSql<types::Bool> for bool {
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

impl FromSql<types::VarChar> for String {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let bytes = not_none!(bytes);
        String::from_utf8(bytes.into()).map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl ToSql<types::VarChar> for String {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<'a> ToSql<types::VarChar> for &'a str {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_all(self.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<types::Text> for String {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        <Self as FromSql<types::VarChar>>::from_sql(bytes)
    }
}

impl ToSql<types::Text> for String {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::VarChar>::to_sql(self, out)
    }
}

impl<'a> ToSql<types::Text> for &'a str {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        ToSql::<types::VarChar>::to_sql(self, out)
    }
}

impl FromSql<types::Binary> for Vec<u8> {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        Ok(not_none!(bytes).into())
    }
}

impl ToSql<types::Binary> for Vec<u8> {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_all(&self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl<'a> ToSql<types::Binary> for &'a [u8] {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        out.write_all(self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error>)
    }
}

use std::borrow::{Cow, ToOwned};
impl<'a, T: ?Sized, ST> ToSql<ST> for Cow<'a, T> where
    ST: NativeSqlType,
    T: 'a + ToOwned + ToSql<ST>,
    T::Owned: ToSql<ST>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        match self {
            &Cow::Borrowed(ref t) => t.to_sql(out),
            &Cow::Owned(ref t) => t.to_sql(out),
        }
    }
}

impl<'a, T: ?Sized, ST> FromSql<ST> for Cow<'a, T> where
    ST: NativeSqlType,
    T: 'a + ToOwned,
    T::Owned: FromSql<ST>,
{
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        T::Owned::from_sql(bytes).map(Cow::Owned)
    }
}

#[test]
fn bool_to_sql() {
    let mut bytes = vec![];
    ToSql::<types::Bool>::to_sql(&true, &mut bytes).unwrap();
    ToSql::<types::Bool>::to_sql(&false, &mut bytes).unwrap();
    assert_eq!(bytes, vec![1u8, 0u8]);
}

#[test]
fn bool_from_sql_treats_null_as_false() {
    let result = <bool as FromSql<types::Bool>>::from_sql(None).unwrap();
    assert!(!result);
}
