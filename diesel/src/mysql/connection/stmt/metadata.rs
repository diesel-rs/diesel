#![allow(unsafe_code)] // module uses ffi
use std::ffi::CStr;
use std::ptr::NonNull;
use std::slice;

use super::ffi;
use crate::mysql::connection::bind::Flags;

pub(in crate::mysql::connection) struct StatementMetadata {
    result: NonNull<ffi::MYSQL_RES>,
}

impl StatementMetadata {
    pub(in crate::mysql::connection) fn new(result: NonNull<ffi::MYSQL_RES>) -> Self {
        StatementMetadata { result }
    }

    pub(in crate::mysql::connection) fn fields(&'_ self) -> &'_ [MysqlFieldMetadata<'_>] {
        unsafe {
            let num_fields = ffi::mysql_num_fields(self.result.as_ptr());
            let field_ptr = ffi::mysql_fetch_fields(self.result.as_ptr());
            if field_ptr.is_null() {
                &[]
            } else {
                slice::from_raw_parts(field_ptr as _, num_fields as usize)
            }
        }
    }
}

impl Drop for StatementMetadata {
    fn drop(&mut self) {
        unsafe { ffi::mysql_free_result(self.result.as_mut()) };
    }
}

#[repr(transparent)]
pub(in crate::mysql::connection) struct MysqlFieldMetadata<'a>(
    ffi::MYSQL_FIELD,
    std::marker::PhantomData<&'a ()>,
);

impl<'a> MysqlFieldMetadata<'a> {
    pub(in crate::mysql::connection) fn field_name(&self) -> Option<&str> {
        if self.0.name.is_null() {
            None
        } else {
            unsafe {
                Some(CStr::from_ptr(self.0.name).to_str().expect(
                    "Expect mysql field names to be UTF-8, because we \
                     requested UTF-8 encoding on connection setup",
                ))
            }
        }
    }

    pub(in crate::mysql::connection) fn field_type(&self) -> ffi::enum_field_types {
        self.0.type_
    }

    pub(in crate::mysql::connection) fn flags(&self) -> Flags {
        Flags::from(self.0.flags)
    }
}
