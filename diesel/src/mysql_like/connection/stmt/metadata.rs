#![allow(unsafe_code)] // module uses ffi
use core::ffi::CStr;
use core::ptr::NonNull;
use core::slice;

use super::ffi;
use crate::mysql_like::connection::bind::Flags;

pub(in crate::mysql_like::connection) struct StatementMetadata {
    fields: Vec<MysqlFieldMetadata>,
}

impl StatementMetadata {
    pub(in crate::mysql_like::connection) fn new(mut result: NonNull<ffi::MYSQL_RES>) -> Self {
        let fields: &[ffi::MYSQL_FIELD] = unsafe {
            let num_fields = ffi::mysql_num_fields(result.as_ptr());
            let field_ptr = ffi::mysql_fetch_fields(result.as_ptr());
            if field_ptr.is_null() {
                &[]
            } else {
                slice::from_raw_parts(field_ptr, num_fields as usize)
            }
        };
        let fields = fields
            .iter()
            .map(|f| {
                let field_name = if f.name.is_null() {
                    None
                } else {
                    unsafe {
                        Some(CStr::from_ptr(f.name).to_str().expect(
                            "Expect mysql field names to be UTF-8, because we \
                     requested UTF-8 encoding on connection setup",
                        ))
                    }
                };
                MysqlFieldMetadata {
                    field_name: field_name.map(Into::into),
                    field_type: f.type_,
                    flags: f.flags,
                }
            })
            .collect::<Vec<_>>();
        unsafe {
            ffi::mysql_free_result(result.as_mut());
        }
        StatementMetadata { fields }
    }

    pub(in crate::mysql_like::connection) fn fields(&'_ self) -> &'_ [MysqlFieldMetadata] {
        &self.fields
    }
}

pub(in crate::mysql_like::connection) struct MysqlFieldMetadata {
    field_name: Option<Box<str>>,
    field_type: ffi::enum_field_types,
    flags: core::ffi::c_uint,
}

impl MysqlFieldMetadata {
    pub(in crate::mysql_like::connection) fn field_name(&self) -> Option<&str> {
        self.field_name.as_deref()
    }

    pub(in crate::mysql_like::connection) fn field_type(&self) -> ffi::enum_field_types {
        self.field_type
    }

    pub(in crate::mysql_like::connection) fn flags(
        &self,
    ) -> Result<Flags, Box<dyn core::error::Error + Send + Sync>> {
        Flags::try_from(self.flags)
    }
}
