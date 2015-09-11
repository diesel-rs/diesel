extern crate byteorder;

use super::{NativeSqlType, FromSql, Nullable};
use Queriable;
use row::Row;

use self::byteorder::{ReadBytesExt, BigEndian};
use std::error::Error;
use std::fmt;

macro_rules! primitive_impls {
    ($($Source:ident -> $Target:ty),+,) => {
        $(
            impl NativeSqlType for super::$Source {}

            impl Queriable<super::$Source> for $Target {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }
        )+
    }
}

primitive_impls! {
    Bool -> bool,

    SmallSerial -> i16,
    Serial -> i32,
    BigSerial -> i64,

    SmallInt -> i16,
    Integer -> i32,
    BigInt -> i64,

    Float -> f32,
    Double -> f64,

    VarChar -> String,

    Binary -> Vec<u8>,
}

#[derive(Debug)]
struct UnexpectedNullError {
    msg: String,
}

impl fmt::Display for UnexpectedNullError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Error for UnexpectedNullError {
    fn description(&self) -> &str {
        &self.msg
    }
}

macro_rules! check_not_null {
    ($is_null:expr) => {
        if $is_null {
            return Err(Box::new(UnexpectedNullError {
                msg: "Unexpected null for non-null column".to_string(),
            }));
        }
    }
}


impl FromSql<super::Bool> for bool {
    fn from_sql(bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        Ok(bytes[0] != 0)
    }
}

impl FromSql<super::SmallInt> for i16 {
    fn from_sql(mut bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        bytes.read_i16::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<super::SmallSerial> for i16 {
    fn from_sql(bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        <Self as FromSql<super::SmallInt>>::from_sql(bytes, is_null)
    }
}

impl FromSql<super::Integer> for i32 {
    fn from_sql(mut bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        bytes.read_i32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<super::Serial> for i32 {
    fn from_sql(bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        <Self as FromSql<super::Integer>>::from_sql(bytes, is_null)
    }
}

impl FromSql<super::BigInt> for i64 {
    fn from_sql(mut bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        bytes.read_i64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<super::BigSerial> for i64 {
    fn from_sql(bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        <Self as FromSql<super::BigInt>>::from_sql(bytes, is_null)
    }
}

impl FromSql<super::Float> for f32 {
    fn from_sql(mut bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        bytes.read_f32::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<super::Double> for f64 {
    fn from_sql(mut bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        bytes.read_f64::<BigEndian>().map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<super::VarChar> for String {
    fn from_sql(bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        String::from_utf8(bytes.into()).map_err(|e| Box::new(e) as Box<Error>)
    }
}

impl FromSql<super::Binary> for Vec<u8> {
    fn from_sql(bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        check_not_null!(is_null);
        Ok(bytes.into())
    }
}

impl<T: NativeSqlType> NativeSqlType for Nullable<T> {}
impl<T, ST> FromSql<Nullable<ST>> for Option<T> where
    T: FromSql<ST>,
    ST: NativeSqlType,
{
    fn from_sql(bytes: &[u8], is_null: bool) -> Result<Self, Box<Error>> {
        if is_null {
            Ok(None)
        } else {
            T::from_sql(bytes, false).map(Some)
        }
    }
}

impl<T, ST> Queriable<Nullable<ST>> for Option<T> where
    T: FromSql<ST> + Queriable<ST>,
    ST: NativeSqlType,
{
    type Row = Self;
    fn build(row: Self) -> Self {
        row
    }
}
