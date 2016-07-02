extern crate pq_sys;
extern crate libc;

use result::{Error, QueryResult, DatabaseErrorInformation, DatabaseErrorKind};
use super::row::PgRow;

use self::pq_sys::*;
use std::ffi::CStr;
use std::{str, slice, mem};

pub struct PgResult {
    internal_result: *mut PGresult,
}

impl PgResult {
    pub fn new(internal_result: *mut PGresult) -> QueryResult<Self> {
        let result_status = unsafe { PQresultStatus(internal_result) };
        match result_status {
            PGRES_COMMAND_OK | PGRES_TUPLES_OK => {
                Ok(PgResult {
                    internal_result: internal_result,
                })
            },
            _ => {
                let error_information = Box::new(PgErrorInformation(internal_result));
                let error_kind = match get_result_field(internal_result, ResultField::SqlState) {
                    Some(error_codes::UNIQUE_VIOLATION) => DatabaseErrorKind::UniqueViolation,
                    _ => DatabaseErrorKind::__Unknown,
                };
                Err(Error::DatabaseError(error_kind, error_information))
            }
        }
    }

    pub fn rows_affected(&self) -> usize {
        unsafe {
            let count_char_ptr = PQcmdTuples(self.internal_result);
            let count_bytes = CStr::from_ptr(count_char_ptr).to_bytes();
            let count_str = str::from_utf8_unchecked(count_bytes);
            match count_str {
                "" => 0,
                _ => count_str.parse().unwrap()
            }
        }
    }

    pub fn num_rows(&self) -> usize {
        unsafe { PQntuples(self.internal_result) as usize }
    }

    pub fn get_row(&self, idx: usize) -> PgRow {
        PgRow::new(self, idx)
    }

    pub fn get(&self, row_idx: usize, col_idx: usize) -> Option<&[u8]> {
        if self.is_null(row_idx, col_idx) {
            None
        } else {
            let row_idx = row_idx as libc::c_int;
            let col_idx = col_idx as libc::c_int;
            unsafe {
                let value_ptr = PQgetvalue(self.internal_result, row_idx, col_idx);
                let value_ptr = mem::transmute::<_, *const u8>(value_ptr);
                let num_bytes = PQgetlength(self.internal_result, row_idx, col_idx);
                Some(slice::from_raw_parts(value_ptr, num_bytes as usize))
            }
        }
    }

    pub fn is_null(&self, row_idx: usize, col_idx: usize) -> bool {
        unsafe {
            0 != PQgetisnull(
                self.internal_result,
                row_idx as libc::c_int,
                col_idx as libc::c_int,
            )
        }
    }
}

impl Drop for PgResult {
    fn drop(&mut self) {
        unsafe { PQclear(self.internal_result) };
    }
}

struct PgErrorInformation(*mut PGresult);

unsafe impl Send for PgErrorInformation {}

impl Drop for PgErrorInformation {
    fn drop(&mut self) {
        unsafe { PQclear(self.0) };
    }
}

impl DatabaseErrorInformation for PgErrorInformation {
    fn message(&self) -> &str {
        match get_result_field(self.0, ResultField::MessagePrimary) {
            Some(e) => e,
            None => unreachable!("Per PGs documentation, all errors should have a message"),
        }
    }

    fn details(&self) -> Option<&str> {
        get_result_field(self.0, ResultField::MessageDetail)
    }

    fn hint(&self) -> Option<&str> {
        get_result_field(self.0, ResultField::MessageHint)
    }

    fn table_name(&self) -> Option<&str> {
        get_result_field(self.0, ResultField::TableName)
    }

    fn column_name(&self) -> Option<&str> {
        get_result_field(self.0, ResultField::ColumnName)
    }

    fn constraint_name(&self) -> Option<&str> {
        get_result_field(self.0, ResultField::ConstraintName)
    }
}

/// Represents valid options to
/// [`PQresultErrorField`](https://www.postgresql.org/docs/current/static/libpq-exec.html#LIBPQ-PQRESULTERRORFIELD)
/// Their values are defined as C preprocessor macros, and therefore are not exported by libpq-sys.
/// Their values can be found in `postgres_ext.h`
#[repr(i32)]
enum ResultField {
    SqlState = 'C' as i32,
    MessagePrimary = 'M' as i32,
    MessageDetail = 'D' as i32,
    MessageHint = 'H' as i32,
    TableName = 't' as i32,
    ColumnName = 'c' as i32,
    ConstraintName = 'n' as i32,
}

fn get_result_field<'a>(res: *mut PGresult, field: ResultField) -> Option<&'a str> {
    let ptr = unsafe { PQresultErrorField(res, field as libc::c_int) };
    if ptr.is_null() {
        return None;
    }

    let c_str = unsafe { CStr::from_ptr(ptr) };
    c_str.to_str().ok()
}

mod error_codes {
    //! These error codes are documented at
    //! https://www.postgresql.org/docs/9.5/static/errcodes-appendix.html
    //!
    //! They are not exposed programatically through libpq.
    pub const UNIQUE_VIOLATION: &'static str = "23505";
}
