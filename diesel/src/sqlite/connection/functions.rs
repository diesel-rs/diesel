#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::raw::RawConnection;
use super::{Sqlite, SqliteAggregateFunction, SqliteBindValue};
use crate::backend::Backend;
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::{Field, PartialRow, Row, RowIndex, RowSealed};
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::HasSqlType;
use crate::sqlite::SqliteValue;
use crate::sqlite::connection::bind_collector::InternalSqliteBindValue;
use crate::sqlite::connection::sqlite_value::OwnedSqliteValue;
use alloc::boxed::Box;
use alloc::string::ToString;

pub(super) fn register<ArgsSqlType, RetSqlType, Args, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    deterministic: bool,
    mut f: F,
) -> QueryResult<()>
where
    F: FnMut(&RawConnection, Args) -> Ret + core::panic::UnwindSafe + Send + 'static,
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

        Ok(f(conn, args))
    })?;
    Ok(())
}

pub(super) fn register_noargs<RetSqlType, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    deterministic: bool,
    mut f: F,
) -> QueryResult<()>
where
    F: FnMut() -> Ret + core::panic::UnwindSafe + Send + 'static,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    conn.register_sql_function(fn_name, 0, deterministic, move |_, _| Ok(f()))?;
    Ok(())
}

pub(super) fn register_aggregate<ArgsSqlType, RetSqlType, Args, Ret, A>(
    conn: &RawConnection,
    fn_name: &str,
) -> QueryResult<()>
where
    A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + core::panic::UnwindSafe,
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

pub(super) fn build_sql_function_args<ArgsSqlType, Args>(
    args: &mut [*mut ffi::sqlite3_value],
) -> Result<Args, Error>
where
    Args: FromSqlRow<ArgsSqlType, Sqlite>,
{
    let row = FunctionRow::new(args);
    Args::build_from_row(&row).map_err(Error::DeserializationError)
}

// clippy is wrong here, the let binding is required
// for lifetime reasons
#[allow(clippy::let_unit_value)]
pub(super) fn process_sql_function_result<RetSqlType, Ret>(
    result: &'_ Ret,
) -> QueryResult<InternalSqliteBindValue<'_>>
where
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let mut metadata_lookup = ();
    let value = SqliteBindValue {
        inner: InternalSqliteBindValue::Null,
    };
    let mut buf = Output::new(value, &mut metadata_lookup);
    let is_null = result.to_sql(&mut buf).map_err(Error::SerializationError)?;

    if let IsNull::Yes = is_null {
        Ok(InternalSqliteBindValue::Null)
    } else {
        Ok(buf.into_inner().inner)
    }
}

struct FunctionRow<'a> {
    args: &'a [Option<OwnedSqliteValue>],
    field_count: usize,
}

impl FunctionRow<'_> {
    #[allow(unsafe_code)] // complicated ptr cast
    fn new(args: &mut [*mut ffi::sqlite3_value]) -> Self {
        let lengths = args.len();
        let args = unsafe {
            core::slice::from_raw_parts(
                // This cast is safe because:
                // * Casting from a pointer to an array to a pointer to the first array
                // element is safe
                // * Casting from a raw pointer to `NonNull<T>` is safe,
                // because `NonNull` is #[repr(transparent)]
                // * Casting from `NonNull<T>` to `OwnedSqliteValue` is safe,
                // as the struct is `#[repr(transparent)]
                // * Casting from `NonNull<T>` to `Option<NonNull<T>>` as the documentation
                // states: "This is so that enums may use this forbidden value as a discriminant –
                // Option<NonNull<T>> has the same size as *mut T"
                // * The last point remains true for `OwnedSqliteValue` as `#[repr(transparent)]
                // guarantees the same layout as the inner type
                args as *mut [*mut ffi::sqlite3_value] as *mut ffi::sqlite3_value
                    as *mut Option<OwnedSqliteValue>,
                lengths,
            )
        };

        Self {
            field_count: lengths,
            args,
        }
    }
}

impl RowSealed for FunctionRow<'_> {}

impl<'a> Row<'a, Sqlite> for FunctionRow<'a> {
    type Field<'f>
        = FunctionArgument<'f>
    where
        'a: 'f,
        Self: 'f;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.field_count
    }

    fn get<'b, I>(&'b self, idx: I) -> Option<Self::Field<'b>>
    where
        'a: 'b,
        Self: crate::row::RowIndex<I>,
    {
        let col_idx = self.idx(idx)?;
        Some(FunctionArgument {
            args: self.args,
            col_idx,
        })
    }

    fn partial_row(&self, range: core::ops::Range<usize>) -> PartialRow<'_, Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl RowIndex<usize> for FunctionRow<'_> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
            Some(idx)
        } else {
            None
        }
    }
}

impl<'a> RowIndex<&'a str> for FunctionRow<'_> {
    fn idx(&self, _idx: &'a str) -> Option<usize> {
        None
    }
}

struct FunctionArgument<'a> {
    args: &'a [Option<OwnedSqliteValue>],
    col_idx: usize,
}

impl<'a> Field<'a, Sqlite> for FunctionArgument<'a> {
    fn field_name(&self) -> Option<&str> {
        None
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value(&self) -> Option<<Sqlite as Backend>::RawValue<'_>> {
        SqliteValue::from_function_row(self.args, self.col_idx)
    }
}
