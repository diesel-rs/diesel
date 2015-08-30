extern crate postgres;

use self::postgres::rows::Row;

use super::{NativeSqlType, FromSql};
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
    Integer -> i32,
}
