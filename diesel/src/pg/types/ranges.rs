use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::collections::Bound;
use std::io::Write;

use crate::deserialize::{self, FromSql, Queryable};
use crate::expression::bound::Bound as SqlBound;
use crate::expression::AsExpression;
use crate::pg::{Pg, PgTypeMetadata, PgValue};
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::*;

// https://github.com/postgres/postgres/blob/113b0045e20d40f726a0a30e33214455e4f1385e/src/include/utils/rangetypes.h#L35-L43
bitflags! {
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

impl<ST, T> AsExpression<Range<ST>> for (Bound<T>, Bound<T>) {
    type Expression = SqlBound<Range<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<'a, ST, T> AsExpression<Range<ST>> for &'a (Bound<T>, Bound<T>) {
    type Expression = SqlBound<Range<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<ST, T> AsExpression<Nullable<Range<ST>>> for (Bound<T>, Bound<T>) {
    type Expression = SqlBound<Nullable<Range<ST>>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<'a, ST, T> AsExpression<Nullable<Range<ST>>> for &'a (Bound<T>, Bound<T>) {
    type Expression = SqlBound<Nullable<Range<ST>>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<T, ST> FromSql<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        let mut bytes = value.as_bytes();
        let flags: RangeFlags = RangeFlags::from_bits_truncate(bytes.read_u8()?);
        let mut lower_bound = Bound::Unbounded;
        let mut upper_bound = Bound::Unbounded;

        if !flags.contains(RangeFlags::LB_INF) {
            let elem_size = bytes.read_i32::<NetworkEndian>()?;
            let (elem_bytes, new_bytes) = bytes.split_at(elem_size as usize);
            bytes = new_bytes;
            let value = T::from_sql(PgValue::new(elem_bytes, value.get_oid()))?;

            lower_bound = if flags.contains(RangeFlags::LB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        if !flags.contains(RangeFlags::UB_INF) {
            let _size = bytes.read_i32::<NetworkEndian>()?;
            let value = T::from_sql(PgValue::new(bytes, value.get_oid()))?;

            upper_bound = if flags.contains(RangeFlags::UB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        Ok((lower_bound, upper_bound))
    }
}

impl<T, ST> Queryable<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg>,
{
    type Row = Self;

    fn build(row: Self) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<ST, T> ToSql<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let mut flags = match self.0 {
            Bound::Included(_) => RangeFlags::LB_INC,
            Bound::Excluded(_) => RangeFlags::empty(),
            Bound::Unbounded => RangeFlags::LB_INF,
        };

        flags |= match self.1 {
            Bound::Included(_) => RangeFlags::UB_INC,
            Bound::Excluded(_) => RangeFlags::empty(),
            Bound::Unbounded => RangeFlags::UB_INF,
        };

        out.write_u8(flags.bits())?;

        let mut buffer = Vec::new();

        match self.0 {
            Bound::Included(ref value) | Bound::Excluded(ref value) => {
                {
                    let mut inner_buffer = Output::new(buffer, out.metadata_lookup());
                    value.to_sql(&mut inner_buffer)?;
                    buffer = inner_buffer.into_inner();
                }
                out.write_u32::<NetworkEndian>(buffer.len() as u32)?;
                out.write_all(&buffer)?;
                buffer.clear();
            }
            Bound::Unbounded => {}
        }

        match self.1 {
            Bound::Included(ref value) | Bound::Excluded(ref value) => {
                {
                    let mut inner_buffer = Output::new(buffer, out.metadata_lookup());
                    value.to_sql(&mut inner_buffer)?;
                    buffer = inner_buffer.into_inner();
                }
                out.write_u32::<NetworkEndian>(buffer.len() as u32)?;
                out.write_all(&buffer)?;
            }
            Bound::Unbounded => {}
        }

        Ok(IsNull::No)
    }
}

impl<ST, T> ToSql<Nullable<Range<ST>>, Pg> for (Bound<T>, Bound<T>)
where
    (Bound<T>, Bound<T>): ToSql<Range<ST>, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        ToSql::<Range<ST>, Pg>::to_sql(self, out)
    }
}

impl HasSqlType<Int4range> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3904, 3905)
    }
}

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

impl HasSqlType<Tstzrange> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3910, 3911)
    }
}

impl HasSqlType<Daterange> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3912, 3913)
    }
}

impl HasSqlType<Int8range> for Pg {
    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata::new(3926, 3927)
    }
}
