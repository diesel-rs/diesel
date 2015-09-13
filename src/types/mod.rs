mod impls;

use row::Row;
use std::error::Error;
use std::io::Write;

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
pub struct Array<T: NativeSqlType>(T);

pub trait NativeSqlType {}

pub trait FromSql<A: NativeSqlType>: Sized {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>>;
}

pub trait FromSqlRow<A: NativeSqlType>: Sized {
    fn build_from_row<T: Row>(row: &mut T) -> Result<Self, Box<Error>>;
}

impl<A, T> FromSqlRow<A> for T where
    A: NativeSqlType,
    T: FromSql<A>,
{
    fn build_from_row<R: Row>(row: &mut R) -> Result<Self, Box<Error>> {
        let bytes = if row.next_is_null() {
            None
        } else {
            Some(row.take())
        };
        Self::from_sql(bytes)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum IsNull {
    Yes,
    No,
}

pub trait ToSql<A: NativeSqlType> {
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>>;
}

pub trait ValuesToSql<A: NativeSqlType> {
    fn values_to_sql(&self) -> Result<Vec<Option<Vec<u8>>>, Box<Error>>;
}

impl<A, T> ValuesToSql<A> for T where
    A: NativeSqlType,
    T: ToSql<A>,
{
    fn values_to_sql(&self) -> Result<Vec<Option<Vec<u8>>>, Box<Error>> {
        let mut bytes = Vec::new();
        let bytes = match try!(self.to_sql(&mut bytes)) {
            IsNull::No => Some(bytes),
            IsNull::Yes => None,
        };
        Ok(vec![bytes])
    }
}
