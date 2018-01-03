use byteorder::{ReadBytesExt, WriteBytesExt};
use std::error::Error;
use std::io::prelude::*;

use backend::Backend;
use types::{self, FromSql, IsNull, ToSql, ToSqlOutput};

impl<DB: Backend<RawValue = [u8]>> FromSql<types::Float, DB> for f32 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
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

impl<DB: Backend> ToSql<types::Float, DB> for f32 {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        out.write_f32::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}

impl<DB: Backend<RawValue = [u8]>> FromSql<types::Double, DB> for f64 {
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
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

impl<DB: Backend> ToSql<types::Double, DB> for f64 {
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, DB>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        out.write_f64::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<Error + Send + Sync>)
    }
}
