//! MySQL and Mariadb shared types

pub(super) mod date_and_time;
mod enum_;
#[cfg(feature = "serde_json")]
mod json;
mod numeric;
mod primitives;

use self::primitives::{decimal_to_integer, f32_to_i64, f64_to_i64, narrow};
use crate::deserialize::{self, FromSql};
use crate::mysql_like::MysqlLikeBackend;
use crate::mysql_like::NumericRepresentation;
use crate::mysql_like::{MysqlType, MysqlValue};
use crate::query_builder::QueryId;
use crate::serialize::{self, IsNull, Output, ToSql};
use crate::sql_types::ops::*;
use crate::sql_types::*;
use crate::sql_types::{self};
use byteorder::{NativeEndian, WriteBytesExt};

impl<DB: MysqlLikeBackend> ToSql<TinyInt, DB> for i8 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_i8(*self).map(|_| IsNull::No).map_err(Into::into)
    }
}

impl<DB: MysqlLikeBackend> FromSql<TinyInt, DB> for i8 {
    fn from_sql(value: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let [byte, ..] = value.as_bytes() else {
            return Err("Received a buffer with an invalid size while trying \
                 to read a Tiny value: Expected at least 1 bytes but got 0"
                .into());
        };
        Ok(i8::from_ne_bytes([*byte]))
    }
}

/// Represents the MySQL unsigned type.
#[derive(Debug, Clone, Copy, Default, SqlType, QueryId)]
pub struct Unsigned<ST: 'static>(ST);

impl<T> Add for Unsigned<T>
where
    T: Add,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

impl<T> Sub for Unsigned<T>
where
    T: Sub,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

impl<T> Mul for Unsigned<T>
where
    T: Mul,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

impl<T> Div for Unsigned<T>
where
    T: Div,
{
    type Rhs = Unsigned<T::Rhs>;
    type Output = Unsigned<T::Output>;
}

impl<DB: MysqlLikeBackend> ToSql<Unsigned<TinyInt>, DB> for u8 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_u8(*self)?;
        Ok(IsNull::No)
    }
}

impl<DB: MysqlLikeBackend> FromSql<Unsigned<TinyInt>, DB> for u8 {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let signed: i16 = FromSql::<SmallInt, DB>::from_sql(bytes)?;
        narrow(signed)
    }
}

impl<DB: MysqlLikeBackend> ToSql<Unsigned<SmallInt>, DB> for u16 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_u16::<NativeEndian>(*self)?;
        Ok(IsNull::No)
    }
}

impl<DB: MysqlLikeBackend> FromSql<Unsigned<SmallInt>, DB> for u16 {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let signed: i32 = FromSql::<Integer, DB>::from_sql(bytes)?;
        narrow(signed)
    }
}

impl<DB: MysqlLikeBackend> ToSql<Unsigned<Integer>, DB> for u32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_u32::<NativeEndian>(*self)?;
        Ok(IsNull::No)
    }
}

impl<DB: MysqlLikeBackend> FromSql<Unsigned<Integer>, DB> for u32 {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        let signed: i64 = FromSql::<BigInt, DB>::from_sql(bytes)?;
        narrow(signed)
    }
}

impl<DB: MysqlLikeBackend> ToSql<Unsigned<BigInt>, DB> for u64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_u64::<NativeEndian>(*self)?;
        Ok(IsNull::No)
    }
}

impl<DB: MysqlLikeBackend> FromSql<Unsigned<BigInt>, DB> for u64 {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        // No signed type covers the whole unsigned range, unlike the narrower widths.
        match value.numeric_value()? {
            NumericRepresentation::Tiny(x) => narrow(x),
            NumericRepresentation::UnsignedTiny(x) => Ok(x.into()),
            NumericRepresentation::Small(x) => narrow(x),
            NumericRepresentation::UnsignedSmall(x) => Ok(x.into()),
            NumericRepresentation::Medium(x) => narrow(x),
            NumericRepresentation::UnsignedMedium(x) => Ok(x.into()),
            NumericRepresentation::Big(x) => narrow(x),
            NumericRepresentation::UnsignedBig(x) => Ok(x),
            NumericRepresentation::Float(x) => narrow(f32_to_i64(x)?),
            NumericRepresentation::Double(x) => narrow(f64_to_i64(x)?),
            NumericRepresentation::Decimal(bytes) => decimal_to_integer(bytes),
        }
    }
}

impl<DB: MysqlLikeBackend> ToSql<Bool, DB> for bool {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        let int_value = i32::from(*self);
        <i32 as ToSql<Integer, DB>>::to_sql(&int_value, &mut out.reborrow())
    }
}

impl<DB: MysqlLikeBackend> FromSql<Bool, DB> for bool {
    fn from_sql(bytes: MysqlValue<'_>) -> deserialize::Result<Self> {
        Ok(bytes.as_bytes().iter().any(|x| *x != 0))
    }
}

impl<DB: MysqlLikeBackend> ToSql<sql_types::SmallInt, DB> for i16 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_i16::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl<DB: MysqlLikeBackend> ToSql<sql_types::Integer, DB> for i32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_i32::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl<DB: MysqlLikeBackend> ToSql<sql_types::BigInt, DB> for i64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_i64::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl<DB: MysqlLikeBackend> ToSql<sql_types::Double, DB> for f64 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_f64::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl<DB: MysqlLikeBackend> ToSql<sql_types::Float, DB> for f32 {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        out.write_f32::<NativeEndian>(*self)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl<DB: MysqlLikeBackend> HasSqlType<Unsigned<TinyInt>> for DB {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedTiny
    }
}

impl<DB: MysqlLikeBackend> HasSqlType<Unsigned<SmallInt>> for DB {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedShort
    }
}

impl<DB: MysqlLikeBackend> HasSqlType<Unsigned<Integer>> for DB {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedLong
    }
}

impl<DB: MysqlLikeBackend> HasSqlType<Unsigned<BigInt>> for DB {
    fn metadata(_lookup: &mut ()) -> MysqlType {
        MysqlType::UnsignedLongLong
    }
}

/// Represents the MySQL datetime type.
///
/// ### [`ToSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
/// - [`time::PrimitiveDateTime`] with `feature = "time"`
/// - [`time::OffsetDateTime`] with `feature = "time"`
///
/// ### [`FromSql`] impls
///
/// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
/// - [`time::PrimitiveDateTime`] with `feature = "time"`
/// - [`time::OffsetDateTime`] with `feature = "time"`
///
/// [`ToSql`]: crate::serialize::ToSql
/// [`FromSql`]: crate::deserialize::FromSql
#[cfg_attr(
    feature = "chrono",
    doc = " [`chrono::NaiveDateTime`]: chrono::naive::NaiveDateTime"
)]
#[cfg_attr(
    not(feature = "chrono"),
    doc = " [`chrono::NaiveDateTime`]: https://docs.rs/chrono/0.4.19/chrono/naive/struct.NaiveDateTime.html"
)]
#[cfg_attr(
    feature = "time",
    doc = " [`time::PrimitiveDateTime`]: time::PrimitiveDateTime"
)]
#[cfg_attr(
    not(feature = "time"),
    doc = " [`time::PrimitiveDateTime`]: https://docs.rs/time/0.3.9/time/struct.PrimitiveDateTime.html"
)]
#[cfg_attr(
    feature = "time",
    doc = " [`time::OffsetDateTime`]: time::OffsetDateTime"
)]
#[cfg_attr(
    not(feature = "time"),
    doc = " [`time::OffsetDateTime`]: https://docs.rs/time/0.3.9/time/struct.OffsetDateTime.html"
)]
#[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
#[diesel(mysql_type(name = "DateTime"))]
#[diesel(mariadb_type(name = "DateTime"))]
pub struct Datetime;

#[cfg(all(test, any(feature = "mysql", feature = "mariadb")))]
mod tests {
    use super::*;

    #[cfg(feature = "mysql")]
    type DB = crate::mysql::Mysql;
    // Not an `any(..)` arm, so that enabling both backends stays a single alias.
    #[cfg(all(feature = "mariadb", not(feature = "mysql")))]
    type DB = crate::mariadb::Mariadb;

    #[diesel_test_helper::test]
    fn empty_tiny_buffer_is_an_error() {
        let empty = MysqlValue::new_internal(&[], MysqlType::Tiny);
        assert!(<i8 as FromSql<TinyInt, DB>>::from_sql(empty).is_err());

        let empty = MysqlValue::new_internal(&[], MysqlType::UnsignedTiny);
        assert!(<u8 as FromSql<Unsigned<TinyInt>, DB>>::from_sql(empty).is_err());
    }

    #[diesel_test_helper::test]
    fn tiny_buffers_keep_their_signedness() {
        let signed = MysqlValue::new_internal(&[0xFF], MysqlType::Tiny);
        assert_eq!(<i8 as FromSql<TinyInt, DB>>::from_sql(signed).unwrap(), -1);

        let unsigned = MysqlValue::new_internal(&[200], MysqlType::UnsignedTiny);
        assert_eq!(
            <u8 as FromSql<Unsigned<TinyInt>, DB>>::from_sql(unsigned).unwrap(),
            200
        );
    }

    // Both used to return those bits reinterpreted under the wrong sign.
    #[diesel_test_helper::test]
    fn signed_value_through_an_unsigned_sql_type_is_an_error() {
        let raw = [0xFF];
        let v = MysqlValue::new_internal(&raw, MysqlType::Tiny);
        assert!(<u8 as FromSql<Unsigned<TinyInt>, DB>>::from_sql(v).is_err());

        let raw = (-1i16).to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::Short);
        assert!(<u16 as FromSql<Unsigned<SmallInt>, DB>>::from_sql(v).is_err());

        let raw = (-1i32).to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::Long);
        assert!(<u32 as FromSql<Unsigned<Integer>, DB>>::from_sql(v).is_err());

        let raw = (-1i64).to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::LongLong);
        assert!(<u64 as FromSql<Unsigned<BigInt>, DB>>::from_sql(v).is_err());
    }

    #[diesel_test_helper::test]
    fn unsigned_bigint_beyond_i64_through_a_signed_sql_type_is_an_error() {
        let raw = u64::MAX.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLongLong);
        assert!(<i64 as FromSql<BigInt, DB>>::from_sql(v).is_err());

        // The same buffer read as what it actually is still decodes.
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLongLong);
        assert_eq!(
            <u64 as FromSql<Unsigned<BigInt>, DB>>::from_sql(v).unwrap(),
            u64::MAX
        );
    }

    #[diesel_test_helper::test]
    fn unsigned_bigint_beyond_i64_arriving_as_a_decimal() {
        let v = MysqlValue::new_internal(b"18446744073709551615", MysqlType::Numeric);
        assert_eq!(
            <u64 as FromSql<Unsigned<BigInt>, DB>>::from_sql(v).unwrap(),
            u64::MAX
        );
    }

    #[diesel_test_helper::test]
    fn unsigned_values_reaching_the_float_readers() {
        let raw = 200u8.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedTiny);
        assert_eq!(<f32 as FromSql<Float, DB>>::from_sql(v).unwrap(), 200.0);

        let raw = 40000u16.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedShort);
        assert_eq!(<f64 as FromSql<Double, DB>>::from_sql(v).unwrap(), 40000.0);

        let raw = u32::MAX.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLong);
        assert_eq!(
            <f64 as FromSql<Double, DB>>::from_sql(v).unwrap(),
            4_294_967_295.0
        );
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLong);
        assert_eq!(
            <f32 as FromSql<Float, DB>>::from_sql(v).unwrap(),
            4_294_967_296.0
        );

        let raw = u64::MAX.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLongLong);
        assert_eq!(
            <f64 as FromSql<Double, DB>>::from_sql(v).unwrap(),
            18_446_744_073_709_551_616.0
        );
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLongLong);
        assert_eq!(
            <f32 as FromSql<Float, DB>>::from_sql(v).unwrap(),
            18_446_744_073_709_551_616.0
        );
    }

    #[cfg(feature = "numeric")]
    #[diesel_test_helper::test]
    fn unsigned_values_reaching_the_decimal_reader() {
        use bigdecimal::BigDecimal;

        let raw = 200u8.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedTiny);
        assert_eq!(
            <BigDecimal as FromSql<Numeric, DB>>::from_sql(v).unwrap(),
            BigDecimal::from(200u8)
        );

        let raw = 40000u16.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedShort);
        assert_eq!(
            <BigDecimal as FromSql<Numeric, DB>>::from_sql(v).unwrap(),
            BigDecimal::from(40000u16)
        );

        let raw = u32::MAX.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLong);
        assert_eq!(
            <BigDecimal as FromSql<Numeric, DB>>::from_sql(v).unwrap(),
            BigDecimal::from(u32::MAX)
        );

        let raw = u64::MAX.to_ne_bytes();
        let v = MysqlValue::new_internal(&raw, MysqlType::UnsignedLongLong);
        assert_eq!(
            <BigDecimal as FromSql<Numeric, DB>>::from_sql(v).unwrap(),
            BigDecimal::from(u64::MAX)
        );
    }
}
