use std::collections::HashMap;
use std::ffi::CStr;
use std::ptr::NonNull;
use std::slice;

use super::ffi;
use crate::mysql::connection::bind::Flags;

pub struct StatementMetadata {
    result: NonNull<ffi::MYSQL_RES>,
    // The strings in this hash map are only valid
    // as long as we do not free the result pointer above
    // We use a 'static lifetime here, because we cannot
    // have a self referential lifetime.
    // Therefore this lifetime must not leave this module
    column_indices: HashMap<&'static str, usize>,
}

impl StatementMetadata {
    pub fn new(result: NonNull<ffi::MYSQL_RES>) -> Self {
        let mut res = StatementMetadata {
            column_indices: HashMap::new(),
            result,
        };
        res.populate_column_indices();
        res
    }

    pub fn fields<'a>(&'a self) -> &'a [MysqlFieldMetadata] {
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

    pub fn column_indices<'a>(&'a self) -> &'a HashMap<&'a str, usize> {
        &self.column_indices
    }

    fn populate_column_indices(&mut self) {
        self.column_indices = self
            .fields()
            .iter()
            .enumerate()
            .filter_map(|(i, field)| unsafe {
                // This is highly unsafe because we create strings slices with a static life time
                // * We cannot use `MysqlFieldMetadata` because of this reason
                // * We cannot have a concrete life time because otherwise this would be
                //   an self referential struct
                // * This relies on the invariant that non of the slices leave this
                //   type with anything other then a concrete life time bound to this
                //   type
                if field.0.name.is_null() {
                    None
                } else {
                    CStr::from_ptr(field.0.name).to_str().ok().map(|f| (f, i))
                }
            })
            .collect()
    }
}

impl Drop for StatementMetadata {
    fn drop(&mut self) {
        unsafe { ffi::mysql_free_result(self.result.as_mut()) };
    }
}

#[repr(transparent)]
pub struct MysqlFieldMetadata<'a>(ffi::MYSQL_FIELD, std::marker::PhantomData<&'a ()>);

impl<'a> MysqlFieldMetadata<'a> {
    pub fn field_name(&self) -> Option<&str> {
        if self.0.name.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(self.0.name).to_str().ok() }
        }
    }

    pub fn field_type(&self) -> ffi::enum_field_types {
        self.0.type_
    }

    pub(crate) fn flags(&self) -> Flags {
        Flags::from(self.0.flags)
    }
}
