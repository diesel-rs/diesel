extern crate mysqlclient_sys as ffi;

mod iterator;
mod metadata;

use std::ffi::CStr;
use std::os::raw as libc;
use std::ptr::NonNull;

use self::iterator::*;
use self::metadata::*;
use super::bind::Binds;
use mysql::MysqlType;
use result::{DatabaseErrorKind, QueryResult};
use sql_types::IsSigned;

pub struct Statement {
    stmt: NonNull<ffi::MYSQL_STMT>,
    input_binds: Option<Binds>,
}

impl Statement {
    pub(crate) fn new(stmt: NonNull<ffi::MYSQL_STMT>) -> Self {
        Statement {
            stmt: stmt,
            input_binds: None,
        }
    }

    pub fn prepare(&self, query: &str) -> QueryResult<()> {
        unsafe {
            ffi::mysql_stmt_prepare(
                self.stmt.as_ptr(),
                query.as_ptr() as *const libc::c_char,
                query.len() as libc::c_ulong,
            );
        }
        self.did_an_error_occur()
    }

    pub fn bind<Iter>(&mut self, binds: Iter) -> QueryResult<()>
    where
        Iter: IntoIterator<Item = (MysqlType, IsSigned, Option<Vec<u8>>)>,
    {
        let mut input_binds = Binds::from_input_data(binds);
        input_binds.with_mysql_binds(|bind_ptr| {
            // This relies on the invariant that the current value of `self.input_binds`
            // will not change without this function being called
            unsafe {
                ffi::mysql_stmt_bind_param(self.stmt.as_ptr(), bind_ptr);
            }
        });
        self.input_binds = Some(input_binds);
        self.did_an_error_occur()
    }

    /// This function should be called instead of `results` on queries which
    /// have no return value. It should never be called on a statement on
    /// which `results` has previously been called?
    pub unsafe fn execute(&self) -> QueryResult<()> {
        ffi::mysql_stmt_execute(self.stmt.as_ptr());
        self.did_an_error_occur()?;
        ffi::mysql_stmt_store_result(self.stmt.as_ptr());
        self.did_an_error_occur()?;
        Ok(())
    }

    pub fn affected_rows(&self) -> usize {
        let affected_rows = unsafe { ffi::mysql_stmt_affected_rows(self.stmt.as_ptr()) };
        affected_rows as usize
    }

    /// This function should be called instead of `execute` for queries which
    /// have a return value. After calling this function, `execute` can never
    /// be called on this statement.
    pub unsafe fn results(
        &mut self,
        types: Vec<(MysqlType, IsSigned)>,
    ) -> QueryResult<StatementIterator> {
        StatementIterator::new(self, types)
    }

    /// This function should be called instead of `execute` for queries which
    /// have a return value. After calling this function, `execute` can never
    /// be called on this statement.
    pub unsafe fn named_results(&mut self) -> QueryResult<NamedStatementIterator> {
        NamedStatementIterator::new(self)
    }

    fn last_error_message(&self) -> String {
        unsafe { CStr::from_ptr(ffi::mysql_stmt_error(self.stmt.as_ptr())) }
            .to_string_lossy()
            .into_owned()
    }

    /// If the pointers referenced by the `MYSQL_BIND` structures are invalidated,
    /// you must call this function again before calling `mysql_stmt_fetch`.
    pub unsafe fn bind_result(&self, binds: *mut ffi::MYSQL_BIND) -> QueryResult<()> {
        ffi::mysql_stmt_bind_result(self.stmt.as_ptr(), binds);
        self.did_an_error_occur()
    }

    pub unsafe fn fetch_column(
        &self,
        bind: &mut ffi::MYSQL_BIND,
        idx: usize,
        offset: usize,
    ) -> QueryResult<()> {
        ffi::mysql_stmt_fetch_column(
            self.stmt.as_ptr(),
            bind,
            idx as libc::c_uint,
            offset as libc::c_ulong,
        );
        self.did_an_error_occur()
    }

    fn metadata(&self) -> QueryResult<StatementMetadata> {
        use result::Error::DeserializationError;

        let result_ptr = unsafe { ffi::mysql_stmt_result_metadata(self.stmt.as_ptr()).as_mut() };
        self.did_an_error_occur()?;
        result_ptr
            .map(StatementMetadata::new)
            .ok_or_else(|| DeserializationError("No metadata exists".into()))
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
        let last_error_number = unsafe { ffi::mysql_stmt_errno(self.stmt.as_ptr()) };
        // These values are not exposed by the C API, but are documented
        // at https://dev.mysql.com/doc/refman/5.7/en/error-messages-server.html
        // and are from the ANSI SQLSTATE standard
        match last_error_number {
            1062 | 1586 | 1859 => DatabaseErrorKind::UniqueViolation,
            1216 | 1217 | 1451 | 1452 | 1830 | 1834 => DatabaseErrorKind::ForeignKeyViolation,
            _ => DatabaseErrorKind::__Unknown,
        }
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        unsafe { ffi::mysql_stmt_close(self.stmt.as_ptr()) };
    }
}
