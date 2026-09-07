//! Dynamic values for MySQL-like backends.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;

use diesel::deserialize::{self, FromSql};
use diesel::mysql_like::data_types::MysqlTime;
use diesel::mysql_like::{MysqlLikeBackend, MysqlType, MysqlValue};
use diesel::row::Field;
use diesel::sql_types as st;

use super::{
    BackendDynamicValue, DynamicDecodeContext, DynamicValue, DynamicValueBackend,
    DynamicValueExtension,
};

/// A MySQL-like backend-only dynamic value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MysqlLikeBackendValue {
    /// A decimal as text.
    Numeric(String),
    /// A date value.
    Date(MysqlTime),
    /// A time value.
    Time(MysqlTime),
    /// A datetime value.
    DateTime(MysqlTime),
    /// A timestamp value.
    Timestamp(MysqlTime),
    /// A bit string in wire byte order.
    Bit(Vec<u8>),
    /// A value with an unknown tag.
    Opaque {
        /// The MySQL-like type tag.
        tag: MysqlType,
        /// The raw wire bytes.
        bytes: Vec<u8>,
    },
}

#[cfg(all(
    feature = "mysql",
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
))]
impl super::private::DynamicValueBackendSeal for diesel::mysql::Mysql {}

#[cfg(all(
    feature = "mariadb",
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
))]
impl super::private::DynamicValueBackendSeal for diesel::mariadb::Mariadb {}
#[cfg(feature = "mysql")]
impl DynamicValueBackend for diesel::mysql::Mysql {
    type BackendValue<E> = MysqlLikeBackendValue;
    type TypeTag = MysqlType;

    fn dynamic_type_tag(value: MysqlValue<'_>) -> Self::TypeTag {
        value.value_type()
    }

    fn decode_dynamic_value<'a, F, Ext>(
        field: &F,
        context: &DynamicDecodeContext<'_, Self>,
        extension: &Ext,
    ) -> deserialize::Result<BackendDynamicValue<Self, Ext::Value>>
    where
        F: Field<'a, Self>,
        Ext: DynamicValueExtension<Self>,
    {
        decode_mysql_like::<Self, _, _>(field, context, extension)
    }
}

#[cfg(feature = "mariadb")]
impl DynamicValueBackend for diesel::mariadb::Mariadb {
    type BackendValue<E> = MysqlLikeBackendValue;
    type TypeTag = MysqlType;

    fn dynamic_type_tag(value: MysqlValue<'_>) -> Self::TypeTag {
        value.value_type()
    }

    fn decode_dynamic_value<'a, F, Ext>(
        field: &F,
        context: &DynamicDecodeContext<'_, Self>,
        extension: &Ext,
    ) -> deserialize::Result<BackendDynamicValue<Self, Ext::Value>>
    where
        F: Field<'a, Self>,
        Ext: DynamicValueExtension<Self>,
    {
        decode_mysql_like::<Self, _, _>(field, context, extension)
    }
}

fn decode_mysql_like<'a, DB, F, Ext>(
    field: &F,
    context: &DynamicDecodeContext<'_, DB>,
    _extension: &Ext,
) -> deserialize::Result<DynamicValue<MysqlLikeBackendValue, Ext::Value>>
where
    DB: MysqlLikeBackend + DynamicValueBackend<TypeTag = MysqlType>,
    F: Field<'a, DB>,
    Ext: DynamicValueExtension<DB>,
{
    if let Some(declared) = context.declared_sql_type() {
        if declared.is_sql_type::<st::Bool>() {
            return read::<st::Bool, bool, _, DB>(field).map(DynamicValue::Bool);
        }
        if declared.is_sql_type::<st::TinyInt>() {
            return read::<st::TinyInt, i8, _, DB>(field)
                .map(i64::from)
                .map(DynamicValue::Integer);
        }
        if declared.is_sql_type::<st::Unsigned<st::TinyInt>>() {
            return read::<st::Unsigned<st::TinyInt>, u8, _, DB>(field)
                .map(u64::from)
                .map(DynamicValue::Unsigned);
        }
        if declared.is_sql_type::<st::SmallInt>() {
            return read::<st::SmallInt, i16, _, DB>(field)
                .map(i64::from)
                .map(DynamicValue::Integer);
        }
        if declared.is_sql_type::<st::Unsigned<st::SmallInt>>() {
            return read::<st::Unsigned<st::SmallInt>, u16, _, DB>(field)
                .map(u64::from)
                .map(DynamicValue::Unsigned);
        }
        if declared.is_sql_type::<st::Integer>() {
            return read::<st::Integer, i32, _, DB>(field)
                .map(i64::from)
                .map(DynamicValue::Integer);
        }
        if declared.is_sql_type::<st::Unsigned<st::Integer>>() {
            return read::<st::Unsigned<st::Integer>, u32, _, DB>(field)
                .map(u64::from)
                .map(DynamicValue::Unsigned);
        }
        if declared.is_sql_type::<st::BigInt>() {
            return read::<st::BigInt, i64, _, DB>(field).map(DynamicValue::Integer);
        }
        if declared.is_sql_type::<st::Unsigned<st::BigInt>>() {
            return read::<st::Unsigned<st::BigInt>, u64, _, DB>(field).map(DynamicValue::Unsigned);
        }
        if declared.is_sql_type::<st::Float>() {
            return read::<st::Float, f32, _, DB>(field)
                .map(f64::from)
                .map(DynamicValue::Float);
        }
        if declared.is_sql_type::<st::Double>() {
            return read::<st::Double, f64, _, DB>(field).map(DynamicValue::Float);
        }
        if declared.is_sql_type::<st::Text>() {
            return read::<st::Text, String, _, DB>(field).map(DynamicValue::Text);
        }
        if declared.is_sql_type::<st::Binary>() {
            return read::<st::Binary, Vec<u8>, _, DB>(field).map(DynamicValue::Bytes);
        }
        if declared.is_sql_type::<st::Numeric>() {
            return read::<st::Numeric, MysqlLikeRawDecimal, _, DB>(field)
                .map(|value| DynamicValue::Backend(MysqlLikeBackendValue::Numeric(value.0)));
        }
        if declared.is_sql_type::<st::Date>() {
            return read::<st::Date, MysqlTime, _, DB>(field)
                .map(|value| DynamicValue::Backend(MysqlLikeBackendValue::Date(value)));
        }
        if declared.is_sql_type::<st::Time>() {
            return read::<st::Time, MysqlTime, _, DB>(field)
                .map(|value| DynamicValue::Backend(MysqlLikeBackendValue::Time(value)));
        }
        if declared.is_sql_type::<st::Datetime>() {
            return read::<st::Datetime, MysqlTime, _, DB>(field)
                .map(|value| DynamicValue::Backend(MysqlLikeBackendValue::DateTime(value)));
        }
        if declared.is_sql_type::<st::Timestamp>() {
            return read::<st::Timestamp, MysqlTime, _, DB>(field)
                .map(|value| DynamicValue::Backend(MysqlLikeBackendValue::Timestamp(value)));
        }
    }

    let value = field.value().ok_or(diesel::result::UnexpectedNullError)?;
    Ok(match context.backend_tag() {
        MysqlType::Tiny => DynamicValue::Integer(read::<st::TinyInt, i8, _, DB>(field)?.into()),
        MysqlType::UnsignedTiny => {
            DynamicValue::Unsigned(read::<st::Unsigned<st::TinyInt>, u8, _, DB>(field)?.into())
        }
        MysqlType::Short => DynamicValue::Integer(read::<st::SmallInt, i16, _, DB>(field)?.into()),
        MysqlType::UnsignedShort => {
            DynamicValue::Unsigned(read::<st::Unsigned<st::SmallInt>, u16, _, DB>(field)?.into())
        }
        MysqlType::Long => DynamicValue::Integer(read::<st::Integer, i32, _, DB>(field)?.into()),
        MysqlType::UnsignedLong => {
            DynamicValue::Unsigned(read::<st::Unsigned<st::Integer>, u32, _, DB>(field)?.into())
        }
        MysqlType::LongLong => DynamicValue::Integer(read::<st::BigInt, i64, _, DB>(field)?),
        MysqlType::UnsignedLongLong => {
            DynamicValue::Unsigned(read::<st::Unsigned<st::BigInt>, u64, _, DB>(field)?)
        }
        MysqlType::Float => DynamicValue::Float(read::<st::Float, f32, _, DB>(field)?.into()),
        MysqlType::Double => DynamicValue::Float(read::<st::Double, f64, _, DB>(field)?),
        MysqlType::String => DynamicValue::Text(read::<st::Text, String, _, DB>(field)?),
        MysqlType::Blob => DynamicValue::Bytes(read::<st::Binary, Vec<u8>, _, DB>(field)?),
        MysqlType::Set | MysqlType::Enum => {
            DynamicValue::Text(read::<st::Text, String, _, DB>(field)?)
        }
        MysqlType::Numeric => {
            let value = read::<st::Numeric, MysqlLikeRawDecimal, _, DB>(field)?;
            DynamicValue::Backend(MysqlLikeBackendValue::Numeric(value.0))
        }
        MysqlType::Date => DynamicValue::Backend(MysqlLikeBackendValue::Date(read::<
            st::Date,
            MysqlTime,
            _,
            DB,
        >(field)?)),
        MysqlType::Time => DynamicValue::Backend(MysqlLikeBackendValue::Time(read::<
            st::Time,
            MysqlTime,
            _,
            DB,
        >(field)?)),
        MysqlType::DateTime => {
            DynamicValue::Backend(MysqlLikeBackendValue::DateTime(read::<
                st::Datetime,
                MysqlTime,
                _,
                DB,
            >(field)?))
        }
        MysqlType::Timestamp => {
            DynamicValue::Backend(MysqlLikeBackendValue::Timestamp(read::<
                st::Timestamp,
                MysqlTime,
                _,
                DB,
            >(field)?))
        }
        MysqlType::Bit => {
            DynamicValue::Backend(MysqlLikeBackendValue::Bit(value.as_bytes().to_vec()))
        }
        tag => DynamicValue::Backend(MysqlLikeBackendValue::Opaque {
            tag,
            bytes: value.as_bytes().to_owned(),
        }),
    })
}

/// A `DECIMAL` as the digits MySQL sent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MysqlLikeRawDecimal(pub String);

impl<DB: MysqlLikeBackend> FromSql<st::Numeric, DB> for MysqlLikeRawDecimal {
    fn from_sql(value: MysqlValue<'_>) -> deserialize::Result<Self> {
        Ok(Self(String::from_utf8(value.as_bytes().to_vec())?))
    }
}

fn read<'a, ST, T, F, DB>(field: &F) -> deserialize::Result<T>
where
    DB: MysqlLikeBackend,
    F: Field<'a, DB>,
    T: FromSql<ST, DB>,
{
    T::from_sql(field.value().ok_or(diesel::result::UnexpectedNullError)?)
}
