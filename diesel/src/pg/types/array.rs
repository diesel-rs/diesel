extern crate byteorder;

use self::byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use std::error::Error;
use std::io::Write;

use backend::Debug;
use pg::{Pg, PgTypeMetadata};
use query_source::Queryable;
use types::*;

impl<T> HasSqlType<Array<T>> for Pg where
    Pg: HasSqlType<T>,
{
    fn metadata() -> PgTypeMetadata {
        PgTypeMetadata {
            oid: <Pg as HasSqlType<T>>::metadata().array_oid,
            array_oid: 0,
        }
    }
}

impl<T> HasSqlType<Array<T>> for Debug where
    Debug: HasSqlType<T>,
{
    fn metadata() {}
}

impl_query_id!(Array<T>);

impl<T> NotNull for Array<T> {
}

impl<T, ST> FromSql<Array<ST>, Pg> for Vec<T> where
    T: FromSql<ST, Pg>,
    Pg: HasSqlType<ST>,
{
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>> {
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
    Pg: HasSqlType<ST>,
    Vec<T>: FromSql<Array<ST>, Pg>,
{
    fn build_from_row<R: ::row::Row<Pg>>(row: &mut R) -> Result<Self, Box<Error+Send+Sync>> {
        FromSql::<Array<ST>, Pg>::from_sql(row.take())
    }
}

impl<T, ST> Queryable<Array<ST>, Pg> for Vec<T> where
    T: FromSql<ST, Pg> + Queryable<ST, Pg>,
    Pg: HasSqlType<ST>,
{
    type Row = Self;
    fn build(row: Self) -> Self {
        row
    }
}

use expression::AsExpression;
use expression::bound::Bound;

macro_rules! array_as_expression {
    ($ty:ty, $sql_type:ty) => {
        impl<'a, ST, T> AsExpression<$sql_type> for $ty where
            Pg: HasSqlType<ST>,
        {
            type Expression = Bound<$sql_type, Self>;

            fn as_expression(self) -> Self::Expression {
                Bound::new(self)
            }
        }
    }
}

array_as_expression!(&'a [T], Array<ST>);
array_as_expression!(&'a [T], Nullable<Array<ST>>);
array_as_expression!(&'a &'a [T], Array<ST>);
array_as_expression!(&'a &'a [T], Nullable<Array<ST>>);
array_as_expression!(Vec<T>, Array<ST>);
array_as_expression!(Vec<T>, Nullable<Array<ST>>);
array_as_expression!(&'a Vec<T>, Array<ST>);
array_as_expression!(&'a Vec<T>, Nullable<Array<ST>>);

impl<'a, ST, T> ToSql<Array<ST>, Pg> for &'a [T] where
    Pg: HasSqlType<ST>,
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        let num_dimensions = 1;
        try!(out.write_i32::<BigEndian>(num_dimensions));
        let flags = 0;
        try!(out.write_i32::<BigEndian>(flags));
        try!(out.write_u32::<BigEndian>(Pg::metadata().oid));
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
    Pg: HasSqlType<ST>,
    for<'a> &'a [T]: ToSql<Array<ST>, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut W) -> Result<IsNull, Box<Error+Send+Sync>> {
        (&self as &[T]).to_sql(out)
    }
}
