use byteorder::{ReadBytesExt, WriteBytesExt};
use std::error::Error;
use std::io::prelude::*;

use crate::backend::Backend;
use crate::deserialize::{self, FromSql};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types;

impl<DB: Backend<RawValue = [u8]>> FromSql<sql_types::Float, DB> for f32 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 4,
            "Received more than 4 bytes while decoding \
             an f32. Was a double accidentally marked as float?"
        );
        bytes
            .read_f32::<DB::ByteOrder>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend> ToSql<sql_types::Float, DB> for f32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_f32::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend<RawValue = [u8]>> FromSql<sql_types::Double, DB> for f64 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        debug_assert!(
            bytes.len() <= 8,
            "Received more than 8 bytes while decoding \
             an f64. Was a numeric accidentally marked as double?"
        );
        bytes
            .read_f64::<DB::ByteOrder>()
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend> ToSql<sql_types::Double, DB> for f64 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_f64::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}
