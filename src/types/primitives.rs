extern crate postgres;

use self::postgres::types::FromSql as NativeFromSql;

use super::{NativeSqlType, FromSql, Nullable, ToSql};
use Queriable;
use row::Row;

macro_rules! primitive_impls {
    ($($Source:ident -> $Target:ty),+,) => {
        $(
            impl NativeSqlType for super::$Source {}
            impl FromSql<super::$Source> for $Target {
                fn from_sql<T: Row>(row: &mut T) -> Self {
                    row.take()
                }
            }

            impl ToSql<super::$Source> for $Target {
            }

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

impl<T: NativeSqlType> NativeSqlType for Nullable<T> {}
impl<T, ST> FromSql<Nullable<ST>> for Option<T> where
    T: FromSql<ST> + NativeFromSql,
    ST: NativeSqlType,
{
    fn from_sql<R: Row>(row: &mut R) -> Self {
        row.take()
    }
}

impl<T, ST> ToSql<Nullable<ST>> for Option<T> where
    T: ToSql<ST>,
    ST: NativeSqlType,
{
}
