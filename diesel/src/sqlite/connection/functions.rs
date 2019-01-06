extern crate libsqlite3_sys as ffi;

use super::raw::RawConnection;
use super::serialized_value::SerializedValue;
use super::{Sqlite, SqliteValue};
use crate::deserialize::{FromSqlRow, Queryable};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::Row;
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::HasSqlType;

pub fn register<ArgsSqlType, RetSqlType, Args, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    deterministic: bool,
    mut f: F,
) -> QueryResult<()>
where
    F: FnMut(Args) -> Ret + Send + 'static,
    Args: Queryable<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let fields_needed = Args::Row::FIELDS_NEEDED;
    if fields_needed > 127 {
        return Err(Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new("SQLite functions cannot take more than 127 parameters".to_string()),
        ));
    }

    conn.register_sql_function(fn_name, fields_needed, deterministic, move |args| {
        let mut row = FunctionRow { args };
        let args_row = Args::Row::build_from_row(&mut row).map_err(Error::DeserializationError)?;
        let args = Args::build(args_row);

        let result = f(args);

        let mut buf = Output::new(Vec::new(), &());
        let is_null = result.to_sql(&mut buf).map_err(Error::SerializationError)?;

        let bytes = if let IsNull::Yes = is_null {
            None
        } else {
            Some(buf.into_inner())
        };

        Ok(SerializedValue {
            ty: Sqlite::metadata(&()),
            data: bytes,
        })
    })?;
    Ok(())
}

struct FunctionRow<'a> {
    args: &'a [*mut ffi::sqlite3_value],
}

impl<'a> Row<Sqlite> for FunctionRow<'a> {
    fn take(&mut self) -> Option<&SqliteValue> {
        self.args.split_first().and_then(|(&first, rest)| {
            self.args = rest;
            unsafe { SqliteValue::new(first) }
        })
    }

    fn next_is_null(&self, count: usize) -> bool {
        self.args[..count]
            .iter()
            .all(|&p| unsafe { SqliteValue::new(p) }.is_none())
    }
}
