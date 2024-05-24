#![allow(unsafe_code)] // ffi code
extern crate pq_sys;

use self::pq_sys::*;
use std::ffi::CStr;
use std::num::NonZeroU32;
use std::os::raw as libc;
use std::rc::Rc;
use std::{slice, str};

use super::raw::{RawConnection, RawResult};
use super::row::PgRow;
use crate::result::{DatabaseErrorInformation, DatabaseErrorKind, Error, QueryResult};
use std::cell::OnceCell;

#[allow(missing_debug_implementations)]
pub struct PgResult {
    internal_result: RawResult,
    column_count: usize,
    row_count: usize,
    // We store field names as pointer
    // as we cannot put a correct lifetime here
    // The value is valid as long as we haven't freed `RawResult`
    column_name_map: OnceCell<Vec<Option<*const str>>>,
}

impl PgResult {
    #[allow(clippy::new_ret_no_self)]
    pub(super) fn new(internal_result: RawResult, conn: &RawConnection) -> QueryResult<Self> {
        let result_status = unsafe { PQresultStatus(internal_result.as_ptr()) };
        match result_status {
            ExecStatusType::PGRES_SINGLE_TUPLE
            | ExecStatusType::PGRES_COMMAND_OK
            | ExecStatusType::PGRES_COPY_IN
            | ExecStatusType::PGRES_COPY_OUT
            | ExecStatusType::PGRES_TUPLES_OK => {
                let column_count = unsafe { PQnfields(internal_result.as_ptr()) as usize };
                let row_count = unsafe { PQntuples(internal_result.as_ptr()) as usize };
                Ok(PgResult {
                    internal_result,
                    column_count,
                    row_count,
                    column_name_map: OnceCell::new(),
                })
            }
            ExecStatusType::PGRES_EMPTY_QUERY => {
                let error_message = "Received an empty query".to_string();
                Err(Error::DatabaseError(
                    DatabaseErrorKind::Unknown,
                    Box::new(error_message),
                ))
            }
            _ => {
                // "clearing" the connection by polling result till we get a null.
                // this will indicate that the previous command is complete and
                // the same connection is ready to process next command.
                // https://www.postgresql.org/docs/current/libpq-async.html
                while conn.get_next_result().map_or(true, |r| r.is_some()) {}

                let mut error_kind =
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
                        Some(error_codes::NOT_NULL_VIOLATION) => {
                            DatabaseErrorKind::NotNullViolation
                        }
                        Some(error_codes::CHECK_VIOLATION) => DatabaseErrorKind::CheckViolation,
                        Some(error_codes::CONNECTION_EXCEPTION)
                        | Some(error_codes::CONNECTION_FAILURE)
                        | Some(error_codes::SQLCLIENT_UNABLE_TO_ESTABLISH_SQLCONNECTION)
                        | Some(error_codes::SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION) => {
                            DatabaseErrorKind::ClosedConnection
                        }
                        _ => DatabaseErrorKind::Unknown,
                    };
                let error_information = Box::new(PgErrorInformation(internal_result));
                let conn_status = conn.get_status();
                if conn_status == ConnStatusType::CONNECTION_BAD {
                    error_kind = DatabaseErrorKind::ClosedConnection;
                }
                Err(Error::DatabaseError(error_kind, error_information))
            }
        }
    }

    pub(super) fn rows_affected(&self) -> usize {
        unsafe {
            let count_char_ptr = PQcmdTuples(self.internal_result.as_ptr());
            let count_bytes = CStr::from_ptr(count_char_ptr).to_bytes();
            // Using from_utf8_unchecked is ok here because, we've set the
            // client encoding to utf8
            let count_str = str::from_utf8_unchecked(count_bytes);
            match count_str {
                "" => 0,
                _ => count_str
                    .parse()
                    .expect("Error parsing `rows_affected` as integer value"),
            }
        }
    }

    pub(super) fn num_rows(&self) -> usize {
        self.row_count
    }

    pub(super) fn get_row(self: Rc<Self>, idx: usize) -> PgRow {
        PgRow::new(self, idx)
    }

    pub(super) fn get(&self, row_idx: usize, col_idx: usize) -> Option<&[u8]> {
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

    pub(super) fn is_null(&self, row_idx: usize, col_idx: usize) -> bool {
        unsafe {
            0 != PQgetisnull(
                self.internal_result.as_ptr(),
                row_idx as libc::c_int,
                col_idx as libc::c_int,
            )
        }
    }

    pub(in crate::pg) fn column_type(&self, col_idx: usize) -> NonZeroU32 {
        let type_oid = unsafe { PQftype(self.internal_result.as_ptr(), col_idx as libc::c_int) };
        NonZeroU32::new(type_oid).expect(
            "Got a zero oid from postgres. If you see this error message \
             please report it as issue on the diesel github bug tracker.",
        )
    }

    #[inline(always)] // benchmarks indicate a ~1.7% improvement in instruction count for this
    pub(super) fn column_name(&self, col_idx: usize) -> Option<&str> {
        self.column_name_map
            .get_or_init(|| {
                (0..self.column_count)
                    .map(|idx| unsafe {
                        // https://www.postgresql.org/docs/13/libpq-exec.html#LIBPQ-PQFNAME
                        // states that the returned ptr is valid till the underlying result is freed
                        // That means we can couple the lifetime to self
                        let ptr = PQfname(self.internal_result.as_ptr(), idx as libc::c_int);
                        if ptr.is_null() {
                            None
                        } else {
                            Some(CStr::from_ptr(ptr).to_str().expect(
                                "Expect postgres field names to be UTF-8, because we \
                     requested UTF-8 encoding on connection setup",
                            ) as *const str)
                        }
                    })
                    .collect()
            })
            .get(col_idx)
            .and_then(|n| {
                n.map(|n: *const str| unsafe {
                    // The pointer is valid for the same lifetime as &self
                    // so we can dereference it without any check
                    &*n
                })
            })
    }

    pub(super) fn column_count(&self) -> usize {
        self.column_count
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

    fn statement_position(&self) -> Option<i32> {
        let str_pos = get_result_field(self.0.as_ptr(), ResultField::StatementPosition)?;
        str_pos.parse::<i32>().ok()
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
    StatementPosition = 'P' as i32,
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
    //! <https://www.postgresql.org/docs/current/errcodes-appendix.html>
    //!
    //! They are not exposed programmatically through libpq.
    pub(in crate::pg::connection) const CONNECTION_EXCEPTION: &str = "08000";
    pub(in crate::pg::connection) const CONNECTION_FAILURE: &str = "08006";
    pub(in crate::pg::connection) const SQLCLIENT_UNABLE_TO_ESTABLISH_SQLCONNECTION: &str = "08001";
    pub(in crate::pg::connection) const SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION: &str =
        "08004";
    pub(in crate::pg::connection) const NOT_NULL_VIOLATION: &str = "23502";
    pub(in crate::pg::connection) const FOREIGN_KEY_VIOLATION: &str = "23503";
    pub(in crate::pg::connection) const UNIQUE_VIOLATION: &str = "23505";
    pub(in crate::pg::connection) const CHECK_VIOLATION: &str = "23514";
    pub(in crate::pg::connection) const READ_ONLY_TRANSACTION: &str = "25006";
    pub(in crate::pg::connection) const SERIALIZATION_FAILURE: &str = "40001";
}
