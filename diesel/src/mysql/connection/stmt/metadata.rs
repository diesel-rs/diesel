use std::collections::HashMap;
use std::ffi::CStr;
use std::slice;

use super::ffi;

pub struct StatementMetadata {
    result: &'static mut ffi::MYSQL_RES,
    column_indices: HashMap<&'static str, usize>,
}

impl StatementMetadata {
    pub fn new(result: &'static mut ffi::MYSQL_RES) -> Self {
        let mut res = StatementMetadata {
            column_indices: HashMap::new(),
            result,
        };
        res.populate_column_indices();
        res
    }

    pub fn fields(&self) -> &[ffi::MYSQL_FIELD] {
        unsafe {
            let ptr = self.result as *const _ as *mut _;
            let num_fields = ffi::mysql_num_fields(ptr);
            let field_ptr = ffi::mysql_fetch_fields(ptr);
            slice::from_raw_parts(field_ptr, num_fields as usize)
        }
    }

    pub fn column_indices(&self) -> &HashMap<&str, usize> {
        &self.column_indices
    }

    fn populate_column_indices(&mut self) {
        self.column_indices = self
            .fields()
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let c_name = unsafe { CStr::from_ptr(field.name) };
                (c_name.to_str().unwrap_or_default(), i)
            })
            .collect()
    }
}

impl Drop for StatementMetadata {
    fn drop(&mut self) {
        unsafe { ffi::mysql_free_result(self.result) };
    }
}
