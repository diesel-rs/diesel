extern crate pq_sys;

use self::pq_sys::*;
use std::ffi::{CStr, CString};
use std::num::NonZeroU32;
use std::os::raw as libc;
use std::{slice, str};

use super::raw::RawResult;
use super::row::PgRow;
use pg::PgConnection;
use result::{DatabaseErrorInformation, DatabaseErrorKind, Error, QueryResult};

pub struct PgResult<'a> {
    internal_result: RawResult,
    pub(crate) connection: &'a PgConnection,
}

impl<'a> PgResult<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(internal_result: RawResult, connection: &'a PgConnection) -> QueryResult<Self> {
        use self::ExecStatusType::*;

        let result_status = unsafe { PQresultStatus(internal_result.as_ptr()) };
        match result_status {
            PGRES_COMMAND_OK | PGRES_TUPLES_OK => Ok(PgResult {
                internal_result,
                connection,
            }),
            PGRES_EMPTY_QUERY => {
                let error_message = "Received an empty query".to_string();
                Err(Error::DatabaseError(
                    DatabaseErrorKind::__Unknown,
                    Box::new(error_message),
                ))
            }
            _ => {
                let error_kind =
                    match get_result_field(internal_result.as_ptr(), ResultField::SqlState) {
                        Some(error_codes::UNIQUE_VIOLATION) => DatabaseErrorKind::UniqueViolation,
                        Some(error_codes::FOREIGN_KEY_VIOLATION) => {
                            DatabaseErrorKind::ForeignKeyViolation
                        }
                        Some(error_codes::SERIALIZATION_FAILURE) => {
                            DatabaseErrorKind::SerializationFailure
                        }
                        Some(error_codes::READ_ONLY_TRANSACTION) => {
                            DatabaseErrorKind::ReadOnlyTransaction
                        }
                        _ => DatabaseErrorKind::__Unknown,
                    };
                let error_information = Box::new(PgErrorInformation(internal_result));
                Err(Error::DatabaseError(error_kind, error_information))
            }
        }
    }

    pub fn rows_affected(&self) -> usize {
        unsafe {
            let count_char_ptr = PQcmdTuples(self.internal_result.as_ptr());
            let count_bytes = CStr::from_ptr(count_char_ptr).to_bytes();
            let count_str = str::from_utf8_unchecked(count_bytes);
            match count_str {
                "" => 0,
                _ => count_str
                    .parse()
                    .expect("Error parsing `rows_affected` as integer value"),
            }
        }
    }

    pub fn num_rows(&self) -> usize {
        unsafe { PQntuples(self.internal_result.as_ptr()) as usize }
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
                let value_ptr =
                    PQgetvalue(self.internal_result.as_ptr(), row_idx, col_idx) as *const u8;
                let num_bytes = PQgetlength(self.internal_result.as_ptr(), row_idx, col_idx);
                Some(slice::from_raw_parts(value_ptr, num_bytes as usize))
            }
        }
    }

    pub fn is_null(&self, row_idx: usize, col_idx: usize) -> bool {
        unsafe {
            0 != PQgetisnull(
                self.internal_result.as_ptr(),
                row_idx as libc::c_int,
                col_idx as libc::c_int,
            )
        }
    }

    pub fn column_type(&self, col_idx: usize) -> NonZeroU32 {
        unsafe {
            NonZeroU32::new(PQftype(
                self.internal_result.as_ptr(),
                col_idx as libc::c_int,
            ))
            .expect("Oid's aren't zero")
        }
    }

    pub fn field_number(&self, column_name: &str) -> Option<usize> {
        let cstr = CString::new(column_name).unwrap_or_default();
        let fnum = unsafe { PQfnumber(self.internal_result.as_ptr(), cstr.as_ptr()) };
        match fnum {
            -1 => None,
            x => Some(x as usize),
        }
    }
}

struct PgErrorInformation(RawResult);

impl DatabaseErrorInformation for PgErrorInformation {
    fn message(&self) -> &str {
        get_result_field(self.0.as_ptr(), ResultField::MessagePrimary)
            .unwrap_or_else(|| self.0.error_message())
    }

    fn details(&self) -> Option<&str> {
        get_result_field(self.0.as_ptr(), ResultField::MessageDetail)
    }

    fn hint(&self) -> Option<&str> {
        get_result_field(self.0.as_ptr(), ResultField::MessageHint)
    }

    fn table_name(&self) -> Option<&str> {
        get_result_field(self.0.as_ptr(), ResultField::TableName)
    }

    fn column_name(&self) -> Option<&str> {
        get_result_field(self.0.as_ptr(), ResultField::ColumnName)
    }

    fn constraint_name(&self) -> Option<&str> {
        get_result_field(self.0.as_ptr(), ResultField::ConstraintName)
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
    //! <https://www.postgresql.org/docs/9.5/static/errcodes-appendix.html>
    //!
    //! They are not exposed programmatically through libpq.
    pub const UNIQUE_VIOLATION: &str = "23505";
    pub const FOREIGN_KEY_VIOLATION: &str = "23503";
    pub const SERIALIZATION_FAILURE: &str = "40001";
    pub const READ_ONLY_TRANSACTION: &str = "25006";
}
