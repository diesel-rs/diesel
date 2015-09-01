mod tuples;
mod primitives;

extern crate postgres;

use self::postgres::rows::Row;

pub struct Bool;

pub struct SmallSerial;
pub struct Serial;
pub struct BigSerial;

pub struct SmallInt;
pub struct Integer;
pub struct BigInt;

pub struct Float;
pub struct Double;

pub struct VarChar;

pub struct Binary;

pub struct Nullable<T: NativeSqlType>(T);

pub trait NativeSqlType {}

pub trait FromSql<A: NativeSqlType> {
    fn from_sql(row: &Row, idx: usize) -> Self;

    fn consumed_columns() -> usize {
        1
    }
}
