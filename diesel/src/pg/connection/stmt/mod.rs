extern crate pq_sys;
extern crate libc;

mod cache;

use std::ffi::CString;
use std::ptr;

use super::result::PgResult;
use result::QueryResult;

pub use self::cache::StatementCache;
pub use super::raw::RawConnection;

pub enum Query {
    Prepared {
        name: CString,
        param_formats: Vec<libc::c_int>,
    },
    Sql {
        query: CString,
        param_types: Option<Vec<u32>>,
    },
}

impl Query {
    pub fn execute(
        &self,
        conn: &RawConnection,
        param_data: &Vec<Option<Vec<u8>>>,
    ) -> QueryResult<PgResult> {
        let params_pointer = param_data.iter()
            .map(|data| data.as_ref().map(|d| d.as_ptr() as *const libc::c_char)
                 .unwrap_or(ptr::null()))
            .collect::<Vec<_>>();
        let param_lengths = param_data.iter()
            .map(|data| data.as_ref().map(|d| d.len() as libc::c_int)
                 .unwrap_or(0))
            .collect::<Vec<_>>();
        let internal_res = match *self {
            Query::Prepared { ref name, ref param_formats } => unsafe {
                conn.exec_prepared(
                    name.as_ptr(),
                    params_pointer.len() as libc::c_int,
                    params_pointer.as_ptr(),
                    param_lengths.as_ptr(),
                    param_formats.as_ptr(),
                    1,
                )
            },
            Query::Sql { ref query, ref param_types } => {
                let param_types_ptr = param_types_to_ptr(param_types.as_ref());
                let param_formats = vec![1; param_data.len()];
                unsafe { conn.exec_params(
                    query.as_ptr(),
                    params_pointer.len() as libc::c_int,
                    param_types_ptr,
                    params_pointer.as_ptr(),
                    param_lengths.as_ptr(),
                    param_formats.as_ptr(),
                    1,
                ) }
            }
        };

        PgResult::new(internal_res)
    }

    pub fn sql(sql: &str, param_types: Option<Vec<u32>>) -> QueryResult<Self> {
        Ok(Query::Sql {
            query: try!(CString::new(sql)),
            param_types: param_types,
        })
    }

    pub fn prepare(
        conn: &RawConnection,
        sql: &str,
        name: &str,
        param_types: &Vec<u32>,
    ) -> QueryResult<Self> {
        let name = try!(CString::new(name));
        let sql = try!(CString::new(sql));

        let internal_result = unsafe {
            conn.prepare(
                name.as_ptr(),
                sql.as_ptr(),
                param_types.len() as libc::c_int,
                param_types_to_ptr(Some(&param_types)),
            )
        };
        try!(PgResult::new(internal_result));

        Ok(Query::Prepared {
            name: name,
            param_formats: vec![1; param_types.len()],
        })
    }
}

fn param_types_to_ptr(param_types: Option<&Vec<u32>>) -> *const pq_sys::Oid {
    param_types
        .map(|types| types.as_ptr())
        .unwrap_or(ptr::null())
}
