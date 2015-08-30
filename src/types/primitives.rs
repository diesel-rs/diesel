extern crate postgres;

use self::postgres::rows::Row;
use self::postgres::types::FromSql as NativeFromSql;

use super::{NativeSqlType, FromSql, Nullable};
use Queriable;

macro_rules! primitive_impls {
    ($($Source:ident -> $Target:ident),+,) => {
        $(
            impl NativeSqlType for super::$Source {}
            impl FromSql<super::$Source> for $Target {
                fn from_sql(row: &Row, idx: usize) -> Self {
                    row.get(idx)
                }
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
    Serial -> i32,
    VarChar -> String,
    TinyInt -> i8,
    SmallInt -> i16,
    Integer -> i32,
    BigInt -> i64,
}

impl<T: NativeSqlType> NativeSqlType for Nullable<T> {}
impl<T, ST> FromSql<Nullable<ST>> for Option<T> where
    T: FromSql<ST> + NativeFromSql,
    ST: NativeSqlType,
{
    fn from_sql(row: &Row, idx: usize) -> Self {
        row.get(idx)
    }
}
