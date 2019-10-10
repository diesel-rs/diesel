use byteorder::{NativeEndian, WriteBytesExt};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::{BigInt, Integer, SmallInt};
use sqlite::Sqlite;
use std::error::Error;
use std::io::Write;

impl ToSql<BigInt, Sqlite> for i64 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        out.write_i64::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

impl ToSql<Integer, Sqlite> for i32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        out.write_i32::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

impl ToSql<SmallInt, Sqlite> for i16 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        out.write_i16::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}
