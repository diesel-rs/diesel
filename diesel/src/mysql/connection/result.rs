extern crate mysqlclient_sys as ffi;

pub struct RawResult(*mut ffi::MYSQL_RES);

impl RawResult {
    pub unsafe fn from_raw(raw: *mut ffi::MYSQL_RES) -> Option<Self> {
        if raw.is_null() {
            None
        } else {
            Some(RawResult(raw))
        }
    }

    pub fn fields(&mut self) -> ResultFields {
        unsafe { ffi::mysql_field_seek(self.0, 0); }
        ResultFields(self)
    }
}

impl Drop for RawResult {
    fn drop(&mut self) {
        unsafe { ffi::mysql_free_result(self.0); }
    }
}

pub struct ResultFields<'a>(&'a mut RawResult);

impl<'a> Iterator for ResultFields<'a> {
    type Item = &'a ffi::MYSQL_FIELD;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            ffi::mysql_fetch_field((self.0).0)
                .as_ref()
        }
    }
}
