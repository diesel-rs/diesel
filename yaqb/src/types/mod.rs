pub mod ops;
mod ord;
mod impls;

pub mod structs {
    pub use super::impls::date_and_time::{PgTimestamp, PgDate, PgTime, PgInterval};
}

pub use self::ord::SqlOrd;

use row::Row;
use std::error::Error;
use std::io::Write;

#[derive(Clone, Copy, Default)] pub struct Bool;

pub type SmallSerial = SmallInt;
pub type Serial = Integer;
pub type BigSerial = BigInt;

#[derive(Clone, Copy, Default)] pub struct SmallInt;
#[derive(Clone, Copy, Default)] pub struct Integer;
#[derive(Clone, Copy, Default)] pub struct BigInt;

#[derive(Clone, Copy, Default)] pub struct Float;
#[derive(Clone, Copy, Default)] pub struct Double;

#[derive(Clone, Copy, Default)] pub struct VarChar;
#[derive(Clone, Copy, Default)] pub struct Text;

#[derive(Clone, Copy, Default)] pub struct Binary;

#[derive(Clone, Copy, Default)] pub struct Date;
#[derive(Clone, Copy, Default)] pub struct Interval;
#[derive(Clone, Copy, Default)] pub struct Time;
#[derive(Clone, Copy, Default)] pub struct Timestamp;

#[derive(Clone, Copy, Default)] pub struct Nullable<T: NativeSqlType>(T);
#[derive(Clone, Copy, Default)] pub struct Array<T: NativeSqlType>(T);

pub trait NativeSqlType {
    fn oid(&self) -> u32;
    fn new() -> Self where Self: Sized;
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
