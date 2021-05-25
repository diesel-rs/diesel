extern crate libsqlite3_sys as ffi;

use super::raw::RawConnection;
use super::serialized_value::SerializedValue;
use super::{Sqlite, SqliteAggregateFunction, SqliteValue};
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::{Field, PartialRow, Row, RowIndex};
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::HasSqlType;
use std::marker::PhantomData;

pub fn register<ArgsSqlType, RetSqlType, Args, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    deterministic: bool,
    mut f: F,
) -> QueryResult<()>
where
    F: FnMut(&RawConnection, Args) -> Ret + std::panic::UnwindSafe + Send + 'static,
    Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let fields_needed = Args::FIELD_COUNT;
    if fields_needed > 127 {
        return Err(Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new("SQLite functions cannot take more than 127 parameters".to_string()),
        ));
    }

    conn.register_sql_function(fn_name, fields_needed, deterministic, move |conn, args| {
        let args = build_sql_function_args::<ArgsSqlType, Args>(args)?;

        let result = f(conn, args);

        process_sql_function_result::<RetSqlType, Ret>(result)
    })?;
    Ok(())
}

pub fn register_noargs<RetSqlType, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    deterministic: bool,
    mut f: F,
) -> QueryResult<()>
where
    F: FnMut() -> Ret + std::panic::UnwindSafe + Send + 'static,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    conn.register_sql_function(fn_name, 0, deterministic, move |_, _| {
        let result = f();
        process_sql_function_result::<RetSqlType, Ret>(result)
    })?;
    Ok(())
}

pub fn register_aggregate<ArgsSqlType, RetSqlType, Args, Ret, A>(
    conn: &RawConnection,
    fn_name: &str,
) -> QueryResult<()>
where
    A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + std::panic::UnwindSafe,
    Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let fields_needed = Args::FIELD_COUNT;
    if fields_needed > 127 {
        return Err(Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new("SQLite functions cannot take more than 127 parameters".to_string()),
        ));
    }

    conn.register_aggregate_function::<ArgsSqlType, RetSqlType, Args, Ret, A>(
        fn_name,
        fields_needed,
    )?;

    Ok(())
}

pub(crate) fn build_sql_function_args<ArgsSqlType, Args>(
    args: &[*mut ffi::sqlite3_value],
) -> Result<Args, Error>
where
    Args: FromSqlRow<ArgsSqlType, Sqlite>,
{
    let row = FunctionRow::new(args);
    Args::build_from_row(&row).map_err(Error::DeserializationError)
}

pub(crate) fn process_sql_function_result<RetSqlType, Ret>(
    result: Ret,
) -> QueryResult<SerializedValue>
where
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let mut metadata_lookup = ();
    let mut buf = Output::new(Vec::new(), &mut metadata_lookup);
    let is_null = result.to_sql(&mut buf).map_err(Error::SerializationError)?;

    let bytes = if let IsNull::Yes = is_null {
        None
    } else {
        Some(buf.into_inner())
    };

    Ok(SerializedValue {
        ty: Sqlite::metadata(&mut ()),
        data: bytes,
    })
}

#[derive(Clone)]
struct FunctionRow<'a> {
    args: &'a [*mut ffi::sqlite3_value],
}

impl<'a> FunctionRow<'a> {
    fn new(args: &'a [*mut ffi::sqlite3_value]) -> Self {
        Self { args }
    }
}

impl<'a> Row<'a, Sqlite> for FunctionRow<'a> {
    type Field = FunctionArgument<'a>;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.args.len()
    }

    fn get<I>(&self, idx: I) -> Option<Self::Field>
    where
        Self: crate::row::RowIndex<I>,
    {
        let idx = self.idx(idx)?;

        self.args.get(idx).map(|arg| FunctionArgument {
            arg: *arg,
            p: PhantomData,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'a> RowIndex<usize> for FunctionRow<'a> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.args.len() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a, 'b> RowIndex<&'a str> for FunctionRow<'b> {
    fn idx(&self, _idx: &'a str) -> Option<usize> {
        None
    }
}

struct FunctionArgument<'a> {
    arg: *mut ffi::sqlite3_value,
    p: PhantomData<&'a ()>,
}

impl<'a> Field<'a, Sqlite> for FunctionArgument<'a> {
    fn field_name(&self) -> Option<&'a str> {
        None
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value(&self) -> Option<crate::backend::RawValue<'a, Sqlite>> {
        unsafe { SqliteValue::new(self.arg) }
    }
}
