use byteorder::{NativeEndian, WriteBytesExt};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::{Double, Float};
use sqlite::Sqlite;
use std::error::Error;
use std::io::Write;

impl ToSql<Double, Sqlite> for f64 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        out.write_f64::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}

impl ToSql<Float, Sqlite> for f32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Sqlite>) -> serialize::Result {
        out.write_f32::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}
