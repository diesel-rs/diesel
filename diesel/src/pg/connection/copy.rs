use core::ffi;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;

use super::raw::RawConnection;
use super::result::PgResult;
use crate::QueryResult;

#[allow(missing_debug_implementations)] // `PgConnection` is not debug
pub(in crate::pg) struct CopyFromSink<'conn> {
    conn: &'conn mut RawConnection,
}

impl<'conn> CopyFromSink<'conn> {
    pub(super) fn new(conn: &'conn mut RawConnection) -> Self {
        Self { conn }
    }

    pub(super) fn finish(self, err: Option<String>) -> QueryResult<()> {
        self.conn.finish_copy_from(err)
    }
}

impl<'conn> Write for CopyFromSink<'conn> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.conn
            .put_copy_data(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[allow(missing_debug_implementations)] // `PgConnection` is not debug
pub struct CopyToBuffer<'conn> {
    conn: &'conn mut RawConnection,
    ptr: *mut ffi::c_char,
    offset: usize,
    len: usize,
    result: PgResult,
}

impl<'conn> CopyToBuffer<'conn> {
    pub(super) fn new(conn: &'conn mut RawConnection, result: PgResult) -> Self {
        Self {
            conn,
            ptr: std::ptr::null_mut(),
            offset: 0,
            len: 0,
            result,
        }
    }

    #[allow(unsafe_code)] // construct a slice from a raw ptr
    pub(crate) fn data_slice(&self) -> &[u8] {
        if !self.ptr.is_null() && self.offset < self.len {
            let slice = unsafe { std::slice::from_raw_parts(self.ptr as *const u8, self.len - 1) };
            &slice[self.offset..]
        } else {
            &[]
        }
    }

    pub(crate) fn get_result(&self) -> &PgResult {
        &self.result
    }
}

impl<'conn> Drop for CopyToBuffer<'conn> {
    #[allow(unsafe_code)] // ffi code
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { pq_sys::PQfreemem(self.ptr as *mut ffi::c_void) };
            self.ptr = std::ptr::null_mut();
        }
    }
}

impl<'conn> Read for CopyToBuffer<'conn> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let data = self.fill_buf()?;
        let len = usize::min(buf.len(), data.len());
        buf[..len].copy_from_slice(&data[..len]);
        self.consume(len);
        Ok(len)
    }
}

impl<'conn> BufRead for CopyToBuffer<'conn> {
    #[allow(unsafe_code)] // ffi code + ptr arithmetic
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.data_slice().is_empty() {
            unsafe {
                if !self.ptr.is_null() {
                    pq_sys::PQfreemem(self.ptr as *mut ffi::c_void);
                    self.ptr = std::ptr::null_mut();
                }
                let len =
                    pq_sys::PQgetCopyData(self.conn.internal_connection.as_ptr(), &mut self.ptr, 0);
                match len {
                    len if len >= 0 => self.len = len as usize + 1,
                    -1 => self.len = 0,
                    _ => {
                        let error = self.conn.last_error_message();
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, error));
                    }
                }
                self.offset = 0;
            }
        }
        Ok(self.data_slice())
    }

    fn consume(&mut self, amt: usize) {
        self.offset = usize::min(self.len, self.offset + amt);
    }
}
