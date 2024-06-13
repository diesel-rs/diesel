#![allow(unsafe_code)] // module uses ffi
use mysqlclient_sys as ffi;
use std::mem;
use std::mem::MaybeUninit;
use std::ops::Index;
use std::os::raw as libc;
use std::ptr::NonNull;

use super::stmt::MysqlFieldMetadata;
use super::stmt::StatementUse;
use crate::mysql::connection::stmt::StatementMetadata;
use crate::mysql::types::date_and_time::MysqlTime;
use crate::mysql::{MysqlType, MysqlValue};
use crate::result::QueryResult;

pub(super) struct PreparedStatementBinds(Binds);

pub(super) struct OutputBinds(Binds);

impl Clone for OutputBinds {
    fn clone(&self) -> Self {
        Self(Binds {
            data: self.0.data.clone(),
        })
    }
}

struct Binds {
    data: Vec<BindData>,
}

impl PreparedStatementBinds {
    pub(super) fn from_input_data<Iter>(input: Iter) -> Self
    where
        Iter: IntoIterator<Item = (MysqlType, Option<Vec<u8>>)>,
    {
        let data = input
            .into_iter()
            .map(BindData::for_input)
            .collect::<Vec<_>>();

        Self(Binds { data })
    }

    pub(super) fn with_mysql_binds<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(*mut ffi::MYSQL_BIND) -> T,
    {
        self.0.with_mysql_binds(f)
    }
}

impl OutputBinds {
    pub(super) fn from_output_types(
        types: &[Option<MysqlType>],
        metadata: &StatementMetadata,
    ) -> Self {
        let data = metadata
            .fields()
            .iter()
            .zip(types.iter().copied().chain(std::iter::repeat(None)))
            .map(|(field, tpe)| BindData::for_output(tpe, field))
            .collect();

        Self(Binds { data })
    }

    pub(super) fn populate_dynamic_buffers(&mut self, stmt: &StatementUse<'_>) -> QueryResult<()> {
        for (i, data) in self.0.data.iter_mut().enumerate() {
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

    pub(super) fn update_buffer_lengths(&mut self) {
        for data in &mut self.0.data {
            data.update_buffer_length();
        }
    }

    pub(super) fn with_mysql_binds<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(*mut ffi::MYSQL_BIND) -> T,
    {
        self.0.with_mysql_binds(f)
    }
}

impl Binds {
    fn with_mysql_binds<F, T>(&mut self, f: F) -> T
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
}

impl Index<usize> for OutputBinds {
    type Output = BindData;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0.data[index]
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct Flags: u32 {
        const NOT_NULL_FLAG = 1;
        const PRI_KEY_FLAG = 2;
        const UNIQUE_KEY_FLAG = 4;
        const MULTIPLE_KEY_FLAG = 8;
        const BLOB_FLAG = 16;
        const UNSIGNED_FLAG = 32;
        const ZEROFILL_FLAG = 64;
        const BINARY_FLAG = 128;
        const ENUM_FLAG = 256;
        const AUTO_INCREMENT_FLAG = 512;
        const TIMESTAMP_FLAG = 1024;
        const SET_FLAG = 2048;
        const NO_DEFAULT_VALUE_FLAG = 4096;
        const ON_UPDATE_NOW_FLAG = 8192;
        const NUM_FLAG = 32768;
        const PART_KEY_FLAG = 16384;
        const GROUP_FLAG = 32768;
        const UNIQUE_FLAG = 65536;
        const BINCMP_FLAG = 130_172;
        const GET_FIXED_FIELDS_FLAG = (1<<18);
        const FIELD_IN_PART_FUNC_FLAG = (1 << 19);
    }
}

impl From<u32> for Flags {
    fn from(flags: u32) -> Self {
        Flags::from_bits(flags).expect(
            "We encountered an unknown type flag while parsing \
             Mysql's type information. If you see this error message \
             please open an issue at diesels github page.",
        )
    }
}

#[derive(Debug)]
pub(super) struct BindData {
    tpe: ffi::enum_field_types,
    bytes: Option<NonNull<u8>>,
    length: libc::c_ulong,
    capacity: usize,
    flags: Flags,
    is_null: ffi::my_bool,
    is_truncated: Option<ffi::my_bool>,
}

// We need to write a manual clone impl
// as we need to clone the underlying buffer
// instead of just copying the pointer
impl Clone for BindData {
    fn clone(&self) -> Self {
        let (ptr, len, capacity) = if let Some(ptr) = self.bytes {
            let slice = unsafe {
                // We know that this points to a slice and the pointer is not null at this
                // location
                // The length pointer is valid as long as none missuses `bind_for_truncated_data`
                // as this is the only location that updates the length field before the corresponding data are
                // written. At the time of writing this comment, the `BindData::bind_for_truncated_data`
                // function is only called by `Binds::populate_dynamic_buffers` which ensures the corresponding
                // invariant.
                std::slice::from_raw_parts(ptr.as_ptr(), self.length as usize)
            };
            let mut vec = slice.to_owned();
            let ptr = NonNull::new(vec.as_mut_ptr());
            let len = vec.len() as libc::c_ulong;
            let capacity = vec.capacity();
            mem::forget(vec);
            (ptr, len, capacity)
        } else {
            (None, 0, 0)
        };
        Self {
            tpe: self.tpe,
            bytes: ptr,
            length: len,
            capacity,
            flags: self.flags,
            is_null: self.is_null,
            is_truncated: self.is_truncated,
        }
    }
}

impl Drop for BindData {
    fn drop(&mut self) {
        if let Some(bytes) = self.bytes {
            std::mem::drop(unsafe {
                // We know that this buffer was allocated by a vector, so constructing a vector from it is fine
                // We know the correct capacity here
                // We use 0 as length to prevent situations where the length is already updated but
                // no date are already written as we could touch uninitialized memory otherwise
                // Using 0 as length is fine as we don't need to call drop for `u8`
                // (as there is no drop impl for primitive types)
                Vec::from_raw_parts(bytes.as_ptr(), 0, self.capacity)
            });
            self.bytes = None;
        }
    }
}

impl BindData {
    fn for_input((tpe, data): (MysqlType, Option<Vec<u8>>)) -> Self {
        let (tpe, flags) = tpe.into();
        let is_null = ffi::my_bool::from(data.is_none());
        let mut bytes = data.unwrap_or_default();
        let ptr = NonNull::new(bytes.as_mut_ptr());
        let len = bytes.len() as libc::c_ulong;
        let capacity = bytes.capacity();
        mem::forget(bytes);
        Self {
            tpe,
            bytes: ptr,
            length: len,
            capacity,
            flags,
            is_null,
            is_truncated: None,
        }
    }

    fn for_output(tpe: Option<MysqlType>, metadata: &MysqlFieldMetadata<'_>) -> Self {
        let (tpe, flags) = if let Some(tpe) = tpe {
            match (tpe, metadata.field_type()) {
                // Those are types where we handle the conversion in diesel itself
                // and do not relay on libmysqlclient
                (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::Tiny, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::UnsignedTiny, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::Short, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::UnsignedShort, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::Long, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::UnsignedLong, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::LongLong, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::UnsignedLongLong, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::Float, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_DECIMAL)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_TINY)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_SHORT)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_LONG)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_FLOAT)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_DOUBLE)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_INT24)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL)
                | (MysqlType::Numeric, ffi::enum_field_types::MYSQL_TYPE_LONGLONG)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_JSON)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_ENUM)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_SET)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_BLOB)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_VAR_STRING)
                | (MysqlType::String, ffi::enum_field_types::MYSQL_TYPE_STRING)
                | (MysqlType::Blob, ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB)
                | (MysqlType::Blob, ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB)
                | (MysqlType::Blob, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB)
                | (MysqlType::Blob, ffi::enum_field_types::MYSQL_TYPE_BLOB)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_ENUM)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_SET)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_BLOB)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_VAR_STRING)
                | (MysqlType::Set, ffi::enum_field_types::MYSQL_TYPE_STRING)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_ENUM)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_SET)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_BLOB)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_VAR_STRING)
                | (MysqlType::Enum, ffi::enum_field_types::MYSQL_TYPE_STRING) => {
                    (metadata.field_type(), metadata.flags())
                }

                (tpe, _) => tpe.into(),
            }
        } else {
            (metadata.field_type(), metadata.flags())
        };
        Self::from_tpe_and_flags((tpe, flags))
    }

    fn from_tpe_and_flags((tpe, flags): (ffi::enum_field_types, Flags)) -> Self {
        // newer mysqlclient versions do not accept a zero sized buffer
        let len = known_buffer_size_for_ffi_type(tpe).unwrap_or(1);
        let mut bytes = vec![0; len];
        let length = bytes.len() as libc::c_ulong;
        let capacity = bytes.capacity();
        let ptr = NonNull::new(bytes.as_mut_ptr());
        mem::forget(bytes);

        Self {
            tpe,
            bytes: ptr,
            length,
            capacity,
            flags,
            is_null: super::raw::ffi_false(),
            is_truncated: Some(super::raw::ffi_false()),
        }
    }

    fn is_truncated(&self) -> bool {
        self.is_truncated.unwrap_or(super::raw::ffi_false()) != super::raw::ffi_false()
    }

    fn is_fixed_size_buffer(&self) -> bool {
        known_buffer_size_for_ffi_type(self.tpe).is_some()
    }

    pub(super) fn value(&'_ self) -> Option<MysqlValue<'_>> {
        if self.is_null() {
            None
        } else {
            let data = self.bytes?;
            let tpe = (self.tpe, self.flags).into();
            let slice = unsafe {
                // We know that this points to a slice and the pointer is not null at this
                // location
                // The length pointer is valid as long as none missuses `bind_for_truncated_data`
                // as this is the only location that updates the length field before the corresponding data are
                // written. At the time of writing this comment, the `BindData::bind_for_truncated_data`
                // function is only called by `Binds::populate_dynamic_buffers` which ensures the corresponding
                // invariant.
                std::slice::from_raw_parts(data.as_ptr(), self.length as usize)
            };
            Some(MysqlValue::new_internal(slice, tpe))
        }
    }

    pub(super) fn is_null(&self) -> bool {
        self.is_null != ffi::my_bool::default()
    }

    fn update_buffer_length(&mut self) {
        use std::cmp::min;

        let actual_bytes_in_buffer = min(self.capacity, self.length as usize);
        self.length = actual_bytes_in_buffer as libc::c_ulong;
    }

    // This function is marked as unsafe as it returns an owned value
    // containing a pointer with a lifetime coupled to self.
    // Callers need to ensure that the returned value cannot outlive `self`
    unsafe fn mysql_bind(&mut self) -> ffi::MYSQL_BIND {
        use std::ptr::addr_of_mut;

        let mut bind: MaybeUninit<ffi::MYSQL_BIND> = mem::MaybeUninit::zeroed();
        let ptr = bind.as_mut_ptr();

        addr_of_mut!((*ptr).buffer_type).write(self.tpe);
        addr_of_mut!((*ptr).buffer).write(
            self.bytes
                .map(|p| p.as_ptr())
                .unwrap_or(std::ptr::null_mut()) as *mut libc::c_void,
        );
        addr_of_mut!((*ptr).buffer_length).write(self.capacity as libc::c_ulong);
        addr_of_mut!((*ptr).length).write(&mut self.length);
        addr_of_mut!((*ptr).is_null).write(&mut self.is_null);
        addr_of_mut!((*ptr).is_unsigned)
            .write(self.flags.contains(Flags::UNSIGNED_FLAG) as ffi::my_bool);

        if let Some(ref mut is_truncated) = self.is_truncated {
            addr_of_mut!((*ptr).error).write(is_truncated);
        }

        // That's what the mysqlclient examples are doing
        bind.assume_init()
    }

    /// Resizes the byte buffer to fit the value of `self.length`, and returns
    /// a tuple of a bind pointing at the truncated data, and the offset to use
    /// in order to read the truncated data into it.
    ///
    /// This invalidates the bind previously returned by `mysql_bind`. Calling
    /// this function is unsafe unless the binds are immediately rebound.
    unsafe fn bind_for_truncated_data(&mut self) -> Option<(ffi::MYSQL_BIND, usize)> {
        if self.is_truncated() {
            if let Some(bytes) = self.bytes {
                let mut bytes = Vec::from_raw_parts(bytes.as_ptr(), self.capacity, self.capacity);
                self.bytes = None;

                let offset = self.capacity;
                let truncated_amount = self.length as usize - offset;

                debug_assert!(
                    truncated_amount > 0,
                    "output buffers were invalidated \
                     without calling `mysql_stmt_bind_result`"
                );

                // reserve space for any missing byte
                // we know the exact size here
                bytes.reserve(truncated_amount);
                self.capacity = bytes.capacity();
                self.bytes = NonNull::new(bytes.as_mut_ptr());
                mem::forget(bytes);

                let mut bind = self.mysql_bind();

                if let Some(ptr) = self.bytes {
                    // Using offset is safe here as we have a u8 array (where std::mem::size_of::<u8> == 1)
                    // and we have a buffer that has at least
                    bind.buffer = ptr.as_ptr().add(offset) as *mut libc::c_void;
                    bind.buffer_length = truncated_amount as libc::c_ulong;
                } else {
                    bind.buffer_length = 0;
                }
                Some((bind, offset))
            } else {
                // offset is zero here as we don't have a buffer yet
                // we know the requested length here so we can just request
                // the correct size
                let mut vec = vec![0_u8; self.length as usize];
                self.capacity = vec.capacity();
                self.bytes = NonNull::new(vec.as_mut_ptr());
                mem::forget(vec);

                let bind = self.mysql_bind();
                // As we did not have a buffer before
                // we couldn't have loaded any data yet, therefore
                // request everything
                Some((bind, 0))
            }
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

impl From<MysqlType> for (ffi::enum_field_types, Flags) {
    fn from(tpe: MysqlType) -> Self {
        use self::ffi::enum_field_types;
        let mut flags = Flags::empty();
        let tpe = match tpe {
            MysqlType::Tiny => enum_field_types::MYSQL_TYPE_TINY,
            MysqlType::Short => enum_field_types::MYSQL_TYPE_SHORT,
            MysqlType::Long => enum_field_types::MYSQL_TYPE_LONG,
            MysqlType::LongLong => enum_field_types::MYSQL_TYPE_LONGLONG,
            MysqlType::Float => enum_field_types::MYSQL_TYPE_FLOAT,
            MysqlType::Double => enum_field_types::MYSQL_TYPE_DOUBLE,
            MysqlType::Time => enum_field_types::MYSQL_TYPE_TIME,
            MysqlType::Date => enum_field_types::MYSQL_TYPE_DATE,
            MysqlType::DateTime => enum_field_types::MYSQL_TYPE_DATETIME,
            MysqlType::Timestamp => enum_field_types::MYSQL_TYPE_TIMESTAMP,
            MysqlType::String => enum_field_types::MYSQL_TYPE_STRING,
            MysqlType::Blob => enum_field_types::MYSQL_TYPE_BLOB,
            MysqlType::Numeric => enum_field_types::MYSQL_TYPE_NEWDECIMAL,
            MysqlType::Bit => enum_field_types::MYSQL_TYPE_BIT,
            MysqlType::UnsignedTiny => {
                flags = Flags::UNSIGNED_FLAG;
                enum_field_types::MYSQL_TYPE_TINY
            }
            MysqlType::UnsignedShort => {
                flags = Flags::UNSIGNED_FLAG;
                enum_field_types::MYSQL_TYPE_SHORT
            }
            MysqlType::UnsignedLong => {
                flags = Flags::UNSIGNED_FLAG;
                enum_field_types::MYSQL_TYPE_LONG
            }
            MysqlType::UnsignedLongLong => {
                flags = Flags::UNSIGNED_FLAG;
                enum_field_types::MYSQL_TYPE_LONGLONG
            }
            MysqlType::Set => {
                flags = Flags::SET_FLAG;
                enum_field_types::MYSQL_TYPE_STRING
            }
            MysqlType::Enum => {
                flags = Flags::ENUM_FLAG;
                enum_field_types::MYSQL_TYPE_STRING
            }
        };
        (tpe, flags)
    }
}

impl From<(ffi::enum_field_types, Flags)> for MysqlType {
    fn from((tpe, flags): (ffi::enum_field_types, Flags)) -> Self {
        use self::ffi::enum_field_types;

        let is_unsigned = flags.contains(Flags::UNSIGNED_FLAG);

        // https://docs.oracle.com/cd/E17952_01/mysql-8.0-en/c-api-data-structures.html
        // https://dev.mysql.com/doc/dev/mysql-server/8.0.12/binary__log__types_8h.html
        // https://dev.mysql.com/doc/internals/en/binary-protocol-value.html
        // https://mariadb.com/kb/en/packet_bindata/
        match tpe {
            enum_field_types::MYSQL_TYPE_TINY if is_unsigned => MysqlType::UnsignedTiny,
            enum_field_types::MYSQL_TYPE_YEAR | enum_field_types::MYSQL_TYPE_SHORT
                if is_unsigned =>
            {
                MysqlType::UnsignedShort
            }
            enum_field_types::MYSQL_TYPE_INT24 | enum_field_types::MYSQL_TYPE_LONG
                if is_unsigned =>
            {
                MysqlType::UnsignedLong
            }
            enum_field_types::MYSQL_TYPE_LONGLONG if is_unsigned => MysqlType::UnsignedLongLong,
            enum_field_types::MYSQL_TYPE_TINY => MysqlType::Tiny,
            enum_field_types::MYSQL_TYPE_SHORT => MysqlType::Short,
            enum_field_types::MYSQL_TYPE_INT24 | enum_field_types::MYSQL_TYPE_LONG => {
                MysqlType::Long
            }
            enum_field_types::MYSQL_TYPE_LONGLONG => MysqlType::LongLong,
            enum_field_types::MYSQL_TYPE_FLOAT => MysqlType::Float,
            enum_field_types::MYSQL_TYPE_DOUBLE => MysqlType::Double,
            enum_field_types::MYSQL_TYPE_DECIMAL | enum_field_types::MYSQL_TYPE_NEWDECIMAL => {
                MysqlType::Numeric
            }
            enum_field_types::MYSQL_TYPE_BIT => MysqlType::Bit,

            enum_field_types::MYSQL_TYPE_TIME => MysqlType::Time,
            enum_field_types::MYSQL_TYPE_DATE => MysqlType::Date,
            enum_field_types::MYSQL_TYPE_DATETIME => MysqlType::DateTime,
            enum_field_types::MYSQL_TYPE_TIMESTAMP => MysqlType::Timestamp,
            // Treat json as string because even mysql 8.0
            // throws errors sometimes if we use json for json
            enum_field_types::MYSQL_TYPE_JSON => MysqlType::String,

            // The documentation states that
            // MYSQL_TYPE_STRING is used for enums and sets
            // but experimentation has shown that
            // just any string like type works, so
            // better be safe here
            enum_field_types::MYSQL_TYPE_BLOB
            | enum_field_types::MYSQL_TYPE_TINY_BLOB
            | enum_field_types::MYSQL_TYPE_MEDIUM_BLOB
            | enum_field_types::MYSQL_TYPE_LONG_BLOB
            | enum_field_types::MYSQL_TYPE_VAR_STRING
            | enum_field_types::MYSQL_TYPE_STRING
                if flags.contains(Flags::ENUM_FLAG) =>
            {
                MysqlType::Enum
            }
            enum_field_types::MYSQL_TYPE_BLOB
            | enum_field_types::MYSQL_TYPE_TINY_BLOB
            | enum_field_types::MYSQL_TYPE_MEDIUM_BLOB
            | enum_field_types::MYSQL_TYPE_LONG_BLOB
            | enum_field_types::MYSQL_TYPE_VAR_STRING
            | enum_field_types::MYSQL_TYPE_STRING
                if flags.contains(Flags::SET_FLAG) =>
            {
                MysqlType::Set
            }

            // "blobs" may contain binary data
            // also "strings" can contain binary data
            // but all only if the binary flag is set
            // (see the check_all_the_types test case)
            enum_field_types::MYSQL_TYPE_BLOB
            | enum_field_types::MYSQL_TYPE_TINY_BLOB
            | enum_field_types::MYSQL_TYPE_MEDIUM_BLOB
            | enum_field_types::MYSQL_TYPE_LONG_BLOB
            | enum_field_types::MYSQL_TYPE_VAR_STRING
            | enum_field_types::MYSQL_TYPE_STRING
                if flags.contains(Flags::BINARY_FLAG) =>
            {
                MysqlType::Blob
            }

            // If the binary flag is not set consider everything as string
            enum_field_types::MYSQL_TYPE_BLOB
            | enum_field_types::MYSQL_TYPE_TINY_BLOB
            | enum_field_types::MYSQL_TYPE_MEDIUM_BLOB
            | enum_field_types::MYSQL_TYPE_LONG_BLOB
            | enum_field_types::MYSQL_TYPE_VAR_STRING
            | enum_field_types::MYSQL_TYPE_STRING => MysqlType::String,

            // unsigned seems to be set for year in any case
            enum_field_types::MYSQL_TYPE_YEAR => unreachable!(
                "The year type should have set the unsigned flag. If you ever \
                 see this error message, something has gone very wrong. Please \
                 open an issue at the diesel github repo in this case"
            ),
            // Null value
            enum_field_types::MYSQL_TYPE_NULL => unreachable!(
                "We ensure at the call side that we do not hit this type here. \
                 If you ever see this error, something has gone very wrong. \
                 Please open an issue at the diesel github repo in this case"
            ),
            // Those exist in libmysqlclient
            // but are just not supported
            //
            enum_field_types::MYSQL_TYPE_VARCHAR
            | enum_field_types::MYSQL_TYPE_ENUM
            | enum_field_types::MYSQL_TYPE_SET
            | enum_field_types::MYSQL_TYPE_GEOMETRY => {
                unimplemented!(
                    "Hit a type that should be unsupported in libmysqlclient. If \
                     you ever see this error, they probably have added support for \
                     one of those types. Please open an issue at the diesel github \
                     repo in this case."
                )
            }

            enum_field_types::MYSQL_TYPE_NEWDATE
            | enum_field_types::MYSQL_TYPE_TIME2
            | enum_field_types::MYSQL_TYPE_DATETIME2
            | enum_field_types::MYSQL_TYPE_TIMESTAMP2 => unreachable!(
                "The mysql documentation states that these types are \
                 only used on the server side, so if you see this error \
                 something has gone wrong. Please open an issue at \
                 the diesel github repo."
            ),
            // depending on the bindings version
            // there might be no unlisted field type
            #[allow(unreachable_patterns)]
            t => unreachable!(
                "Unsupported type encountered: {t:?}. \
                 If you ever see this error, something has gone wrong. \
                 Please open an issue at the diesel github \
                 repo in this case."
            ),
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
        | t::MYSQL_TYPE_TIMESTAMP => Some(size_of::<MysqlTime>()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::statement_cache::MaybeCached;
    use crate::deserialize::FromSql;
    use crate::mysql::connection::stmt::Statement;
    use crate::prelude::*;
    use crate::sql_types::*;
    #[cfg(feature = "numeric")]
    use std::str::FromStr;

    fn to_value<ST, T>(
        bind: &BindData,
    ) -> Result<T, Box<(dyn std::error::Error + Send + Sync + 'static)>>
    where
        T: FromSql<ST, crate::mysql::Mysql> + std::fmt::Debug,
    {
        let meta = (bind.tpe, bind.flags).into();
        dbg!(meta);

        let value = bind.value().expect("Is not null");
        let value = MysqlValue::new_internal(value.as_bytes(), meta);

        dbg!(T::from_sql(value))
    }

    #[cfg(feature = "extras")]
    #[test]
    fn check_all_the_types() {
        let conn = &mut crate::test_helpers::connection();

        crate::sql_query("DROP TABLE IF EXISTS all_mysql_types CASCADE")
            .execute(conn)
            .unwrap();
        crate::sql_query(
            "CREATE TABLE all_mysql_types (
                    tiny_int TINYINT NOT NULL,
                    small_int SMALLINT NOT NULL,
                    medium_int MEDIUMINT NOT NULL,
                    int_col INTEGER NOT NULL,
                    big_int BIGINT NOT NULL,
                    unsigned_int INTEGER UNSIGNED NOT NULL,
                    zero_fill_int INTEGER ZEROFILL NOT NULL,
                    numeric_col NUMERIC(20,5) NOT NULL,
                    decimal_col DECIMAL(20,5) NOT NULL,
                    float_col FLOAT NOT NULL,
                    double_col DOUBLE NOT NULL,
                    bit_col BIT(8) NOT NULL,
                    date_col DATE NOT NULL,
                    date_time DATETIME NOT NULL,
                    timestamp_col TIMESTAMP NOT NULL,
                    time_col TIME NOT NULL,
                    year_col YEAR NOT NULL,
                    char_col CHAR(30) NOT NULL,
                    varchar_col VARCHAR(30) NOT NULL,
                    binary_col BINARY(30) NOT NULL,
                    varbinary_col VARBINARY(30) NOT NULL,
                    blob_col BLOB NOT NULL,
                    text_col TEXT NOT NULL,
                    enum_col ENUM('red', 'green', 'blue') NOT NULL,
                    set_col SET('one', 'two') NOT NULL,
                    geom GEOMETRY NOT NULL,
                    point_col POINT NOT NULL,
                    linestring_col LINESTRING NOT NULL,
                    polygon_col POLYGON NOT NULL,
                    multipoint_col MULTIPOINT NOT NULL,
                    multilinestring_col MULTILINESTRING NOT NULL,
                    multipolygon_col MULTIPOLYGON NOT NULL,
                    geometry_collection GEOMETRYCOLLECTION NOT NULL,
                    json_col JSON NOT NULL
            )",
        )
        .execute(conn)
        .unwrap();
        crate::sql_query(
                "INSERT INTO all_mysql_types VALUES (
                    0, -- tiny_int
                    1, -- small_int
                    2, -- medium_int
                    3, -- int_col
                    -5, -- big_int
                    42, -- unsigned_int
                    1, -- zero_fill_int
                    -999.999, -- numeric_col,
                    3.14, -- decimal_col,
                    1.23, -- float_col
                    4.5678, -- double_col
                    b'10101010', -- bit_col
                    '1000-01-01', -- date_col
                    '9999-12-31 12:34:45.012345', -- date_time
                    '2020-01-01 10:10:10', -- timestamp_col
                    '23:01:01', -- time_col
                    2020, -- year_col
                    'abc', -- char_col
                    'foo', -- varchar_col
                    'a ', -- binary_col
                    'a ', -- varbinary_col
                    'binary', -- blob_col
                    'some text whatever', -- text_col
                    'red', -- enum_col
                    'one', -- set_col
                    ST_GeomFromText('POINT(1 1)'), -- geom
                    ST_PointFromText('POINT(1 1)'), -- point_col
                    ST_LineStringFromText('LINESTRING(0 0,1 1,2 2)'), -- linestring_col
                    ST_PolygonFromText('POLYGON((0 0,10 0,10 10,0 10,0 0),(5 5,7 5,7 7,5 7, 5 5))'), -- polygon_col
                    ST_MultiPointFromText('MULTIPOINT(0 0,10 10,10 20,20 20)'), -- multipoint_col
                    ST_MultiLineStringFromText('MULTILINESTRING((10 48,10 21,10 0),(16 0,16 23,16 48))'), -- multilinestring_col
                    ST_MultiPolygonFromText('MULTIPOLYGON(((28 26,28 0,84 0,84 42,28 26),(52 18,66 23,73 9,48 6,52 18)),((59 18,67 18,67 13,59 13,59 18)))'), -- multipolygon_col
                    ST_GeomCollFromText('GEOMETRYCOLLECTION(POINT(1 1),LINESTRING(0 0,1 1,2 2,3 3,4 4))'), -- geometry_collection
                    '{\"key1\": \"value1\", \"key2\": \"value2\"}' -- json_col
)",
            ).execute(conn)
            .unwrap();

        let stmt = crate::mysql::connection::prepared_query(
            &crate::sql_query(
                "SELECT
                    tiny_int, small_int, medium_int, int_col,
                    big_int, unsigned_int, zero_fill_int,
                    numeric_col, decimal_col, float_col, double_col, bit_col,
                    date_col, date_time, timestamp_col, time_col, year_col,
                    char_col, varchar_col, binary_col, varbinary_col, blob_col,
                    text_col, enum_col, set_col, ST_AsText(geom), ST_AsText(point_col), ST_AsText(linestring_col),
                    ST_AsText(polygon_col), ST_AsText(multipoint_col), ST_AsText(multilinestring_col),
                    ST_AsText(multipolygon_col), ST_AsText(geometry_collection), json_col
                 FROM all_mysql_types",
            ),
            &mut conn.statement_cache,
            &mut conn.raw_connection,
            &mut conn.instrumentation,
        ).unwrap();

        let metadata = stmt.metadata().unwrap();
        let mut output_binds =
            OutputBinds::from_output_types(&vec![None; metadata.fields().len()], &metadata);
        let stmt = stmt.execute_statement(&mut output_binds).unwrap();
        stmt.populate_row_buffers(&mut output_binds).unwrap();

        let results: Vec<(BindData, &_)> = output_binds
            .0
            .data
            .into_iter()
            .zip(metadata.fields())
            .collect::<Vec<_>>();

        let tiny_int_col = &results[0].0;
        assert_eq!(tiny_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_TINY);
        assert!(tiny_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!tiny_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<TinyInt, i8>(tiny_int_col), Ok(0)));

        let small_int_col = &results[1].0;
        assert_eq!(small_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_SHORT);
        assert!(small_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!small_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<SmallInt, i16>(small_int_col), Ok(1)));

        let medium_int_col = &results[2].0;
        assert_eq!(medium_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_INT24);
        assert!(medium_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!medium_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<Integer, i32>(medium_int_col), Ok(2)));

        let int_col = &results[3].0;
        assert_eq!(int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG);
        assert!(int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<Integer, i32>(int_col), Ok(3)));

        let big_int_col = &results[4].0;
        assert_eq!(big_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONGLONG);
        assert!(big_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(!big_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<TinyInt, i8>(big_int_col), Ok(-5)));

        let unsigned_int_col = &results[5].0;
        assert_eq!(unsigned_int_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG);
        assert!(unsigned_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(unsigned_int_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(
            to_value::<Unsigned<Integer>, u32>(unsigned_int_col),
            Ok(42)
        ));

        let zero_fill_int_col = &results[6].0;
        assert_eq!(
            zero_fill_int_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG
        );
        assert!(zero_fill_int_col.flags.contains(Flags::NUM_FLAG));
        assert!(zero_fill_int_col.flags.contains(Flags::ZEROFILL_FLAG));
        assert!(matches!(to_value::<Integer, i32>(zero_fill_int_col), Ok(1)));

        let numeric_col = &results[7].0;
        assert_eq!(
            numeric_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL
        );
        assert!(numeric_col.flags.contains(Flags::NUM_FLAG));
        assert!(!numeric_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert_eq!(
            to_value::<Numeric, bigdecimal::BigDecimal>(numeric_col).unwrap(),
            bigdecimal::BigDecimal::from_str("-999.99900").unwrap()
        );

        let decimal_col = &results[8].0;
        assert_eq!(
            decimal_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_NEWDECIMAL
        );
        assert!(decimal_col.flags.contains(Flags::NUM_FLAG));
        assert!(!decimal_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert_eq!(
            to_value::<Numeric, bigdecimal::BigDecimal>(decimal_col).unwrap(),
            bigdecimal::BigDecimal::from_str("3.14000").unwrap()
        );

        let float_col = &results[9].0;
        assert_eq!(float_col.tpe, ffi::enum_field_types::MYSQL_TYPE_FLOAT);
        assert!(float_col.flags.contains(Flags::NUM_FLAG));
        assert!(!float_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert_eq!(to_value::<Float, f32>(float_col).unwrap(), 1.23);

        let double_col = &results[10].0;
        assert_eq!(double_col.tpe, ffi::enum_field_types::MYSQL_TYPE_DOUBLE);
        assert!(double_col.flags.contains(Flags::NUM_FLAG));
        assert!(!double_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert_eq!(to_value::<Double, f64>(double_col).unwrap(), 4.5678);

        let bit_col = &results[11].0;
        assert_eq!(bit_col.tpe, ffi::enum_field_types::MYSQL_TYPE_BIT);
        assert!(!bit_col.flags.contains(Flags::NUM_FLAG));
        assert!(bit_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(!bit_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Blob, Vec<u8>>(bit_col).unwrap(), vec![170]);

        let date_col = &results[12].0;
        assert_eq!(date_col.tpe, ffi::enum_field_types::MYSQL_TYPE_DATE);
        assert!(!date_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Date, chrono::NaiveDate>(date_col).unwrap(),
            chrono::NaiveDate::from_ymd_opt(1000, 1, 1).unwrap(),
        );

        let date_time_col = &results[13].0;
        assert_eq!(
            date_time_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_DATETIME
        );
        assert!(!date_time_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Datetime, chrono::NaiveDateTime>(date_time_col).unwrap(),
            chrono::NaiveDateTime::parse_from_str("9999-12-31 12:34:45", "%Y-%m-%d %H:%M:%S")
                .unwrap()
        );

        let timestamp_col = &results[14].0;
        assert_eq!(
            timestamp_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_TIMESTAMP
        );
        assert!(!timestamp_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Datetime, chrono::NaiveDateTime>(timestamp_col).unwrap(),
            chrono::NaiveDateTime::parse_from_str("2020-01-01 10:10:10", "%Y-%m-%d %H:%M:%S")
                .unwrap()
        );

        let time_col = &results[15].0;
        assert_eq!(time_col.tpe, ffi::enum_field_types::MYSQL_TYPE_TIME);
        assert!(!time_col.flags.contains(Flags::NUM_FLAG));
        assert_eq!(
            to_value::<Time, chrono::NaiveTime>(time_col).unwrap(),
            chrono::NaiveTime::from_hms_opt(23, 01, 01).unwrap()
        );

        let year_col = &results[16].0;
        assert_eq!(year_col.tpe, ffi::enum_field_types::MYSQL_TYPE_YEAR);
        assert!(year_col.flags.contains(Flags::NUM_FLAG));
        assert!(year_col.flags.contains(Flags::UNSIGNED_FLAG));
        assert!(matches!(to_value::<SmallInt, i16>(year_col), Ok(2020)));

        let char_col = &results[17].0;
        assert_eq!(char_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!char_col.flags.contains(Flags::NUM_FLAG));
        assert!(!char_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!char_col.flags.contains(Flags::SET_FLAG));
        assert!(!char_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!char_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(char_col).unwrap(), "abc");

        let varchar_col = &results[18].0;
        assert_eq!(
            varchar_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING
        );
        assert!(!varchar_col.flags.contains(Flags::NUM_FLAG));
        assert!(!varchar_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!varchar_col.flags.contains(Flags::SET_FLAG));
        assert!(!varchar_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!varchar_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(varchar_col).unwrap(), "foo");

        let binary_col = &results[19].0;
        assert_eq!(binary_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!binary_col.flags.contains(Flags::NUM_FLAG));
        assert!(!binary_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!binary_col.flags.contains(Flags::SET_FLAG));
        assert!(!binary_col.flags.contains(Flags::ENUM_FLAG));
        assert!(binary_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Blob, Vec<u8>>(binary_col).unwrap(),
            b"a \0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
        );

        let varbinary_col = &results[20].0;
        assert_eq!(
            varbinary_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING
        );
        assert!(!varbinary_col.flags.contains(Flags::NUM_FLAG));
        assert!(!varbinary_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!varbinary_col.flags.contains(Flags::SET_FLAG));
        assert!(!varbinary_col.flags.contains(Flags::ENUM_FLAG));
        assert!(varbinary_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Blob, Vec<u8>>(varbinary_col).unwrap(), b"a ");

        let blob_col = &results[21].0;
        assert_eq!(blob_col.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!blob_col.flags.contains(Flags::NUM_FLAG));
        assert!(blob_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!blob_col.flags.contains(Flags::SET_FLAG));
        assert!(!blob_col.flags.contains(Flags::ENUM_FLAG));
        assert!(blob_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Blob, Vec<u8>>(blob_col).unwrap(), b"binary");

        let text_col = &results[22].0;
        assert_eq!(text_col.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!text_col.flags.contains(Flags::NUM_FLAG));
        assert!(text_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!text_col.flags.contains(Flags::SET_FLAG));
        assert!(!text_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!text_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(text_col).unwrap(),
            "some text whatever"
        );

        let enum_col = &results[23].0;
        assert_eq!(enum_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!enum_col.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col.flags.contains(Flags::SET_FLAG));
        assert!(enum_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(enum_col).unwrap(), "red");

        let set_col = &results[24].0;
        assert_eq!(set_col.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col.flags.contains(Flags::SET_FLAG));
        assert!(!set_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(set_col).unwrap(), "one");

        let geom = &results[25].0;
        assert_eq!(geom.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!geom.flags.contains(Flags::NUM_FLAG));
        assert!(!geom.flags.contains(Flags::BLOB_FLAG));
        assert!(!geom.flags.contains(Flags::SET_FLAG));
        assert!(!geom.flags.contains(Flags::ENUM_FLAG));
        assert!(!geom.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(geom).unwrap(), "POINT(1 1)");

        let point_col = &results[26].0;
        assert_eq!(point_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!point_col.flags.contains(Flags::NUM_FLAG));
        assert!(!point_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!point_col.flags.contains(Flags::SET_FLAG));
        assert!(!point_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!point_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(point_col).unwrap(), "POINT(1 1)");

        let linestring_col = &results[27].0;
        assert_eq!(
            linestring_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!linestring_col.flags.contains(Flags::NUM_FLAG));
        assert!(!linestring_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!linestring_col.flags.contains(Flags::SET_FLAG));
        assert!(!linestring_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!linestring_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(linestring_col).unwrap(),
            "LINESTRING(0 0,1 1,2 2)"
        );

        let polygon_col = &results[28].0;
        assert_eq!(polygon_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!polygon_col.flags.contains(Flags::NUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!polygon_col.flags.contains(Flags::SET_FLAG));
        assert!(!polygon_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(polygon_col).unwrap(),
            "POLYGON((0 0,10 0,10 10,0 10,0 0),(5 5,7 5,7 7,5 7,5 5))"
        );

        let multipoint_col = &results[29].0;
        assert_eq!(
            multipoint_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!multipoint_col.flags.contains(Flags::NUM_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::SET_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!multipoint_col.flags.contains(Flags::BINARY_FLAG));
        // older mysql and mariadb versions get back another encoding here
        // we test for both as there seems to be no clear pattern when one or
        // the other is returned
        let multipoint_res = to_value::<Text, String>(multipoint_col).unwrap();
        assert!(
            multipoint_res == "MULTIPOINT((0 0),(10 10),(10 20),(20 20))"
                || multipoint_res == "MULTIPOINT(0 0,10 10,10 20,20 20)"
        );

        let multilinestring_col = &results[30].0;
        assert_eq!(
            multilinestring_col.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!multilinestring_col.flags.contains(Flags::NUM_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::SET_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!multilinestring_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(multilinestring_col).unwrap(),
            "MULTILINESTRING((10 48,10 21,10 0),(16 0,16 23,16 48))"
        );

        let polygon_col = &results[31].0;
        assert_eq!(polygon_col.tpe, ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB);
        assert!(!polygon_col.flags.contains(Flags::NUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!polygon_col.flags.contains(Flags::SET_FLAG));
        assert!(!polygon_col.flags.contains(Flags::ENUM_FLAG));
        assert!(!polygon_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
                to_value::<Text, String>(polygon_col).unwrap(),
                "MULTIPOLYGON(((28 26,28 0,84 0,84 42,28 26),(52 18,66 23,73 9,48 6,52 18)),((59 18,67 18,67 13,59 13,59 18)))"
            );

        let geometry_collection = &results[32].0;
        assert_eq!(
            geometry_collection.tpe,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB
        );
        assert!(!geometry_collection.flags.contains(Flags::NUM_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::BLOB_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::SET_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::ENUM_FLAG));
        assert!(!geometry_collection.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(geometry_collection).unwrap(),
            "GEOMETRYCOLLECTION(POINT(1 1),LINESTRING(0 0,1 1,2 2,3 3,4 4))"
        );

        let json_col = &results[33].0;
        // mariadb >= 10.2 and mysql >=8.0 are supporting a json type
        // from those mariadb >= 10.3 and mysql >= 8.0 are reporting
        // json here, so we assert that we get back json
        // mariadb 10.5 returns again blob
        assert!(
            json_col.tpe == ffi::enum_field_types::MYSQL_TYPE_JSON
                || json_col.tpe == ffi::enum_field_types::MYSQL_TYPE_BLOB
        );
        assert!(!json_col.flags.contains(Flags::NUM_FLAG));
        assert!(json_col.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col.flags.contains(Flags::SET_FLAG));
        assert!(!json_col.flags.contains(Flags::ENUM_FLAG));
        assert!(json_col.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(json_col).unwrap(),
            "{\"key1\": \"value1\", \"key2\": \"value2\"}"
        );
    }

    fn query_single_table(
        query: &'static str,
        conn: &MysqlConnection,
        bind_tpe: impl Into<(ffi::enum_field_types, Flags)>,
    ) -> BindData {
        let stmt: Statement = conn.raw_connection.prepare(query).unwrap();
        let stmt = MaybeCached::CannotCache(stmt);

        let bind = BindData::from_tpe_and_flags(bind_tpe.into());

        let mut binds = OutputBinds(Binds { data: vec![bind] });

        let stmt = stmt.execute_statement(&mut binds).unwrap();
        stmt.populate_row_buffers(&mut binds).unwrap();

        binds.0.data.remove(0)
    }

    fn input_bind(
        query: &'static str,
        conn: &MysqlConnection,
        id: i32,
        (mut field, tpe): (Vec<u8>, impl Into<(ffi::enum_field_types, Flags)>),
    ) {
        let mut stmt = conn.raw_connection.prepare(query).unwrap();
        let length = field.len() as _;
        let (tpe, flags) = tpe.into();
        let capacity = field.capacity();
        let ptr = NonNull::new(field.as_mut_ptr());
        mem::forget(field);

        let field_bind = BindData {
            tpe,
            bytes: ptr,
            capacity,
            length,
            flags,
            is_null: ffi::FALSE,
            is_truncated: None,
        };

        let mut bytes = id.to_be_bytes().to_vec();
        let length = bytes.len() as _;
        let capacity = bytes.capacity();
        let ptr = NonNull::new(bytes.as_mut_ptr());
        mem::forget(bytes);

        let id_bind = BindData {
            tpe: ffi::enum_field_types::MYSQL_TYPE_LONG,
            bytes: ptr,
            capacity,
            length,
            flags: Flags::empty(),
            is_null: ffi::FALSE,
            is_truncated: None,
        };

        let binds = PreparedStatementBinds(Binds {
            data: vec![id_bind, field_bind],
        });
        stmt.input_bind(binds).unwrap();
        stmt.did_an_error_occur().unwrap();
        let stmt = MaybeCached::CannotCache(stmt);
        unsafe {
            stmt.execute().unwrap();
        }
    }

    #[test]
    fn check_json_bind() {
        table! {
            json_test {
                id -> Integer,
                json_field -> Text,
            }
        }

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query("DROP TABLE IF EXISTS json_test CASCADE")
            .execute(conn)
            .unwrap();

        crate::sql_query(
            "CREATE TABLE json_test(id INTEGER PRIMARY KEY, json_field JSON NOT NULL)",
        )
        .execute(conn)
        .unwrap();

        crate::sql_query("INSERT INTO json_test(id, json_field) VALUES (1, '{\"key1\": \"value1\", \"key2\": \"value2\"}')").execute(conn).unwrap();

        let json_col_as_json = query_single_table(
            "SELECT json_field FROM json_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
        );

        assert_eq!(json_col_as_json.tpe, ffi::enum_field_types::MYSQL_TYPE_JSON);
        assert!(!json_col_as_json.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_json).unwrap(),
            "{\"key1\": \"value1\", \"key2\": \"value2\"}"
        );

        let json_col_as_text = query_single_table(
            "SELECT json_field FROM json_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(json_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!json_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_text).unwrap(),
            "{\"key1\": \"value1\", \"key2\": \"value2\"}"
        );
        assert_eq!(
            json_col_as_json.value().unwrap().as_bytes(),
            json_col_as_text.value().unwrap().as_bytes()
        );

        crate::sql_query("DELETE FROM json_test")
            .execute(conn)
            .unwrap();

        input_bind(
            "INSERT INTO json_test(id, json_field) VALUES (?, ?)",
            conn,
            41,
            (
                b"{\"abc\": 42}".to_vec(),
                MysqlType::String,
                //                (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
            ),
        );

        let json_col_as_json = query_single_table(
            "SELECT json_field FROM json_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
        );

        assert_eq!(json_col_as_json.tpe, ffi::enum_field_types::MYSQL_TYPE_JSON);
        assert!(!json_col_as_json.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_json).unwrap(),
            "{\"abc\": 42}"
        );

        let json_col_as_text = query_single_table(
            "SELECT json_field FROM json_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(json_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!json_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_text).unwrap(),
            "{\"abc\": 42}"
        );
        assert_eq!(
            json_col_as_json.value().unwrap().as_bytes(),
            json_col_as_text.value().unwrap().as_bytes()
        );

        crate::sql_query("DELETE FROM json_test")
            .execute(conn)
            .unwrap();

        input_bind(
            "INSERT INTO json_test(id, json_field) VALUES (?, ?)",
            conn,
            41,
            (b"{\"abca\": 42}".to_vec(), MysqlType::String),
        );

        let json_col_as_json = query_single_table(
            "SELECT json_field FROM json_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_JSON, Flags::empty()),
        );

        assert_eq!(json_col_as_json.tpe, ffi::enum_field_types::MYSQL_TYPE_JSON);
        assert!(!json_col_as_json.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_json.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_json).unwrap(),
            "{\"abca\": 42}"
        );

        let json_col_as_text = query_single_table(
            "SELECT json_field FROM json_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(json_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!json_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!json_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&json_col_as_text).unwrap(),
            "{\"abca\": 42}"
        );
        assert_eq!(
            json_col_as_json.value().unwrap().as_bytes(),
            json_col_as_text.value().unwrap().as_bytes()
        );
    }

    #[test]
    fn check_enum_bind() {
        let conn = &mut crate::test_helpers::connection();

        crate::sql_query("DROP TABLE IF EXISTS enum_test CASCADE")
            .execute(conn)
            .unwrap();

        crate::sql_query("CREATE TABLE enum_test(id INTEGER PRIMARY KEY, enum_field ENUM('red', 'green', 'blue') NOT NULL)").execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO enum_test(id, enum_field) VALUES (1, 'green')")
            .execute(conn)
            .unwrap();

        let enum_col_as_enum: BindData =
            query_single_table("SELECT enum_field FROM enum_test", conn, MysqlType::Enum);

        assert_eq!(
            enum_col_as_enum.tpe,
            ffi::enum_field_types::MYSQL_TYPE_STRING
        );
        assert!(!enum_col_as_enum.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_enum.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&enum_col_as_enum).unwrap(),
            "green"
        );

        for tpe in &[
            ffi::enum_field_types::MYSQL_TYPE_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING,
            ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB,
        ] {
            let enum_col_as_text = query_single_table(
                "SELECT enum_field FROM enum_test",
                conn,
                (*tpe, Flags::ENUM_FLAG),
            );

            assert_eq!(enum_col_as_text.tpe, *tpe);
            assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
            assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
            assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
            assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
            assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
            assert_eq!(
                to_value::<Text, String>(&enum_col_as_text).unwrap(),
                "green"
            );
            assert_eq!(
                enum_col_as_enum.value().unwrap().as_bytes(),
                enum_col_as_text.value().unwrap().as_bytes()
            );
        }

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(
            to_value::<Text, String>(&enum_col_as_text).unwrap(),
            "green"
        );
        assert_eq!(
            enum_col_as_enum.value().unwrap().as_bytes(),
            enum_col_as_text.value().unwrap().as_bytes()
        );

        crate::sql_query("DELETE FROM enum_test")
            .execute(conn)
            .unwrap();

        input_bind(
            "INSERT INTO enum_test(id, enum_field) VALUES (?, ?)",
            conn,
            41,
            (b"blue".to_vec(), MysqlType::Enum),
        );

        let enum_col_as_enum =
            query_single_table("SELECT enum_field FROM enum_test", conn, MysqlType::Enum);

        assert_eq!(
            enum_col_as_enum.tpe,
            ffi::enum_field_types::MYSQL_TYPE_STRING
        );
        assert!(!enum_col_as_enum.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_enum.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_enum).unwrap(), "blue");

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_text).unwrap(), "blue");
        assert_eq!(
            enum_col_as_enum.value().unwrap().as_bytes(),
            enum_col_as_text.value().unwrap().as_bytes()
        );

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_text).unwrap(), "blue");
        assert_eq!(
            enum_col_as_enum.value().unwrap().as_bytes(),
            enum_col_as_text.value().unwrap().as_bytes()
        );

        crate::sql_query("DELETE FROM enum_test")
            .execute(conn)
            .unwrap();

        input_bind(
            "INSERT INTO enum_test(id, enum_field) VALUES (?, ?)",
            conn,
            41,
            (
                b"red".to_vec(),
                (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
            ),
        );

        let enum_col_as_enum =
            query_single_table("SELECT enum_field FROM enum_test", conn, MysqlType::Enum);

        assert_eq!(
            enum_col_as_enum.tpe,
            ffi::enum_field_types::MYSQL_TYPE_STRING
        );
        assert!(!enum_col_as_enum.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_enum.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_enum.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_enum).unwrap(), "red");

        let enum_col_as_text = query_single_table(
            "SELECT enum_field FROM enum_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::ENUM_FLAG),
        );

        assert_eq!(enum_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!enum_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(enum_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!enum_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&enum_col_as_text).unwrap(), "red");
        assert_eq!(
            enum_col_as_enum.value().unwrap().as_bytes(),
            enum_col_as_text.value().unwrap().as_bytes()
        );
    }

    #[test]
    fn check_set_bind() {
        let conn = &mut crate::test_helpers::connection();

        crate::sql_query("DROP TABLE IF EXISTS set_test CASCADE")
            .execute(conn)
            .unwrap();

        crate::sql_query("CREATE TABLE set_test(id INTEGER PRIMARY KEY, set_field SET('red', 'green', 'blue') NOT NULL)").execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO set_test(id, set_field) VALUES (1, 'green')")
            .execute(conn)
            .unwrap();

        let set_col_as_set: BindData =
            query_single_table("SELECT set_field FROM set_test", conn, MysqlType::Set);

        assert_eq!(set_col_as_set.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col_as_set.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_set.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_set).unwrap(), "green");

        for tpe in &[
            ffi::enum_field_types::MYSQL_TYPE_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_VAR_STRING,
            ffi::enum_field_types::MYSQL_TYPE_TINY_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_MEDIUM_BLOB,
            ffi::enum_field_types::MYSQL_TYPE_LONG_BLOB,
        ] {
            let set_col_as_text = query_single_table(
                "SELECT set_field FROM set_test",
                conn,
                (*tpe, Flags::SET_FLAG),
            );

            assert_eq!(set_col_as_text.tpe, *tpe);
            assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
            assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
            assert!(set_col_as_text.flags.contains(Flags::SET_FLAG));
            assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
            assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
            assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "green");
            assert_eq!(
                set_col_as_set.value().unwrap().as_bytes(),
                set_col_as_text.value().unwrap().as_bytes()
            );
        }
        let set_col_as_text = query_single_table(
            "SELECT set_field FROM set_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::empty()),
        );

        assert_eq!(set_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "green");
        assert_eq!(
            set_col_as_set.value().unwrap().as_bytes(),
            set_col_as_text.value().unwrap().as_bytes()
        );

        crate::sql_query("DELETE FROM set_test")
            .execute(conn)
            .unwrap();

        input_bind(
            "INSERT INTO set_test(id, set_field) VALUES (?, ?)",
            conn,
            41,
            (b"blue".to_vec(), MysqlType::Set),
        );

        let set_col_as_set =
            query_single_table("SELECT set_field FROM set_test", conn, MysqlType::Set);

        assert_eq!(set_col_as_set.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col_as_set.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_set.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_set).unwrap(), "blue");

        let set_col_as_text = query_single_table(
            "SELECT set_field FROM set_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::SET_FLAG),
        );

        assert_eq!(set_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "blue");
        assert_eq!(
            set_col_as_set.value().unwrap().as_bytes(),
            set_col_as_text.value().unwrap().as_bytes()
        );

        crate::sql_query("DELETE FROM set_test")
            .execute(conn)
            .unwrap();

        input_bind(
            "INSERT INTO set_test(id, set_field) VALUES (?, ?)",
            conn,
            41,
            (b"red".to_vec(), MysqlType::String),
        );

        let set_col_as_set =
            query_single_table("SELECT set_field FROM set_test", conn, MysqlType::Set);

        assert_eq!(set_col_as_set.tpe, ffi::enum_field_types::MYSQL_TYPE_STRING);
        assert!(!set_col_as_set.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_set.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_set.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_set).unwrap(), "red");

        let set_col_as_text = query_single_table(
            "SELECT set_field FROM set_test",
            conn,
            (ffi::enum_field_types::MYSQL_TYPE_BLOB, Flags::SET_FLAG),
        );

        assert_eq!(set_col_as_text.tpe, ffi::enum_field_types::MYSQL_TYPE_BLOB);
        assert!(!set_col_as_text.flags.contains(Flags::NUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BLOB_FLAG));
        assert!(set_col_as_text.flags.contains(Flags::SET_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::ENUM_FLAG));
        assert!(!set_col_as_text.flags.contains(Flags::BINARY_FLAG));
        assert_eq!(to_value::<Text, String>(&set_col_as_text).unwrap(), "red");
        assert_eq!(
            set_col_as_set.value().unwrap().as_bytes(),
            set_col_as_text.value().unwrap().as_bytes()
        );
    }
}
