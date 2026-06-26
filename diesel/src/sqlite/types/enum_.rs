use crate::deserialize::FromSql;
use crate::serialize::{IsNull, ToSql};
use crate::sqlite::{SqliteBindValue, SqliteValue};
use crate::types::enum_::{EnumMapping, EnumVariant, IntMapping};
use core::fmt::Display;

impl<T, ST> EnumMapping<crate::sqlite::Sqlite> for IntMapping<T, ST>
where
    T: ToSql<ST, crate::sqlite::Sqlite> + FromSql<ST, crate::sqlite::Sqlite>,
    for<'a> SqliteBindValue<'a>: From<T>,
    i128: TryInto<T, Error: Display> + TryFrom<T, Error: Display>,
{
    fn map_to_database_value<'b>(
        output: &mut crate::serialize::Output<'b, '_, crate::sqlite::Sqlite>,
        variant: &'static EnumVariant,
    ) -> crate::serialize::Result {
        let v = variant.discriminant.try_into().map_err(|e| {
            alloc::format!(
                "Failed to convert discriminate to {}: {e}",
                core::any::type_name::<T>()
            )
        })?;
        output.set_value(v);
        Ok(IsNull::No)
    }

    fn map_from_database_value(
        raw: SqliteValue<'_, '_, '_>,
        type_name: &'static str,
        variants: &'static [EnumVariant],
    ) -> crate::deserialize::Result<usize> {
        let i = <T as FromSql<ST, crate::sqlite::Sqlite>>::from_sql(raw)?;
        Self::from_discriminant(type_name, variants, i)
    }
}
