extern crate mysqlclient_sys as ffi;

use mysql::MysqlType;
use std::mem;
use std::os::{raw as libc};

pub struct Binds {
    data: Vec<Vec<u8>>,
    lengths: Vec<libc::c_ulong>,
    is_nulls: Vec<ffi::my_bool>,
    mysql_binds: Vec<ffi::MYSQL_BIND>,
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
        };
        unsafe { res.link_mysql_bind_pointers(); }
        res
    }

    pub fn mysql_binds(&mut self) -> &mut [ffi::MYSQL_BIND] {
        &mut self.mysql_binds
    }

    // This function relies on the invariant that no further mutations to this
    // struct will occur after this function has been called.
    unsafe fn link_mysql_bind_pointers(&mut self) {
        for (i, data) in self.data.iter_mut().enumerate() {
            self.mysql_binds[i].buffer = data.as_mut_ptr() as *mut libc::c_void;
            self.mysql_binds[i].buffer_length = data.capacity() as libc::c_ulong;
            self.mysql_binds[i].length = &mut self.lengths[i];
            self.mysql_binds[i].is_null = &mut self.is_nulls[i];
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
