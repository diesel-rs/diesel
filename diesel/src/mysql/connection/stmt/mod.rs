extern crate mysqlclient_sys as ffi;

mod iterator;

use std::os::{raw as libc};
use std::ffi::CStr;

use mysql::MysqlType;
use result::QueryResult;
use self::iterator::StatementIterator;
use super::bind::Binds;
use super::result::RawResult;

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

    pub fn bind(&mut self, binds: Vec<(MysqlType, Option<Vec<u8>>)>) -> QueryResult<()> {
        let mut input_binds = Binds::from_input_data(binds);
        let bind_ptr = input_binds.mysql_binds().as_mut_ptr();
        self.input_binds = Some(input_binds);
        // This relies on the invariant that the current value of `self.input_binds`
        // will not change without this function being called
        unsafe { ffi::mysql_stmt_bind_param(self.stmt, bind_ptr); }
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

    pub fn results(&mut self) -> QueryResult<StatementIterator> {
        StatementIterator::new(self)
    }

    fn result_metadata(&self) -> QueryResult<Option<RawResult>> {
        let result = unsafe {
            let result_ptr = ffi::mysql_stmt_result_metadata(self.stmt);
            RawResult::from_raw(result_ptr)
        };
        self.did_an_error_occur()?;
        Ok(result)
    }

    fn last_error_message(&self) -> String {
        unsafe { CStr::from_ptr(ffi::mysql_stmt_error(self.stmt)) }
            .to_string_lossy()
            .into_owned()
    }

    fn did_an_error_occur(&self) -> QueryResult<()> {
        use result::DatabaseErrorKind;
        use result::Error::DatabaseError;

        let error_message = self.last_error_message();
        if error_message.is_empty() {
            Ok(())
        } else {
            Err(DatabaseError(
                DatabaseErrorKind::__Unknown,
                Box::new(error_message),
            ))
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

