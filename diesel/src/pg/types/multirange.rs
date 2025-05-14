use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::Write;
use std::ops::Bound;

use crate::deserialize::{self, Defaultable, FromSql};
use crate::expression::bound::Bound as SqlBound;
use crate::expression::AsExpression;
use crate::pg::{Pg, PgTypeMetadata, PgValue};
use crate::query_builder::bind_collector::ByteWrapper;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::*;

// from `SELECT oid, typname FROM pg_catalog.pg_type where typname LIKE '%multirange'`;
macro_rules! multirange_has_sql_type {
    ($ty:ty, $oid:expr, $array_oid:expr) => {
        #[cfg(feature = "postgres_backend")]
        impl HasSqlType<$ty> for Pg {
            fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
                PgTypeMetadata::new($oid, $array_oid)
            }
        }
    };
}
multirange_has_sql_type!(Datemultirange, 4535, 6155);
multirange_has_sql_type!(Int4multirange, 4451, 6150);
multirange_has_sql_type!(Int8multirange, 4536, 6157);
multirange_has_sql_type!(Nummultirange, 4532, 6151);
multirange_has_sql_type!(Tsmultirange, 4533, 6152);
multirange_has_sql_type!(Tstzmultirange, 4534, 6153);

macro_rules! multirange_as_expression {
    ($ty:ty, $sql_type:ty) => {
        #[cfg(feature = "postgres_backend")]
        // this simplifies the macro implementation
        // as some macro calls use this lifetime
        #[allow(clippy::extra_unused_lifetimes)]
        impl<'a, 'b, ST: 'static, T> AsExpression<$sql_type> for $ty {
            type Expression = SqlBound<$sql_type, Self>;
            fn as_expression(self) -> Self::Expression {
                SqlBound::new(self)
            }
        }
    };
}

macro_rules! multirange_as_expressions {
    ($ty:ty) => {
        multirange_as_expression!(&'a [$ty], Multirange<ST>);
        multirange_as_expression!(&'a [$ty], Nullable<Multirange<ST>>);
        multirange_as_expression!(&'a &'b [$ty], Multirange<ST>);
        multirange_as_expression!(&'a &'b [$ty], Nullable<Multirange<ST>>);
        multirange_as_expression!(Vec<$ty>, Multirange<ST>);
        multirange_as_expression!(Vec<$ty>, Nullable<Multirange<ST>>);
        multirange_as_expression!(&'a Vec<$ty>, Multirange<ST>);
        multirange_as_expression!(&'a Vec<$ty>, Nullable<Multirange<ST>>);
        multirange_as_expression!(&'a &'b Vec<$ty>, Multirange<ST>);
        multirange_as_expression!(&'a &'b Vec<$ty>, Nullable<Multirange<ST>>);
    };
}

multirange_as_expressions!((Bound<T>, Bound<T>));
multirange_as_expressions!(std::ops::Range<T>);
multirange_as_expressions!(std::ops::RangeInclusive<T>);
multirange_as_expressions!(std::ops::RangeToInclusive<T>);
multirange_as_expressions!(std::ops::RangeFrom<T>);
multirange_as_expressions!(std::ops::RangeTo<T>);

#[cfg(feature = "postgres_backend")]
impl<T, ST> FromSql<Multirange<ST>, Pg> for Vec<(Bound<T>, Bound<T>)>
where
    T: FromSql<ST, Pg> + Defaultable,
{
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        let len = bytes.read_u32::<NetworkEndian>()?;

        (0..len)
            .map(|_| {
                let range_size: usize = bytes.read_i32::<NetworkEndian>()?.try_into()?;
                let (range_bytes, new_bytes) = bytes.split_at(range_size);
                bytes = new_bytes;
                FromSql::from_sql(PgValue::new_internal(range_bytes, &value))
            })
            .collect()
    }
}

fn to_sql<'c, ST, T, I>(iter: I, out: &mut Output<'_, '_, Pg>) -> serialize::Result
where
    ST: 'static,
    T: ToSql<ST, Pg> + 'c,
    I: Iterator<Item = (Bound<&'c T>, Bound<&'c T>)> + ExactSizeIterator,
{
    out.write_u32::<NetworkEndian>(iter.len().try_into()?)?;

    let mut buffer = Vec::new();
    for value in iter {
        {
            let mut inner_buffer = Output::new(ByteWrapper(&mut buffer), out.metadata_lookup());
            ToSql::<Range<ST>, Pg>::to_sql(&value, &mut inner_buffer)?;
        }
        let buffer_len: i32 = buffer.len().try_into()?;
        out.write_i32::<NetworkEndian>(buffer_len)?;
        out.write_all(&buffer)?;
        buffer.clear();
    }

    Ok(IsNull::No)
}

#[cfg(feature = "postgres_backend")]
impl<T, ST> ToSql<Multirange<ST>, Pg> for [(Bound<T>, Bound<T>)]
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        to_sql(self.iter().map(|r| (r.0.as_ref(), r.1.as_ref())), out)
    }
}

#[cfg(feature = "postgres_backend")]
impl<T, ST> ToSql<Multirange<ST>, Pg> for Vec<(Bound<T>, Bound<T>)>
where
    T: ToSql<ST, Pg>,
    [(Bound<T>, Bound<T>)]: ToSql<Multirange<ST>, Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        ToSql::<Multirange<ST>, Pg>::to_sql(self.as_slice(), out)
    }
}

use std::ops::RangeBounds;
macro_rules! multirange_std_to_sql {
    ($ty:ty) => {
        #[cfg(feature = "postgres_backend")]
        impl<ST, T> ToSql<Multirange<ST>, Pg> for [$ty]
        where
            ST: 'static,
            T: ToSql<ST, Pg>,
        {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                to_sql(
                    self.into_iter().map(|r| (r.start_bound(), r.end_bound())),
                    out,
                )
            }
        }

        #[cfg(feature = "postgres_backend")]
        impl<T, ST> ToSql<Multirange<ST>, Pg> for Vec<$ty>
        where
            T: ToSql<ST, Pg>,
            [$ty]: ToSql<Multirange<ST>, Pg>,
        {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                ToSql::<Multirange<ST>, Pg>::to_sql(self.as_slice(), out)
            }
        }
    };
}

multirange_std_to_sql!(std::ops::Range<T>);
multirange_std_to_sql!(std::ops::RangeInclusive<T>);
multirange_std_to_sql!(std::ops::RangeFrom<T>);
multirange_std_to_sql!(std::ops::RangeTo<T>);
multirange_std_to_sql!(std::ops::RangeToInclusive<T>);

macro_rules! multirange_to_sql_nullable {
    ($ty:ty) => {
        impl<T, ST> ToSql<Nullable<Multirange<ST>>, Pg> for [$ty]
        where
            ST: 'static,
            [$ty]: ToSql<ST, Pg>,
            T: ToSql<ST, Pg>,
        {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                ToSql::<Multirange<ST>, Pg>::to_sql(self, out)
            }
        }

        impl<T, ST> ToSql<Nullable<Multirange<ST>>, Pg> for Vec<$ty>
        where
            ST: 'static,
            Vec<$ty>: ToSql<ST, Pg>,
            T: ToSql<ST, Pg>,
        {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                ToSql::<Multirange<ST>, Pg>::to_sql(self, out)
            }
        }
    };
}

multirange_to_sql_nullable!((Bound<T>, Bound<T>));
multirange_to_sql_nullable!(std::ops::Range<T>);
multirange_to_sql_nullable!(std::ops::RangeInclusive<T>);
multirange_to_sql_nullable!(std::ops::RangeFrom<T>);
multirange_to_sql_nullable!(std::ops::RangeTo<T>);
multirange_to_sql_nullable!(std::ops::RangeToInclusive<T>);
