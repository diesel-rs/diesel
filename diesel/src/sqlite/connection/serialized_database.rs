#![allow(unsafe_code)]
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use crate::result::{DatabaseErrorKind, Error};
use crate::QueryResult;
use std::ops::Deref;
use std::ptr::NonNull;

/// Owns a database serialization returned by `sqlite3_serialize` and releases any allocated buffer with `sqlite3_free`.
#[derive(Debug)]
pub struct SerializedDatabase {
    state: State,
}

#[derive(Debug)]
enum State {
    /// A successful serialization owning a SQLite allocation.
    Owned { data: NonNull<u8>, len: i64 },
    /// A valid serialization of an empty deserialized database.
    Empty,
    /// SQLite failed to allocate the output buffer.
    AllocationFailed,
}

impl SerializedDatabase {
    /// Creates a new `SerializedDatabase` with the given data pointer and length.
    ///
    /// # Safety
    ///
    /// `data` must exclusively own a successful `sqlite3_serialize` allocation that this value may free, and, if `0 <= len <= isize::MAX`, that allocation must hold `len` initialized bytes.
    pub(crate) unsafe fn new(data: NonNull<u8>, len: i64) -> Self {
        Self {
            state: State::Owned { data, len },
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            state: State::Empty,
        }
    }

    pub(crate) fn allocation_failed() -> Self {
        Self {
            state: State::AllocationFailed,
        }
    }

    /// Returns a slice of the serialized database.
    ///
    /// # Panics
    ///
    /// Panics if SQLite failed to allocate the buffer holding the serialized database, or if it reported a size this platform cannot address.
    #[deprecated(note = "Use `SerializedDatabase::try_as_slice` instead")]
    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    pub fn as_slice(&self) -> &[u8] {
        self.expect_slice()
    }

    #[cfg(not(all(feature = "with-deprecated", not(feature = "without-deprecated"))))]
    fn as_slice(&self) -> &[u8] {
        self.expect_slice()
    }

    /// Returns a slice of the serialized database, the out of memory error if
    /// SQLite failed to allocate the buffer holding it, or a conversion error
    /// if SQLite reported a size this platform cannot address.
    pub fn try_as_slice(&self) -> QueryResult<&[u8]> {
        match self.state {
            State::Owned { data, len } => {
                // `from_raw_parts` requires a length no larger than `isize::MAX`.
                let len = isize::try_from(len).map_err(Error::IntegerConversion)?;
                let len = usize::try_from(len).map_err(Error::IntegerConversion)?;
                // SAFETY: `new` guarantees an exclusively owned immutable allocation of `len` initialized bytes for every addressable `len`.
                Ok(unsafe { core::slice::from_raw_parts(data.as_ptr(), len) })
            }
            State::Empty => Ok(&[]),
            State::AllocationFailed => Err(Error::DatabaseError(
                DatabaseErrorKind::Unknown,
                Box::new("out of memory".to_string()),
            )),
        }
    }

    fn expect_slice(&self) -> &[u8] {
        match self.try_as_slice() {
            Ok(slice) => slice,
            Err(e) => panic!("Cannot access the serialized database: {e}"),
        }
    }
}

impl Deref for SerializedDatabase {
    type Target = [u8];

    #[allow(deprecated)] // no other way to implement this
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl Drop for SerializedDatabase {
    /// Deallocates the memory of the serialized database when it goes out of scope.
    fn drop(&mut self) {
        if let State::Owned { data, .. } = self.state {
            // SAFETY: `new` transfers one SQLite allocation that remains owned until this single `Drop` call.
            unsafe {
                ffi::sqlite3_free(data.as_ptr() as _);
            }
        }
    }
}

#[cfg(all(test, target_pointer_width = "32"))]
mod tests {
    use super::{ffi, SerializedDatabase};
    use crate::result::Error;
    use core::ptr::NonNull;

    // A serialization one byte past `isize::MAX` still fits `usize`, so only an `isize` bound
    // keeps it from building a slice `from_raw_parts` rejects. Its allocation is still freed.
    #[diesel_test_helper::test]
    fn oversized_serialization_is_rejected() {
        let len = i64::try_from(isize::MAX).expect("`isize::MAX` fits `i64`") + 1;
        // SAFETY: SQLite's allocator requires no connection and returns an exclusive allocation.
        let data = unsafe { ffi::sqlite3_malloc(1) };
        let data = NonNull::new(data.cast::<u8>()).expect("SQLite allocates a single byte");
        // SAFETY: `data` exclusively owns a SQLite allocation and `len` exceeds `isize::MAX`, so no byte count is required of it.
        let serialized = unsafe { SerializedDatabase::new(data, len) };

        assert!(matches!(
            serialized.try_as_slice(),
            Err(Error::IntegerConversion(_))
        ));
    }
}
