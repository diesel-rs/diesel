use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::collections::Bound;
use std::error::Error;
use std::io::Write;

use crate::deserialize::{self, Defaultable, FromSql, Queryable};
use crate::expression::bound::Bound as SqlBound;
use crate::expression::AsExpression;
use crate::pg::{Pg, PgTypeMetadata, PgValue};
use crate::query_builder::bind_collector::ByteWrapper;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::*;

// https://github.com/postgres/postgres/blob/113b0045e20d40f726a0a30e33214455e4f1385e/src/include/utils/rangetypes.h#L35-L43
bitflags::bitflags! {
    struct RangeFlags: u8 {
        const EMPTY = 0x01;
        const LB_INC = 0x02;
        const UB_INC = 0x04;
        const LB_INF = 0x08;
        const UB_INF = 0x10;
        const LB_NULL = 0x20;
        const UB_NULL = 0x40;
        const CONTAIN_EMPTY = 0x80;
    }
}

macro_rules! range_as_expression {
    ($ty:ty; $sql_type:ty) => {
        #[cfg(feature = "postgres_backend")]
        // this simplifies the macro implementation
        // as some macro calls use this lifetime
        #[allow(clippy::extra_unused_lifetimes)]
        impl<'a, ST: 'static, T> AsExpression<$sql_type> for $ty {
            type Expression = SqlBound<$sql_type, Self>;

            fn as_expression(self) -> Self::Expression {
                SqlBound::new(self)
            }
        }
    };
}

range_as_expression!((Bound<T>, Bound<T>); Range<ST>);
range_as_expression!(&'a (Bound<T>, Bound<T>); Range<ST>);
range_as_expression!((Bound<T>, Bound<T>); Nullable<Range<ST>>);
range_as_expression!(&'a (Bound<T>, Bound<T>); Nullable<Range<ST>>);

range_as_expression!(std::ops::Range<T>; Range<ST>);
range_as_expression!(&'a std::ops::Range<T>; Range<ST>);
range_as_expression!(std::ops::Range<T>; Nullable<Range<ST>>);
range_as_expression!(&'a std::ops::Range<T>; Nullable<Range<ST>>);

range_as_expression!(std::ops::RangeInclusive<T>; Range<ST>);
range_as_expression!(&'a std::ops::RangeInclusive<T>; Range<ST>);
range_as_expression!(std::ops::RangeInclusive<T>; Nullable<Range<ST>>);
range_as_expression!(&'a std::ops::RangeInclusive<T>; Nullable<Range<ST>>);

range_as_expression!(std::ops::RangeToInclusive<T>; Range<ST>);
range_as_expression!(&'a std::ops::RangeToInclusive<T>; Range<ST>);
range_as_expression!(std::ops::RangeToInclusive<T>; Nullable<Range<ST>>);
range_as_expression!(&'a std::ops::RangeToInclusive<T>; Nullable<Range<ST>>);

range_as_expression!(std::ops::RangeFrom<T>; Range<ST>);
range_as_expression!(&'a std::ops::RangeFrom<T>; Range<ST>);
range_as_expression!(std::ops::RangeFrom<T>; Nullable<Range<ST>>);
range_as_expression!(&'a std::ops::RangeFrom<T>; Nullable<Range<ST>>);

range_as_expression!(std::ops::RangeTo<T>; Range<ST>);
range_as_expression!(&'a std::ops::RangeTo<T>; Range<ST>);
range_as_expression!(std::ops::RangeTo<T>; Nullable<Range<ST>>);
range_as_expression!(&'a std::ops::RangeTo<T>; Nullable<Range<ST>>);

#[cfg(feature = "postgres_backend")]
impl<T, ST> FromSql<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg> + Defaultable,
{
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        let flags: RangeFlags = RangeFlags::from_bits_truncate(bytes.read_u8()?);
        let mut lower_bound = Bound::Unbounded;
        let mut upper_bound = Bound::Unbounded;

        if flags.contains(RangeFlags::EMPTY) {
            lower_bound = Bound::Excluded(T::default_value());
        } else if !flags.contains(RangeFlags::LB_INF) {
            let elem_size = bytes.read_i32::<NetworkEndian>()?;
            let (elem_bytes, new_bytes) = bytes.split_at(elem_size.try_into()?);
            bytes = new_bytes;
            let value = T::from_sql(PgValue::new_internal(elem_bytes, &value))?;

            lower_bound = if flags.contains(RangeFlags::LB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        if flags.contains(RangeFlags::EMPTY) {
            upper_bound = Bound::Excluded(T::default_value());
        } else if !flags.contains(RangeFlags::UB_INF) {
            let _size = bytes.read_i32::<NetworkEndian>()?;
            let value = T::from_sql(PgValue::new_internal(bytes, &value))?;

            upper_bound = if flags.contains(RangeFlags::UB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        Ok((lower_bound, upper_bound))
    }
}

#[cfg(feature = "postgres_backend")]
impl<T, ST> Queryable<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg> + Defaultable,
{
    type Row = Self;

    fn build(row: Self) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[cfg(feature = "postgres_backend")]
fn to_sql<ST, T>(
    start: Bound<&T>,
    end: Bound<&T>,
    out: &mut Output<'_, '_, Pg>,
) -> serialize::Result
where
    T: ToSql<ST, Pg>,
{
    let mut flags = match start {
        Bound::Included(_) => RangeFlags::LB_INC,
        Bound::Excluded(_) => RangeFlags::empty(),
        Bound::Unbounded => RangeFlags::LB_INF,
    };

    flags |= match end {
        Bound::Included(_) => RangeFlags::UB_INC,
        Bound::Excluded(_) => RangeFlags::empty(),
        Bound::Unbounded => RangeFlags::UB_INF,
    };

    out.write_u8(flags.bits())?;

    let mut buffer = Vec::new();

    match start {
        Bound::Included(ref value) | Bound::Excluded(ref value) => {
            {
                let mut inner_buffer = Output::new(ByteWrapper(&mut buffer), out.metadata_lookup());
                value.to_sql(&mut inner_buffer)?;
            }
            out.write_u32::<NetworkEndian>(buffer.len().try_into()?)?;
            out.write_all(&buffer)?;
            buffer.clear();
        }
        Bound::Unbounded => {}
    }

    match end {
        Bound::Included(ref value) | Bound::Excluded(ref value) => {
            {
                let mut inner_buffer = Output::new(ByteWrapper(&mut buffer), out.metadata_lookup());
                value.to_sql(&mut inner_buffer)?;
            }
            out.write_u32::<NetworkEndian>(buffer.len().try_into()?)?;
            out.write_all(&buffer)?;
        }
        Bound::Unbounded => {}
    }

    Ok(IsNull::No)
}

#[cfg(feature = "postgres_backend")]
impl<ST, T> ToSql<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        to_sql(self.0.as_ref(), self.1.as_ref(), out)
    }
}

use std::ops::RangeBounds;
macro_rules! range_std_to_sql {
    ($ty:ty) => {
        #[cfg(feature = "postgres_backend")]
        impl<ST, T> ToSql<Range<ST>, Pg> for $ty
        where
            ST: 'static,
            T: ToSql<ST, Pg>,
        {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                to_sql(self.start_bound(), self.end_bound(), out)
            }
        }
    };
}

range_std_to_sql!(std::ops::Range<T>);
range_std_to_sql!(std::ops::RangeInclusive<T>);
range_std_to_sql!(std::ops::RangeFrom<T>);
range_std_to_sql!(std::ops::RangeTo<T>);
range_std_to_sql!(std::ops::RangeToInclusive<T>);

macro_rules! range_to_sql_nullable {
    ($ty:ty) => {
        #[cfg(feature = "postgres_backend")]
        impl<ST, T> ToSql<Nullable<Range<ST>>, Pg> for $ty
        where
            ST: 'static,
            $ty: ToSql<Range<ST>, Pg>,
        {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
                ToSql::<Range<ST>, Pg>::to_sql(self, out)
            }
        }
    };
}
range_to_sql_nullable!((Bound<T>, Bound<T>));
range_to_sql_nullable!(std::ops::Range<T>);
range_to_sql_nullable!(std::ops::RangeInclusive<T>);
range_to_sql_nullable!(std::ops::RangeFrom<T>);
range_to_sql_nullable!(std::ops::RangeTo<T>);
range_to_sql_nullable!(std::ops::RangeToInclusive<T>);

#[cfg(feature = "postgres_backend")]
impl HasSqlType<Int4range> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3904, 3905)
    }
}

#[cfg(feature = "postgres_backend")]
impl HasSqlType<Numrange> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3906, 3907)
    }
}

impl HasSqlType<Tsrange> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3908, 3909)
    }
}

#[cfg(feature = "postgres_backend")]
impl HasSqlType<Tstzrange> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3910, 3911)
    }
}

#[cfg(feature = "postgres_backend")]
impl HasSqlType<Daterange> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3912, 3913)
    }
}

#[cfg(feature = "postgres_backend")]
impl HasSqlType<Int8range> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3926, 3927)
    }
}

#[cfg(feature = "postgres_backend")]
impl ToSql<RangeBoundEnum, Pg> for RangeBound {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let literal = match self {
            Self::LowerBoundInclusiveUpperBoundInclusive => "[]",
            Self::LowerBoundInclusiveUpperBoundExclusive => "[)",
            Self::LowerBoundExclusiveUpperBoundInclusive => "(]",
            Self::LowerBoundExclusiveUpperBoundExclusive => "()",
        };
        out.write_all(literal.as_bytes())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}
