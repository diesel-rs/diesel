extern crate pq_sys;
extern crate libc;

use result::{Error, QueryResult};
use super::raw::RawConnection;
use super::row::PgRow;

use self::pq_sys::*;
use std::ffi::CStr;
use std::{str, slice, mem};

pub struct PgResult {
    internal_result: *mut PGresult,
}

impl PgResult {
    pub fn new(conn: &RawConnection, internal_result: *mut PGresult) -> QueryResult<Self> {
        let result_status = unsafe { PQresultStatus(internal_result) };
        match result_status {
            PGRES_COMMAND_OK | PGRES_TUPLES_OK => {
                Ok(PgResult {
                    internal_result: internal_result,
                })
            },
            _ => Err(Error::DatabaseError(conn.last_error_message())),
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
