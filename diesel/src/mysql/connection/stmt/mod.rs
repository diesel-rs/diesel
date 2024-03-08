#![allow(unsafe_code)] // module uses ffi
use mysqlclient_sys as ffi;
use std::ffi::CStr;
use std::os::raw as libc;
use std::ptr::NonNull;

use super::bind::{OutputBinds, PreparedStatementBinds};
use crate::connection::statement_cache::MaybeCached;
use crate::mysql::MysqlType;
use crate::result::{DatabaseErrorKind, Error, QueryResult};

pub(super) mod iterator;
mod metadata;

pub(super) use self::metadata::{MysqlFieldMetadata, StatementMetadata};

#[allow(dead_code, missing_debug_implementations)]
// https://github.com/rust-lang/rust/issues/81658
pub struct Statement {
    stmt: NonNull<ffi::MYSQL_STMT>,
    input_binds: Option<PreparedStatementBinds>,
}

impl Statement {
    pub(crate) fn new(stmt: NonNull<ffi::MYSQL_STMT>) -> Self {
        Statement {
            stmt,
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
        Iter: IntoIterator<Item = (MysqlType, Option<Vec<u8>>)>,
    {
        let input_binds = PreparedStatementBinds::from_input_data(binds);
        self.input_bind(input_binds)
    }

    pub(super) fn input_bind(
        &mut self,
        mut input_binds: PreparedStatementBinds,
    ) -> QueryResult<()> {
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

    fn last_error_message(&self) -> String {
        unsafe { CStr::from_ptr(ffi::mysql_stmt_error(self.stmt.as_ptr())) }
            .to_string_lossy()
            .into_owned()
    }

    pub(super) fn metadata(&self) -> QueryResult<StatementMetadata> {
        use crate::result::Error::DeserializationError;

        let result_ptr = unsafe { ffi::mysql_stmt_result_metadata(self.stmt.as_ptr()) };
        self.did_an_error_occur()?;
        NonNull::new(result_ptr)
            .map(StatementMetadata::new)
            .ok_or_else(|| DeserializationError("No metadata exists".into()))
    }

    pub(super) fn did_an_error_occur(&self) -> QueryResult<()> {
        use crate::result::Error::DatabaseError;

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
        // at https://dev.mysql.com/doc/refman/8.0/en/server-error-reference.html
        // and are from the ANSI SQLSTATE standard
        match last_error_number {
            1062 | 1586 | 1859 => DatabaseErrorKind::UniqueViolation,
            1216 | 1217 | 1451 | 1452 | 1830 | 1834 => DatabaseErrorKind::ForeignKeyViolation,
            1792 => DatabaseErrorKind::ReadOnlyTransaction,
            1048 | 1364 => DatabaseErrorKind::NotNullViolation,
            3819 => DatabaseErrorKind::CheckViolation,
            1213 => DatabaseErrorKind::SerializationFailure,
            _ => DatabaseErrorKind::Unknown,
        }
    }

    /// If the pointers referenced by the `MYSQL_BIND` structures are invalidated,
    /// you must call this function again before calling `mysql_stmt_fetch`.
    pub unsafe fn bind_result(&self, binds: *mut ffi::MYSQL_BIND) -> QueryResult<()> {
        ffi::mysql_stmt_bind_result(self.stmt.as_ptr(), binds);
        self.did_an_error_occur()
    }
}

impl<'a> MaybeCached<'a, Statement> {
    pub(super) fn execute_statement(
        self,
        binds: &mut OutputBinds,
    ) -> QueryResult<StatementUse<'a>> {
        unsafe {
            binds.with_mysql_binds(|bind_ptr| self.bind_result(bind_ptr))?;
            self.execute()
        }
    }

    /// This function should be called instead of `results` on queries which
    /// have no return value. It should never be called on a statement on
    /// which `results` has previously been called?
    pub(super) unsafe fn execute(self) -> QueryResult<StatementUse<'a>> {
        ffi::mysql_stmt_execute(self.stmt.as_ptr());
        self.did_an_error_occur()?;
        ffi::mysql_stmt_store_result(self.stmt.as_ptr());
        let ret = StatementUse { inner: self };
        ret.inner.did_an_error_occur()?;
        Ok(ret)
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        unsafe { ffi::mysql_stmt_close(self.stmt.as_ptr()) };
    }
}

#[allow(missing_debug_implementations)]
pub(super) struct StatementUse<'a> {
    inner: MaybeCached<'a, Statement>,
}

impl<'a> StatementUse<'a> {
    pub(in crate::mysql::connection) fn affected_rows(&self) -> usize {
        let affected_rows = unsafe { ffi::mysql_stmt_affected_rows(self.inner.stmt.as_ptr()) };
        affected_rows as usize
    }

    /// This function should be called after `execute` only
    /// otherwise it's not guaranteed to return a valid result
    pub(in crate::mysql::connection) unsafe fn result_size(&mut self) -> QueryResult<usize> {
        let size = ffi::mysql_stmt_num_rows(self.inner.stmt.as_ptr());
        usize::try_from(size).map_err(|e| Error::DeserializationError(Box::new(e)))
    }

    pub(super) fn populate_row_buffers(&self, binds: &mut OutputBinds) -> QueryResult<Option<()>> {
        let next_row_result = unsafe { ffi::mysql_stmt_fetch(self.inner.stmt.as_ptr()) };
        match next_row_result as libc::c_uint {
            ffi::MYSQL_NO_DATA => Ok(None),
            ffi::MYSQL_DATA_TRUNCATED => binds.populate_dynamic_buffers(self).map(Some),
            0 => {
                binds.update_buffer_lengths();
                Ok(Some(()))
            }
            _error => self.inner.did_an_error_occur().map(Some),
        }
    }

    pub(in crate::mysql::connection) unsafe fn fetch_column(
        &self,
        bind: &mut ffi::MYSQL_BIND,
        idx: usize,
        offset: usize,
    ) -> QueryResult<()> {
        ffi::mysql_stmt_fetch_column(
            self.inner.stmt.as_ptr(),
            bind,
            idx as libc::c_uint,
            offset as libc::c_ulong,
        );
        self.inner.did_an_error_occur()
    }

    /// If the pointers referenced by the `MYSQL_BIND` structures are invalidated,
    /// you must call this function again before calling `mysql_stmt_fetch`.
    pub(in crate::mysql::connection) unsafe fn bind_result(
        &self,
        binds: *mut ffi::MYSQL_BIND,
    ) -> QueryResult<()> {
        self.inner.bind_result(binds)
    }
}

impl<'a> Drop for StatementUse<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::mysql_stmt_free_result(self.inner.stmt.as_ptr());
        }
    }
}
