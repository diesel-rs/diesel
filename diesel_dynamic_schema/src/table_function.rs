//! Table-valued functions whose columns and parameters are dynamic [`Column`]s
//! of one schema-less [`Table`], with [`DynamicRow`] arguments and rows.

use crate::dynamic_value::{Any, DynamicRow};
use crate::{Column, Table};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::borrow::Borrow;
use diesel::deserialize::{self, FromSql};
use diesel::result::{DatabaseErrorKind, Error, QueryResult};
use diesel::row::Row;
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::sqlite::{
    Sqlite, SqliteBindValue, SqliteTableFunctionColumns, SqliteTableFunctionParameters,
    SqliteTableFunctionRow,
};

/// The error of a declaration that cannot be registered, of the same kind
/// diesel reports for its own registration checks.
fn registration_error(message: String) -> Error {
    Error::DatabaseError(DatabaseErrorKind::UnableToSendCommand, Box::new(message))
}

/// Fails unless `column` belongs to the schema-less table `function_name`.
fn check_table<T, U, C>(
    column: &Column<Table<T, U>, C, Any>,
    function_name: &str,
) -> QueryResult<()>
where
    T: Borrow<str>,
    C: Borrow<str>,
{
    let table = column.table();
    if table.schema().is_some() {
        return Err(registration_error(format!(
            "the table-valued function `{function_name}` cannot have a schema"
        )));
    }
    if table.name().borrow() != function_name {
        return Err(registration_error(format!(
            "the column `{}` belongs to `{}` and not to the table-valued function `{function_name}`",
            column.name().borrow(),
            table.name().borrow(),
        )));
    }
    Ok(())
}

impl<T, U, C> SqliteTableFunctionColumns<Table<T, U>> for Vec<Column<Table<T, U>, C, Any>>
where
    T: Borrow<str>,
    C: Borrow<str>,
{
    type SqlType = Any;

    fn function_name(&self) -> QueryResult<&str> {
        let first = self.first().ok_or_else(|| {
            registration_error("a table-valued function must return at least one column".into())
        })?;
        let function_name = first.table().name().borrow();
        for column in self {
            check_table(column, function_name)?;
        }
        Ok(function_name)
    }

    fn column_names(&self) -> Vec<&str> {
        self.iter().map(|column| column.name().borrow()).collect()
    }

    fn relation_column_names(&self) -> Option<Vec<&str>> {
        None
    }
}

impl<T, U, C, I> SqliteTableFunctionParameters<Table<T, U>, DynamicRow<I>>
    for Vec<Column<Table<T, U>, C, Any>>
where
    T: Borrow<str>,
    C: Borrow<str>,
    I: FromSql<Any, Sqlite>,
{
    fn parameter_names(&self, function_name: &str) -> QueryResult<Vec<&str>> {
        self.iter()
            .map(|column| {
                check_table(column, function_name)?;
                Ok(column.name().borrow())
            })
            .collect()
    }

    fn arguments<'a>(&self, row: &impl Row<'a, Sqlite>) -> deserialize::Result<DynamicRow<I>> {
        DynamicRow::from_row(row)
    }
}

impl<I> SqliteTableFunctionRow<Any> for DynamicRow<I>
where
    I: ToSql<Any, Sqlite>,
{
    fn for_each_value(&self, mut sink: impl FnMut(SqliteBindValue<'_>)) -> QueryResult<()> {
        for value in self {
            let mut metadata_lookup = ();
            let mut out = Output::new(SqliteBindValue::from(None::<i64>), &mut metadata_lookup);
            match value.to_sql(&mut out).map_err(Error::SerializationError)? {
                IsNull::Yes => sink(SqliteBindValue::from(None::<i64>)),
                IsNull::No => sink(out.into_inner()),
            }
        }
        Ok(())
    }
}
