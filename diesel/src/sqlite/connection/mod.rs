extern crate libsqlite3_sys as ffi;

#[doc(hidden)]
pub mod raw;
mod into_sqlite_result;
mod sqlite_value;
mod statement_iterator;
mod stmt;

pub use self::sqlite_value::SqliteValue;

use std::os::raw as libc;
use std::rc::Rc;

use connection::*;
use deserialize::{Queryable, QueryableByName};
use query_builder::*;
use query_builder::bind_collector::RawBytesBindCollector;
use result::*;
use self::into_sqlite_result::IntoSqliteResult;
use self::raw::RawConnection;
use self::statement_iterator::*;
use self::stmt::{Statement, StatementUse};
use sql_types::HasSqlType;
use sqlite::Sqlite;

/// Connections for the SQLite backend. Unlike other backends, "connection URLs"
/// for SQLite are file paths or special identifiers like `:memory`.
#[allow(missing_debug_implementations)]
pub struct SqliteConnection {
    statement_cache: StatementCache<Sqlite, Statement>,
    raw_connection: Rc<RawConnection>,
    transaction_manager: AnsiTransactionManager,
}

// This relies on the invariant that RawConnection or Statement are never
// leaked. If a reference to one of those was held on a different thread, this
// would not be thread safe.
unsafe impl Send for SqliteConnection {}

impl SimpleConnection for SqliteConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.raw_connection.exec(query)
    }
}

impl Connection for SqliteConnection {
    type Backend = Sqlite;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        RawConnection::establish(database_url).map(|conn| SqliteConnection {
            statement_cache: StatementCache::new(),
            raw_connection: Rc::new(conn),
            transaction_manager: AnsiTransactionManager::new(),
        })
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        try!(self.batch_execute(query));
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        let mut statement = try!(self.prepare_query(&source.as_query()));
        let statement_use = StatementUse::new(&mut statement);
        let iter = StatementIterator::new(statement_use);
        iter.collect()
    }

    #[doc(hidden)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Self::Backend> + QueryId,
        U: QueryableByName<Self::Backend>,
    {
        let mut statement = self.prepare_query(source)?;
        let statement_use = StatementUse::new(&mut statement);
        let iter = NamedStatementIterator::new(statement_use)?;
        iter.collect()
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let mut statement = try!(self.prepare_query(source));
        let mut statement_use = StatementUse::new(&mut statement);
        try!(statement_use.run());
        Ok(self.raw_connection.rows_affected_by_last_query())
    }

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }
}

/// Context is a wrapper for the SQLite function evaluation context.
#[derive(Debug)]
pub struct Context<'a> {
    ctx: *mut ffi::sqlite3_context,
    args: &'a [*mut ffi::sqlite3_value],
}

use types::FromSql;

// Context is translated from rusqlite
impl<'a> Context<'a> {
    /// Returns the number of arguments to the function.
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// Returns `true` when there is no argument.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    /// Returns the `idx`th argument as a `T`.
    ///
    /// # Failure
    ///
    /// Will panic if `idx` is greater than or equal to `self.len()`.
    ///
    /// Will return Err if the underlying SQLite type cannot be converted to a `T`.
    pub fn get<A, T>(&self, idx: usize) -> QueryResult<T>
    where
        Sqlite: HasSqlType<A>,
        T: FromSql<A, Sqlite>
    {
        let _arg = self.args[idx];
/*
        let value = unsafe { ValueRef::from_value(arg) };
        FromSql::column_result(value).map_err(|err| match err {
                                                  FromSqlError::InvalidType => {
                Error::InvalidFunctionParameterType(idx, value.data_type())
            }
                                                  FromSqlError::OutOfRange(i) => {
                                                      Error::IntegralValueOutOfRange(idx as c_int,
                                                                                     i)
                                                  }
                                                  FromSqlError::Other(err) => {
                Error::FromSqlConversionFailure(idx, value.data_type(), err)
            }
        })
*/
        unimplemented!()
    }
}

unsafe extern "C" fn free_boxed_value<T>(p: *mut ::std::os::raw::c_void) {
    let _: Box<T> = Box::from_raw(::std::mem::transmute(p));
}

impl SqliteConnection {
    fn prepare_query<T: QueryFragment<Sqlite> + QueryId>(
        &self,
        source: &T,
    ) -> QueryResult<MaybeCached<Statement>> {
        let mut statement = try!(self.cached_prepared_statement(source));

        let mut bind_collector = RawBytesBindCollector::<Sqlite>::new();
        try!(source.collect_binds(&mut bind_collector, &()));
        let metadata = bind_collector.metadata;
        let binds = bind_collector.binds;
        for (tpe, value) in metadata.into_iter().zip(binds) {
            try!(statement.bind(tpe, value));
        }

        Ok(statement)
    }

    fn cached_prepared_statement<T: QueryFragment<Sqlite> + QueryId>(
        &self,
        source: &T,
    ) -> QueryResult<MaybeCached<Statement>> {
        self.statement_cache.cached_statement(source, &[], |sql| {
            Statement::prepare(&self.raw_connection, sql)
        })
    }

    /// Expose a function to SQL
    pub fn create_scalar_function<F, T>(
        &mut self,
        fn_name: &str,
        n_arg: libc::c_int,
        deterministic: bool,
        x_func: F
    ) -> QueryResult<()>
    where
        F: FnMut(&Context) -> T,
        T: IntoSqliteResult
    {
        // create_scalar_function is translated from rusqlite

        unsafe extern "C" fn call_boxed_closure<F, T>(
            ctx: *mut ffi::sqlite3_context,
            argc: libc::c_int,
            argv: *mut *mut ffi::sqlite3_value
        )
        where
            F: FnMut(&Context) -> T,
            T: IntoSqliteResult
        {
            use std::{slice, mem};

            let ctx = Context {
                ctx: ctx,
                args: slice::from_raw_parts(argv, argc as usize),
            };

            let boxed_f: *mut F = mem::transmute(ffi::sqlite3_user_data(ctx.ctx));
            assert!(!boxed_f.is_null(), "Internal error - null function pointer");

            let t = (*boxed_f)(&ctx);

            t.into_sqlite_result(ctx.ctx);
        }

        let boxed_f: *mut F = Box::into_raw(Box::new(x_func));
        let c_name = ::std::ffi::CString::new(fn_name)?;
        let mut flags = ffi::SQLITE_UTF8;
        if deterministic {
            flags |= ffi::SQLITE_DETERMINISTIC;
        }
        let result = unsafe {
            ffi::sqlite3_create_function_v2(
                self.raw_connection.internal_connection,
                c_name.as_ptr(),
                n_arg,
                flags,
                ::std::mem::transmute(boxed_f),
                Some(call_boxed_closure::<F, T>),
                None,
                None,
                Some(free_boxed_value::<F>)
            )
        };

        match result {
            ffi::SQLITE_OK => Ok(()),
            err_code => {
                let _message = error_message(err_code);
                unimplemented!("Return appropriate Err(..)");
            }
        }
    }
}

fn error_message(err_code: libc::c_int) -> &'static str {
    ffi::code_to_str(err_code)
}

#[cfg(test)]
mod tests {
    use dsl::sql;
    use prelude::*;
    use super::*;
    use sql_types::Integer;

    #[test]
    fn prepared_statements_are_cached_when_run() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let query = ::select(1.into_sql::<Integer>());

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn sql_literal_nodes_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let query = ::select(sql::<Integer>("1"));

        assert_eq!(Ok(1), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_sql_literal_nodes_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = ::select(one_as_expr.eq(sql::<Integer>("1")));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_vec_are_not_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = ::select(one_as_expr.eq_any(vec![1, 2, 3]));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(0, connection.statement_cache.len());
    }

    #[test]
    fn queries_containing_in_with_subselect_are_cached() {
        let connection = SqliteConnection::establish(":memory:").unwrap();
        let one_as_expr = 1.into_sql::<Integer>();
        let query = ::select(one_as_expr.eq_any(::select(one_as_expr)));

        assert_eq!(Ok(true), query.get_result(&connection));
        assert_eq!(1, connection.statement_cache.len());
    }

    #[test]
    fn create_scalar_function() {
        fn f(_: &Context) -> i32 { panic!(); }

        let mut connection = SqliteConnection::establish(":memory:").unwrap();

        let result = connection.create_scalar_function("f", 0, true, f);
        assert_eq!(Ok(()), result);
    }

    #[test]
    fn create_scalar_function_return_i32() {
        use expression::sql_literal::sql;

        fn f(_: &Context) -> i32 {
            42
        }

        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        connection.create_scalar_function("f", 0, true, f).unwrap();

        let query = sql("SELECT f()");
        assert_eq!(Ok(42), query.get_result(&connection));
    }

    #[test]
    fn create_scalar_function_return_cstring() {
        use std::ffi::CString;
        use expression::sql_literal::sql;

        fn f(_: &Context) -> CString {
            CString::new("Meaning of life").unwrap()
        }

        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        connection.create_scalar_function("f", 0, true, f).unwrap();

        use types;
        let query = sql::<types::Text>("SELECT f()");
        assert_eq!(Ok("Meaning of life".to_string()), query.get_result(&connection));
    }

    #[test]
    fn create_scalar_function_return_cstr() {
        use std::ffi::CStr;
        use expression::sql_literal::sql;

        fn f(_: &Context) -> &'static CStr {
            CStr::from_bytes_with_nul(b"Meaning of life\0").unwrap()
        }

        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        connection.create_scalar_function("f", 0, true, f).unwrap();

        use types;
        let query = sql::<types::Text>("SELECT f()");
        assert_eq!(Ok("Meaning of life".to_string()), query.get_result(&connection));
    }
}
