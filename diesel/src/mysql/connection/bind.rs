extern crate mysqlclient_sys as ffi;

use std::mem;
use std::os::raw as libc;

use super::stmt::Statement;
use crate::mysql::{MysqlType, MysqlTypeMetadata, MysqlValue};
use crate::result::QueryResult;

pub struct Binds {
    data: Vec<BindData>,
}

impl Binds {
    pub fn from_input_data<Iter>(input: Iter) -> Self
    where
        Iter: IntoIterator<Item = (MysqlTypeMetadata, Option<Vec<u8>>)>,
    {
        let data = input
            .into_iter()
            .map(|(metadata, bytes)| {
                BindData::for_input(metadata.data_type, metadata.is_unsigned as _, bytes)
            })
            .collect();

        Binds { data }
    }

    pub fn from_output_types(types: Vec<MysqlTypeMetadata>) -> Self {
        let data = types
            .into_iter()
            .map(|metadata| (metadata.data_type.into(), metadata.is_unsigned as _))
            .map(BindData::for_output)
            .collect();

        Binds { data }
    }

    pub fn from_result_metadata(fields: &[ffi::MYSQL_FIELD]) -> Self {
        let data = fields
            .iter()
            .map(|field| (field.type_, is_field_unsigned(field)))
            .map(BindData::for_output)
            .collect();

        Binds { data }
    }

    pub fn with_mysql_binds<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(*mut ffi::MYSQL_BIND) -> T,
    {
        let mut binds = self
            .data
            .iter_mut()
            .map(|x| unsafe { x.mysql_bind() })
            .collect::<Vec<_>>();
        f(binds.as_mut_ptr())
    }

    pub fn populate_dynamic_buffers(&mut self, stmt: &Statement) -> QueryResult<()> {
        for (i, data) in self.data.iter_mut().enumerate() {
            data.did_numeric_overflow_occur()?;
            // This is safe because we are re-binding the invalidated buffers
            // at the end of this function
            unsafe {
                if let Some((mut bind, offset)) = data.bind_for_truncated_data() {
                    stmt.fetch_column(&mut bind, i, offset)?
                } else {
                    data.update_buffer_length()
                }
            }
        }

        unsafe { self.with_mysql_binds(|bind_ptr| stmt.bind_result(bind_ptr)) }
    }

    pub fn update_buffer_lengths(&mut self) {
        for data in &mut self.data {
            data.update_buffer_length();
        }
    }

    pub fn field_data(&self, idx: usize) -> Option<MysqlValue<'_>> {
        let data = &self.data[idx];
        let tpe = data.tpe.into();
        self.data[idx].bytes().map(|bytes| {
            MysqlValue::new(
                bytes,
                MysqlTypeMetadata {
                    data_type: tpe,
                    is_unsigned: data.is_unsigned != 0,
                },
            )
        })
    }
}

struct BindData {
    tpe: ffi::enum_field_types,
    bytes: Vec<u8>,
    length: libc::c_ulong,
    is_null: ffi::my_bool,
    is_truncated: Option<ffi::my_bool>,
    is_unsigned: ffi::my_bool,
}

impl BindData {
    fn for_input(tpe: MysqlType, is_unsigned: ffi::my_bool, data: Option<Vec<u8>>) -> Self {
        let is_null = if data.is_none() { 1 } else { 0 };
        let bytes = data.unwrap_or_default();
        let length = bytes.len() as libc::c_ulong;

        BindData {
            tpe: tpe.into(),
            bytes,
            length,
            is_null,
            is_truncated: None,
            is_unsigned,
        }
    }

    fn for_output((tpe, is_unsigned): (ffi::enum_field_types, ffi::my_bool)) -> Self {
        let bytes = known_buffer_size_for_ffi_type(tpe)
            .map(|len| vec![0; len])
            .unwrap_or_default();
        let length = bytes.len() as libc::c_ulong;

        BindData {
            tpe,
            bytes,
            length,
            is_null: 0,
            is_truncated: Some(0),
            is_unsigned,
        }
    }

    fn is_truncated(&self) -> bool {
        self.is_truncated.unwrap_or(0) != 0
    }

    fn is_fixed_size_buffer(&self) -> bool {
        known_buffer_size_for_ffi_type(self.tpe).is_some()
    }

    fn bytes(&self) -> Option<&[u8]> {
        if self.is_null == 0 {
            Some(&*self.bytes)
        } else {
            None
        }
    }

    fn update_buffer_length(&mut self) {
        use std::cmp::min;

        let actual_bytes_in_buffer = min(self.bytes.capacity(), self.length as usize);
        unsafe { self.bytes.set_len(actual_bytes_in_buffer) }
    }

    unsafe fn mysql_bind(&mut self) -> ffi::MYSQL_BIND {
        let mut bind: ffi::MYSQL_BIND = mem::zeroed();
        bind.buffer_type = self.tpe;
        bind.buffer = self.bytes.as_mut_ptr() as *mut libc::c_void;
        bind.buffer_length = self.bytes.capacity() as libc::c_ulong;
        bind.length = &mut self.length;
        bind.is_null = &mut self.is_null;
        bind.is_unsigned = self.is_unsigned;

        if let Some(ref mut is_truncated) = self.is_truncated {
            bind.error = is_truncated;
        }

        bind
    }

    /// Resizes the byte buffer to fit the value of `self.length`, and returns
    /// a tuple of a bind pointing at the truncated data, and the offset to use
    /// in order to read the truncated data into it.
    ///
    /// This invalidates the bind previously returned by `mysql_bind`. Calling
    /// this function is unsafe unless the binds are immediately rebound.
    unsafe fn bind_for_truncated_data(&mut self) -> Option<(ffi::MYSQL_BIND, usize)> {
        if self.is_truncated() {
            let offset = self.bytes.capacity();
            let truncated_amount = self.length as usize - offset;

            debug_assert!(
                truncated_amount > 0,
                "output buffers were invalidated \
                 without calling `mysql_stmt_bind_result`"
            );
            self.bytes.set_len(offset);
            self.bytes.reserve(truncated_amount);
            self.bytes.set_len(self.length as usize);

            let mut bind = self.mysql_bind();
            bind.buffer = self.bytes[offset..].as_mut_ptr() as *mut libc::c_void;
            bind.buffer_length = truncated_amount as libc::c_ulong;
            Some((bind, offset))
        } else {
            None
        }
    }

    fn did_numeric_overflow_occur(&self) -> QueryResult<()> {
        use crate::result::Error::DeserializationError;

        if self.is_truncated() && self.is_fixed_size_buffer() {
            Err(DeserializationError(
                "Numeric overflow/underflow occurred".into(),
            ))
        } else {
            Ok(())
        }
    }
}

impl From<MysqlType> for ffi::enum_field_types {
    fn from(tpe: MysqlType) -> Self {
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
            MysqlType::Numeric => MYSQL_TYPE_NEWDECIMAL,
        }
    }
}

impl From<ffi::enum_field_types> for MysqlType {
    fn from(tpe: ffi::enum_field_types) -> Self {
        use self::ffi::enum_field_types::*;

        match tpe {
            MYSQL_TYPE_TINY => MysqlType::Tiny,
            MYSQL_TYPE_SHORT => MysqlType::Short,
            MYSQL_TYPE_INT24 | MYSQL_TYPE_LONG => MysqlType::Long,
            MYSQL_TYPE_LONGLONG => MysqlType::LongLong,
            MYSQL_TYPE_FLOAT => MysqlType::Float,
            MYSQL_TYPE_DOUBLE => MysqlType::Double,
            MYSQL_TYPE_TIME => MysqlType::Time,
            MYSQL_TYPE_DATE => MysqlType::Date,
            MYSQL_TYPE_DATETIME => MysqlType::DateTime,
            MYSQL_TYPE_TIMESTAMP => MysqlType::Timestamp,
            MYSQL_TYPE_STRING => MysqlType::String,
            MYSQL_TYPE_BLOB => MysqlType::Blob,
            MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL => MysqlType::Numeric,
            MYSQL_TYPE_NULL
            | MYSQL_TYPE_YEAR
            | MYSQL_TYPE_NEWDATE
            | MYSQL_TYPE_VARCHAR
            | MYSQL_TYPE_BIT
            | MYSQL_TYPE_TIMESTAMP2
            | MYSQL_TYPE_DATETIME2
            | MYSQL_TYPE_TIME2
            | MYSQL_TYPE_JSON
            | MYSQL_TYPE_ENUM
            | MYSQL_TYPE_SET
            | MYSQL_TYPE_TINY_BLOB
            | MYSQL_TYPE_MEDIUM_BLOB
            | MYSQL_TYPE_LONG_BLOB
            | MYSQL_TYPE_VAR_STRING
            | MYSQL_TYPE_GEOMETRY => unimplemented!(),
        }
    }
}

fn known_buffer_size_for_ffi_type(tpe: ffi::enum_field_types) -> Option<usize> {
    use self::ffi::enum_field_types as t;
    use std::mem::size_of;

    match tpe {
        t::MYSQL_TYPE_TINY => Some(1),
        t::MYSQL_TYPE_YEAR | t::MYSQL_TYPE_SHORT => Some(2),
        t::MYSQL_TYPE_INT24 | t::MYSQL_TYPE_LONG | t::MYSQL_TYPE_FLOAT => Some(4),
        t::MYSQL_TYPE_LONGLONG | t::MYSQL_TYPE_DOUBLE => Some(8),
        t::MYSQL_TYPE_TIME
        | t::MYSQL_TYPE_DATE
        | t::MYSQL_TYPE_DATETIME
        | t::MYSQL_TYPE_TIMESTAMP => Some(size_of::<ffi::MYSQL_TIME>()),
        _ => None,
    }
}

fn is_field_unsigned(field: &ffi::MYSQL_FIELD) -> ffi::my_bool {
    const UNSIGNED_FLAG: libc::c_uint = 32;
    (field.flags & UNSIGNED_FLAG > 0) as _
}
