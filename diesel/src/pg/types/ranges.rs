use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::collections::Bound;
use std::io::Write;
use std::ops::{Range, RangeFrom, RangeInclusive, RangeToInclusive, RangeTo};

use deserialize::{self, FromSql, FromSqlRow, Queryable};
use expression::bound::Bound as SqlBound;
use expression::AsExpression;
use pg::{Pg, PgMetadataLookup, PgTypeMetadata};
use serialize::{self, IsNull, Output, ToSql};
use sql_types::{Range as SqlRange, *};

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

macro_rules! setup_impls {
    ($target:ty) => {
        impl<T, ST> Queryable<SqlRange<ST>, Pg> for $target
        where
            T: FromSql<ST, Pg> + Queryable<ST, Pg>,
        {
            type Row = Self;
            fn build(row: Self) -> Self {
                row
            }
        }

        impl<ST, T> AsExpression<SqlRange<ST>> for $target {
            type Expression = SqlBound<SqlRange<ST>, Self>;

            fn as_expression(self) -> Self::Expression {
                SqlBound::new(self)
            }
        }

        impl<'a, ST, T> AsExpression<SqlRange<ST>> for &'a $target {
            type Expression = SqlBound<SqlRange<ST>, Self>;

            fn as_expression(self) -> Self::Expression {
                SqlBound::new(self)
            }
        }

        impl<ST, T> AsExpression<Nullable<SqlRange<ST>>> for $target {
            type Expression = SqlBound<Nullable<SqlRange<ST>>, Self>;

            fn as_expression(self) -> Self::Expression {
                SqlBound::new(self)
            }
        }

        impl<'a, ST, T> AsExpression<Nullable<SqlRange<ST>>> for &'a $target {
            type Expression = SqlBound<Nullable<SqlRange<ST>>, Self>;

            fn as_expression(self) -> Self::Expression {
                SqlBound::new(self)
            }
        }

        impl<T, ST> FromSqlRow<SqlRange<ST>, Pg> for $target
        where
            $target: FromSql<SqlRange<ST>, Pg>,
        {
            fn build_from_row<R: ::row::Row<Pg>>(row: &mut R) -> deserialize::Result<Self> {
                FromSql::<SqlRange<ST>, Pg>::from_sql(row.take())
            }
        }

        impl<ST, T> ToSql<Nullable<SqlRange<ST>>, Pg> for $target
        where
            $target: ToSql<SqlRange<ST>, Pg>,
        {
            fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
                ToSql::<SqlRange<ST>, Pg>::to_sql(self, out)
            }
        }
    };
}

// (Bound<T>, Bound<T>) as range representation

setup_impls! {(Bound<T>, Bound<T>)}

impl<T, ST> FromSql<SqlRange<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let mut bytes = not_none!(bytes);
        let flags: RangeFlags = RangeFlags::from_bits_truncate(bytes.read_u8()?);
        let mut lower_bound = Bound::Unbounded;
        let mut upper_bound = Bound::Unbounded;

        if !flags.contains(RangeFlags::LB_INF) {
            let elem_size = bytes.read_i32::<NetworkEndian>()?;
            let (elem_bytes, new_bytes) = bytes.split_at(elem_size as usize);
            bytes = new_bytes;
            let value = T::from_sql(Some(elem_bytes))?;

            lower_bound = if flags.contains(RangeFlags::LB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        if !flags.contains(RangeFlags::UB_INF) {
            let _size = bytes.read_i32::<NetworkEndian>()?;
            let value = T::from_sql(Some(bytes))?;

            upper_bound = if flags.contains(RangeFlags::UB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        Ok((lower_bound, upper_bound))
    }
}

impl<ST, T> ToSql<SqlRange<ST>, Pg> for (Bound<T>, Bound<T>)
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

macro_rules! bounds_err {
    ($name:ident, $expected_lower:ident, $expected_upper:ident, $got:expr) => {
        Err(format!(
            "Unexpected bounds for {}. Expected bounds to be {}..{}, got $got",
            "$name", "$expected_lower", "$expected_upper"
        )
        .into())
    };
}

// `std::ops::Range`

setup_impls! {Range<T>}

impl<T, ST> FromSql<SqlRange<ST>, Pg> for Range<T>
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (lower_bound, upper_bound) = FromSql::from_sql(bytes)?;

        match (lower_bound, upper_bound) {
            (Bound::Included(start), Bound::Excluded(end)) => Ok(Self { start, end }),
            _erroneous_bounds => bounds_err!(Range, Included, Excluded, _erroneous_bounds),
        }
    }
}

impl<ST, T> ToSql<SqlRange<ST>, Pg> for Range<T>
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let range = (Bound::Included(&self.start), Bound::Excluded(&self.end));
        ToSql::<SqlRange<ST>, Pg>::to_sql(&range, out)?;

        Ok(IsNull::No)
    }
}

// `std::ops::RangeFrom`

setup_impls! {RangeFrom<T>}

impl<T, ST> FromSql<SqlRange<ST>, Pg> for RangeFrom<T>
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (lower_bound, upper_bound) = FromSql::from_sql(bytes)?;

        match (lower_bound, upper_bound) {
            (Bound::Included(start), Bound::Unbounded) => Ok(Self { start }),
            _erroneous_bounds => bounds_err!(Range, Included, Unbounded, _erroneous_bounds),
        }
    }
}

impl<ST, T> ToSql<SqlRange<ST>, Pg> for RangeFrom<T>
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let range = (Bound::Included(&self.start), Bound::Unbounded);
        ToSql::<SqlRange<ST>, Pg>::to_sql(&range, out)?;

        Ok(IsNull::No)
    }
}

// `std::ops::RangeInclusive`

setup_impls! {RangeInclusive<T>}

impl<T, ST> FromSql<SqlRange<ST>, Pg> for RangeInclusive<T>
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (lower_bound, upper_bound) = FromSql::from_sql(bytes)?;

        match (lower_bound, upper_bound) {
            (Bound::Included(start), Bound::Included(end)) => Ok(Self::new(start, end)),
            _erroneous_bounds => bounds_err!(Range, Included, Included, _erroneous_bounds),
        }
    }
}

impl<ST, T> ToSql<SqlRange<ST>, Pg> for RangeInclusive<T>
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let start = self.start();
        let end = self.end();
        let range = (Bound::Included(start), Bound::Included(end));
        ToSql::<SqlRange<ST>, Pg>::to_sql(&range, out)?;

        Ok(IsNull::No)
    }
}

// `std::ops::RangeToInclusive`

setup_impls! {RangeToInclusive<T>}

impl<T, ST> FromSql<SqlRange<ST>, Pg> for RangeToInclusive<T>
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (lower_bound, upper_bound) = FromSql::from_sql(bytes)?;

        match (lower_bound, upper_bound) {
            (Bound::Unbounded, Bound::Included(end)) => Ok(Self { end }),
            _erroneous_bounds => bounds_err!(Range, Unbounded, Included, _erroneous_bounds),
        }
    }
}

impl<ST, T> ToSql<SqlRange<ST>, Pg> for RangeToInclusive<T>
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let range = (Bound::Unbounded, Bound::Included(&self.end));
        ToSql::<SqlRange<ST>, Pg>::to_sql(&range, out)?;

        Ok(IsNull::No)
    }
}

// `std::ops::RangeTo`

setup_impls! {RangeTo<T>}

impl<T, ST> FromSql<SqlRange<ST>, Pg> for RangeTo<T>
where
    T: FromSql<ST, Pg>,
{
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (lower_bound, upper_bound) = FromSql::from_sql(bytes)?;

        match (lower_bound, upper_bound) {
            (Bound::Unbounded, Bound::Excluded(end)) => Ok(Self { end }),
            _erroneous_bounds => bounds_err!(Range, Unbounded, Included, _erroneous_bounds),
        }
    }
}

impl<ST, T> ToSql<SqlRange<ST>, Pg> for RangeTo<T>
where
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let range = (Bound::Unbounded, Bound::Excluded(&self.end));
        ToSql::<SqlRange<ST>, Pg>::to_sql(&range, out)?;

        Ok(IsNull::No)
    }
}

// Built-in PG ranges

macro_rules! impl_has_sql_type {
    ($target:ty, $oid:expr, $array_oid:expr) => {
        impl HasSqlType<$target> for Pg {
            fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
                PgTypeMetadata {
                    oid: $oid,
                    array_oid: $array_oid,
                }
            }
        }
    };
}

impl_has_sql_type! {Int4range, 3904, 3905}
impl_has_sql_type! {Numrange, 3906, 3907}
impl_has_sql_type! {Tsrange, 3908, 3909}
impl_has_sql_type! {Tstzrange, 3910, 3911}
impl_has_sql_type! {Daterange, 3912, 3913}
impl_has_sql_type! {Int8range, 3926, 3927}
