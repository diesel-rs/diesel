#![allow(unsafe_code)] // ffi calls
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[expect(missing_debug_implementations)]
pub struct SqliteBlob<'conn> {
    pub(crate) blob: core::ptr::NonNull<ffi::sqlite3_blob>,
    pub(crate) read_index: usize,

    pub(crate) blob_size: usize,
    pub(crate) _pd: core::marker::PhantomData<&'conn mut ffi::sqlite3_blob>,
}

impl Drop for SqliteBlob<'_> {
    fn drop(&mut self) {
        use crate::util::std_compat::panicking;

        if let Err(error_message) = self.close_inner() {
            if panicking() {
                #[cfg(feature = "std")]
                eprintln!("Error closing SQLite blob: {error_message}");
            } else {
                panic!("Error closing SQLite blob: {error_message}");
            }
        }
    }
}

impl SqliteBlob<'_> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.blob_size
    }

    /// Close the handle
    ///
    /// Even if an error is returned, the handle is still closed (from the sqlite documentation):
    ///
    /// > The BLOB handle is closed unconditionally. Even if this routine returns an error code,
    /// > the handle is still closed.
    pub fn close(mut self) -> Result<(), crate::result::Error> {
        self.close_inner()
    }

    fn close_inner(&mut self) -> Result<(), crate::result::Error> {
        // SAFETY: From the sqlite3_blob_close documentation:
        //
        //     If an error occurs while committing the transaction, an error code is returned and
        //     the transaction rolled back.
        //
        // As we are in read-only mode here, this is not an issue
        let close_result = unsafe { ffi::sqlite3_blob_close(self.blob.as_ptr()) };

        if close_result != ffi::SQLITE_OK {
            let error_message = super::error_message(close_result);
            return Err(crate::result::Error::ClosingHandle(error_message));
        }

        Ok(())
    }
}

fn to_io_error(error: core::num::TryFromIntError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, Box::new(error))
}

impl std::io::Read for SqliteBlob<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let buflen: i32 = buf.len().try_into().map_err(to_io_error)?;
        let offset: i32 = self.read_index.try_into().map_err(to_io_error)?;

        // From the sqlite docs:
        //
        // > If offset iOffset is less than N bytes from the end of the BLOB, SQLITE_ERROR is returned and no data is read.
        //
        // Thus we need to make sure to not provide a buffer that is too big for the remaining data
        // from the blob.
        let read_length: i32 = (i32::try_from(self.blob_size)
            .map_err(to_io_error)?
            .saturating_sub(offset))
        .min(buflen);

        let ret = unsafe {
            ffi::sqlite3_blob_read(
                self.blob.as_ptr(),
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                read_length,
                offset,
            )
        };

        if ret != ffi::SQLITE_OK {
            let error_message = crate::sqlite::connection::error_message(ret);
            return Err(std::io::Error::other(error_message.to_string()));
        }

        self.read_index += usize::try_from(read_length).map_err(to_io_error)?;
        debug_assert!(self.read_index <= self.blob_size);

        usize::try_from(read_length).map_err(to_io_error)
    }
}

impl std::io::Seek for SqliteBlob<'_> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Start(n) => {
                self.read_index = usize::try_from(n).map_err(to_io_error)?.min(self.blob_size);
            }
            std::io::SeekFrom::End(n) => {
                self.read_index = if n.is_positive() {
                    self.blob_size
                } else {
                    self.blob_size - usize::try_from(n.unsigned_abs()).map_err(to_io_error)?
                };
            }
            std::io::SeekFrom::Current(n) => {
                let n = isize::try_from(n).map_err(to_io_error)?;

                if n.is_negative() {
                    self.read_index = if self.read_index < n.unsigned_abs() {
                        0
                    } else {
                        self.read_index - n.unsigned_abs()
                    };
                } else {
                    self.read_index = (self.read_index + n.unsigned_abs()).min(self.blob_size);
                }
            }
        }

        u64::try_from(self.read_index).map_err(to_io_error)
    }
}
