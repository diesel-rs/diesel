#![allow(unsafe_code)]
extern crate libsqlite3_sys as ffi;

use std::ops::Deref;

/// `SerializedDatabase` is a wrapper for a serialized database that is dynamically allocated by calling `sqlite3_serialize`.
/// This RAII wrapper is necessary to deallocate the memory when it goes out of scope with `sqlite3_free`.
#[derive(Debug)]
pub struct SerializedDatabase {
    data: *mut u8,
    len: usize,
}

impl SerializedDatabase {
    /// Creates a new `SerializedDatabase` with the given data pointer and length.
    pub fn new(data: *mut u8, len: usize) -> Self {
        Self { data, len }
    }

    /// Returns a slice of the serialized database.
    pub fn as_slice(&self) -> &[u8] {
        // The pointer is never null because we don't pass the NO_COPY flag
        unsafe { std::slice::from_raw_parts(self.data, self.len) }
    }
}

impl Deref for SerializedDatabase {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl Drop for SerializedDatabase {
    /// Deallocates the memory of the serialized database when it goes out of scope.
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_free(self.data as _);
        }
    }
}
