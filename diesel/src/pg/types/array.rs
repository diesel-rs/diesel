use byteorder::{ReadBytesExt, WriteBytesExt, NetworkEndian};
use std::error::Error;
use std::fmt;
use std::io::Write;

use backend::Debug;
use pg::{Pg, PgTypeMetadata, PgMetadataLookup};
use query_source::Queryable;
use types::*;

impl<T> HasSqlType<Array<T>> for Pg where
    Pg: HasSqlType<T>,
{
    fn metadata(lookup: &PgMetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata {
            oid: <Pg as HasSqlType<T>>::metadata(lookup).array_oid,
            array_oid: 0,
        }
    }
}

impl<T> HasSqlType<Array<T>> for Debug where
    Debug: HasSqlType<T>,
{
    fn metadata(_: &()) {}
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
        let num_dimensions = try!(bytes.read_i32::<NetworkEndian>());
        let has_null = try!(bytes.read_i32::<NetworkEndian>()) != 0;
        let _oid = try!(bytes.read_i32::<NetworkEndian>());

        if num_dimensions == 0 {
            return Ok(Vec::new())
        }

        let num_elements = try!(bytes.read_i32::<NetworkEndian>());
        let lower_bound = try!(bytes.read_i32::<NetworkEndian>());

        assert_eq!(num_dimensions, 1, "multi-dimensional arrays are not supported");
        assert_eq!(lower_bound, 1, "lower bound must be 1");

        (0..num_elements).map(|_| {
            let elem_size = try!(bytes.read_i32::<NetworkEndian>());
            if has_null && elem_size == -1 {
                T::from_sql(None)
            } else {
                let (elem_bytes, new_bytes) = bytes.split_at(elem_size as usize);
                bytes = new_bytes;
                T::from_sql(Some(elem_bytes))
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

#[cfg(not(feature="unstable"))]
impl<T, ST> FromSqlRow<Nullable<Array<ST>>, Pg> for Option<Vec<T>> where
    Pg: HasSqlType<ST>,
    Option<Vec<T>>: FromSql<Nullable<Array<ST>>, Pg>,
{
    fn build_from_row<R: ::row::Row<Pg>>(row: &mut R) -> Result<Self, Box<Error+Send+Sync>> {
        FromSql::<Nullable<Array<ST>>, Pg>::from_sql(row.take())
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
        impl<'a, 'b, ST, T> AsExpression<$sql_type> for $ty where
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
array_as_expression!(&'a &'b [T], Array<ST>);
array_as_expression!(&'a &'b [T], Nullable<Array<ST>>);
array_as_expression!(Vec<T>, Array<ST>);
array_as_expression!(Vec<T>, Nullable<Array<ST>>);
array_as_expression!(&'a Vec<T>, Array<ST>);
array_as_expression!(&'a Vec<T>, Nullable<Array<ST>>);

impl<'a, ST, T> ToSql<Array<ST>, Pg> for &'a [T] where
    Pg: HasSqlType<ST>,
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        let num_dimensions = 1;
        try!(out.write_i32::<NetworkEndian>(num_dimensions));
        let flags = 0;
        try!(out.write_i32::<NetworkEndian>(flags));
        let element_oid = Pg::metadata(out.metadata_lookup()).oid;
        try!(out.write_u32::<NetworkEndian>(element_oid));
        try!(out.write_i32::<NetworkEndian>(self.len() as i32));
        let lower_bound = 1;
        try!(out.write_i32::<NetworkEndian>(lower_bound));

        let mut buffer = out.with_buffer(Vec::new());
        for elem in self.iter() {
            let is_null = try!(elem.to_sql(&mut buffer));
            if let IsNull::No = is_null {
                try!(out.write_i32::<NetworkEndian>(buffer.len() as i32));
                try!(out.write_all(&buffer));
                buffer.clear();
            } else {
                // https://github.com/postgres/postgres/blob/82f8107b92c9104ec9d9465f3f6a4c6dab4c124a/src/backend/utils/adt/arrayfuncs.c#L1461
                try!(out.write_i32::<NetworkEndian>(-1));
            }
        }

        Ok(IsNull::No)
    }
}

impl<'a, ST, T> ToSql<Nullable<Array<ST>>, Pg> for &'a [T] where
    Pg: HasSqlType<ST>,
    &'a [T]: ToSql<Array<ST>, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        ToSql::<Array<ST>, Pg>::to_sql(self, out)
    }
}

impl<ST, T> ToSql<Array<ST>, Pg> for Vec<T> where
    Pg: HasSqlType<ST>,
    for<'a> &'a [T]: ToSql<Array<ST>, Pg>,
    T: fmt::Debug,
{
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        (self as &[T]).to_sql(out)
    }
}

impl<ST, T> ToSql<Nullable<Array<ST>>, Pg> for Vec<T> where
    Pg: HasSqlType<ST>,
    Vec<T>: ToSql<Array<ST>, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut ToSqlOutput<W, Pg>) -> Result<IsNull, Box<Error+Send+Sync>> {
        ToSql::<Array<ST>, Pg>::to_sql(self, out)
    }
}
