mod tuples;
mod primitives;

use row::Row;
use std::error::Error;

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

pub trait FromSql<A: NativeSqlType>: Sized {
    fn from_sql<T: Row>(row: &mut T) -> Result<Self, Box<Error>>;
}
