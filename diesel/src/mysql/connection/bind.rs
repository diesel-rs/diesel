extern crate mysqlclient_sys as ffi;

use mysql::MysqlType;
use std::mem;
use std::os::{raw as libc};

pub struct Binds {
    data: Vec<Vec<u8>>,
    lengths: Vec<libc::c_ulong>,
    is_nulls: Vec<ffi::my_bool>,
    mysql_binds: Vec<ffi::MYSQL_BIND>,
    errors: Option<Vec<ffi::my_bool>>,
}

impl Binds {
    pub fn from_input_data(input: Vec<(MysqlType, Option<Vec<u8>>)>) -> Self {
        let is_nulls = input.iter()
            .map(|&(_, ref data)| if data.is_none() { 1 } else { 0 })
            .collect();
        let (types, data): (Vec<_>, Vec<_>) = input.into_iter()
            .map(|(t, data)| (t, data.unwrap_or(Vec::new()))).unzip();
        let lengths = data.iter().map(|x| x.len() as libc::c_ulong).collect();

        let mysql_binds = types.into_iter().map(|tpe| {
            let mut bind: ffi::MYSQL_BIND = unsafe { mem::zeroed() };
            bind.buffer_type = mysql_type_to_ffi_type(tpe);
            bind
        }).collect();

        let mut res = Binds {
            data: data,
            lengths: lengths,
            is_nulls: is_nulls,
            mysql_binds: mysql_binds,
            errors: None,
        };
        unsafe { res.link_mysql_bind_pointers(); }
        res
    }

    pub fn from_output_types<Iter>(types: Iter) -> Self where
        Iter: IntoIterator<Item=ffi::enum_field_types>,
    {
        let mysql_binds = types.into_iter().map(|tpe| {
            let mut bind: ffi::MYSQL_BIND = unsafe { mem::zeroed() };
            bind.buffer_type = tpe;
            bind
        }).collect::<Vec<_>>();
        let is_nulls = vec![0; mysql_binds.len()];
        let errors = is_nulls.clone();
        let data = mysql_binds.iter().map(|bind| {
            match known_buffer_size_for_ffi_type(bind.buffer_type) {
                Some(size) => vec![0; size as usize],
                None => Vec::new(),
            }
        }).collect::<Vec<_>>();
        let lengths = data.iter().map(|x| x.len() as libc::c_ulong).collect();

        let mut res = Binds {
            data: data,
            lengths: lengths,
            is_nulls: is_nulls,
            mysql_binds: mysql_binds,
            errors: Some(errors),
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
        use std::cmp::min;

        for (i, buffer) in self.data.iter_mut().enumerate() {
            if self.errors.as_ref().unwrap()[i] != 0 {
                let written_bytes = buffer.len();
                let needed_capacity = self.lengths[i] as usize;
                buffer.reserve(needed_capacity - written_bytes);
                self.mysql_binds[i].buffer = buffer.as_mut_ptr() as *mut libc::c_void;
                mem::swap(&mut self.lengths[i], &mut self.mysql_binds[i].buffer_length);
                let load_result = unsafe {
                    ffi::mysql_stmt_fetch_column(
                        stmt,
                        &mut self.mysql_binds[i],
                        i as libc::c_uint,
                        written_bytes as libc::c_ulong,
                    )
                };
                if load_result != 0 {
                    return;
                }
            }

            let actual_bytes_in_buffer = min(
                self.mysql_binds[i].buffer_length,
                self.lengths[i],
            );
            unsafe { buffer.set_len(actual_bytes_in_buffer as usize) }
        }
    }

    pub fn reset_dynamic_buffers(&mut self) {
        for (i, bind) in self.mysql_binds.iter().enumerate() {
            if known_buffer_size_for_ffi_type(bind.buffer_type).is_none() {
                self.lengths[i] = 0;
                unsafe { self.data[i].set_len(0) };
            }
        }
    }

    pub fn field_data(&self, idx: usize) -> Option<&[u8]> {
        if self.is_nulls[idx] == 0 {
            Some(&*self.data[idx])
        } else {
            None
        }
    }

    // This function relies on the invariant that no further mutations to this
    // struct will occur after this function has been called.
    unsafe fn link_mysql_bind_pointers(&mut self) {
        for (i, data) in self.data.iter_mut().enumerate() {
            self.mysql_binds[i].buffer = data.as_mut_ptr() as *mut libc::c_void;
            self.mysql_binds[i].buffer_length = data.capacity() as libc::c_ulong;
            self.mysql_binds[i].length = &mut self.lengths[i];
            self.mysql_binds[i].is_null = &mut self.is_nulls[i];

            if let Some(ref mut errors) = self.errors {
                self.mysql_binds[i].error = &mut errors[i];
            }
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

fn known_buffer_size_for_ffi_type(tpe: ffi::enum_field_types) -> Option<libc::c_ulong> {
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
            Some(size_of::<ffi::MYSQL_TIME>() as libc::c_ulong),
        _ => None,
    }
}
