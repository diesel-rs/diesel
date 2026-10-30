use core::ffi;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use super::raw::RawConnection;
use super::result::PgResult;
use crate::QueryResult;

#[allow(missing_debug_implementations)] // `PgConnection` is not debug
pub struct CopyFromSink<'conn> {
    conn: &'conn mut RawConnection,
    finished: bool,
}

impl<'conn> CopyFromSink<'conn> {
    pub(super) fn new(conn: &'conn mut RawConnection) -> Self {
        Self {
            conn,
            finished: false,
        }
    }

    pub(super) fn finish(mut self, err: Option<String>) -> QueryResult<()> {
        self.conn
            .finish_copy_from(err)
            .inspect(|_| self.finished = true)
    }
}

impl Write for CopyFromSink<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.conn
            .put_copy_data(buf)
            .map_err(std::io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for CopyFromSink<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.conn.consume_results();
        }
    }
}

#[allow(missing_debug_implementations)] // `PgConnection` is not debug
pub struct CopyToBuffer<'conn> {
    conn: &'conn mut RawConnection,
    ptr: *mut ffi::c_char,
    offset: usize,
    len: usize,
    result: PgResult,
    done: bool,
}

impl<'conn> CopyToBuffer<'conn> {
    pub(super) fn new(conn: &'conn mut RawConnection, result: PgResult) -> Self {
        Self {
            conn,
            ptr: core::ptr::null_mut(),
            offset: 0,
            len: 0,
            result,
            done: false,
        }
    }

    #[allow(unsafe_code)] // construct a slice from a raw ptr
    pub(crate) fn data_slice(&self) -> &[u8] {
        if !self.ptr.is_null() && self.offset < self.len {
            let slice = unsafe { core::slice::from_raw_parts(self.ptr as *const u8, self.len - 1) };
            &slice[self.offset..]
        } else {
            &[]
        }
    }

    pub(crate) fn get_result(&self) -> &PgResult {
        &self.result
    }
}

impl Drop for CopyToBuffer<'_> {
    #[allow(unsafe_code)] // ffi code
    fn drop(&mut self) {
        let _ = self.conn.consume_results();
        if !self.ptr.is_null() {
            unsafe { pq_sys::PQfreemem(self.ptr as *mut ffi::c_void) };
            self.ptr = core::ptr::null_mut();
        }
    }
}

impl Read for CopyToBuffer<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data = self.fill_buf()?;
        let len = usize::min(buf.len(), data.len());
        buf[..len].copy_from_slice(&data[..len]);
        self.consume(len);
        Ok(len)
    }
}

impl BufRead for CopyToBuffer<'_> {
    #[allow(unsafe_code)] // ffi code + ptr arithmetic
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.data_slice().is_empty() && !self.done {
            let len = unsafe {
                if !self.ptr.is_null() {
                    pq_sys::PQfreemem(self.ptr as *mut ffi::c_void);
                    self.ptr = core::ptr::null_mut();
                }
                pq_sys::PQgetCopyData(self.conn.internal_connection.as_ptr(), &mut self.ptr, 0)
            };
            match len {
                len if len >= 0 => {
                    self.len = 1 + usize::try_from(len).map_err(std::io::Error::other)?
                }
                -1 => {
                    self.done = true;
                    self.len = 0;
                    self.conn.consume_results().map_err(std::io::Error::other)?;
                }
                _ => {
                    let error = self.conn.last_error_message();
                    self.done = true;
                    self.len = 0;
                    return Err(std::io::Error::other(error));
                }
            }
            self.offset = 0;
        }
        Ok(self.data_slice())
    }

    fn consume(&mut self, amt: usize) {
        self.offset = usize::min(self.len, self.offset + amt);
    }
}
