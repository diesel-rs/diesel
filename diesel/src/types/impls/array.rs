extern crate byteorder;

use self::byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use std::error::Error;
use std::io::Write;

use backend::Pg;
use query_source::Queryable;
use row::Row;
use super::option::UnexpectedNullError;
use types::{NativeSqlType, FromSql, FromSqlRow, ToSql, Array, IsNull, NotNull};

impl<T: NativeSqlType> NativeSqlType for Array<T> {
    fn oid() -> u32 {
        T::array_oid()
    }

    fn array_oid() -> u32 {
        panic!("Multidimensional arrays are unsupported");
    }
}

impl<T: NativeSqlType> NotNull for Array<T> {
}

impl<T, ST> FromSql<Array<ST>, Pg> for Vec<T> where
    T: FromSql<ST, Pg>,
    ST: NativeSqlType,
{
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error>> {
        let mut bytes = not_none!(bytes);
        let num_dimensions = try!(bytes.read_i32::<BigEndian>());
        let has_null = try!(bytes.read_i32::<BigEndian>()) != 0;
        let _oid = try!(bytes.read_i32::<BigEndian>());

        if num_dimensions == 0 {
            return Ok(Vec::new())
        }

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

impl<T, ST> FromSqlRow<Array<ST>, Pg> for Vec<T> where
    ST: NativeSqlType,
    Vec<T>: FromSql<Array<ST>, Pg>,
{
    fn build_from_row<R: Row>(row: &mut R) -> Result<Self, Box<Error>> {
        FromSql::<Array<ST>, Pg>::from_sql(row.take())
    }
}

impl<T, ST> Queryable<Array<ST>, Pg> for Vec<T> where
    T: FromSql<ST, Pg> + Queryable<ST, Pg>,
    ST: NativeSqlType,
{
    type Row = Self;
    fn build(row: Self) -> Self {
        row
    }
}

use expression::AsExpression;
use expression::bound::Bound;

impl<'a, ST, T> AsExpression<Array<ST>> for &'a [T] where
    ST: NativeSqlType,
{
    type Expression = Bound<Array<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<ST, T> AsExpression<Array<ST>> for Vec<T> where
    ST: NativeSqlType,
{
    type Expression = Bound<Array<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'a, ST, T> AsExpression<Array<ST>> for &'a Vec<T> where
    ST: NativeSqlType,
{
    type Expression = Bound<Array<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'a, ST, T> ToSql<Array<ST>, Pg> for &'a [T] where
    ST: NativeSqlType,
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        let num_dimensions = 1;
        try!(out.write_i32::<BigEndian>(num_dimensions));
        let flags = 0;
        try!(out.write_i32::<BigEndian>(flags));
        try!(out.write_u32::<BigEndian>(ST::oid()));
        try!(out.write_i32::<BigEndian>(self.len() as i32));
        let lower_bound = 1;
        try!(out.write_i32::<BigEndian>(lower_bound));

        let mut buffer = Vec::new();
        for elem in self.iter() {
            let is_null = try!(elem.to_sql(&mut buffer));
            assert!(is_null == IsNull::No, "Arrays containing null are not supported");
            try!(out.write_i32::<BigEndian>(buffer.len() as i32));
            try!(out.write_all(&buffer));
            buffer.clear();
        }

        Ok(IsNull::No)
    }
}

impl<ST, T> ToSql<Array<ST>, Pg> for Vec<T> where
    ST: NativeSqlType,
    for<'a> &'a [T]: ToSql<Array<ST>, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error>> {
        (&self as &[T]).to_sql(out)
    }
}
