#![allow(unsafe_code)]
extern crate libsqlite3_sys as ffi;

#[derive(Debug)]
pub struct SerializedDatabase {
    data: *mut u8,
    len: usize,
}

impl SerializedDatabase {
    pub fn new(data: *mut u8, len: usize) -> Self {
        Self { data, len }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.data, self.len) }
    }
}

impl Drop for SerializedDatabase {
    fn drop(&mut self) {
        unsafe {
            // Call the FFI function to free the memory
            ffi::sqlite3_free(self.data as _);
        }
    }
}
