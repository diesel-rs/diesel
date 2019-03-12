use byteorder::{ReadBytesExt, WriteBytesExt};
use std::error::Error;
use std::io::prelude::*;

use backend::{Backend, HasRawValue};
use deserialize::{self, FromSql};
use serialize::{self, IsNull, Output, ToSql};
use sql_types;

impl<DB> FromSql<sql_types::SmallInt, DB> for i16
where
    DB: Backend + for<'a> HasRawValue<'a, RawValue = &'a [u8]>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 2,
            "Received more than 2 bytes decoding i16. \
             Was an Integer expression accidentally identified as SmallInt?"
        );
        debug_assert!(
            bytes.len() >= 2,
            "Received fewer than 2 bytes decoding i16. \
             Was an expression of a different type accidentally identified \
             as SmallInt?"
        );
        bytes
            .read_i16::<DB::ByteOrder>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend> ToSql<sql_types::SmallInt, DB> for i16 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_i16::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB> FromSql<sql_types::Integer, DB> for i32
where
    DB: Backend + for<'a> HasRawValue<'a, RawValue = &'a [u8]>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 4,
            "Received more than 4 bytes decoding i32. \
             Was a BigInt expression accidentally identified as Integer?"
        );
        debug_assert!(
            bytes.len() >= 4,
            "Received fewer than 4 bytes decoding i32. \
             Was a SmallInt expression accidentally identified as Integer?"
        );
        bytes
            .read_i32::<DB::ByteOrder>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend> ToSql<sql_types::Integer, DB> for i32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_i32::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB> FromSql<sql_types::BigInt, DB> for i64
where
    DB: Backend + for<'a> HasRawValue<'a, RawValue = &'a [u8]>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 8,
            "Received more than 8 bytes decoding i64. \
             Was an expression of a different type misidentified as BigInt?"
        );
        debug_assert!(
            bytes.len() >= 8,
            "Received fewer than 8 bytes decoding i64. \
             Was an Integer expression misidentified as BigInt?"
        );
        bytes
            .read_i64::<DB::ByteOrder>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend> ToSql<sql_types::BigInt, DB> for i64 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_i64::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}
