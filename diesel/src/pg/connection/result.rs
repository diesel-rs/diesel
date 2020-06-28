use std::num::NonZeroU32;
use std::str;

use super::row::PgRow;
use crate::result::{DatabaseErrorInformation, DatabaseErrorKind, Error, QueryResult};

pub struct PgResult {
    internal_result: libpq::Result,
}

impl PgResult {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(internal_result: libpq::Result) -> QueryResult<Self> {
        use libpq::Status::*;

        match internal_result.status() {
            CommandOk | TupplesOk => Ok(PgResult { internal_result }),
            EmptyQuery => {
                let error_message = "Received an empty query".to_string();
                Err(Error::DatabaseError(
                    DatabaseErrorKind::__Unknown,
                    Box::new(error_message),
                ))
            }
            _ => {
                use libpq::state::*;
                let code = internal_result
                    .error_field(libpq::result::ErrorField::Sqlstate)
                    .unwrap_or_default();
                let state = libpq::State::from_code(&code);
                let error_kind = match state {
                    UNIQUE_VIOLATION => DatabaseErrorKind::UniqueViolation,
                    FOREIGN_KEY_VIOLATION => DatabaseErrorKind::ForeignKeyViolation,
                    T_R_SERIALIZATION_FAILURE => DatabaseErrorKind::SerializationFailure,
                    READ_ONLY_SQL_TRANSACTION => DatabaseErrorKind::ReadOnlyTransaction,
                    NOT_NULL_VIOLATION => DatabaseErrorKind::NotNullViolation,
                    CHECK_VIOLATION => DatabaseErrorKind::CheckViolation,
                    _ => DatabaseErrorKind::__Unknown,
                };
                let error_information = Box::new(PgErrorInformation(internal_result));
                Err(Error::DatabaseError(error_kind, error_information))
            }
        }
    }

    pub fn rows_affected(&self) -> usize {
        self.internal_result.cmd_tuples()
    }

    pub fn num_rows(&self) -> usize {
        self.internal_result.ntuples()
    }

    pub fn get_row(&self, idx: usize) -> PgRow {
        PgRow::new(self, idx)
    }

    pub fn get(&self, row_idx: usize, col_idx: usize) -> Option<&[u8]> {
        self.internal_result.value(row_idx, col_idx)
    }

    pub fn is_null(&self, row_idx: usize, col_idx: usize) -> bool {
        self.internal_result.is_null(row_idx, col_idx)
    }

    pub fn column_type(&self, col_idx: usize) -> NonZeroU32 {
        NonZeroU32::new(
            self.internal_result
                .field_type(col_idx)
                .map(|x| x.oid)
                .unwrap_or_default(),
        )
        .expect("Oid's aren't zero")
    }

    pub fn field_number(&self, column_name: &str) -> Option<usize> {
        self.internal_result.field_number(column_name)
    }
}

struct PgErrorInformation(libpq::Result);

impl DatabaseErrorInformation for PgErrorInformation {
    fn message(&self) -> &str {
        self.0
            .error_field(libpq::result::ErrorField::MessagePrimary)
            .unwrap_or_default()
    }

    fn details(&self) -> Option<&str> {
        self.0.error_field(libpq::result::ErrorField::MessageDetail)
    }

    fn hint(&self) -> Option<&str> {
        self.0.error_field(libpq::result::ErrorField::MessageHint)
    }

    fn table_name(&self) -> Option<&str> {
        self.0.error_field(libpq::result::ErrorField::TableName)
    }

    fn column_name(&self) -> Option<&str> {
        self.0.error_field(libpq::result::ErrorField::ColumnName)
    }

    fn constraint_name(&self) -> Option<&str> {
        self.0
            .error_field(libpq::result::ErrorField::ConstraintName)
    }
}
