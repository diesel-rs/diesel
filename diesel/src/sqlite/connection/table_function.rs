use super::bind_collector::OwnedSqliteBindValue;
use super::functions::{FunctionRow, process_sql_function_result};
use super::{Sqlite, SqliteBindValue, SqliteConnection, virtual_table};
use crate::deserialize::{self, FromSqlRow, StaticallySizedRow};
use crate::expression::Expression;
use crate::internal::table_macro::{Identifier, StaticQueryFragment};
use crate::query_builder::QueryBuilder;
use crate::query_source::{QueryRelationField, View};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::Row;
use crate::serialize::ToSql;
use crate::sql_types::HasSqlType;
use crate::sqlite::query_builder::SqliteQueryBuilder;
use crate::sqlite::{BorrowedSqliteConnection, SqliteFunctionBehavior};
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub(super) type TableFunctionCallback = dyn Fn(&mut BorrowedSqliteConnection, &FunctionRow<'_>) -> QueryResult<Vec<OwnedSqliteBindValue>>
    + Send;

/// A registered table-valued function, owned by SQLite until it drops the module.
pub(super) struct TableFunction {
    pub(super) name: CString,
    pub(super) declaration: CString,
    pub(super) columns: usize,
    pub(super) parameters: usize,
    pub(super) behavior: SqliteFunctionBehavior,
    pub(super) callback: Box<TableFunctionCallback>,
}

/// The result columns of a table-valued function declared by the relation `V`.
///
/// Diesel implements it for tuples of columns of one [`view!`](crate::view!)
/// without a schema, as eponymous tables exist only in `main`.
/// Another crate can implement it for columns named at run time.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no list of columns of the relation `{V}`",
    note = "the columns of a table-valued function come from one `view!` without a schema, written as a tuple such as `(view::column,)`"
)]
pub trait SqliteTableFunctionColumns<V> {
    /// The SQL type of one row of these columns, which [`SqliteTableFunctionRow`] matches.
    type SqlType;

    /// The name the function is called by, or an error when the columns do not
    /// declare one function.
    fn function_name(&self) -> QueryResult<&str>;

    /// The names of the result columns, in order.
    fn column_names(&self) -> Vec<&str>;

    /// Every column of the declaring relation, when the relation lists them.
    ///
    /// Registration then checks that the result columns and the parameters name
    /// each of them once.
    fn relation_column_names(&self) -> Option<Vec<&str>>;
}

/// The parameters of a table-valued function declared by the relation `V`,
/// read as `Args` when the function runs.
///
/// Diesel implements it for `()` and for tuples of columns of one
/// [`view!`](crate::view!). Another crate can implement it for columns named
/// at run time.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no list of parameters of the relation `{V}` readable as `{Args}`",
    note = "the parameters are `()` or a tuple of columns of the same `view!` as the result columns, and the callback takes a tuple with one Rust value per parameter"
)]
pub trait SqliteTableFunctionParameters<V, Args> {
    /// The names of the parameters, in order, or an error when one of them does
    /// not belong to the function `function_name`.
    fn parameter_names(&self, function_name: &str) -> QueryResult<Vec<&str>>;

    /// Reads the arguments of one call from `row`, which holds one field per parameter.
    fn arguments<'a>(&self, row: &impl Row<'a, Sqlite>) -> deserialize::Result<Args>;
}

/// One row a table-valued function returns, with one value per result column
/// typed `ST`.
///
/// Diesel implements it for tuples. Another crate can implement it for rows
/// whose width is known only at run time, which registration checks.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no row of the result columns typed `{ST}`",
    note = "a row is a tuple with one value per result column, in the order of the columns, each serializable as that column's SQL type"
)]
pub trait SqliteTableFunctionRow<ST> {
    /// Hands each value of the row to `sink`, in column order.
    fn for_each_value(&self, sink: impl FnMut(SqliteBindValue<'_>)) -> QueryResult<()>;
}

mod private {
    /// The column names of a tuple of relation fields, known at compile time.
    pub trait StaticColumnNames {
        fn names() -> alloc::vec::Vec<&'static str>;
    }
}

use private::StaticColumnNames;

impl<V> SqliteTableFunctionParameters<V, ()> for () {
    fn parameter_names(&self, _function_name: &str) -> QueryResult<Vec<&str>> {
        Ok(Vec::new())
    }

    fn arguments<'a>(&self, _row: &impl Row<'a, Sqlite>) -> deserialize::Result<()> {
        Ok(())
    }
}

macro_rules! table_function_tuple_impls {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<$($T,)+> StaticColumnNames for ($($T,)+)
            where
                $($T: QueryRelationField,)+
            {
                fn names() -> Vec<&'static str> {
                    alloc::vec![$(<$T as QueryRelationField>::NAME,)+]
                }
            }

            impl<V, $($T,)+> SqliteTableFunctionColumns<V> for ($($T,)+)
            where
                V: View + StaticQueryFragment<Component = Identifier<'static>>,
                V::AllColumns: StaticColumnNames,
                $($T: QueryRelationField<QueryRelation = V>,)+
            {
                type SqlType = ($(<$T as Expression>::SqlType,)+);

                fn function_name(&self) -> QueryResult<&str> {
                    Ok(V::STATIC_COMPONENT.0)
                }

                fn column_names(&self) -> Vec<&str> {
                    <Self as StaticColumnNames>::names()
                }

                fn relation_column_names(&self) -> Option<Vec<&str>> {
                    Some(<V::AllColumns as StaticColumnNames>::names())
                }
            }

            impl<V, Args, $($T,)+> SqliteTableFunctionParameters<V, Args> for ($($T,)+)
            where
                $($T: QueryRelationField<QueryRelation = V>,)+
                Args: FromSqlRow<($(<$T as Expression>::SqlType,)+), Sqlite>
                    + StaticallySizedRow<($(<$T as Expression>::SqlType,)+), Sqlite>,
            {
                fn parameter_names(&self, _function_name: &str) -> QueryResult<Vec<&str>> {
                    Ok(<Self as StaticColumnNames>::names())
                }

                fn arguments<'a>(&self, row: &impl Row<'a, Sqlite>) -> deserialize::Result<Args> {
                    Args::build_from_row(row)
                }
            }

            impl<$($T,)+ $($ST,)+> SqliteTableFunctionRow<($($ST,)+)> for ($($T,)+)
            where
                $($T: ToSql<$ST, Sqlite>,)+
                $(Sqlite: HasSqlType<$ST>,)+
            {
                fn for_each_value(&self, mut sink: impl FnMut(SqliteBindValue<'_>)) -> QueryResult<()> {
                    $(
                        sink(SqliteBindValue {
                            inner: process_sql_function_result::<$ST, $T>(&self.$idx)?,
                        });
                    )+
                    Ok(())
                }
            }
        )+
    };
}

crate::for_each_tuple!(table_function_tuple_impls);

impl SqliteConnection {
    /// Registers a table-valued function, a relation called with arguments in `FROM`
    /// like SQLite's built-in `json_each`.
    ///
    /// A [`view!`](crate::view!) declares the function. `columns` lists the view's
    /// result columns and `parameters` the columns every call binds, and together
    /// they name each column of the view once. SQLite exposes the parameters as
    /// hidden columns, so the query DSL calls the function by filtering them with
    /// `eq`, and `name(a)` in SQL is the same call. A query that leaves a parameter
    /// unbound fails to prepare.
    ///
    /// Each call runs `function` once with its arguments and collects every row it
    /// returns before the query reads the first. `function` may run queries on the
    /// connection it receives, including ones calling this same table-valued
    /// function. An error it returns, or a panic when unwinding is enabled, fails
    /// the statement calling it.
    ///
    /// Of the `behavior` flags only [`INNOCUOUS`](SqliteFunctionBehavior::INNOCUOUS)
    /// and [`DIRECTONLY`](SqliteFunctionBehavior::DIRECTONLY) apply to a table, and
    /// the others are rejected. Registering a name again replaces the function and
    /// clears the statement cache, so cached queries call the new one. A
    /// registration from inside a callback does not reach the cache of the
    /// connection running that callback.
    ///
    /// See: [Table-valued functions](https://sqlite.org/vtab.html#tabfunc2)
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # // This test registers a Rust callback that sqlite invokes through the
    /// # // FFI boundary, which is not supported by miri `-Zmiri-native-lib`.
    /// # #[cfg(miri)]
    /// # fn main() {}
    /// # #[cfg(not(miri))]
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// use diesel::connection::SimpleConnection;
    /// use diesel::sqlite::SqliteFunctionBehavior;
    ///
    /// table! {
    ///     grants (id) {
    ///         id -> Integer,
    ///         role -> Text,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// // The table-valued function `grantees(role)`.
    /// diesel::view! {
    ///     grantees {
    ///         name -> Text,
    ///         role -> Text,
    ///     }
    /// }
    ///
    /// # fn run_test() -> QueryResult<()> {
    /// # let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.batch_execute(
    ///     "CREATE TABLE grants (id INTEGER PRIMARY KEY, role TEXT NOT NULL, name TEXT NOT NULL);
    ///      INSERT INTO grants (role, name) VALUES ('admin', 'bob'), ('admin', 'alice'), ('guest', 'carol');",
    /// )?;
    ///
    /// conn.register_table_valued_function(
    ///     (grantees::name,),
    ///     (grantees::role,),
    ///     SqliteFunctionBehavior::INNOCUOUS,
    ///     |conn, (role,): (String,)| {
    ///         grants::table
    ///             .filter(grants::role.eq(role))
    ///             .select((grants::name,))
    ///             .order(grants::name)
    ///             .load::<(String,)>(conn)
    ///     },
    /// )?;
    ///
    /// let admins = grantees::view
    ///     .filter(grantees::role.eq("admin"))
    ///     .select(grantees::name)
    ///     .load::<String>(&mut conn)?;
    /// assert_eq!(admins, ["alice", "bob"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_table_valued_function<V, Columns, Parameters, Args, Rows, F>(
        &mut self,
        columns: Columns,
        parameters: Parameters,
        behavior: SqliteFunctionBehavior,
        function: F,
    ) -> QueryResult<()>
    where
        Columns: SqliteTableFunctionColumns<V>,
        Parameters:
            SqliteTableFunctionParameters<V, Args> + Send + core::panic::UnwindSafe + 'static,
        Rows: IntoIterator,
        Rows::Item: SqliteTableFunctionRow<Columns::SqlType>,
        F: Fn(&mut BorrowedSqliteConnection, Args) -> QueryResult<Rows>
            + Send
            + core::panic::UnwindSafe
            + 'static,
    {
        let unsupported =
            behavior - (SqliteFunctionBehavior::INNOCUOUS | SqliteFunctionBehavior::DIRECTONLY);
        if !unsupported.is_empty() {
            return Err(registration_error(format!(
                "table-valued functions do not support {unsupported:?}"
            )));
        }
        let (name, declaration, width, parameter_count) = {
            let name = columns.function_name()?;
            let column_names = columns.column_names();
            let parameter_names = parameters.parameter_names(name)?;
            if column_names.is_empty() {
                return Err(registration_error(format!(
                    "`{name}` must return at least one column"
                )));
            }
            if let Some(mut relation_columns) = columns.relation_column_names() {
                let mut declared = column_names
                    .iter()
                    .chain(&parameter_names)
                    .copied()
                    .collect::<Vec<_>>();
                declared.sort_unstable();
                relation_columns.sort_unstable();
                if declared != relation_columns {
                    return Err(registration_error(format!(
                        "the columns and parameters of `{name}` must name each of its columns once"
                    )));
                }
            }
            (
                CString::new(name)?,
                table_declaration(&column_names, &parameter_names)?,
                column_names.len(),
                parameter_names.len(),
            )
        };
        virtual_table::register(
            &self.raw_connection,
            TableFunction {
                name,
                declaration,
                columns: width,
                parameters: parameter_count,
                behavior,
                callback: Box::new(move |connection, arguments| {
                    let arguments = parameters
                        .arguments(arguments)
                        .map_err(Error::DeserializationError)?;
                    let mut values = Vec::new();
                    for row in function(connection, arguments)? {
                        let start = values.len();
                        row.for_each_value(|value| {
                            values.push(OwnedSqliteBindValue::from(&value.inner));
                        })?;
                        let returned = values.len() - start;
                        if returned != width {
                            return Err(Error::SerializationError(
                                format!(
                                    "a table-valued function row has {returned} values for {width} columns"
                                )
                                .into(),
                            ));
                        }
                    }
                    Ok(values)
                }),
            },
        )?;
        // SQLite keeps prepared statements bound to a replaced module, so cached
        // statements would keep calling the earlier function.
        self.statement_cache.clear();
        Ok(())
    }
}

fn registration_error(message: String) -> Error {
    Error::DatabaseError(DatabaseErrorKind::UnableToSendCommand, Box::new(message))
}

/// The `CREATE TABLE` statement that declares a table-valued function's columns
/// to SQLite, with the parameters as hidden columns.
fn table_declaration(columns: &[&str], parameters: &[&str]) -> QueryResult<CString> {
    let mut sql = SqliteQueryBuilder::new();
    sql.push_sql("CREATE TABLE x(");
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            sql.push_sql(", ");
        }
        sql.push_identifier(column)?;
    }
    for parameter in parameters {
        sql.push_sql(", ");
        sql.push_identifier(parameter)?;
        sql.push_sql(" HIDDEN");
    }
    sql.push_sql(")");
    Ok(CString::new(sql.finish())?)
}

// miri doesn't support callback over ffi at this point
#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    use crate::connection::{LoadConnection, SimpleConnection};
    use crate::prelude::*;
    use crate::sql_types::Text;
    use alloc::vec;

    table! {
        grants (id) {
            id -> Integer,
            role -> Text,
            name -> Text,
        }
    }

    crate::view! {
        grantees {
            name -> Text,
            role -> Text,
        }
    }

    crate::view! {
        typed {
            number -> Integer,
            label -> Nullable<Text>,
            payload -> Binary,
            ratio -> Double,
        }
    }

    crate::view! {
        countdown {
            value -> Integer,
            start -> Integer,
        }
    }

    crate::view! {
        single {
            value -> Text,
        }
    }

    crate::view! {
        clashing {
            #[sql_name = "role"]
            first -> Text,
            #[sql_name = "role"]
            second -> Text,
        }
    }

    crate::view! {
        quoted {
            #[sql_name = "out`put \"value\""]
            output -> Text,
            #[sql_name = "in`put"]
            input -> Text,
        }
    }

    crate::allow_tables_to_appear_in_same_query!(grants, grantees);

    fn connection() -> SqliteConnection {
        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        connection
            .batch_execute(
                "CREATE TABLE grants (id INTEGER PRIMARY KEY, role TEXT NOT NULL, name TEXT NOT NULL);
                 INSERT INTO grants (role, name) VALUES ('admin', 'alice'), ('admin', 'bob'), ('guest', 'carol');",
            )
            .unwrap();
        connection
    }

    /// Registers `grantees(role)`, which reads `grants` through its own query.
    fn register_grantees(connection: &mut SqliteConnection, behavior: SqliteFunctionBehavior) {
        connection
            .register_table_valued_function(
                (grantees::name,),
                (grantees::role,),
                behavior,
                |connection, (role,): (String,)| {
                    grants::table
                        .filter(grants::role.eq(role))
                        .select((grants::name,))
                        .order(grants::name)
                        .load::<(String,)>(connection)
                },
            )
            .unwrap();
    }

    /// Registers `single`, which takes no arguments and returns `rows`.
    fn register_single<F>(
        connection: &mut SqliteConnection,
        behavior: SqliteFunctionBehavior,
        rows: F,
    ) where
        F: Fn() -> QueryResult<Vec<(String,)>> + Send + core::panic::UnwindSafe + 'static,
    {
        connection
            .register_table_valued_function((single::value,), (), behavior, move |_, ()| rows())
            .unwrap();
    }

    /// Loads the first column of every row of a SQL query as text.
    fn texts(connection: &mut SqliteConnection, query: &str) -> QueryResult<Vec<String>> {
        let mut texts = Vec::new();
        for row in connection.load(crate::sql_query(query))? {
            texts.push(
                row?.get_value::<Text, String, _>(0)
                    .map_err(Error::DeserializationError)?,
            );
        }
        Ok(texts)
    }

    #[diesel_test_helper::test]
    fn column_names_with_quote_characters_are_declared_verbatim() {
        let connection = &mut connection();
        connection
            .register_table_valued_function(
                (quoted::output,),
                (quoted::input,),
                SqliteFunctionBehavior::empty(),
                |_, (input,): (String,)| Ok(vec![(format!("{input}!"),)]),
            )
            .unwrap();

        let output = quoted::view
            .filter(quoted::input.eq("echo"))
            .select(quoted::output)
            .load::<String>(connection);
        assert_eq!(output.unwrap(), ["echo!"]);
    }

    #[diesel_test_helper::test]
    fn the_query_dsl_calls_the_function() {
        let connection = &mut connection();
        register_grantees(connection, SqliteFunctionBehavior::empty());

        let admins = grantees::view
            .filter(grantees::role.eq("admin"))
            .select(grantees::name)
            .order(grantees::name)
            .load::<String>(connection);
        assert_eq!(admins.unwrap(), ["alice", "bob"]);
        let everyone = grantees::view
            .filter(grantees::role.eq_any(["admin", "guest"]))
            .select(grantees::name)
            .order(grantees::name)
            .load::<String>(connection);
        assert_eq!(everyone.unwrap(), ["alice", "bob", "carol"]);
        let rows = grantees::view
            .filter(grantees::role.eq("guest"))
            .load::<(String, String)>(connection);
        assert_eq!(rows.unwrap(), [("carol".to_string(), "guest".to_string())]);
        assert_eq!(
            texts(connection, "SELECT name FROM grantees('admin')").unwrap(),
            ["alice", "bob"]
        );
    }

    #[diesel_test_helper::test]
    fn registering_a_name_again_replaces_the_function_in_cached_statements() {
        let connection = &mut connection();
        register_single(connection, SqliteFunctionBehavior::empty(), || {
            Ok(vec![("first".to_string(),)])
        });
        let load = |connection: &mut SqliteConnection| {
            single::view
                .select(single::value)
                .load::<String>(connection)
                .unwrap()
        };
        assert_eq!(load(connection), ["first"]);

        register_single(connection, SqliteFunctionBehavior::empty(), || {
            Ok(vec![("second".to_string(),)])
        });
        assert_eq!(load(connection), ["second"]);
    }

    #[diesel_test_helper::test]
    fn joins_bind_the_parameters_from_another_table() {
        let connection = &mut connection();
        register_grantees(connection, SqliteFunctionBehavior::empty());

        let pairs = grants::table
            .inner_join(grantees::view.on(grantees::role.eq(grants::role)))
            .select((grants::name, grantees::name))
            .order((grants::name, grantees::name))
            .load::<(String, String)>(connection);
        assert_eq!(
            pairs.unwrap(),
            [
                ("alice".to_string(), "alice".to_string()),
                ("alice".to_string(), "bob".to_string()),
                ("bob".to_string(), "alice".to_string()),
                ("bob".to_string(), "bob".to_string()),
                ("carol".to_string(), "carol".to_string()),
            ]
        );
        let peers = grants::table
            .left_join(
                grantees::view.on(grantees::role
                    .eq(grants::role)
                    .and(grantees::name.ne(grants::name))),
            )
            .select((grants::name, grantees::name.nullable()))
            .order(grants::name)
            .load::<(String, Option<String>)>(connection);
        assert_eq!(
            peers.unwrap(),
            [
                ("alice".to_string(), Some("bob".to_string())),
                ("bob".to_string(), Some("alice".to_string())),
                ("carol".to_string(), None),
            ]
        );
        let admins = grants::table
            .filter(
                grants::name.eq_any(
                    grantees::view
                        .filter(grantees::role.eq("admin"))
                        .select(grantees::name),
                ),
            )
            .select(grants::id)
            .order(grants::id)
            .load::<i32>(connection);
        assert_eq!(admins.unwrap(), [1, 2]);
    }

    #[diesel_test_helper::test]
    fn values_keep_their_sql_types() {
        let connection = &mut connection();
        connection
            .register_table_valued_function(
                (typed::number, typed::label, typed::payload, typed::ratio),
                (),
                SqliteFunctionBehavior::empty(),
                |_, ()| {
                    Ok(vec![
                        (7, Some("seven"), vec![0_u8, 1, 2], 1.5),
                        (0, None, Vec::new(), -0.5),
                    ])
                },
            )
            .unwrap();

        let rows = typed::view.load::<(i32, Option<String>, Vec<u8>, f64)>(connection);
        assert_eq!(
            rows.unwrap(),
            [
                (7, Some("seven".to_string()), vec![0, 1, 2], 1.5),
                (0, None, Vec::new(), -0.5),
            ]
        );
        assert_eq!(
            texts(
                connection,
                "SELECT typeof(number) || ' ' || typeof(label) || ' ' || typeof(payload) || ' ' || typeof(ratio) FROM typed"
            )
            .unwrap(),
            ["integer text blob real", "integer null blob real"]
        );
    }

    #[diesel_test_helper::test]
    fn errors_fail_the_statement() {
        let connection = &mut connection();
        register_single(connection, SqliteFunctionBehavior::empty(), || {
            Err(Error::QueryBuilderError("no rows today".into()))
        });

        let error = single::view.load::<(String,)>(connection).unwrap_err();
        assert!(error.to_string().contains("no rows today"), "{error}");
    }

    // wasm32-unknown-unknown aborts on panic, so there is nothing to catch there.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn a_panic_fails_the_statement() {
        let connection = &mut connection();
        register_single(connection, SqliteFunctionBehavior::empty(), || {
            panic!("boom")
        });

        let error = single::view.load::<(String,)>(connection).unwrap_err();
        assert!(error.to_string().contains("`single` panicked"), "{error}");
        let name = grants::table
            .find(1)
            .select(grants::name)
            .first::<String>(connection);
        assert_eq!(name, Ok("alice".to_string()));
    }

    #[diesel_test_helper::test]
    fn a_missing_argument_fails_to_prepare() {
        let connection = &mut connection();
        register_grantees(connection, SqliteFunctionBehavior::empty());

        for result in [
            grantees::view
                .select(grantees::name)
                .load::<String>(connection),
            grantees::view
                .filter(grantees::role.gt("a"))
                .select(grantees::name)
                .load::<String>(connection),
        ] {
            let error = result.unwrap_err();
            assert!(error.to_string().contains("no query solution"), "{error}");
        }
    }

    #[diesel_test_helper::test]
    fn registration_rejects_invalid_declarations() {
        let connection = &mut connection();
        let no_rows = |_: &mut BorrowedSqliteConnection, _: (String,)| Ok(Vec::<(String,)>::new());

        let error = connection
            .register_table_valued_function(
                (grantees::name,),
                (grantees::role,),
                SqliteFunctionBehavior::DETERMINISTIC,
                no_rows,
            )
            .unwrap_err();
        assert!(error.to_string().contains("DETERMINISTIC"), "{error}");
        let error = connection
            .register_table_valued_function(
                (grantees::name,),
                (),
                SqliteFunctionBehavior::empty(),
                |_, ()| Ok(Vec::<(String,)>::new()),
            )
            .unwrap_err();
        assert!(
            error.to_string().contains(
                "the columns and parameters of `grantees` must name each of its columns once"
            ),
            "{error}"
        );
        let error = connection
            .register_table_valued_function(
                (grantees::name, grantees::role),
                (grantees::role,),
                SqliteFunctionBehavior::empty(),
                |_, _: (String,)| Ok(Vec::<(String, String)>::new()),
            )
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must name each of its columns once"),
            "{error}"
        );
    }

    #[diesel_test_helper::test]
    fn a_rejected_declaration_reports_sqlite_message() {
        let connection = &mut connection();
        connection
            .register_table_valued_function(
                (clashing::first,),
                (clashing::second,),
                SqliteFunctionBehavior::empty(),
                |_, _: (String,)| Ok(Vec::<(String,)>::new()),
            )
            .unwrap();

        let error = clashing::view
            .filter(clashing::second.eq("admin"))
            .select(clashing::first)
            .load::<String>(connection)
            .unwrap_err();
        assert!(
            error.to_string().contains("duplicate column name: role"),
            "{error}"
        );
    }

    #[diesel_test_helper::test]
    fn behavior_controls_use_from_views_with_an_untrusted_schema() {
        let connection = &mut connection();
        connection.set_trusted_schema(false).unwrap();
        register_grantees(connection, SqliteFunctionBehavior::INNOCUOUS);
        register_single(connection, SqliteFunctionBehavior::empty(), || {
            Ok(vec![("hidden".to_string(),)])
        });
        connection
            .batch_execute(
                "CREATE VIEW admins AS SELECT name FROM grantees('admin');
                 CREATE VIEW untrusted_view AS SELECT value FROM single;",
            )
            .unwrap();

        assert_eq!(
            texts(connection, "SELECT name FROM admins").unwrap(),
            ["alice", "bob"]
        );
        let error = texts(connection, "SELECT value FROM untrusted_view").unwrap_err();
        assert!(
            error.to_string().contains("unsafe use of virtual table"),
            "{error}"
        );
    }

    #[diesel_test_helper::test]
    fn directonly_refuses_use_from_views() {
        let connection = &mut connection();
        register_grantees(connection, SqliteFunctionBehavior::DIRECTONLY);
        connection
            .batch_execute("CREATE VIEW admins AS SELECT name FROM grantees('admin');")
            .unwrap();

        assert_eq!(
            texts(connection, "SELECT name FROM grantees('admin')").unwrap(),
            ["alice", "bob"]
        );
        let error = texts(connection, "SELECT name FROM admins").unwrap_err();
        assert!(
            error.to_string().contains("unsafe use of virtual table"),
            "{error}"
        );
    }

    #[diesel_test_helper::test]
    fn a_trigger_program_reads_what_it_wrote_through_the_function() {
        let connection = &mut connection();
        register_grantees(connection, SqliteFunctionBehavior::INNOCUOUS);
        connection
            .batch_execute(
                "CREATE TABLE requests (role TEXT NOT NULL, name TEXT NOT NULL);
                 CREATE TABLE audit (name TEXT NOT NULL);
                 CREATE TRIGGER grant_request AFTER INSERT ON requests BEGIN
                     INSERT INTO grants (role, name) VALUES (NEW.role, NEW.name);
                     INSERT INTO audit (name) SELECT name FROM grantees(NEW.role);
                     INSERT INTO grants (role, name) SELECT NEW.role, name || '+' FROM grantees(NEW.role);
                 END;
                 INSERT INTO requests (role, name) VALUES ('guest', 'dave');",
            )
            .unwrap();

        assert_eq!(
            texts(connection, "SELECT name FROM audit ORDER BY name").unwrap(),
            ["carol", "dave"]
        );
        let guests = grantees::view
            .filter(grantees::role.eq("guest"))
            .select(grantees::name)
            .order(grantees::name)
            .load::<String>(connection);
        assert_eq!(guests.unwrap(), ["carol", "carol+", "dave", "dave+"]);
    }

    #[diesel_test_helper::test]
    fn a_function_may_call_itself() {
        let connection = &mut connection();
        connection
            .register_table_valued_function(
                (countdown::value,),
                (countdown::start,),
                SqliteFunctionBehavior::empty(),
                |connection, (start,): (i32,)| {
                    let mut rows = vec![(start,)];
                    if start > 0 {
                        rows.extend(
                            countdown::view
                                .filter(countdown::start.eq(start - 1))
                                .select((countdown::value,))
                                .load::<(i32,)>(connection)?,
                        );
                    }
                    Ok(rows)
                },
            )
            .unwrap();

        let values = countdown::view
            .filter(countdown::start.eq(3))
            .select(countdown::value)
            .load::<i32>(connection);
        assert_eq!(values.unwrap(), [3, 2, 1, 0]);
    }

    /// Columns named at run time, the way a crate outside diesel declares them.
    struct RuntimeColumns {
        function: &'static str,
        columns: Vec<&'static str>,
    }

    impl SqliteTableFunctionColumns<RuntimeColumns> for RuntimeColumns {
        type SqlType = ();

        fn function_name(&self) -> QueryResult<&str> {
            Ok(self.function)
        }

        fn column_names(&self) -> Vec<&str> {
            self.columns.clone()
        }

        fn relation_column_names(&self) -> Option<Vec<&str>> {
            None
        }
    }

    /// A row whose width is known only at run time.
    struct RuntimeRow(Vec<i64>);

    impl SqliteTableFunctionRow<()> for RuntimeRow {
        fn for_each_value(&self, mut sink: impl FnMut(SqliteBindValue<'_>)) -> QueryResult<()> {
            for value in &self.0 {
                sink(SqliteBindValue::from(*value));
            }
            Ok(())
        }
    }

    #[diesel_test_helper::test]
    fn a_declaration_named_at_run_time_is_checked_row_by_row() {
        let connection = &mut connection();
        let register = |connection: &mut SqliteConnection, row: Vec<i64>| {
            connection
                .register_table_valued_function(
                    RuntimeColumns {
                        function: "runtime",
                        columns: vec!["first", "second"],
                    },
                    (),
                    SqliteFunctionBehavior::empty(),
                    move |_, ()| Ok([RuntimeRow(row.clone())]),
                )
                .unwrap();
        };

        register(connection, vec![1, 2]);
        assert_eq!(
            texts(connection, "SELECT first || ',' || second FROM runtime").unwrap(),
            ["1,2"]
        );

        register(connection, vec![1]);
        let error = texts(connection, "SELECT first FROM runtime").unwrap_err();
        assert!(
            error.to_string().contains("has 1 values for 2 columns"),
            "{error}"
        );
    }
}
