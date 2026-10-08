#![allow(unsafe_code)] // ffi calls
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;
use crate::query_source::{ColumnHasTable, NamedTable};
use crate::result::Error;

/// A read only SQLite Blob
///
/// This interface allows to incrementally read a blob from a SQLite database.
/// Notably this type implements [`std::io::Read`] and [`std::io::Seek`] to integrate
/// with standard Rust IO mechanisms.
///
/// You can use [`SqliteConnection::get_read_only_blob`](super::SqliteConnection::get_read_only_blob)
/// to get a new instance of this type
///
/// See the [SQLite documentation](https://sqlite.org/c3ref/blob_open.html) for more details
#[expect(missing_debug_implementations)]
#[cfg_attr(not(feature = "std"), expect(dead_code))]
pub struct SqliteReadOnlyBlob<'conn> {
    // `None` once closed, so the handle is closed at most once
    pub(crate) blob: Option<core::ptr::NonNull<ffi::sqlite3_blob>>,
    pub(crate) read_index: usize,

    pub(crate) blob_size: usize,
    pub(crate) _pd: core::marker::PhantomData<&'conn mut ffi::sqlite3_blob>,
}

impl Drop for SqliteReadOnlyBlob<'_> {
    fn drop(&mut self) {
        use crate::util::std_compat::panicking;

        // `close` already closed the handle
        if self.blob.is_none() {
            return;
        }

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

impl SqliteReadOnlyBlob<'_> {
    /// Is the blob storage empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The size of the underlying blob in bytes
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
        let Some(blob) = self.blob.take() else {
            return Err(crate::result::Error::ClosingHandle("handle already closed"));
        };

        // SAFETY: `blob` came from a successful `sqlite3_blob_open`, and taking it out of `self`
        // guarantees that this open handle is closed at most once.
        //
        // From the sqlite3_blob_close documentation:
        //
        //     If an error occurs while committing the transaction, an error code is returned and
        //     the transaction rolled back.
        //
        // As we are in read-only mode here, this is not an issue
        let close_result = unsafe { ffi::sqlite3_blob_close(blob.as_ptr()) };

        if close_result != ffi::SQLITE_OK {
            let error_message = super::error_message(close_result);
            return Err(crate::result::Error::ClosingHandle(error_message));
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
#[allow(clippy::std_instead_of_core)] // needs a newer rust version
fn to_io_error(error: core::num::TryFromIntError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, Box::new(error))
}

// SEE https://github.com/rust-lang/rust/issues/48331
#[cfg(feature = "std")]
impl std::io::Read for SqliteReadOnlyBlob<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let Some(blob) = self.blob else {
            return Err(std::io::Error::other("SQLite blob handle is closed"));
        };
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

        // SAFETY: `blob` is `Some`, so the handle is open, and `'conn` keeps its connection alive.
        // `read_length` is at most `buf.len()` and the bytes left after `offset`, so the write
        // stays within `buf` and the read within the blob.
        let ret = unsafe {
            ffi::sqlite3_blob_read(
                blob.as_ptr(),
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

#[cfg(feature = "std")]
impl std::io::Seek for SqliteReadOnlyBlob<'_> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Start(n) => {
                self.read_index = usize::try_from(n).map_err(to_io_error)?.min(self.blob_size);
            }
            std::io::SeekFrom::End(n) => {
                self.read_index = if n.is_positive() {
                    self.blob_size
                } else {
                    self.blob_size
                        .checked_sub(usize::try_from(n.unsigned_abs()).map_err(to_io_error)?)
                        .ok_or(std::io::ErrorKind::InvalidInput)?
                };
            }
            std::io::SeekFrom::Current(n) => {
                let n = isize::try_from(n).map_err(to_io_error)?;

                if n.is_negative() {
                    self.read_index = self
                        .read_index
                        .checked_sub(n.unsigned_abs())
                        .ok_or(std::io::ErrorKind::InvalidInput)?;
                } else {
                    self.read_index = (self.read_index + n.unsigned_abs()).min(self.blob_size);
                }
            }
        }

        u64::try_from(self.read_index).map_err(to_io_error)
    }
}

impl SqliteConnection {
    /// Returns an object that can be used to stream a BLOB from the database
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # table! {
    /// #     myblobs {
    /// #         id -> Integer,
    /// #         mydata -> Blob,
    /// #     }
    /// # }
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io::Read;
    /// use diesel::connection::SimpleConnection;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.batch_execute("CREATE TABLE myblobs (id INTEGER PRIMARY KEY, mydata BLOB)")?;
    /// conn.batch_execute("INSERT INTO myblobs (mydata) VALUES ('abc')")?;
    /// let mut data = conn.get_read_only_blob(myblobs::mydata, 1)?;
    /// let mut buf = vec![];
    /// data.read_to_end(&mut buf)?;
    /// assert_eq!(buf, b"abc");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_read_only_blob<'conn, 'query, U>(
        &'conn self,
        blob_column: U,
        row_id: i64,
    ) -> Result<SqliteReadOnlyBlob<'conn>, Error>
    where
        'query: 'conn,
        U: ColumnHasTable,
        U::Table: NamedTable,
    {
        let table = blob_column.table();

        let database_name = table.schema().unwrap_or("main");
        let column_name = blob_column.name();
        let table_name = table.table();

        self.raw_connection
            .blob_open(database_name, table_name, column_name, row_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    #[diesel_test_helper::test]
    fn read_bytes_from_blob() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
                data2 -> Blob,
            }
        }

        use std::io::Read;

        let conn = &mut connection();

        let _ =
            crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB, data2 BLOB)")
                .execute(conn);

        let _ = crate::sql_query(
            "INSERT INTO blobs (data, data2) VALUES ('abc', 'def'), ('123', '456')",
        )
        .execute(conn);

        let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        let mut buf = vec![];
        data.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, b"abc");

        let mut data2 = conn.get_read_only_blob(blobs::data2, 1).unwrap();
        let mut buf = vec![];
        data2.read_to_end(&mut buf).unwrap();

        assert_eq!(buf, b"def");
    }

    #[diesel_test_helper::test]
    fn read_seek_bytes() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        use std::io::Read;
        use std::io::Seek;
        use std::io::SeekFrom;

        let conn = &mut connection();

        let _ = crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('abcdefghi')").execute(conn);

        let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"a");

        // Seek one forward
        assert_eq!(data.seek(SeekFrom::Current(1)).unwrap(), 2);

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"c");

        // Seek back to start
        assert_eq!(data.seek(SeekFrom::Start(0)).unwrap(), 0);

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"a");

        // Seek relative to end
        assert_eq!(data.seek(SeekFrom::End(-2)).unwrap(), 7);

        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 1);
        assert_eq!(&buf, b"h");

        // Seek after end
        data.seek(SeekFrom::Current(100)).unwrap();

        // Now we don't get any bytes back
        let mut buf = [0; 1];
        assert_eq!(data.read(&mut buf).unwrap(), 0);
    }

    #[diesel_test_helper::test]
    fn before_start_blob_seeks_return_errors_without_moving_cursor() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        use std::io::{ErrorKind, Read, Seek, SeekFrom};

        let conn = &mut connection();
        crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO blobs (data) VALUES ('abc')")
            .execute(conn)
            .unwrap();

        let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        for position in [
            SeekFrom::End(-4),
            SeekFrom::Current(-2),
            SeekFrom::End(i64::MIN),
            SeekFrom::Current(i64::MIN),
        ] {
            assert_eq!(data.seek(SeekFrom::Start(1)).unwrap(), 1);
            assert_eq!(
                data.seek(position).unwrap_err().kind(),
                ErrorKind::InvalidInput
            );
            assert_eq!(data.stream_position().unwrap(), 1);

            let mut buf = [0; 1];
            data.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"b");
        }

        assert_eq!(data.seek(SeekFrom::End(-3)).unwrap(), 0);
        let mut buf = [0; 1];
        data.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"a");
        assert_eq!(data.seek(SeekFrom::Current(-1)).unwrap(), 0);
    }

    #[diesel_test_helper::test]
    fn use_conn_after_blob_drop() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        let conn = &mut connection();

        let _ = crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('abc')").execute(conn);

        let data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        drop(data);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('def')").execute(conn);
    }

    #[diesel_test_helper::test]
    fn use_conn_after_blob_close() {
        // Explicit close previously let `Drop` close the native handle a second time.
        // Reusing the connection verifies that the handle is closed exactly once.
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        let conn = &mut connection();

        crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn)
            .unwrap();
        assert_eq!(
            crate::sql_query("INSERT INTO blobs (data) VALUES ('abc')")
                .execute(conn)
                .unwrap(),
            1
        );

        let data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        data.close().unwrap();

        assert_eq!(
            crate::sql_query("INSERT INTO blobs (data) VALUES ('def')")
                .execute(conn)
                .unwrap(),
            1
        );
        assert_eq!(blobs::table.count().get_result::<i64>(conn).unwrap(), 2);
    }

    #[diesel_test_helper::test]
    fn blob_transaction() {
        table! {
            blobs {
                id -> Integer,
                data -> Blob,
            }
        }

        use std::io::Read;

        let conn = &mut connection();

        let _ = crate::sql_query("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)")
            .execute(conn);

        let _ = crate::sql_query("INSERT INTO blobs (data) VALUES ('abc')").execute(conn);

        {
            let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
            let mut buf = vec![];
            data.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, b"abc");
        }

        let res = conn.exclusive_transaction(|conn| {
            crate::sql_query("UPDATE blobs SET data = 'def' WHERE id = 1").execute(conn)?;

            let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
            let mut buf = vec![];
            data.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, b"def");

            Result::<(), _>::Err(Error::RollbackTransaction)
        });

        assert_eq!(res.unwrap_err(), Error::RollbackTransaction);

        let mut data = conn.get_read_only_blob(blobs::data, 1).unwrap();
        let mut buf = vec![];
        data.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"abc");
    }
}
