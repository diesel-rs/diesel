use crate::sql_types::EnumSqlType;
use crate::types::enum_::{EnumMapping, EnumTypeMapping, EnumVariant, IntMapping};
use std::io::Write;

impl EnumMapping<crate::mysql::Mysql> for EnumTypeMapping {
    fn map_to_database_value<'b>(
        output: &mut crate::serialize::Output<'b, '_, crate::mysql::Mysql>,
        variant: &'static EnumVariant,
    ) -> crate::serialize::Result {
        output.write_all(variant.sql_name.as_bytes())?;
        Ok(crate::serialize::IsNull::No)
    }

    fn map_from_database_value(
        raw: <crate::mysql::Mysql as crate::backend::Backend>::RawValue<'_>,
        type_name: &'static str,
        variants: &'static [EnumVariant],
    ) -> crate::deserialize::Result<usize> {
        let name = raw.as_bytes();
        variants
            .iter()
            .position(|v| v.sql_name.as_bytes() == name)
            .ok_or_else(|| {
                format!(
                    "Invalid enum variant `{}` for `{type_name}`. Allowed variants are {}",
                    String::from_utf8_lossy(name),
                    variants
                        .iter()
                        .map(|v| format!("`{}`", v.sql_name))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .into()
            })
    }
}

impl EnumSqlType<true, crate::mysql::Mysql>
    for crate::sql_types::Unsigned<crate::sql_types::BigInt>
{
    type Strategy = IntMapping<u64, Self>;
}

impl EnumSqlType<true, crate::mysql::Mysql>
    for crate::sql_types::Unsigned<crate::sql_types::Integer>
{
    type Strategy = IntMapping<u32, Self>;
}

impl EnumSqlType<true, crate::mysql::Mysql>
    for crate::sql_types::Unsigned<crate::sql_types::SmallInt>
{
    type Strategy = IntMapping<u16, Self>;
}

impl EnumSqlType<true, crate::mysql::Mysql>
    for crate::sql_types::Unsigned<crate::sql_types::TinyInt>
{
    type Strategy = IntMapping<u8, Self>;
}
