mod tuples;
mod primitives;

extern crate postgres;

use self::postgres::rows::Row;

pub struct Serial;
pub struct VarChar;
pub struct Integer;

pub trait NativeSqlType {}

pub trait FromSql<A: NativeSqlType> {
    fn from_sql(row: &Row, idx: usize) -> Self;
}
