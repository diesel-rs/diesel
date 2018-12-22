use byteorder::{WriteBytesExt}; // ReadBytesExt
use std::error::Error;
use std::io::prelude::*;

use backend::Backend;
//use deserialize::{self, FromSql};
use serialize::{self, IsNull, Output, ToSql};
use sql_types;

impl<DB: Backend> ToSql<sql_types::Float, DB> for f32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        out.write_f32::<DB::ByteOrder>(*self)
            .map(|_| IsNull::No)
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
