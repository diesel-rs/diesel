#![allow(unsafe_code)] // ffi code
extern crate pq_sys;

use std::ffi::CString;
use std::os::raw as libc;
use std::ptr;

use super::result::PgResult;
use crate::pg::PgTypeMetadata;
use crate::result::QueryResult;

use super::raw::RawConnection;

enum StatementKind {
    Unnamed {
        sql: Option<CString>,
        param_types: Option<Vec<u32>>,
    },
    Named {
        name: CString,
    },
}

pub(crate) struct Statement {
    kind: StatementKind,
    param_formats: Vec<libc::c_int>,
}

impl Statement {
    pub(super) fn execute(
        &self,
        raw_connection: &mut RawConnection,
        param_data: &[Option<Vec<u8>>],
        row_by_row: bool,
    ) -> QueryResult<PgResult> {
        let params_pointer = param_data
            .iter()
            .map(|data| {
                data.as_ref()
                    .map(|d| d.as_ptr() as *const libc::c_char)
                    .unwrap_or(ptr::null())
            })
            .collect::<Vec<_>>();
        let param_lengths = param_data
            .iter()
            .map(|data| data.as_ref().map(|d| d.len().try_into()).unwrap_or(Ok(0)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::result::Error::SerializationError(Box::new(e)))?;
        let param_count: libc::c_int = params_pointer
            .len()
            .try_into()
            .map_err(|e| crate::result::Error::SerializationError(Box::new(e)))?;

        let name = match &self.kind {
            StatementKind::Named { name } => Some(name),
            _ => None,
        };

        if let Some(name) = name {
            unsafe {
                // execute the previously prepared statement
                // in autocommit mode, this will be a new transaction
                raw_connection.send_query_prepared(
                    name.as_ptr(),
                    param_count,
                    params_pointer.as_ptr(),
                    param_lengths.as_ptr(),
                    self.param_formats.as_ptr(),
                    1,
                )
            }?;
        } else {
            // execute the unnamed prepared statement using send_query_params
            // which internally calls PQsendQueryParams, making sure the
            // prepare and execute happens in a single transaction. This
            // makes sure these are handled by PgBouncer.
            // See https://github.com/diesel-rs/diesel/pull/4539
            if let StatementKind::Unnamed {
                sql: Some(ref sql),
                param_types,
            } = &self.kind
            {
                unsafe {
                    raw_connection.send_query_params(
                        sql.as_ptr(),
                        param_count,
                        param_types_to_ptr(param_types.as_ref()),
                        params_pointer.as_ptr(),
                        param_lengths.as_ptr(),
                        self.param_formats.as_ptr(),
                        1,
                    )
                }?;
            }
        }

        if row_by_row {
            raw_connection.enable_row_by_row_mode()?;
        }
        Ok(raw_connection.get_next_result()?.expect("Is never none"))
    }

    pub(super) fn prepare(
        raw_connection: &mut RawConnection,
        sql: &str,
        name: Option<&str>,
        param_types: &[PgTypeMetadata],
    ) -> QueryResult<Self> {
        let query_name = match is_cached {
            PrepareForCache::Yes { counter } => Some(format!("__diesel_stmt_{counter}")),
            PrepareForCache::No => None,
        };
        let name = query_name.as_deref();
        let name_cstr = CString::new(name.unwrap_or(""))?;
        let sql_cstr = CString::new(sql)?;
        let param_types_vec = param_types
            .iter()
            .map(|x| x.oid())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::result::Error::SerializationError(Box::new(e)))?;

        // For unnamed statements, we'll return a Statement object without
        // actually preparing it. This allows us to use send_query_params
        // later in the execute call. This is needed to better interface
        // with PgBouncer which cannot handle unnamed prepared statements
        // when those are prepared and executed in separate transactions.
        // See https://github.com/diesel-rs/diesel/pull/4539
        if query_name.is_none() {
            return Ok(Statement {
                kind: StatementKind::Unnamed {
                    sql: Some(sql_cstr),
                    param_types: Some(param_types_vec),
                },
                param_formats: vec![1; param_types.len()],
            });
        }

        // For named/cached statements, prepare as usual using a prepare phase and then
        // an execute phase
        let internal_result = unsafe {
            let param_count: libc::c_int = param_types
                .len()
                .try_into()
                .map_err(|e| crate::result::Error::SerializationError(Box::new(e)))?;
            raw_connection.prepare(
                name_cstr.as_ptr(),
                sql_cstr.as_ptr(),
                param_count,
                param_types_to_ptr(Some(&param_types_vec)),
            )
        };
        PgResult::new(internal_result?, raw_connection)?;

        Ok(Statement {
            kind: StatementKind::Named { name: name_cstr },
            param_formats: vec![1; param_types.len()],
        })
    }
}

fn param_types_to_ptr(param_types: Option<&Vec<u32>>) -> *const pq_sys::Oid {
    param_types
        .map(|types| types.as_ptr())
        .unwrap_or(ptr::null())
}
