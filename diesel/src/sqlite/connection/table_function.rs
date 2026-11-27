use super::bind_collector::{OwnedSqliteBindValue, SqliteBindValueRef};
use super::functions::{FunctionArgument, FunctionRow, process_sql_function_result};
use super::{Sqlite, SqliteBindValue, SqliteConnection, virtual_table};
use crate::deserialize::{self, FromSqlRow, StaticallySizedRow};
use crate::query_builder::QueryBuilder;
use crate::query_source::{QueryRelation, QueryRelationField};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::{PartialRow, Row, RowIndex, RowSealed};
use crate::serialize::ToSql;
use crate::sql_types::HasSqlType;
use crate::sqlite::{BorrowedSqliteConnection, SqliteQueryBuilder};
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub(super) type TableFunctionCallback = dyn Fn(&mut BorrowedSqliteConnection, &FunctionRow<'_>) -> QueryResult<Vec<OwnedSqliteBindValue>>
    + Send;

/// A registered table-valued function owned by SQLite until it drops the module.
pub(super) struct TableFunction {
    pub(super) name: CString,
    pub(super) declaration: CString,
    pub(super) columns: usize,
    pub(super) parameters: usize,
    pub(super) callback: Box<TableFunctionCallback>,
}

/// The positional arguments of a SQLite table function call.
///
/// Arguments implement [`Row`] in parameter declaration order and can be deserialized with [`FromSqlRow`].
/// They have no field names, so name lookup always returns `None`.
pub struct SqliteFunctionArguments<'a> {
    row: &'a FunctionRow<'a>,
}

impl core::fmt::Debug for SqliteFunctionArguments<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SqliteFunctionArguments")
            .field("field_count", &self.row.field_count())
            .finish_non_exhaustive()
    }
}

impl RowSealed for SqliteFunctionArguments<'_> {}

impl<'a> Row<'a, Sqlite> for SqliteFunctionArguments<'a> {
    type Field<'field>
        = FunctionArgument<'field>
    where
        'a: 'field,
        Self: 'field;
    type InnerPartialRow = Self;

    fn field_count(&self) -> usize {
        self.row.field_count()
    }

    fn get<'field, Index>(&'field self, index: Index) -> Option<Self::Field<'field>>
    where
        'a: 'field,
        Self: RowIndex<Index>,
    {
        self.row.get(self.idx(index)?)
    }

    fn partial_row(&self, range: core::ops::Range<usize>) -> PartialRow<'_, Self> {
        PartialRow::new(self, range)
    }
}

impl RowIndex<usize> for SqliteFunctionArguments<'_> {
    fn idx(&self, index: usize) -> Option<usize> {
        self.row.idx(index)
    }
}

impl RowIndex<&str> for SqliteFunctionArguments<'_> {
    fn idx(&self, _index: &str) -> Option<usize> {
        None
    }
}

mod private {
    pub trait Tuple {}
    pub trait ColumnNames {}
    pub trait ResultRow<SqlType> {}
}

#[doc(hidden)]
pub trait SqliteTableFunctionColumnNames: private::ColumnNames {
    fn names() -> Vec<&'static str>;
}

#[doc(hidden)]
pub trait SqliteTableFunctionArgs<SqlType>: private::Tuple + Sized {
    const FIELD_COUNT: usize;
    fn arguments(row: &SqliteFunctionArguments<'_>) -> deserialize::Result<Self>;
}

impl private::Tuple for () {}

impl SqliteTableFunctionArgs<()> for () {
    const FIELD_COUNT: usize = 0;

    fn arguments(_row: &SqliteFunctionArguments<'_>) -> deserialize::Result<Self> {
        Ok(())
    }
}

/// A tuple containing one serializable value per table function result column.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no row of the result columns typed `{SqlType}`",
    note = "a row is a tuple with one value per result column, in column order"
)]
pub trait SqliteTableFunctionRow<SqlType>: private::ResultRow<SqlType> {
    #[doc(hidden)]
    type Values: IntoIterator<Item = SqliteBindValue<'static>>;

    #[doc(hidden)]
    fn values(&self) -> QueryResult<Self::Values>;
}

macro_rules! table_function_tuple_impls {
    ($(
        $Tuple:tt {
            $(($index:tt) -> $Value:ident, $SqlType:ident, $Unused:ident,)+
        }
    )+) => {
        $(
            impl<$($Value,)+> private::Tuple for ($($Value,)+) {}

            impl<$($Value,)+> private::ColumnNames for ($($Value,)+)
            where $($Value: QueryRelationField,)+ {}

            impl<$($Value,)+> SqliteTableFunctionColumnNames for ($($Value,)+)
            where $($Value: QueryRelationField,)+ {
                fn names() -> Vec<&'static str> {
                    alloc::vec![$(<$Value as QueryRelationField>::NAME,)+]
                }
            }

            impl<ArgumentsSqlType, $($Value,)+> SqliteTableFunctionArgs<ArgumentsSqlType> for ($($Value,)+)
            where Self: FromSqlRow<ArgumentsSqlType, Sqlite> + StaticallySizedRow<ArgumentsSqlType, Sqlite>,
            {
                const FIELD_COUNT: usize = <Self as StaticallySizedRow<ArgumentsSqlType, Sqlite>>::FIELD_COUNT;

                fn arguments(row: &SqliteFunctionArguments<'_>) -> deserialize::Result<Self> {
                    Self::build_from_row(row)
                }
            }

            impl<$($Value,)+ $($SqlType,)+> private::ResultRow<($($SqlType,)+)> for ($($Value,)+)
            where
                $($Value: ToSql<$SqlType, Sqlite>,)+
                $(Sqlite: HasSqlType<$SqlType>,)+
            {}

            impl<$($Value,)+ $($SqlType,)+> SqliteTableFunctionRow<($($SqlType,)+)> for ($($Value,)+)
            where
                $($Value: ToSql<$SqlType, Sqlite>,)+
                $(Sqlite: HasSqlType<$SqlType>,)+
            {
                type Values = [SqliteBindValue<'static>; [$(stringify!($Value),)+].len()];

                fn values(&self) -> QueryResult<Self::Values> {
                    Ok([$(owned_value(process_sql_function_result::<$SqlType, $Value>(&self.$index)?),)+])
                }
            }
        )+
    };
}

crate::for_each_tuple!(table_function_tuple_impls);

fn owned_value(value: SqliteBindValueRef<'_>) -> SqliteBindValue<'static> {
    let inner = match value {
        SqliteBindValueRef::BorrowedString(value) => SqliteBindValueRef::String(value.into()),
        SqliteBindValueRef::BorrowedBinary(value) => SqliteBindValueRef::Binary(value.into()),
        SqliteBindValueRef::String(value) => SqliteBindValueRef::String(value),
        SqliteBindValueRef::Binary(value) => SqliteBindValueRef::Binary(value),
        SqliteBindValueRef::I32(value) => SqliteBindValueRef::I32(value),
        SqliteBindValueRef::I64(value) => SqliteBindValueRef::I64(value),
        SqliteBindValueRef::F64(value) => SqliteBindValueRef::F64(value),
        SqliteBindValueRef::Null => SqliteBindValueRef::Null,
    };
    SqliteBindValue { inner }
}

#[doc(hidden)]
pub fn register_table_function<Relation, ArgsSqlType, Args, Rows, F>(
    connection: &mut SqliteConnection,
    name: &str,
    parameters: &[&str],
    function: F,
) -> QueryResult<()>
where
    Relation: QueryRelation,
    Relation::AllColumns: SqliteTableFunctionColumnNames,
    Args: SqliteTableFunctionArgs<ArgsSqlType>,
    Rows: IntoIterator,
    Rows::Item: SqliteTableFunctionRow<Relation::SqlType>,
    F: Fn(&mut BorrowedSqliteConnection, Args) -> QueryResult<Rows>
        + Send
        + core::panic::UnwindSafe
        + 'static,
{
    if parameters.len() != Args::FIELD_COUNT {
        return Err(registration_error(format!(
            "`{name}` has an argument count mismatch"
        )));
    }
    register_table_function_with_rows(
        connection,
        name,
        &Relation::AllColumns::names(),
        parameters,
        move |connection, arguments| {
            let arguments = Args::arguments(arguments).map_err(Error::DeserializationError)?;
            Ok(function(connection, arguments)?
                .into_iter()
                .map(|row| row.values()))
        },
    )
}

fn register_table_function_with_rows<Rows, Values, F>(
    connection: &mut SqliteConnection,
    name: &str,
    columns: &[&str],
    parameters: &[&str],
    function: F,
) -> QueryResult<()>
where
    Rows: IntoIterator<Item = QueryResult<Values>>,
    Values: IntoIterator<Item = SqliteBindValue<'static>>,
    F: Fn(&mut BorrowedSqliteConnection, &SqliteFunctionArguments<'_>) -> QueryResult<Rows>
        + Send
        + core::panic::UnwindSafe
        + 'static,
{
    if columns.is_empty() {
        return Err(registration_error(format!(
            "`{name}` must return at least one column"
        )));
    }
    for (index, column) in columns.iter().chain(parameters).enumerate() {
        if columns
            .iter()
            .chain(parameters)
            .take(index)
            .any(|previous| previous.eq_ignore_ascii_case(column))
        {
            return Err(registration_error(format!(
                "`{name}` declares duplicate column `{column}`"
            )));
        }
    }
    let width = columns.len();
    virtual_table::register(
        &connection.raw_connection,
        TableFunction {
            name: CString::new(name)?,
            declaration: table_declaration(columns, parameters)?,
            columns: width,
            parameters: parameters.len(),
            callback: Box::new(move |connection, row| {
                let arguments = SqliteFunctionArguments { row };
                let mut values = Vec::new();
                for row in function(connection, &arguments)? {
                    let start = values.len();
                    values.extend(
                        row?.into_iter()
                            .map(|value| OwnedSqliteBindValue::from(value.inner)),
                    );
                    let returned = values.len() - start;
                    if returned != width {
                        return Err(Error::SerializationError(format!(
                            "a table-valued function row has {returned} values for {width} columns"
                        ).into()));
                    }
                }
                Ok(values)
            }),
        },
    )?;
    // Prepared statements retain the module they were compiled against.
    connection.statement_cache.clear();
    Ok(())
}

fn registration_error(message: String) -> Error {
    Error::DatabaseError(DatabaseErrorKind::UnableToSendCommand, Box::new(message))
}

/// Declares visible result columns followed by hidden parameter columns without type affinity.
fn table_declaration(columns: &[&str], parameters: &[&str]) -> QueryResult<CString> {
    let mut statement = SqliteQueryBuilder::new();
    statement.push_sql("CREATE TABLE x(");
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            statement.push_sql(", ");
        }
        statement.push_identifier(column)?;
    }
    for parameter in parameters {
        statement.push_sql(", ");
        statement.push_identifier(parameter)?;
        statement.push_sql(" HIDDEN");
    }
    statement.push_sql(")");
    Ok(CString::new(statement.finish())?)
}
