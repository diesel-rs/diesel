mod tuples;
mod primitives;

extern crate postgres;

use self::postgres::rows::Row;

pub struct Serial;
pub struct VarChar;
pub struct TinyInt;
pub struct SmallInt;
pub struct Integer;
pub struct BigInt;
pub struct Nullable<T: NativeSqlType>(T);

pub trait NativeSqlType {}

pub trait FromSql<A: NativeSqlType> {
    fn from_sql(row: &Row, idx: usize) -> Self;
}
