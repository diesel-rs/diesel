use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::collections::Bound;
use std::io::Write;

use deserialize::{self, FromSql, FromSqlRow, Queryable};
use expression::bound::Bound as SqlBound;
use expression::AsExpression;
use pg::{Pg, PgMetadataLookup, PgTypeMetadata, PgValue};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::*;

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

impl<T, ST> Queryable<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg> + Queryable<ST, Pg>,
{
    type Row = Self;
    fn build(row: Self) -> Self {
        row
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

impl<T, ST> FromSqlRow<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    (Bound<T>, Bound<T>): FromSql<Range<ST>, Pg>,
{
    fn build_from_row<R: ::row::Row<Pg>>(row: &mut R) -> deserialize::Result<Self> {
        FromSql::<Range<ST>, Pg>::from_sql(row.take())
    }
}

impl<T, ST> FromSql<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(bytes: Option<PgValue>) -> deserialize::Result<Self> {
        let value = not_none!(bytes);
        let mut bytes = value.as_bytes();
        let flags: RangeFlags = RangeFlags::from_bits_truncate(bytes.read_u8()?);
        let mut lower_bound = Bound::Unbounded;
        let mut upper_bound = Bound::Unbounded;

        if !flags.contains(RangeFlags::LB_INF) {
            let elem_size = bytes.read_i32::<NetworkEndian>()?;
            let (elem_bytes, new_bytes) = bytes.split_at(elem_size as usize);
            bytes = new_bytes;
            let value = T::from_sql(Some(PgValue::new(
                elem_bytes,
                value.get_oid(),
                value.get_metadata_lookup(),
            )))?;

            lower_bound = if flags.contains(RangeFlags::LB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        if !flags.contains(RangeFlags::UB_INF) {
            let _size = bytes.read_i32::<NetworkEndian>()?;
            let value = T::from_sql(Some(PgValue::new(
                bytes,
                value.get_oid(),
                value.get_metadata_lookup(),
            )))?;

            upper_bound = if flags.contains(RangeFlags::UB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        Ok((lower_bound, upper_bound))
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

        match self.0 {
            Bound::Included(ref value) | Bound::Excluded(ref value) => {
                let mut buffer = out.with_buffer(Vec::new());

                value.to_sql(&mut buffer)?;
                out.write_u32::<NetworkEndian>(buffer.len() as u32)?;
                out.write_all(&buffer)?;
            }
            Bound::Unbounded => {}
        }

        match self.1 {
            Bound::Included(ref value) | Bound::Excluded(ref value) => {
                let mut buffer = out.with_buffer(Vec::new());

                value.to_sql(&mut buffer)?;
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
    fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata {
            oid: 3904,
            array_oid: 3905,
        }
    }
}

impl HasSqlType<Numrange> for Pg {
    fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata {
            oid: 3906,
            array_oid: 3907,
        }
    }
}

impl HasSqlType<Tsrange> for Pg {
    fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata {
            oid: 3908,
            array_oid: 3909,
        }
    }
}

impl HasSqlType<Tstzrange> for Pg {
    fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata {
            oid: 3910,
            array_oid: 3911,
        }
    }
}

impl HasSqlType<Daterange> for Pg {
    fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata {
            oid: 3912,
            array_oid: 3913,
        }
    }
}

impl HasSqlType<Int8range> for Pg {
    fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
        PgTypeMetadata {
            oid: 3926,
            array_oid: 3927,
        }
    }
}
