#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::raw::RawConnection;
use super::{Sqlite, SqliteAggregateFunction, SqliteBindValue, SqliteConnection};
use crate::backend::Backend;
use crate::deserialize::{FromSqlRow, StaticallySizedRow};
use crate::result::{DatabaseErrorKind, Error, QueryResult};
use crate::row::{Field, PartialRow, Row, RowIndex, RowSealed};
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::HasSqlType;
use crate::sqlite::SqliteFunctionBehavior;
use crate::sqlite::SqliteValue;
use crate::sqlite::connection::bind_collector::SqliteBindValueRef;
use crate::sqlite::connection::sqlite_value::OwnedSqliteValue;
use alloc::boxed::Box;
use alloc::string::ToString;

pub(super) fn register<ArgsSqlType, RetSqlType, Args, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    behavior: SqliteFunctionBehavior,
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

    conn.register_sql_function(fn_name, fields_needed, behavior, move |conn, args| {
        let args = build_sql_function_args::<ArgsSqlType, Args>(args, conn.internal_connection)?;

        Ok(f(conn, args))
    })?;
    Ok(())
}

pub(super) fn register_noargs<RetSqlType, Ret, F>(
    conn: &RawConnection,
    fn_name: &str,
    behavior: SqliteFunctionBehavior,
    mut f: F,
) -> QueryResult<()>
where
    F: FnMut() -> Ret + core::panic::UnwindSafe + Send + 'static,
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    conn.register_sql_function(fn_name, 0, behavior, move |_, _| Ok(f()))?;
    Ok(())
}

pub(super) fn register_aggregate<ArgsSqlType, RetSqlType, Args, Ret, A>(
    conn: &RawConnection,
    fn_name: &str,
    behavior: SqliteFunctionBehavior,
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
        behavior,
    )?;

    Ok(())
}

pub(super) fn build_sql_function_args<ArgsSqlType, Args>(
    args: &mut [*mut ffi::sqlite3_value],
    connection: core::ptr::NonNull<ffi::sqlite3>,
) -> Result<Args, Error>
where
    Args: FromSqlRow<ArgsSqlType, Sqlite>,
{
    let row = FunctionRow::new(args, connection);
    Args::build_from_row(&row).map_err(Error::DeserializationError)
}

// clippy is wrong here, the let binding is required
// for lifetime reasons
#[allow(clippy::let_unit_value)]
pub(super) fn process_sql_function_result<RetSqlType, Ret>(
    result: &'_ Ret,
) -> QueryResult<SqliteBindValueRef<'_>>
where
    Ret: ToSql<RetSqlType, Sqlite>,
    Sqlite: HasSqlType<RetSqlType>,
{
    let mut metadata_lookup = ();
    let value = SqliteBindValue {
        inner: SqliteBindValueRef::Null,
    };
    let mut buf = Output::new(value, &mut metadata_lookup);
    let is_null = result.to_sql(&mut buf).map_err(Error::SerializationError)?;

    if let IsNull::Yes = is_null {
        Ok(SqliteBindValueRef::Null)
    } else {
        Ok(buf.into_inner().inner)
    }
}

struct FunctionRow<'a> {
    args: &'a [Option<OwnedSqliteValue>],
    field_count: usize,
    connection: core::ptr::NonNull<ffi::sqlite3>,
}

impl FunctionRow<'_> {
    #[allow(unsafe_code)] // complicated ptr cast
    fn new(
        args: &mut [*mut ffi::sqlite3_value],
        connection: core::ptr::NonNull<ffi::sqlite3>,
    ) -> Self {
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
            connection,
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
            connection: self.connection,
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
    connection: core::ptr::NonNull<ffi::sqlite3>,
}

impl<'a> Field<'a, Sqlite> for FunctionArgument<'a> {
    fn field_name(&self) -> Option<&str> {
        None
    }

    fn is_null(&self) -> bool {
        self.value().is_none()
    }

    fn value(&self) -> Option<<Sqlite as Backend>::RawValue<'_>> {
        SqliteValue::from_function_row(self.args, self.col_idx, self.connection)
    }
}

impl SqliteConnection {
    #[doc(hidden)]
    pub fn register_sql_function<ArgsSqlType, RetSqlType, Args, Ret, F>(
        &mut self,
        fn_name: &str,
        behavior: SqliteFunctionBehavior,
        mut f: F,
    ) -> QueryResult<()>
    where
        F: FnMut(Args) -> Ret + core::panic::UnwindSafe + Send + 'static,
        Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        register(&self.raw_connection, fn_name, behavior, move |_, args| {
            f(args)
        })
    }

    #[doc(hidden)]
    pub fn register_noarg_sql_function<RetSqlType, Ret, F>(
        &mut self,
        fn_name: &str,
        behavior: SqliteFunctionBehavior,
        f: F,
    ) -> QueryResult<()>
    where
        F: FnMut() -> Ret + core::panic::UnwindSafe + Send + 'static,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        register_noargs(&self.raw_connection, fn_name, behavior, f)
    }

    #[doc(hidden)]
    pub fn register_aggregate_function<ArgsSqlType, RetSqlType, Args, Ret, A>(
        &mut self,
        fn_name: &str,
        behavior: SqliteFunctionBehavior,
    ) -> QueryResult<()>
    where
        A: SqliteAggregateFunction<Args, Output = Ret> + 'static + Send + core::panic::UnwindSafe,
        Args: FromSqlRow<ArgsSqlType, Sqlite> + StaticallySizedRow<ArgsSqlType, Sqlite>,
        Ret: ToSql<RetSqlType, Sqlite>,
        Sqlite: HasSqlType<RetSqlType>,
    {
        register_aggregate::<_, _, _, _, A>(&self.raw_connection, fn_name, behavior)
    }

    /// Register a collation function.
    ///
    /// `collation` must always return the same answer given the same inputs.
    /// If `collation` panics and unwinds the stack, the process is aborted, since it is used
    /// across a C FFI boundary, which cannot be unwound across and there is no way to
    /// signal failures via the SQLite interface in this case..
    ///
    /// If the name is already registered it will be overwritten.
    ///
    /// This method will return an error if registering the function fails, either due to an
    /// out-of-memory situation or because a collation with that name already exists and is
    /// currently being used in parallel by a query.
    ///
    /// The collation needs to be specified when creating a table:
    /// `CREATE TABLE my_table ( str TEXT COLLATE MY_COLLATION )`,
    /// where `MY_COLLATION` corresponds to name passed as `collation_name`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// // sqlite NOCASE only works for ASCII characters,
    /// // this collation allows handling UTF-8 (barring locale differences)
    /// conn.register_collation("RUSTNOCASE", |rhs, lhs| {
    ///     rhs.to_lowercase().cmp(&lhs.to_lowercase())
    /// })
    /// # }
    /// ```
    pub fn register_collation<F>(&mut self, collation_name: &str, collation: F) -> QueryResult<()>
    where
        F: Fn(&str, &str) -> core::cmp::Ordering + Send + 'static + core::panic::UnwindSafe,
    {
        self.raw_connection
            .register_collation_function(collation_name, collation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::SimpleConnection;
    use crate::prelude::*;
    use crate::sql_types::{Integer, Text};

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    #[declare_sql_function]
    extern "SQL" {
        fn fun_case(x: Text) -> Text;
        fn my_add(x: Integer, y: Integer) -> Integer;
        fn answer() -> Integer;
        fn add_counter(x: Integer) -> Integer;

        #[aggregate]
        fn my_sum(expr: Integer) -> Integer;
        #[aggregate]
        fn range_max(expr1: Integer, expr2: Integer, expr3: Integer) -> Nullable<Integer>;
    }

    #[diesel_test_helper::test]
    fn register_custom_function() {
        let connection = &mut connection();
        fun_case_utils::register_impl(connection, |x: String| {
            x.chars()
                .enumerate()
                .map(|(i, c)| {
                    if i % 2 == 0 {
                        c.to_lowercase().to_string()
                    } else {
                        c.to_uppercase().to_string()
                    }
                })
                .collect::<String>()
        })
        .unwrap();

        let mapped_string = crate::select(fun_case("foobar"))
            .get_result::<String>(connection)
            .unwrap();
        assert_eq!("fOoBaR", mapped_string);
    }

    #[diesel_test_helper::test]
    fn register_multiarg_function() {
        let connection = &mut connection();
        my_add_utils::register_impl(connection, |x: i32, y: i32| x + y).unwrap();

        let added = crate::select(my_add(1, 2)).get_result::<i32>(connection);
        assert_eq!(Ok(3), added);
    }

    #[diesel_test_helper::test]
    fn register_noarg_function() {
        let connection = &mut connection();
        answer_utils::register_impl(connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[diesel_test_helper::test]
    fn register_nondeterministic_noarg_function() {
        let connection = &mut connection();
        answer_utils::register_nondeterministic_impl(connection, || 42).unwrap();

        let answer = crate::select(answer()).get_result::<i32>(connection);
        assert_eq!(Ok(42), answer);
    }

    #[diesel_test_helper::test]
    fn register_nondeterministic_function() {
        let connection = &mut connection();
        let mut y = 0;
        add_counter_utils::register_nondeterministic_impl(connection, move |x: i32| {
            y += 1;
            x + y
        })
        .unwrap();

        let added = crate::select((add_counter(1), add_counter(1), add_counter(1)))
            .get_result::<(i32, i32, i32)>(connection);
        assert_eq!(Ok((2, 3, 4)), added);
    }

    #[derive(Default)]
    struct MySum {
        sum: i32,
    }

    impl SqliteAggregateFunction<i32> for MySum {
        type Output = i32;

        fn step(&mut self, expr: i32) {
            self.sum += expr;
        }

        fn finalize(aggregator: Option<Self>) -> Self::Output {
            aggregator.map(|a| a.sum).unwrap_or_default()
        }
    }

    table! {
        my_sum_example {
            id -> Integer,
            value -> Integer,
        }
    }

    #[diesel_test_helper::test]
    fn register_aggregate_function() {
        use self::my_sum_example::dsl::*;

        let connection = &mut connection();
        crate::sql_query(
            "CREATE TABLE my_sum_example (id integer primary key autoincrement, value integer)",
        )
        .execute(connection)
        .unwrap();
        crate::sql_query("INSERT INTO my_sum_example (value) VALUES (1), (2), (3)")
            .execute(connection)
            .unwrap();

        my_sum_utils::register_impl_with_behavior::<MySum, _>(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
        )
        .unwrap();

        let result = my_sum_example
            .select(my_sum(value))
            .get_result::<i32>(connection);
        assert_eq!(Ok(6), result);
    }

    #[diesel_test_helper::test]
    fn register_aggregate_function_returns_finalize_default_on_empty_set() {
        use self::my_sum_example::dsl::*;

        let connection = &mut connection();
        crate::sql_query(
            "CREATE TABLE my_sum_example (id integer primary key autoincrement, value integer)",
        )
        .execute(connection)
        .unwrap();

        my_sum_utils::register_impl_with_behavior::<MySum, _>(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
        )
        .unwrap();

        let result = my_sum_example
            .select(my_sum(value))
            .get_result::<i32>(connection);
        assert_eq!(Ok(0), result);
    }

    #[derive(Default)]
    struct RangeMax<T> {
        max_value: Option<T>,
    }

    impl<T: Default + Ord + Copy + Clone> SqliteAggregateFunction<(T, T, T)> for RangeMax<T> {
        type Output = Option<T>;

        fn step(&mut self, (x0, x1, x2): (T, T, T)) {
            let max = if x0 >= x1 && x0 >= x2 {
                x0
            } else if x1 >= x0 && x1 >= x2 {
                x1
            } else {
                x2
            };

            self.max_value = match self.max_value {
                Some(current_max_value) if max > current_max_value => Some(max),
                None => Some(max),
                _ => self.max_value,
            };
        }

        fn finalize(aggregator: Option<Self>) -> Self::Output {
            aggregator?.max_value
        }
    }

    table! {
        range_max_example {
            id -> Integer,
            value1 -> Integer,
            value2 -> Integer,
            value3 -> Integer,
        }
    }

    #[diesel_test_helper::test]
    fn register_aggregate_multiarg_function() {
        use self::range_max_example::dsl::*;

        let connection = &mut connection();
        crate::sql_query(
            r#"CREATE TABLE range_max_example (
                id integer primary key autoincrement,
                value1 integer,
                value2 integer,
                value3 integer
            )"#,
        )
        .execute(connection)
        .unwrap();
        crate::sql_query(
            "INSERT INTO range_max_example (value1, value2, value3) VALUES (3, 2, 1), (2, 2, 2)",
        )
        .execute(connection)
        .unwrap();

        range_max_utils::register_impl_with_behavior::<RangeMax<i32>, _, _, _>(
            connection,
            SqliteFunctionBehavior::DETERMINISTIC,
        )
        .unwrap();
        let result = range_max_example
            .select(range_max(value1, value2, value3))
            .get_result::<Option<i32>>(connection)
            .unwrap();
        assert_eq!(Some(3), result);
    }

    table! {
        my_collation_example {
            id -> Integer,
            value -> Text,
        }
    }

    #[diesel_test_helper::test]
    fn register_collation_function() {
        use self::my_collation_example::dsl::*;

        let connection = &mut connection();

        connection
            .register_collation("RUSTNOCASE", |rhs, lhs| {
                rhs.to_lowercase().cmp(&lhs.to_lowercase())
            })
            .unwrap();

        crate::sql_query(
                "CREATE TABLE my_collation_example (id integer primary key autoincrement, value text collate RUSTNOCASE)",
            ).execute(connection)
            .unwrap();
        crate::sql_query(
            "INSERT INTO my_collation_example (value) VALUES ('foo'), ('FOo'), ('f00')",
        )
        .execute(connection)
        .unwrap();

        let result = my_collation_example
            .filter(value.eq("foo"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["foo".to_owned(), "FOo".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("FOO"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["foo".to_owned(), "FOo".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("f00"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["f00".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("F00"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(
            Ok(&["f00".to_owned()][..]),
            result.as_ref().map(|vec| vec.as_ref())
        );

        let result = my_collation_example
            .filter(value.eq("oof"))
            .select(value)
            .load::<String>(connection);
        assert_eq!(Ok(&[][..]), result.as_ref().map(|vec| vec.as_ref()));
    }

    #[diesel_test_helper::test]
    fn aggregate_function_works_with_aligned_data() {
        #[derive(Debug, Default)]
        #[repr(align(64))]
        struct OverAligned;

        impl SqliteAggregateFunction<i32> for OverAligned {
            type Output = i64;

            fn step(&mut self, _value: i32) {
                let need = core::mem::align_of::<Self>();
                let got = core::mem::align_of_val(self);
                assert_eq!(need, got);
            }

            fn finalize(_agg: Option<Self>) -> i64 {
                0
            }
        }
        #[declare_sql_function]
        extern "SQL" {
            #[aggregate]
            fn over_aligned_sum(x: Integer) -> diesel::sql_types::BigInt;
        }

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        over_aligned_sum_utils::register_impl::<OverAligned, _>(&mut conn).unwrap();

        diesel::select(over_aligned_sum(1))
            .execute(&mut conn)
            .unwrap();
    }

    #[diesel_test_helper::test]
    fn sum_twice() {
        #[derive(Default)]
        struct Sum(i32);

        impl SqliteAggregateFunction<i32> for Sum {
            type Output = i32;

            fn step(&mut self, value: i32) {
                self.0 += value;
            }

            fn finalize(agg: Option<Self>) -> i32 {
                agg.map(|s| s.0).unwrap_or_default()
            }
        }

        #[declare_sql_function]
        extern "SQL" {
            #[aggregate]
            fn my_sum(x: Integer) -> Integer;
        }

        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        my_sum_utils::register_impl::<Sum, _>(&mut conn).unwrap();

        conn.batch_execute(
            "
            CREATE TABLE test(key1 INTEGER, key2 INTEGER);
            INSERT INTO test(key1, key2) VALUES (1, 2), (2, 4), (3, 6);
",
        )
        .unwrap();

        table! {
            test (key1, key2) {
                key1 -> Integer,
                key2 -> Integer,
            }
        }

        let (first_res, second_res) = test::table
            .select((my_sum(test::key1), my_sum(test::key2)))
            .get_result::<(i32, i32)>(&mut conn)
            .unwrap();

        assert_eq!(first_res, 6);
        assert_eq!(second_res, 12);

        conn.batch_execute("DELETE FROM test").unwrap();
        let (first_res, second_res) = test::table
            .select((my_sum(test::key1), my_sum(test::key2)))
            .get_result::<(i32, i32)>(&mut conn)
            .unwrap();

        assert_eq!(first_res, 0);
        assert_eq!(second_res, 0);
    }

    // ---- DIRECTONLY / INNOCUOUS function behavior tests ----

    #[declare_sql_function]
    extern "SQL" {
        fn directonly_fn() -> Integer;
        fn innocuous_fn() -> Integer;
    }

    #[diesel_test_helper::test]
    fn directonly_function_blocked_from_view() {
        let conn = &mut connection();

        // Register a DIRECTONLY function
        directonly_fn_utils::register_impl_with_behavior(
            conn,
            SqliteFunctionBehavior::DIRECTONLY,
            || 42,
        )
        .unwrap();

        // Direct call works
        let result = crate::select(directonly_fn()).get_result::<i32>(conn);
        assert_eq!(Ok(42), result);

        // Create a view that calls the function
        crate::sql_query("CREATE VIEW test_view AS SELECT directonly_fn() AS val")
            .execute(conn)
            .unwrap();

        // Disable trusted schema so DIRECTONLY is enforced from schema objects
        conn.set_trusted_schema(false).unwrap();

        // Querying the view should fail because the function is DIRECTONLY
        let result = crate::sql_query("SELECT val FROM test_view").execute(conn);
        assert!(result.is_err());
    }

    #[diesel_test_helper::test]
    fn innocuous_function_allowed_from_view_with_untrusted_schema() {
        let conn = &mut connection();

        // Register an INNOCUOUS function
        innocuous_fn_utils::register_impl_with_behavior(
            conn,
            SqliteFunctionBehavior::DETERMINISTIC | SqliteFunctionBehavior::INNOCUOUS,
            || 99,
        )
        .unwrap();

        // Create a view that calls the function
        crate::sql_query("CREATE VIEW innocuous_view AS SELECT innocuous_fn() AS val")
            .execute(conn)
            .unwrap();

        // Disable trusted schema
        conn.set_trusted_schema(false).unwrap();

        // Querying the view should succeed because the function is INNOCUOUS
        let result = crate::sql_query("SELECT val FROM innocuous_view").execute(conn);
        assert!(result.is_ok());
    }
}
