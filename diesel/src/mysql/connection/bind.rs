extern crate mysqlclient_sys as ffi;

use mysql::MysqlType;
use std::mem;
use std::os::{raw as libc};

pub struct Binds {
    data: Vec<BindData>,
    mysql_binds: Vec<ffi::MYSQL_BIND>,
}

impl Binds {
    pub fn from_input_data(input: Vec<(MysqlType, Option<Vec<u8>>)>) -> Self {
        let (mysql_binds, data) = input.into_iter().map(|(tpe, bytes)| {
            let mut bind: ffi::MYSQL_BIND = unsafe { mem::zeroed() };
            bind.buffer_type = mysql_type_to_ffi_type(tpe);
            let data = BindData::for_input(bytes);
            (bind, data)
        }).unzip();

        let mut res = Binds {
            data: data,
            mysql_binds: mysql_binds,
        };
        unsafe { res.link_mysql_bind_pointers(); }
        res
    }

    pub fn from_output_types<Iter>(types: Iter) -> Self where
        Iter: IntoIterator<Item=ffi::enum_field_types>,
    {
        let (mysql_binds, data) = types.into_iter().map(|tpe| {
            let mut bind: ffi::MYSQL_BIND = unsafe { mem::zeroed() };
            bind.buffer_type = tpe;
            let data = BindData::for_output(tpe);
            (bind, data)
        }).unzip();

        let mut res = Binds {
            data: data,
            mysql_binds: mysql_binds,
        };
        unsafe { res.link_mysql_bind_pointers(); }
        res
    }

    pub fn mysql_binds(&mut self) -> &mut [ffi::MYSQL_BIND] {
        &mut self.mysql_binds
    }

    /// The caller of this function must immediately check for errors
    /// after return
    pub fn populate_dynamic_buffers(&mut self, stmt: *mut ffi::MYSQL_STMT) {
        for (i, data) in self.data.iter_mut().enumerate() {
            let bind = &mut self.mysql_binds[i];
            if data.is_truncated() {
                data.reserve_to_fit(bind);
                let load_result = unsafe {
                    ffi::mysql_stmt_fetch_column(
                        stmt,
                        bind,
                        i as libc::c_uint,
                        0,
                    )
                };
                if load_result != 0 {
                    return;
                }
            }

            data.update_buffer_len(bind);
        }
    }

    pub fn reset_dynamic_buffers(&mut self) {
        for (data, bind) in self.data.iter_mut().zip(&mut self.mysql_binds) {
            if known_buffer_size_for_ffi_type(bind.buffer_type).is_none() {
                data.reset();
            }
        }
    }

    pub fn field_data(&self, idx: usize) -> Option<&[u8]> {
        self.data[idx].bytes()
    }

    unsafe fn link_mysql_bind_pointers(&mut self) {
        for (data, bind) in self.data.iter_mut().zip(&mut self.mysql_binds) {
            data.link_mysql_bind_pointers(bind);
        }
    }
}

struct BindData {
    bytes: Vec<u8>,
    length: libc::c_ulong,
    is_null: ffi::my_bool,
    is_truncated: Option<ffi::my_bool>,
}

impl BindData {
    fn for_input(data: Option<Vec<u8>>) -> Self {
        let is_null = if data.is_none() { 1 } else { 0 };
        let bytes = data.unwrap_or(Vec::new());
        let length = bytes.len() as libc::c_ulong;

        BindData {
            bytes: bytes,
            length: length,
            is_null: is_null,
            is_truncated: None,
        }
    }

    fn for_output(tpe: ffi::enum_field_types) -> Self {
        let bytes = known_buffer_size_for_ffi_type(tpe)
            .map(|len| vec![0; len])
            .unwrap_or(Vec::new());
        let length = bytes.len() as libc::c_ulong;

        BindData {
            bytes: bytes,
            length: length,
            is_null: 0,
            is_truncated: Some(0),
        }
    }

    fn is_truncated(&self) -> bool {
        self.is_truncated.unwrap_or(0) != 0
    }

    fn bytes(&self) -> Option<&[u8]> {
        if self.is_null == 0 {
            Some(&*self.bytes)
        } else {
            None
        }
    }

    fn reset(&mut self) {
        unsafe { self.bytes.set_len(0) };
        self.length = 0;
    }

    fn reserve_to_fit(&mut self, bind: &mut ffi::MYSQL_BIND) {
        let written_bytes = self.bytes.capacity();
        let needed_capacity = self.length as usize;
        if needed_capacity > written_bytes {
            self.bytes.reserve(needed_capacity - written_bytes);
            // Re-link the buffer pointer since we just re-allocated
            bind.buffer = self.bytes.as_mut_ptr() as *mut libc::c_void;
            bind.buffer_length = self.bytes.capacity() as libc::c_ulong;
        }
    }

    fn update_buffer_len(&mut self, bind: &ffi::MYSQL_BIND) {
        use std::cmp::min;

        let actual_bytes_in_buffer = min(bind.buffer_length, self.length);
        unsafe { self.bytes.set_len(actual_bytes_in_buffer as usize) }
    }

    unsafe fn link_mysql_bind_pointers(&mut self, bind: &mut ffi::MYSQL_BIND) {
        bind.buffer = self.bytes.as_mut_ptr() as *mut libc::c_void;
        bind.buffer_length = self.bytes.capacity() as libc::c_ulong;
        bind.length = &mut self.length;
        bind.is_null = &mut self.is_null;

        if let Some(ref mut is_truncated) = self.is_truncated {
            bind.error = is_truncated;
        }
    }
}

fn mysql_type_to_ffi_type(tpe: MysqlType) -> ffi::enum_field_types {
    use self::ffi::enum_field_types::*;

    match tpe {
        MysqlType::Tiny => MYSQL_TYPE_TINY,
        MysqlType::Short => MYSQL_TYPE_SHORT,
        MysqlType::Long => MYSQL_TYPE_LONG,
        MysqlType::LongLong => MYSQL_TYPE_LONGLONG,
        MysqlType::Float => MYSQL_TYPE_FLOAT,
        MysqlType::Double => MYSQL_TYPE_DOUBLE,
        MysqlType::Time => MYSQL_TYPE_TIME,
        MysqlType::Date => MYSQL_TYPE_DATE,
        MysqlType::DateTime => MYSQL_TYPE_DATETIME,
        MysqlType::Timestamp => MYSQL_TYPE_TIMESTAMP,
        MysqlType::String => MYSQL_TYPE_STRING,
        MysqlType::Blob => MYSQL_TYPE_BLOB,
    }
}

fn known_buffer_size_for_ffi_type(tpe: ffi::enum_field_types) -> Option<usize> {
    use std::mem::size_of;
    use self::ffi::enum_field_types as t;

    match tpe {
        t::MYSQL_TYPE_TINY => Some(1),
        t::MYSQL_TYPE_YEAR |
        t::MYSQL_TYPE_SHORT => Some(2),
        t::MYSQL_TYPE_INT24 |
        t::MYSQL_TYPE_LONG |
        t::MYSQL_TYPE_FLOAT => Some(4),
        t::MYSQL_TYPE_LONGLONG |
        t::MYSQL_TYPE_DOUBLE => Some(8),
        t::MYSQL_TYPE_TIME |
        t::MYSQL_TYPE_DATE |
        t::MYSQL_TYPE_DATETIME |
        t::MYSQL_TYPE_TIMESTAMP =>
            Some(size_of::<ffi::MYSQL_TIME>()),
        _ => None,
    }
}
