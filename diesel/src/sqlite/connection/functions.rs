extern crate libsqlite3_sys as ffi;

use super::raw::RawConnection;
use super::row::PrivateSqliteRow;
use super::serialized_value::SerializedValue;
use super::{Sqlite, SqliteAggregateFunction};
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::{Field, PartialRow, Row, RowIndex};
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::HasSqlType;
use crate::sqlite::connection::sqlite_value::OwnedSqliteValue;
use crate::sqlite::SqliteValue;
use std::cell::{Ref, RefCell};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::DerefMut;
use std::rc::Rc;

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
    args: &mut [*mut ffi::sqlite3_value],
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

struct FunctionRow<'a> {
    // we use `ManuallyDrop` to prevent dropping the content of the internal vector
    // as this buffer is owned by sqlite not by diesel
    args: Rc<RefCell<ManuallyDrop<PrivateSqliteRow<'a>>>>,
    field_count: usize,
    marker: PhantomData<&'a ffi::sqlite3_value>,
}

impl<'a> Drop for FunctionRow<'a> {
    fn drop(&mut self) {
        if let Some(args) = Rc::get_mut(&mut self.args) {
            if let PrivateSqliteRow::Duplicated { column_names, .. } =
                DerefMut::deref_mut(RefCell::get_mut(args))
            {
                if let Some(inner) = Rc::get_mut(column_names) {
                    // an empty Vector does not allocate according to the documentation
                    // so this prevents leaking memory
                    std::mem::drop(std::mem::replace(inner, Vec::new()));
                }
            }
        }
    }
}

impl<'a> FunctionRow<'a> {
    fn new(args: &mut [*mut ffi::sqlite3_value]) -> Self {
        let lenghts = args.len();
        let args = unsafe {
            Vec::from_raw_parts(
                // This cast is safe because:
                // * Casting from a pointer to an arry to a pointer to the first array
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
                // * It's unsafe to drop the vector (and the vector elements)
                // because of this we wrap the vector (or better the Row)
                // Into `ManualDrop` to prevent the dropping
                args as *mut [*mut ffi::sqlite3_value] as *mut ffi::sqlite3_value
                    as *mut Option<OwnedSqliteValue>,
                lenghts,
                lenghts,
            )
        };

        Self {
            field_count: lenghts,
            args: Rc::new(RefCell::new(ManuallyDrop::new(
                PrivateSqliteRow::Duplicated {
                    values: args,
                    column_names: Rc::new(vec![None; lenghts]),
                },
            ))),
            marker: PhantomData,
        }
    }
}

impl<'a> Row<'a, Sqlite> for FunctionRow<'a> {
    type Field = FunctionArgument<'a>;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.field_count
    }

    fn get<I>(&self, idx: I) -> Option<Self::Field>
    where
        Self: crate::row::RowIndex<I>,
    {
        let idx = self.idx(idx)?;
        Some(FunctionArgument {
            args: self.args.clone(),
            col_idx: idx as i32,
        })
    }

    fn partial_row(&self, range: std::ops::Range<usize>) -> PartialRow<Self::InnerPartialRow> {
        PartialRow::new(self, range)
    }
}

impl<'a> RowIndex<usize> for FunctionRow<'a> {
    fn idx(&self, idx: usize) -> Option<usize> {
        if idx < self.field_count() {
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
    args: Rc<RefCell<ManuallyDrop<PrivateSqliteRow<'a>>>>,
    col_idx: i32,
}

impl<'a> Field<'a, Sqlite> for FunctionArgument<'a> {
    fn field_name(&self) -> Option<&str> {
        None
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value<'b>(&'b self) -> Option<crate::backend::RawValue<'b, Sqlite>>
    where
        'a: 'b,
    {
        SqliteValue::new(
            Ref::map(self.args.borrow(), |drop| std::ops::Deref::deref(drop)),
            self.col_idx,
        )
    }
}
