extern crate mysqlclient_sys as ffi;

mod iterator;

use std::os::{raw as libc};
use std::ffi::CStr;

use mysql::MysqlType;
use result::{QueryResult, DatabaseErrorKind};
use self::iterator::StatementIterator;
use super::bind::Binds;

pub struct Statement {
    stmt: *mut ffi::MYSQL_STMT,
    input_binds: Option<Binds>,
}

impl Statement {
    pub fn new(stmt: *mut ffi::MYSQL_STMT) -> Self {
        Statement {
            stmt: stmt,
            input_binds: None,
        }
    }

    pub fn prepare(&self, query: &str) -> QueryResult<()> {
        unsafe {
            ffi::mysql_stmt_prepare(
                self.stmt,
                query.as_ptr() as *const libc::c_char,
                query.len() as libc::c_ulong,
            );
        }
        self.did_an_error_occur()
    }

    pub fn bind<Iter>(&mut self, binds: Iter) -> QueryResult<()> where
        Iter: IntoIterator<Item=(MysqlType, Option<Vec<u8>>)>,
    {
        let mut input_binds = Binds::from_input_data(binds);
        input_binds.with_mysql_binds(|bind_ptr| {
            // This relies on the invariant that the current value of `self.input_binds`
            // will not change without this function being called
            unsafe { ffi::mysql_stmt_bind_param(self.stmt, bind_ptr); }
        });
        self.input_binds = Some(input_binds);
        self.did_an_error_occur()
    }

    pub fn execute(&self) -> QueryResult<()> {
        unsafe { ffi::mysql_stmt_execute(self.stmt); }
        self.did_an_error_occur()
    }

    pub fn affected_rows(&self) -> usize {
        let affected_rows = unsafe { ffi::mysql_stmt_affected_rows(self.stmt) };
        affected_rows as usize
    }

    pub fn results(&mut self, types: Vec<MysqlType>) -> QueryResult<StatementIterator> {
        StatementIterator::new(self, types)
    }

    fn last_error_message(&self) -> String {
        unsafe { CStr::from_ptr(ffi::mysql_stmt_error(self.stmt)) }
            .to_string_lossy()
            .into_owned()
    }

    /// If the pointers referenced by the `MYSQL_BIND` structures are invalidated,
    /// you must call this function again before calling `mysql_stmt_fetch`.
    pub unsafe fn bind_result(&self, binds: *mut ffi::MYSQL_BIND) -> QueryResult<()> {
        ffi::mysql_stmt_bind_result(self.stmt, binds);
        self.did_an_error_occur()
    }

    pub unsafe fn fetch_column(&self, bind: &mut ffi::MYSQL_BIND, idx: usize, offset: usize)
        -> QueryResult<()>
    {
        ffi::mysql_stmt_fetch_column(
            self.stmt,
            bind,
            idx as libc::c_uint,
            offset as libc::c_ulong,
        );
        self.did_an_error_occur()
    }

    fn did_an_error_occur(&self) -> QueryResult<()> {
        use result::Error::DatabaseError;

        let error_message = self.last_error_message();
        if error_message.is_empty() {
            Ok(())
        } else {
            Err(DatabaseError(
                self.last_error_type(),
                Box::new(error_message),
            ))
        }
    }

    fn last_error_type(&self) -> DatabaseErrorKind {
        let last_error_number = unsafe  { ffi::mysql_stmt_errno(self.stmt) };
        // These values are not exposed by the C API, but are documented
        // at https://dev.mysql.com/doc/refman/5.7/en/error-messages-server.html
        // and are from the ANSI SQLSTATE standard
        match last_error_number {
            1062 | 1586 | 1859 => DatabaseErrorKind::UniqueViolation,
            _ => DatabaseErrorKind::__Unknown,
        }
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        let drop_result = unsafe { ffi::mysql_stmt_close(self.stmt) };
        // FIXME: Remove this before we ship this feature. We don't really care
        // about any of the error cases that can occur, but I suspect we'll need
        // to stick an `Rc<RawConnection>` on this struct to ensure the right
        // drop order once prepared statement caching is added. This is mostly
        // here so I don't forget.
        assert_eq!(0, drop_result, "@sgrif forgot to delete this assertion. Please open a github issue");
    }
}
