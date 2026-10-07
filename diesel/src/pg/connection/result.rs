#![allow(unsafe_code)] // ffi code
extern crate pq_sys;

use self::pq_sys::*;
use alloc::rc::Rc;
use core::ffi as libc;
use core::num::NonZeroU32;
use core::str;

use super::raw::{RawConnection, RawResult, ResultField};
use super::row::PgRow;
use crate::result::{DatabaseErrorInformation, DatabaseErrorKind, Error, QueryResult};
use core::cell::OnceCell;

#[allow(missing_debug_implementations)]
pub struct PgResult {
    internal_result: RawResult,
    column_count: libc::c_int,
    row_count: libc::c_int,
    // We store field names as pointer
    // as we cannot put a correct lifetime here
    // The value is valid as long as we haven't freed `RawResult`
    column_name_map: OnceCell<Vec<Option<*const str>>>,
}

impl PgResult {
    #[allow(clippy::new_ret_no_self)]
    pub(super) fn new(internal_result: RawResult, conn: &RawConnection) -> QueryResult<Self> {
        match internal_result.result_status() {
            ExecStatusType::PGRES_SINGLE_TUPLE
            | ExecStatusType::PGRES_COMMAND_OK
            | ExecStatusType::PGRES_COPY_IN
            | ExecStatusType::PGRES_COPY_OUT
            | ExecStatusType::PGRES_TUPLES_OK => {
                let column_count = internal_result.column_count();
                let row_count = internal_result.row_count();
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

                let mut error_kind = match internal_result.get_result_field(ResultField::SqlState) {
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
                    Some(error_codes::NOT_NULL_VIOLATION) => DatabaseErrorKind::NotNullViolation,
                    Some(error_codes::CHECK_VIOLATION) => DatabaseErrorKind::CheckViolation,
                    Some(error_codes::RESTRICT_VIOLATION) => DatabaseErrorKind::RestrictViolation,
                    Some(error_codes::EXCLUSION_VIOLATION) => DatabaseErrorKind::ExclusionViolation,
                    Some(error_codes::CONNECTION_EXCEPTION)
                    | Some(error_codes::CONNECTION_FAILURE)
                    | Some(error_codes::SQLCLIENT_UNABLE_TO_ESTABLISH_SQLCONNECTION)
                    | Some(error_codes::SQLSERVER_REJECTED_ESTABLISHMENT_OF_SQLCONNECTION) => {
                        DatabaseErrorKind::ClosedConnection
                    }
                    _ => DatabaseErrorKind::Unknown,
                };
                let error_information = Box::new(PgErrorInformation {
                    result: internal_result,
                });
                let conn_status = conn.get_status();
                if conn_status == ConnStatusType::CONNECTION_BAD {
                    error_kind = DatabaseErrorKind::ClosedConnection;
                }
                Err(Error::DatabaseError(error_kind, error_information))
            }
        }
    }

    pub(super) fn rows_affected(&self) -> QueryResult<usize> {
        let rows = self.internal_result.rows_affected();
        let count_str = rows
            .to_str()
            .map_err(|e| Error::DeserializationError(Box::new(e)))?;
        match count_str {
            "" => Ok(0),
            _ => count_str
                .parse()
                .map_err(|e| Error::DeserializationError(Box::new(e))),
        }
    }

    pub(super) fn num_rows(&self) -> usize {
        self.row_count.try_into().expect(
            "Diesel expects to run on a >= 32 bit OS \
                (or libpq is giving out negative row count)",
        )
    }

    pub(super) fn get_row(self: Rc<Self>, idx: usize) -> PgRow {
        PgRow::new(self, idx)
    }

    pub(super) fn get(&self, row_idx: usize, col_idx: usize) -> Option<&[u8]> {
        if self.is_null(row_idx, col_idx) {
            None
        } else {
            let row_idx = row_idx.try_into().ok()?;
            let col_idx = col_idx.try_into().ok()?;
            Some(self.internal_result.get_bytes(row_idx, col_idx))
        }
    }

    pub(super) fn is_null(&self, row_idx: usize, col_idx: usize) -> bool {
        let row_idx = row_idx
            .try_into()
            .expect("Row indices are expected to fit into 32 bit");
        let col_idx = col_idx
            .try_into()
            .expect("Column indices are expected to fit into 32 bit");

        self.internal_result.is_null(row_idx, col_idx) != 0
    }

    pub(in crate::pg) fn column_type(&self, col_idx: usize) -> NonZeroU32 {
        let col_idx: i32 = col_idx
            .try_into()
            .expect("Column indices are expected to fit into 32 bit");
        let type_oid = self.internal_result.column_type(col_idx);
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
                    .map(|idx| {
                        self.internal_result
                            .column_name(idx as libc::c_int)
                            .map(|name| {
                                name.to_str().expect(
                                    "Expect postgres field names to be UTF-8, because we \
-                     requested UTF-8 encoding on connection setup",
                                ) as *const str
                            })
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
        self.column_count.try_into().expect(
            "Diesel expects to run on a >= 32 bit OS \
                (or libpq is giving out negative column count)",
        )
    }
}

struct PgErrorInformation {
    result: RawResult,
}

impl DatabaseErrorInformation for PgErrorInformation {
    fn message(&self) -> &str {
        self.result
            .get_result_field(ResultField::MessagePrimary)
            .unwrap_or_else(|| self.result.error_message())
    }

    fn details(&self) -> Option<&str> {
        self.result.get_result_field(ResultField::MessageDetail)
    }

    fn hint(&self) -> Option<&str> {
        self.result.get_result_field(ResultField::MessageHint)
    }

    fn table_name(&self) -> Option<&str> {
        self.result.get_result_field(ResultField::TableName)
    }

    fn column_name(&self) -> Option<&str> {
        self.result.get_result_field(ResultField::ColumnName)
    }

    fn constraint_name(&self) -> Option<&str> {
        self.result.get_result_field(ResultField::ConstraintName)
    }

    fn statement_position(&self) -> Option<i32> {
        let str_pos = self
            .result
            .get_result_field(ResultField::StatementPosition)?;
        str_pos.parse::<i32>().ok()
    }
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
    pub(in crate::pg::connection) const RESTRICT_VIOLATION: &str = "23001";
    pub(in crate::pg::connection) const NOT_NULL_VIOLATION: &str = "23502";
    pub(in crate::pg::connection) const FOREIGN_KEY_VIOLATION: &str = "23503";
    pub(in crate::pg::connection) const UNIQUE_VIOLATION: &str = "23505";
    pub(in crate::pg::connection) const CHECK_VIOLATION: &str = "23514";
    pub(in crate::pg::connection) const EXCLUSION_VIOLATION: &str = "23P01";
    pub(in crate::pg::connection) const READ_ONLY_TRANSACTION: &str = "25006";
    pub(in crate::pg::connection) const SERIALIZATION_FAILURE: &str = "40001";
}
