use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::collections::Bound;
use std::error::Error;
use std::io::Write;

use expression::{AsExpression, NonAggregate};
use expression::bound::Bound as SqlBound;
use pg::Pg;
use query_source::Queryable;
use types::*;

// https://github.com/postgres/postgres/blob/113b0045e20d40f726a0a30e33214455e4f1385e/src/include/utils/rangetypes.h#L35-L43
bitflags! {
    struct RangeFlags: u8 {
        const RANGE_EMPTY = 0x01;
        const RANGE_LB_INC = 0x02;
        const RANGE_UB_INC = 0x04;
        const RANGE_LB_INF = 0x08;
        const RANGE_UB_INF = 0x10;
        const RANGE_LB_NULL = 0x20;
        const RANGE_UB_NULL = 0x40;
        const RANGE_CONTAIN_EMPTY = 0x80;
    }
}

impl_query_id!(Range<T>);
impl<T> NotNull for Range<T> {}
impl<T> SingleValue for Range<T> {}
impl<T> NonAggregate for Range<T> {}

impl<T, ST> Queryable<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg> + Queryable<ST, Pg>,
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
{
    type Row = Self;
    fn build(row: Self) -> Self {
        row
    }
}

impl<ST, T> AsExpression<Range<ST>> for (Bound<T>, Bound<T>)
where
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
{
    type Expression = SqlBound<Range<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<'a, ST, T> AsExpression<Range<ST>> for &'a (Bound<T>, Bound<T>)
where
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
{
    type Expression = SqlBound<Range<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<ST, T> AsExpression<Nullable<Range<ST>>> for (Bound<T>, Bound<T>)
where
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
{
    type Expression = SqlBound<Nullable<Range<ST>>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<'a, ST, T> AsExpression<Nullable<Range<ST>>> for &'a (Bound<T>, Bound<T>)
where
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
{
    type Expression = SqlBound<Nullable<Range<ST>>, Self>;

    fn as_expression(self) -> Self::Expression {
        SqlBound::new(self)
    }
}

impl<T, ST> FromSqlRow<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    (Bound<T>, Bound<T>): FromSql<Range<ST>, Pg>,
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
{
    fn build_from_row<R: ::row::Row<Pg>>(row: &mut R) -> Result<Self, Box<Error + Send + Sync>> {
        FromSql::<Range<ST>, Pg>::from_sql(row.take())
    }
}

impl<T, ST> FromSql<Range<ST>, Pg> for (Bound<T>, Bound<T>)
where
    T: FromSql<ST, Pg>,
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
{
    fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error + Send + Sync>> {
        let mut bytes = not_none!(bytes);
        let flags: RangeFlags = RangeFlags::from_bits_truncate(bytes.read_u8()?);
        let mut lower_bound = Bound::Unbounded;
        let mut upper_bound = Bound::Unbounded;

        if !flags.contains(RANGE_LB_INF) {
            let elem_size = bytes.read_i32::<NetworkEndian>()?;
            let (elem_bytes, new_bytes) = bytes.split_at(elem_size as usize);
            bytes = new_bytes;
            let value = T::from_sql(Some(elem_bytes))?;

            lower_bound = if flags.contains(RANGE_LB_INC) {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }

        if !flags.contains(RANGE_UB_INF) {
            let _size = bytes.read_i32::<NetworkEndian>()?;
            let value = T::from_sql(Some(bytes))?;

            upper_bound = if flags.contains(RANGE_UB_INC) {
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
    Pg: HasSqlType<ST> + HasSqlType<Range<ST>>,
    T: ToSql<ST, Pg>,
{
    fn to_sql<W: Write>(
        &self,
        out: &mut ToSqlOutput<W, Pg>,
    ) -> Result<IsNull, Box<Error + Send + Sync>> {
        let mut flags = match self.0 {
            Bound::Included(_) => RANGE_LB_INC,
            Bound::Excluded(_) => RangeFlags::empty(),
            Bound::Unbounded => RANGE_LB_INF,
        };

        flags |= match self.1 {
            Bound::Included(_) => RANGE_UB_INC,
            Bound::Excluded(_) => RangeFlags::empty(),
            Bound::Unbounded => RANGE_UB_INF,
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

primitive_impls!(Int4range -> (pg: (3904, 3905)));
primitive_impls!(Numrange -> (pg: (3906, 3907)));
primitive_impls!(Tsrange -> (pg: (3908, 3909)));
primitive_impls!(Tstzrange -> (pg: (3910, 3911)));
primitive_impls!(Daterange -> (pg: (3912, 3913)));
primitive_impls!(Int8range -> (pg: (3926, 3927)));
