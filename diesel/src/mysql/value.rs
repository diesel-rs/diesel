extern crate mysqlclient_sys as ffi;

use byteorder::*;
use std::cell::Cell;
use std::error::Error;
use std::mem;

/// Represents a value returned by a MySQL server.
///
/// MySQL is much less strict about keeping types the same than PostgreSQL. For
/// example, any numeric literal will be interpreted as 64-bit, and there is no
/// way to shrink that on the server side.
///
/// Because of this, numeric deserialization on MySQL always needs to check the
/// type code of what we got back, and do more permissive coercion than we would
/// do for a stricter database.
#[derive(Debug)]
pub struct MysqlValue {
    type_code: Cell<ffi::enum_field_types>,
    /// Even though the data is definitely stored in a valid &[u8], we cannot
    /// have a lifetime appear on this struct. While it'd be great to have this
    /// struct just be unsized, it's virtually impossible to construct an
    /// unsized type in Rust today without boxing.
    data: Cell<*const [u8]>,
}

impl Default for MysqlValue {
    fn default() -> Self {
        Self {
            type_code: Cell::new(ffi::enum_field_types::MYSQL_TYPE_NULL),
            // Can't use ptr::null() here, it requires T: Sized
            data: Cell::new(unsafe { mem::zeroed() }),
        }
    }
}

impl MysqlValue {
    pub(crate) fn update(&self, bytes: &[u8], type_code: ffi::enum_field_types) -> &Self {
        self.data.set(bytes as *const _);
        self.type_code.set(type_code);
        self
    }

    /// Checks that the type code is valid, and interprets the data as a
    /// `MYSQL_TIME` pointer
    pub fn time_value(&self) -> Result<ffi::MYSQL_TIME, Box<Error + Send + Sync>> {
        use self::ffi::enum_field_types as t;

        match self.type_code() {
            t::MYSQL_TYPE_TIME
            | t::MYSQL_TYPE_DATE
            | t::MYSQL_TYPE_DATETIME
            | t::MYSQL_TYPE_TIMESTAMP => {} // valid type code
            _ => return Err(self.invalid_type_code("timestamp")),
        }
        let bytes_ptr = self.bytes()?.as_ptr() as *const ffi::MYSQL_TIME;
        unsafe { Ok(*bytes_ptr) }
    }

    /// Returns the numeric representation of this value, based on the type code.
    /// Returns an error if the type code is not numeric.
    pub fn numeric_value(&self) -> Result<NumericRepresentation, Box<Error + Send + Sync>> {
        use self::ffi::enum_field_types as t;
        use self::NumericRepresentation::*;

        let mut bytes = self.bytes()?;
        Ok(match self.type_code() {
            t::MYSQL_TYPE_TINY => Tiny(bytes[0] as i8),
            t::MYSQL_TYPE_SHORT => Small(bytes.read_i16::<NativeEndian>()?),
            t::MYSQL_TYPE_INT24 | t::MYSQL_TYPE_LONG => Medium(bytes.read_i32::<NativeEndian>()?),
            t::MYSQL_TYPE_LONGLONG => Big(bytes.read_i64::<NativeEndian>()?),
            t::MYSQL_TYPE_FLOAT => Float(bytes.read_f32::<NativeEndian>()?),
            t::MYSQL_TYPE_DOUBLE => Double(bytes.read_f64::<NativeEndian>()?),
            t::MYSQL_TYPE_DECIMAL | t::MYSQL_TYPE_NEWDECIMAL => Decimal(bytes),
            _ => return Err(self.invalid_type_code("number")),
        })
    }

    /// Returns the raw bytes received from the server
    pub fn bytes(&self) -> Result<&[u8], Box<Error + Send + Sync>> {
        let bytes = unsafe { self.data.get().as_ref() };
        Ok(not_none!(bytes))
    }

    fn type_code(&self) -> ffi::enum_field_types {
        self.type_code.get()
    }

    fn invalid_type_code(&self, expected: &str) -> Box<Error + Send + Sync> {
        format!(
            "Invalid representation received for {}: {:?}",
            expected,
            self.type_code()
        ).into()
    }
}

/// Represents all possible forms MySQL transmits integers
#[derive(Debug, Clone, Copy)]
pub enum NumericRepresentation<'a> {
    /// Correponds to `MYSQL_TYPE_TINY`
    Tiny(i8),
    /// Correponds to `MYSQL_TYPE_SHORT`
    Small(i16),
    /// Correponds to `MYSQL_TYPE_INT24` and `MYSQL_TYPE_LONG`
    Medium(i32),
    /// Correponds to `MYSQL_TYPE_LONGLONG`
    Big(i64),
    /// Correponds to `MYSQL_TYPE_FLOAT`
    Float(f32),
    /// Correponds to `MYSQL_TYPE_DOUBLE`
    Double(f64),
    /// Correponds to `MYSQL_TYPE_DECIMAL` and `MYSQL_TYPE_NEWDECIMAL`
    Decimal(&'a [u8]),
}
