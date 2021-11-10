use crate::query_builder::BindCollector;
use crate::serialize::{IsNull, Output};
use crate::sql_types::HasSqlType;
use crate::sqlite::{Sqlite, SqliteType};
use crate::QueryResult;

#[derive(Debug)]
pub struct SqliteBindCollector<'a> {
    pub(in crate::sqlite) binds: Vec<SqliteBindValue<'a>>,
    pub(in crate::sqlite) metadata: Vec<SqliteType>,
}

impl SqliteBindCollector<'_> {
    pub(in crate::sqlite) fn new() -> Self {
        Self {
            binds: Vec::new(),
            metadata: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum SqliteBindValue<'a> {
    BorrowedString(&'a str),
    String(Box<str>),
    BorrowedBinary(&'a [u8]),
    Binary(Box<[u8]>),
    SmallInt(i16),
    Integer(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Null,
}

impl std::fmt::Display for SqliteBindValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = match self {
            SqliteBindValue::BorrowedString(_) | SqliteBindValue::String(_) => "Text",
            SqliteBindValue::BorrowedBinary(_) | SqliteBindValue::Binary(_) => "Binary",
            SqliteBindValue::SmallInt(_) => "SmallInt",
            SqliteBindValue::Integer(_) => "Integer",
            SqliteBindValue::BigInt(_) => "BigInt",
            SqliteBindValue::Float(_) => "Float",
            SqliteBindValue::Double(_) => "Double",
            SqliteBindValue::Null => "Null",
        };
        f.write_str(n)
    }
}

impl SqliteBindValue<'_> {
    pub(in crate::sqlite) fn result_of(self, ctx: &mut libsqlite3_sys::sqlite3_context) {
        use libsqlite3_sys as ffi;
        use std::os::raw as libc;
        // This unsafe block assumes the following invariants:
        //
        // - `ctx` points to valid memory
        unsafe {
            match self {
                SqliteBindValue::BorrowedString(s) => ffi::sqlite3_result_text(
                    ctx,
                    s.as_ptr() as *const libc::c_char,
                    s.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValue::String(s) => ffi::sqlite3_result_text(
                    ctx,
                    s.as_ptr() as *const libc::c_char,
                    s.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValue::Binary(b) => ffi::sqlite3_result_blob(
                    ctx,
                    b.as_ptr() as *const libc::c_void,
                    b.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValue::BorrowedBinary(b) => ffi::sqlite3_result_blob(
                    ctx,
                    b.as_ptr() as *const libc::c_void,
                    b.len() as libc::c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                SqliteBindValue::SmallInt(s) => ffi::sqlite3_result_int(ctx, libc::c_int::from(s)),
                SqliteBindValue::Integer(i) => ffi::sqlite3_result_int(ctx, i as libc::c_int),
                SqliteBindValue::BigInt(l) => ffi::sqlite3_result_int64(ctx, l),
                SqliteBindValue::Float(f) => {
                    ffi::sqlite3_result_double(ctx, libc::c_double::from(f))
                }
                SqliteBindValue::Double(d) => ffi::sqlite3_result_double(ctx, d as libc::c_double),
                SqliteBindValue::Null => ffi::sqlite3_result_null(ctx),
            }
        }
    }
}

impl<'a> BindCollector<'a, Sqlite> for SqliteBindCollector<'a> {
    type Buffer = SqliteBindValue<'a>;

    fn push_bound_value<T, U>(&mut self, bind: &'a U, metadata_lookup: &mut ()) -> QueryResult<()>
    where
        Sqlite: crate::sql_types::HasSqlType<T>,
        U: crate::serialize::ToSql<T, Sqlite>,
    {
        let mut to_sql_output = Output::new(SqliteBindValue::Null, metadata_lookup);
        let is_null = bind
            .to_sql(&mut to_sql_output)
            .map_err(crate::result::Error::SerializationError)?;
        let bind = to_sql_output.into_inner();
        let metadata = Sqlite::metadata(metadata_lookup);
        match is_null {
            IsNull::No => self.binds.push(bind),
            IsNull::Yes => self.binds.push(SqliteBindValue::Null),
        }
        self.metadata.push(metadata);
        Ok(())
    }
}
