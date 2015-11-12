extern crate byteorder;

use Queriable;
use self::byteorder::{ReadBytesExt, BigEndian};
use types::{self, NativeSqlType, FromSql, Array};
use super::option::UnexpectedNullError;
use std::error::Error;

impl<T: NativeSqlType> NativeSqlType for Array<T> {
    fn oid() -> u32 {
        let oid = T::oid();
        if oid == types::Bool::oid() { 1000 }
        else if oid == types::SmallInt::oid() { 1005 }
        else if oid == types::Integer::oid() { 1007 }
        else if oid == types::BigInt::oid() { 1016 }

        else if oid == types::Float::oid() { 1021 }
        else if oid == types::Double::oid() { 1022 }

        else if oid == types::VarChar::oid() { 1015 }
        else if oid == types::Text::oid() { 1009 }

        else if oid == types::Binary::oid() { 1001 }
        else { 0 }
    }
}

impl<T, ST> FromSql<Array<ST>> for Vec<T> where
    T: FromSql<ST>,
    ST: NativeSqlType,
{
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        let num_dimensions = try!(bytes.read_i32::<BigEndian>());
        let has_null = try!(bytes.read_i32::<BigEndian>()) != 0;
        let _oid = try!(bytes.read_i32::<BigEndian>());
        let num_elements = try!(bytes.read_i32::<BigEndian>());
        let lower_bound = try!(bytes.read_i32::<BigEndian>());

        assert!(num_dimensions == 1, "multi-dimensional arrays are not supported");
        assert!(lower_bound == 1, "lower bound must be 1");

        (0..num_elements).map(|_| {
            let elem_size = try!(bytes.read_i32::<BigEndian>());
            if has_null && elem_size == -1 {
                T::from_sql(None)
            } else {
                let (elem_bytes, new_bytes) = bytes.split_at(elem_size as usize);
                bytes = new_bytes;
                T::from_sql(Some(&elem_bytes))
            }
        }).collect()
    }
}

impl<T, ST> Queriable<Array<ST>> for Vec<T> where
    T: FromSql<ST> + Queriable<ST>,
    ST: NativeSqlType,
{
    type Row = Self;
    fn build(row: Self) -> Self {
        row
    }
}
