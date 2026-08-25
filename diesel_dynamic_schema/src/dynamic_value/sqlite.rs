//! Dynamic values for SQLite.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::convert::Infallible;

use diesel::deserialize::{self, FromSql};
use diesel::row::Field;
use diesel::sql_types as st;
use diesel::sqlite::{Sqlite, SqliteType, SqliteValue};

use super::{
    BackendDynamicValue, DynamicDecodeContext, DynamicValue, DynamicValueBackend,
    DynamicValueExtension,
};

#[cfg(not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))]
impl super::private::DynamicValueBackendSeal for Sqlite {}

impl DynamicValueBackend for Sqlite {
    type BackendValue<E> = Infallible;
    type TypeTag = Option<SqliteType>;

    fn dynamic_type_tag(value: SqliteValue<'_, '_, '_>) -> Self::TypeTag {
        value.value_type()
    }

    fn decode_dynamic_value<'a, F, Ext>(
        field: &F,
        context: &DynamicDecodeContext<'_, Self>,
        _extension: &Ext,
    ) -> deserialize::Result<BackendDynamicValue<Self, Ext::Value>>
    where
        F: Field<'a, Self>,
        Ext: DynamicValueExtension<Self>,
    {
        if let Some(declared) = context.declared_sql_type() {
            if declared.is_sql_type::<st::Bool>() {
                return read::<st::Bool, bool, _>(field).map(DynamicValue::Bool);
            }
            if declared.is_sql_type::<st::SmallInt>() {
                return read::<st::SmallInt, i16, _>(field)
                    .map(i64::from)
                    .map(DynamicValue::Integer);
            }
            if declared.is_sql_type::<st::Integer>() {
                return read::<st::Integer, i32, _>(field)
                    .map(i64::from)
                    .map(DynamicValue::Integer);
            }
            if declared.is_sql_type::<st::BigInt>() {
                return read::<st::BigInt, i64, _>(field).map(DynamicValue::Integer);
            }
            if declared.is_sql_type::<st::Float>() {
                return read::<st::Float, f32, _>(field)
                    .map(f64::from)
                    .map(DynamicValue::Float);
            }
            if declared.is_sql_type::<st::Double>() {
                return read::<st::Double, f64, _>(field).map(DynamicValue::Float);
            }
            if declared.is_sql_type::<st::Text>() {
                return read::<st::Text, String, _>(field).map(DynamicValue::Text);
            }
            if declared.is_sql_type::<st::Binary>() {
                return read::<st::Binary, Vec<u8>, _>(field).map(DynamicValue::Bytes);
            }
        }

        let mut value = field.value().ok_or(diesel::result::UnexpectedNullError)?;
        Ok(match value.value_type() {
            None => DynamicValue::Null,
            Some(SqliteType::SmallInt | SqliteType::Integer | SqliteType::Long) => {
                DynamicValue::Integer(value.read_long())
            }
            Some(SqliteType::Float | SqliteType::Double) => {
                DynamicValue::Float(value.read_double())
            }
            Some(SqliteType::Text) => DynamicValue::Text(value.read_text().to_owned()),
            Some(SqliteType::Binary) => DynamicValue::Bytes(value.read_blob().to_owned()),
        })
    }
}

fn read<'a, ST, T, F>(field: &F) -> deserialize::Result<T>
where
    F: Field<'a, Sqlite>,
    T: FromSql<ST, Sqlite>,
{
    T::from_sql(field.value().ok_or(diesel::result::UnexpectedNullError)?)
}
