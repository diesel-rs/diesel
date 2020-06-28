#![allow(clippy::too_many_arguments)]

use std::{ptr, str};

use super::result::PgResult;
use crate::result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct RawConnection {
    internal_connection: libpq::Connection,
}

impl RawConnection {
    pub fn establish(database_url: &str) -> ConnectionResult<Self> {
        libpq::Connection::new(database_url)
            .map_err(ConnectionError::BadConnection)
            .map(|internal_connection| Self {
                internal_connection,
            })
    }

    pub fn set_notice_processor(&self, notice_processor: libpq::connection::NoticeProcessor) {
        unsafe {
            self.internal_connection
                .set_notice_processor(notice_processor, ptr::null_mut());
        }
    }

    pub fn exec(&self, query: &str) -> QueryResult<PgResult> {
        PgResult::new(self.internal_connection.exec(query))
    }

    pub fn exec_prepared(
        &self,
        stmt_name: Option<&str>,
        param_values: &[Option<Vec<u8>>],
        param_formats: &[libpq::Format],
        result_format: libpq::Format,
    ) -> QueryResult<PgResult> {
        PgResult::new(self.internal_connection.exec_prepared(
            stmt_name,
            param_values,
            param_formats,
            result_format,
        ))
    }

    pub fn prepare(
        &self,
        stmt_name: Option<&str>,
        query: &str,
        param_types: &[libpq::Oid],
    ) -> QueryResult<PgResult> {
        PgResult::new(
            self.internal_connection
                .prepare(stmt_name, query, param_types),
        )
    }
}
