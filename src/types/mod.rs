extern crate postgres;

mod tuples;
mod primitives;

use row::Row;

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
    fn from_sql<T: Row>(row: &mut T) -> Self;
}

pub trait ToSql<A: NativeSqlType>: postgres::types::ToSql {}
