mod ord;
mod impls;

pub mod structs {
    pub use super::impls::date_and_time::PgTimestamp;
}

pub use self::ord::SqlOrd;

use row::Row;
use std::error::Error;
use std::io::Write;

#[derive(Clone, Copy)] pub struct Bool;

#[derive(Clone, Copy)] pub struct SmallSerial;
#[derive(Clone, Copy)] pub struct Serial;
#[derive(Clone, Copy)] pub struct BigSerial;

#[derive(Clone, Copy)] pub struct SmallInt;
#[derive(Clone, Copy)] pub struct Integer;
#[derive(Clone, Copy)] pub struct BigInt;

#[derive(Clone, Copy)] pub struct Float;
#[derive(Clone, Copy)] pub struct Double;

#[derive(Clone, Copy)] pub struct VarChar;
#[derive(Clone, Copy)] pub struct Text;

#[derive(Clone, Copy)] pub struct Binary;

#[derive(Clone, Copy)] pub struct Timestamp;

#[derive(Clone, Copy)] pub struct Nullable<T: NativeSqlType>(T);
#[derive(Clone, Copy)] pub struct Array<T: NativeSqlType>(T);

pub trait NativeSqlType {
    fn oid() -> u32;
}

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
        Self::from_sql(row.take())
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

impl<'a, A, T> ToSql<A> for &'a T where
    A: NativeSqlType,
    T: ToSql<A>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        (*self).to_sql(out)
    }
}
